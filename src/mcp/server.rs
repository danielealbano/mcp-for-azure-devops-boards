use crate::azure::client::AzureDevOpsClient;
use crate::azure::{boards, iterations, organizations, projects, tags, work_items};
use crate::compact_llm;
use once_cell::sync::Lazy;
use regex::Regex;
use rmcp::{
    ErrorData as McpError,
    handler::server::router::tool::ToolRouter,
    handler::server::wrapper::Parameters,
    model::{CallToolResult, Content, ErrorCode, Implementation, ServerCapabilities, ServerInfo},
    schemars,
    schemars::JsonSchema,
    serde::Deserialize,
    tool, tool_handler, tool_router,
};
use serde_json::Value;
use std::sync::Arc;

// Static regex patterns for text cleaning (compiled once, reused many times)
static RE_SPACES: Lazy<Regex> = Lazy::new(|| Regex::new(r" +").unwrap());
static RE_NEWLINES: Lazy<Regex> = Lazy::new(|| Regex::new(r"\n+").unwrap());
static RE_LEADING_WS: Lazy<Regex> = Lazy::new(|| Regex::new(r"\n[ ]+").unwrap());
static RE_TRAILING_WS: Lazy<Regex> = Lazy::new(|| Regex::new(r"[ ]+\n").unwrap());
static RE_DASHES: Lazy<Regex> = Lazy::new(|| Regex::new(r"-{3,}\n").unwrap());
static RE_IMAGE: Lazy<Regex> = Lazy::new(|| Regex::new(r"(?i)\[image\]").unwrap());

/// Custom deserializer for non-empty strings
fn deserialize_non_empty_string<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;
    if s.trim().is_empty() {
        return Err(serde::de::Error::custom("field cannot be empty"));
    }
    Ok(s.trim().to_string())
}

/// Recursively simplifies the JSON output to reduce token usage for LLMs.
/// It removes "_links", "url", "descriptor", "imageUrl", "avatar" and simplifies field names.
/// It also flattens the "fields" object to the root level and removes redundant properties.
fn simplify_work_item_json(value: &mut Value) {
    match value {
        Value::Object(map) => {
            // Remove unnecessary fields at the top level and in nested objects
            map.remove("url");
            map.remove("_links");
            map.remove("descriptor");
            map.remove("imageUrl");
            map.remove("avatar");

            // Process "fields" if present (specific to Work Items)
            if let Some(Value::Object(mut fields_map)) = map.remove("fields") {
                let mut simplified_fields = serde_json::Map::new();

                // Collect keys to process
                let keys: Vec<String> = fields_map.keys().cloned().collect();

                for key in keys {
                    if let Some(mut val) = fields_map.remove(&key) {
                        // Simplify Identity fields (objects with displayName, uniqueName, etc.)
                        if let Value::Object(ref obj) = val
                            && let Some(Value::String(name)) = obj.get("displayName")
                        {
                            let mut display_value = name.clone();
                            if let Some(Value::String(unique_name)) = obj.get("uniqueName")
                                && !unique_name.is_empty()
                            {
                                display_value = format!("{} <{}>", name, unique_name);
                            }
                            val = Value::String(display_value);
                        }

                        // Simplify field names and filter out unwanted fields
                        let new_key = if key.starts_with("System.") {
                            key.strip_prefix("System.").unwrap().to_string()
                        } else if key.starts_with("Microsoft.VSTS.Common.") {
                            key.strip_prefix("Microsoft.VSTS.Common.")
                                .unwrap()
                                .to_string()
                        } else if key.starts_with("Microsoft.VSTS.Scheduling.") {
                            key.strip_prefix("Microsoft.VSTS.Scheduling.")
                                .unwrap()
                                .to_string()
                        } else if key.starts_with("Microsoft.VSTS.CMMI.") {
                            key.strip_prefix("Microsoft.VSTS.CMMI.")
                                .unwrap()
                                .to_string()
                        } else if key.contains("_Kanban.Column") {
                            // Handle dynamic WEF_..._Kanban.Column -> Column
                            "Column".to_string()
                        } else if key.contains("_Kanban.Lane") {
                            // Handle dynamic WEF_..._Kanban.Lane -> Lane
                            "Lane".to_string()
                        } else {
                            key
                        };

                        // Skip unwanted fields
                        if matches!(
                            new_key.as_str(),
                            "ActivatedBy"
                                | "ActivatedDate"
                                | "BoardColumnDone"
                                | "ClosedBy"
                                | "ClosedDate"
                                | "Column.Done"
                                | "CommentCount"
                                | "Reason"
                                | "ResolvedBy"
                                | "ResolvedDate"
                                | "State"
                                | "StateChangeDate"
                        ) {
                            continue;
                        }

                        // Rename BoardColumn to Column and BoardLane to Lane
                        let final_key = match new_key.as_str() {
                            "BoardColumn" => "Column".to_string(),
                            "BoardLane" => "Lane".to_string(),
                            "AcceptanceCriteria" => "Acceptance".to_string(),
                            "TeamProject" => "Project".to_string(),
                            "WorkItemType" => "Type".to_string(),
                            "IterationPath" => "Iteration".to_string(),
                            _ => new_key,
                        };

                        // Convert HTML to text for specific fields
                        if matches!(
                            final_key.as_str(),
                            "Acceptance" | "Description" | "Justification"
                        ) {
                            if let Value::String(html_content) = &val {
                                // Convert HTML to plain text, width doesn't matter as we don't need wrapping
                                if let Ok(mut plain_text) =
                                    html2text::from_read(html_content.as_bytes(), usize::MAX)
                                {
                                    // Normalize newlines: replace \r with \n
                                    plain_text = plain_text.replace('\r', "\n");

                                    // Normalize tabulations: replace \t with 1 space
                                    plain_text = plain_text.replace('\t', " ");

                                    // Normalize emdashes: replace ─ with -
                                    plain_text = plain_text.replace('─', "-");

                                    // Remove multiple consecutive spaces
                                    plain_text =
                                        RE_SPACES.replace_all(&plain_text, " ").to_string();

                                    // Collapse multiple consecutive newlines into single newlines
                                    plain_text =
                                        RE_NEWLINES.replace_all(&plain_text, "\n").to_string();

                                    // Remove leading whitespace before newlines (spaces, tabs, etc.)
                                    plain_text =
                                        RE_LEADING_WS.replace_all(&plain_text, "\n").to_string();

                                    // Remove trailing whitespace before newlines (spaces, tabs, etc.)
                                    plain_text =
                                        RE_TRAILING_WS.replace_all(&plain_text, "\n").to_string();

                                    // Collapse 3+ dashes followed by newline to just 3 dashes + newline
                                    plain_text =
                                        RE_DASHES.replace_all(&plain_text, "---\n").to_string();

                                    // Remove [Image] strings (case insensitive)
                                    plain_text = RE_IMAGE.replace_all(&plain_text, "").to_string();

                                    val = Value::String(plain_text.trim().to_string());
                                }
                            }
                        }

                        // Optimize Tags field by removing spaces after semicolons
                        if final_key == "Tags" {
                            if let Value::String(tags) = &val {
                                val = Value::String(tags.replace("; ", ";"));
                            }
                        }

                        // Abbreviate Type field to just first letter
                        if final_key == "Type" {
                            if let Value::String(type_val) = &val {
                                if let Some(first_char) = type_val.chars().next() {
                                    val = Value::String(first_char.to_string());
                                }
                            }
                        }

                        // Only insert if not already present (prefer existing values)
                        if !simplified_fields.contains_key(&final_key) {
                            simplified_fields.insert(final_key, val);
                        }
                    }
                }

                // Flatten: move all simplified fields to the root level
                for (k, v) in simplified_fields {
                    map.insert(k, v);
                }
            }

            // Recursively process all remaining values
            for (_, v) in map.iter_mut() {
                simplify_work_item_json(v);
            }
        }
        Value::Array(arr) => {
            // Recursively process all array elements
            for item in arr.iter_mut() {
                simplify_work_item_json(item);
            }
        }
        _ => {}
    }
}

/// Converts work items JSON to CSV format with dynamic column detection.
/// Only includes columns that have at least one non-null value across all items.
fn work_items_to_csv(json_value: &Value) -> Result<String, String> {
    // Define all possible fields in preferred order
    let all_fields = vec![
        "id",
        "Type",
        "Title",
        "Description",
        "Acceptance",
        "Column",
        "Lane",
        "Priority",
        "AssignedTo",
        "CreatedBy",
        "CreatedDate",
        "ChangedBy",
        "ChangedDate",
        "AreaPath",
        "Iteration",
        "Project",
        "Tags",
        "StartDate",
        "TargetDate",
        "Effort",
        "Risk",
        "Justification",
        "ValueArea",
        "StackRank",
        "History",
        "comments",
    ];

    // Normalize input to array
    let items = match json_value {
        Value::Array(arr) => arr.as_slice(),
        Value::Object(_) => std::slice::from_ref(json_value),
        _ => return Err("Invalid input: expected object or array".to_string()),
    };

    if items.is_empty() {
        return Ok(String::new());
    }

    // Detect which fields actually have values
    let mut active_fields = Vec::new();
    for field in &all_fields {
        let has_value = items.iter().any(|item| {
            item.get(field)
                .map(|v| !v.is_null() && v.as_str().map_or(true, |s| !s.is_empty()))
                .unwrap_or(false)
        });
        if has_value {
            active_fields.push(*field);
        }
    }

    // Build CSV
    let mut wtr = csv::Writer::from_writer(vec![]);

    // Write header
    wtr.write_record(&active_fields)
        .map_err(|e| format!("Failed to write CSV header: {}", e))?;

    // Write rows
    for item in items {
        let row: Vec<String> = active_fields
            .iter()
            .map(|field| {
                item.get(*field)
                    .and_then(|v| match v {
                        Value::String(s) => {
                            // Escape newlines and tabs for better LLM consumption
                            let escaped = s
                                .replace('\n', "\\n")
                                .replace('\t', "\\t")
                                .replace('\r', ""); // Remove carriage returns entirely
                            Some(escaped)
                        }
                        Value::Number(n) => Some(n.to_string()),
                        Value::Bool(b) => Some(b.to_string()),
                        Value::Array(_) if *field == "comments" => {
                            // Serialize comments array as compact JSON using compact_llm
                            compact_llm::to_compact_string(v).ok()
                        }
                        _ => None,
                    })
                    .unwrap_or_default()
            })
            .collect();

        wtr.write_record(&row)
            .map_err(|e| format!("Failed to write CSV row: {}", e))?;
    }

    wtr.flush()
        .map_err(|e| format!("Failed to flush CSV writer: {}", e))?;

    let csv_bytes = wtr
        .into_inner()
        .map_err(|e| format!("Failed to get CSV bytes: {}", e))?;

    String::from_utf8(csv_bytes).map_err(|e| format!("Failed to convert CSV to string: {}", e))
}

#[derive(Clone)]
pub struct AzureMcpServer {
    client: Arc<AzureDevOpsClient>,
    tool_router: ToolRouter<Self>,
}

#[derive(Deserialize, JsonSchema)]
struct GetBoardArgs {
    /// AzDO org name
    #[serde(deserialize_with = "deserialize_non_empty_string")]
    organization: String,
    /// AzDO project name
    #[serde(deserialize_with = "deserialize_non_empty_string")]
    project: String,
    /// Team ID or name
    team_id: String,
    /// Board ID or name
    board_id: String,
}

#[derive(Deserialize, JsonSchema)]
struct ListBoardsArgs {
    /// AzDO org name
    #[serde(deserialize_with = "deserialize_non_empty_string")]
    organization: String,
    /// AzDO project name
    #[serde(deserialize_with = "deserialize_non_empty_string")]
    project: String,
    /// Team ID or name
    team_id: String,
}

#[derive(Deserialize, JsonSchema)]
struct ListTeamsArgs {
    /// AzDO org name
    #[serde(deserialize_with = "deserialize_non_empty_string")]
    organization: String,
    /// AzDO project name
    #[serde(deserialize_with = "deserialize_non_empty_string")]
    project: String,
}

#[derive(Deserialize, JsonSchema)]
struct ListOrganizationsArgs {
    // No parameters needed
}

#[derive(Deserialize, JsonSchema)]
struct ListProjectsArgs {
    /// AzDO org name
    #[serde(deserialize_with = "deserialize_non_empty_string")]
    organization: String,
}

#[derive(Deserialize, JsonSchema)]
struct ListWorkItemTypesArgs {
    /// AzDO org name
    #[serde(deserialize_with = "deserialize_non_empty_string")]
    organization: String,
    /// AzDO project name
    #[serde(deserialize_with = "deserialize_non_empty_string")]
    project: String,
}

#[derive(Deserialize, JsonSchema)]
struct ListTagsArgs {
    /// AzDO org name
    #[serde(deserialize_with = "deserialize_non_empty_string")]
    organization: String,
    /// AzDO project name
    #[serde(deserialize_with = "deserialize_non_empty_string")]
    project: String,
}

#[derive(Deserialize, JsonSchema)]
struct GetTeamArgs {
    /// AzDO org name
    #[serde(deserialize_with = "deserialize_non_empty_string")]
    organization: String,
    /// AzDO project name
    #[serde(deserialize_with = "deserialize_non_empty_string")]
    project: String,
    /// Team ID or name
    team_id: String,
}

#[derive(Deserialize, JsonSchema)]
struct GetTeamCurrentIterationArgs {
    /// AzDO org
    #[serde(deserialize_with = "deserialize_non_empty_string")]
    organization: String,
    /// AzDO project
    #[serde(deserialize_with = "deserialize_non_empty_string")]
    project: String,
    /// Team ID or name
    team_id: String,
}

#[derive(Deserialize, JsonSchema)]
struct GetTeamIterationsArgs {
    /// AzDO org
    #[serde(deserialize_with = "deserialize_non_empty_string")]
    organization: String,
    /// AzDO project
    #[serde(deserialize_with = "deserialize_non_empty_string")]
    project: String,
    /// Team ID or name
    team_id: String,
}

#[derive(Deserialize, JsonSchema)]
struct ListTeamMembersArgs {
    /// AzDO org name
    #[serde(deserialize_with = "deserialize_non_empty_string")]
    organization: String,
    /// AzDO project name
    #[serde(deserialize_with = "deserialize_non_empty_string")]
    project: String,
    /// Team ID or name
    team_id: String,
}

#[derive(Deserialize, JsonSchema)]
struct GetCurrentUserArgs {
    // No parameters needed
}

#[derive(Deserialize, JsonSchema)]
struct GetWorkItemArgs {
    /// AzDO org name
    #[serde(deserialize_with = "deserialize_non_empty_string")]
    organization: String,
    /// AzDO project name
    #[serde(deserialize_with = "deserialize_non_empty_string")]
    project: String,
    /// Work item ID
    id: i64,
    /// Include the latest N comments (optional). Set to -1 for all comments.
    #[serde(default)]
    include_latest_n_comments: Option<i32>,
}

#[derive(Deserialize, JsonSchema)]
struct GetWorkItemsArgs {
    /// AzDO org name
    #[serde(deserialize_with = "deserialize_non_empty_string")]
    organization: String,
    /// AzDO project name
    #[serde(deserialize_with = "deserialize_non_empty_string")]
    project: String,
    /// Work item IDs (comma-separated or array)
    ids: Vec<i64>,
    /// Include the latest N comments (optional). Set to -1 for all comments.
    #[serde(default)]
    include_latest_n_comments: Option<i32>,
}

#[derive(Deserialize, JsonSchema)]
struct QueryWorkItemsArgsWiql {
    /// AzDO org name
    #[serde(deserialize_with = "deserialize_non_empty_string")]
    organization: String,
    /// AzDO project name
    #[serde(deserialize_with = "deserialize_non_empty_string")]
    project: String,
    /// WIQL query string (e.g., "SELECT [System.Id] FROM WorkItems WHERE [System.State] = 'Active'")
    query: String,
    /// Include the latest N comments (optional). Set to -1 for all comments.
    #[serde(default)]
    include_latest_n_comments: Option<i32>,
}

#[derive(Deserialize, JsonSchema)]
struct CreateWorkItemArgs {
    /// AzDO org name
    #[serde(deserialize_with = "deserialize_non_empty_string")]
    organization: String,
    /// AzDO project name
    #[serde(deserialize_with = "deserialize_non_empty_string")]
    project: String,

    // Required fields
    /// Type of work item (User Story, Epic, Feature, etc.)
    work_item_type: String,

    /// Work item title
    title: String,

    // Core optional fields
    /// Work item description (Basic HTML supported)
    #[serde(default)]
    description: Option<String>,

    /// User to assign the work item to (email or display name)
    #[serde(default)]
    assigned_to: Option<String>,

    /// Area path (e.g., "MyProject\\Team1")
    #[serde(default)]
    area_path: Option<String>,

    /// Iteration path (e.g., "MyProject\\Sprint 1"), use azdo_get_team_current_iteration to get the current iteration
    #[serde(default)]
    iteration_path: Option<String>,

    /// Initial state (New, Active, Resolved, etc.)
    #[serde(default)]
    state: Option<String>,

    // Board placement
    /// Board column to place the work item in
    #[serde(default)]
    board_column: Option<String>,

    /// Board row/swimlane to place the work item in
    #[serde(default)]
    board_row: Option<String>,

    // Priority and severity
    /// Priority (1-4, where 1 is highest)
    #[serde(default)]
    priority: Option<u32>,

    /// Severity for bugs (Critical, High, Medium, Low)
    #[serde(default)]
    severity: Option<String>,

    // Effort and planning
    /// Story points for estimation
    #[serde(default)]
    story_points: Option<f64>,

    /// Effort estimate in hours
    #[serde(default)]
    effort: Option<f64>,

    /// Remaining work in hours
    #[serde(default)]
    remaining_work: Option<f64>,

    // Categorization
    /// Comma-separated tags
    #[serde(default)]
    tags: Option<String>,

    /// Activity type (Development, Testing, Documentation, etc.)
    #[serde(default)]
    activity: Option<String>,

    // Relationships
    /// ID of parent work item
    #[serde(default)]
    parent_id: Option<u32>,

    // Dates
    /// Start date (YYYY-MM-DD)
    #[serde(default)]
    start_date: Option<String>,

    /// Target/due date (YYYY-MM-DD)
    #[serde(default)]
    target_date: Option<String>,

    // Additional context
    /// Acceptance criteria
    #[serde(default)]
    acceptance_criteria: Option<String>,

    /// Reproduction steps
    #[serde(default)]
    repro_steps: Option<String>,

    /// Optional extra fields as JSON string (for custom fields)
    #[serde(default)]
    fields: Option<String>,
}

#[derive(Deserialize, JsonSchema)]
struct UpdateWorkItemArgs {
    /// AzDO org name
    #[serde(deserialize_with = "deserialize_non_empty_string")]
    organization: String,
    /// AzDO project name
    #[serde(deserialize_with = "deserialize_non_empty_string")]
    project: String,
    /// Work item ID to update
    id: u32,

    /// Work item title
    #[serde(default)]
    title: Option<String>,

    /// Work item description (Basic HTML supported)
    #[serde(default)]
    description: Option<String>,

    /// User to assign the work item to (email or display name)
    #[serde(default)]
    assigned_to: Option<String>,

    /// Area path (e.g., "MyProject\\Team1")
    #[serde(default)]
    area_path: Option<String>,

    /// Iteration path (e.g., "MyProject\\Sprint 1")
    #[serde(default)]
    iteration_path: Option<String>,

    /// State (New, Active, Resolved, Closed, etc.)
    #[serde(default)]
    state: Option<String>,

    /// Board column to place the work item in
    #[serde(default)]
    board_column: Option<String>,

    /// Board row/swimlane to place the work item in
    #[serde(default)]
    board_row: Option<String>,

    /// Priority (1-4, where 1 is highest)
    #[serde(default)]
    priority: Option<u32>,

    /// Severity for bugs (Critical, High, Medium, Low)
    #[serde(default)]
    severity: Option<String>,

    /// Story points for estimation
    #[serde(default)]
    story_points: Option<f64>,

    /// Effort estimate in hours
    #[serde(default)]
    effort: Option<f64>,

    /// Remaining work in hours
    #[serde(default)]
    remaining_work: Option<f64>,

    /// Comma-separated tags (e.g., "bug, critical, ui")
    #[serde(default)]
    tags: Option<String>,

    /// Activity type (Development, Testing, Documentation, etc.)
    #[serde(default)]
    activity: Option<String>,

    /// Start date (YYYY-MM-DD)
    #[serde(default)]
    start_date: Option<String>,

    /// Target/due date (YYYY-MM-DD)
    #[serde(default)]
    target_date: Option<String>,

    /// Acceptance criteria (for user stories)
    #[serde(default)]
    acceptance_criteria: Option<String>,

    /// Reproduction steps (for bugs)
    #[serde(default)]
    repro_steps: Option<String>,

    /// Optional extra fields as JSON string (for custom fields)
    #[serde(default)]
    fields: Option<String>,
}

#[derive(Deserialize, JsonSchema)]
struct AddCommentArgs {
    /// AzDO org name
    #[serde(deserialize_with = "deserialize_non_empty_string")]
    organization: String,
    /// AzDO project name
    #[serde(deserialize_with = "deserialize_non_empty_string")]
    project: String,
    /// Work item ID to add comment to
    work_item_id: u32,
    /// Comment text (supports markdown)
    text: String,
}

#[derive(Deserialize, JsonSchema)]
struct LinkWorkItemsArgs {
    /// AzDO org name
    #[serde(deserialize_with = "deserialize_non_empty_string")]
    organization: String,
    /// AzDO project name
    #[serde(deserialize_with = "deserialize_non_empty_string")]
    project: String,
    /// Source work item ID
    source_id: u32,
    /// Target work item ID
    target_id: u32,
    /// Link type: "Parent", "Child", "Related", "Duplicate", "Dependency"
    link_type: String,
}

#[derive(Deserialize, JsonSchema)]
struct QueryWorkItemsArgs {
    /// AzDO org name
    #[serde(deserialize_with = "deserialize_non_empty_string")]
    organization: String,
    /// AzDO project name
    #[serde(deserialize_with = "deserialize_non_empty_string")]
    project: String,

    /// Area path to filter by (e.g., "MyProject\\Team1"). Uses UNDER operator to include child paths.
    #[serde(default)]
    area_path: Option<String>,

    /// Iteration path to filter by (e.g., "MyProject\\Sprint 1"). Uses UNDER operator to include child paths.
    #[serde(default)]
    iteration_path: Option<String>,

    /// Filter by creation date (from). Format: YYYY-MM-DD or YYYY-MM-DDTHH:MM:SSZ
    #[serde(default)]
    created_date_from: Option<String>,

    /// Filter by creation date (to). Format: YYYY-MM-DD or YYYY-MM-DDTHH:MM:SSZ
    #[serde(default)]
    created_date_to: Option<String>,

    /// Filter by modified date (from). Format: YYYY-MM-DD or YYYY-MM-DDTHH:MM:SSZ
    #[serde(default)]
    modified_date_from: Option<String>,

    /// Filter by modified date (to). Format: YYYY-MM-DD or YYYY-MM-DDTHH:MM:SSZ
    #[serde(default)]
    modified_date_to: Option<String>,

    /// Board columns to include (e.g., ["Active", "Resolved"])
    #[serde(default)]
    include_board_column: Vec<String>,

    /// Board rows/swimlanes to include (e.g., ["General", "Scraping Platform"])
    #[serde(default)]
    include_board_row: Vec<String>,

    /// Work item types to include (e.g., ["Bug", "User Story"])
    #[serde(default)]
    include_work_item_type: Vec<String>,

    /// States to include (e.g., ["Active", "Resolved"])
    #[serde(default)]
    include_state: Vec<String>,

    /// Board columns to exclude
    #[serde(default)]
    exclude_board_column: Vec<String>,

    /// Board rows/swimlanes to exclude
    #[serde(default)]
    exclude_board_row: Vec<String>,

    /// Work item types to exclude
    #[serde(default)]
    exclude_work_item_type: Vec<String>,

    /// States to exclude (e.g., ["Closed", "Removed"])
    #[serde(default)]
    exclude_state: Vec<String>,

    /// Assignees to include (e.g., ["John Doe", "jane@example.com"])
    #[serde(default)]
    include_assigned_to: Vec<String>,

    /// Assignees to exclude
    #[serde(default)]
    exclude_assigned_to: Vec<String>,

    /// Tags to include (e.g., ["bug", "critical"])
    #[serde(default)]
    include_tags: Vec<String>,

    /// Tags to exclude (e.g., ["wontfix"])
    #[serde(default)]
    exclude_tags: Vec<String>,

    /// Include the latest N comments (optional). Set to -1 for all comments.
    #[serde(default)]
    include_latest_n_comments: Option<i32>,
}

#[tool_router]
impl AzureMcpServer {
    pub fn new(client: AzureDevOpsClient) -> Self {
        Self {
            client: Arc::new(client),
            tool_router: Self::tool_router(),
        }
    }

    #[tool(description = "List teams in the project")]
    async fn azdo_list_teams(
        &self,
        args: Parameters<ListTeamsArgs>,
    ) -> Result<CallToolResult, McpError> {
        log::info!("Tool invoked: azdo_list_teams");
        let teams = boards::list_teams(&self.client, &args.0.organization, &args.0.project)
            .await
            .map_err(|e| McpError {
                code: ErrorCode(-32000),
                message: e.to_string().into(),
                data: None,
            })?;

        // Extract just the team names for compact response
        let team_names: Vec<String> = teams.into_iter().map(|team| team.name).collect();

        Ok(CallToolResult::success(vec![Content::text(
            compact_llm::to_compact_string(&team_names).unwrap(),
        )]))
    }

    #[tool(description = "List team members")]
    async fn azdo_list_team_members(
        &self,
        args: Parameters<ListTeamMembersArgs>,
    ) -> Result<CallToolResult, McpError> {
        log::info!("Tool invoked: azdo_list_team_members");
        let members = self
            .client
            .list_team_members(&args.0.organization, &args.0.project, &args.0.team_id)
            .await
            .map_err(|e| McpError {
                code: ErrorCode(-32000),
                message: e.to_string().into(),
                data: None,
            })?;

        let mut wtr = csv::WriterBuilder::new()
            .has_headers(false)
            .from_writer(vec![]);

        for member in members {
            wtr.write_record(&[member.identity.display_name, member.identity.unique_name])
                .map_err(|e| McpError {
                    code: ErrorCode(-32000),
                    message: format!("Failed to write CSV: {}", e).into(),
                    data: None,
                })?;
        }

        wtr.flush().map_err(|e| McpError {
            code: ErrorCode(-32000),
            message: format!("Failed to flush CSV: {}", e).into(),
            data: None,
        })?;

        let csv_bytes = wtr.into_inner().map_err(|e| McpError {
            code: ErrorCode(-32000),
            message: format!("Failed to get CSV bytes: {}", e).into(),
            data: None,
        })?;

        let data = String::from_utf8(csv_bytes).map_err(|e| McpError {
            code: ErrorCode(-32000),
            message: format!("Failed to convert CSV to string: {}", e).into(),
            data: None,
        })?;

        Ok(CallToolResult::success(vec![Content::text(data)]))
    }

    #[tool(description = "Get current user profile")]
    async fn azdo_get_current_user(
        &self,
        _args: Parameters<GetCurrentUserArgs>,
    ) -> Result<CallToolResult, McpError> {
        log::info!("Tool invoked: azdo_get_current_user");
        let profile = organizations::get_profile(&self.client)
            .await
            .map_err(|e| McpError {
                code: ErrorCode(-32000),
                message: e.to_string().into(),
                data: None,
            })?;

        let mut wtr = csv::WriterBuilder::new()
            .has_headers(false)
            .from_writer(vec![]);

        wtr.write_record(&[profile.display_name, profile.email_address])
            .map_err(|e| McpError {
                code: ErrorCode(-32000),
                message: format!("Failed to write CSV: {}", e).into(),
                data: None,
            })?;

        wtr.flush().map_err(|e| McpError {
            code: ErrorCode(-32000),
            message: format!("Failed to flush CSV: {}", e).into(),
            data: None,
        })?;

        let csv_bytes = wtr.into_inner().map_err(|e| McpError {
            code: ErrorCode(-32000),
            message: format!("Failed to get CSV bytes: {}", e).into(),
            data: None,
        })?;

        let data = String::from_utf8(csv_bytes).map_err(|e| McpError {
            code: ErrorCode(-32000),
            message: format!("Failed to convert CSV to string: {}", e).into(),
            data: None,
        })?;

        Ok(CallToolResult::success(vec![Content::text(data)]))
    }

    #[tool(description = "List AzDO organizations")]
    async fn azdo_list_organizations(
        &self,
        _args: Parameters<ListOrganizationsArgs>,
    ) -> Result<CallToolResult, McpError> {
        log::info!("Tool invoked: azdo_list_organizations");

        // First, get the user's profile to obtain their member ID
        let profile = organizations::get_profile(&self.client)
            .await
            .map_err(|e| McpError {
                code: ErrorCode(-32000),
                message: format!("Failed to get user profile: {}", e).into(),
                data: None,
            })?;

        // Then, list all organizations for this member ID
        let orgs = organizations::list_organizations(&self.client, &profile.id)
            .await
            .map_err(|e| McpError {
                code: ErrorCode(-32000),
                message: e.to_string().into(),
                data: None,
            })?;

        // Extract just the organization names for compact response
        let org_names: Vec<String> = orgs.into_iter().map(|org| org.account_name).collect();

        Ok(CallToolResult::success(vec![Content::text(
            compact_llm::to_compact_string(&org_names).unwrap(),
        )]))
    }

    #[tool(description = "List projects in an organization")]
    async fn azdo_list_projects(
        &self,
        args: Parameters<ListProjectsArgs>,
    ) -> Result<CallToolResult, McpError> {
        log::info!("Tool invoked: azdo_list_projects");
        let projects = projects::list_projects(&self.client, &args.0.organization)
            .await
            .map_err(|e| McpError {
                code: ErrorCode(-32000),
                message: e.to_string().into(),
                data: None,
            })?;

        // Extract just the project names for compact response
        let project_names: Vec<String> = projects.into_iter().map(|project| project.name).collect();

        Ok(CallToolResult::success(vec![Content::text(
            compact_llm::to_compact_string(&project_names).unwrap(),
        )]))
    }

    #[tool(description = "Get team details")]
    async fn azdo_get_team(
        &self,
        args: Parameters<GetTeamArgs>,
    ) -> Result<CallToolResult, McpError> {
        log::info!("Tool invoked: azdo_get_team(team_id={})", args.0.team_id);
        let team = boards::get_team(
            &self.client,
            &args.0.organization,
            &args.0.project,
            &args.0.team_id,
        )
        .await
        .map_err(|e| McpError {
            code: ErrorCode(-32000),
            message: e.to_string().into(),
            data: None,
        })?;

        Ok(CallToolResult::success(vec![Content::text(
            compact_llm::to_compact_string(&team).unwrap(),
        )]))
    }

    #[tool(description = "List work item types")]
    async fn azdo_list_work_item_types(
        &self,
        args: Parameters<ListWorkItemTypesArgs>,
    ) -> Result<CallToolResult, McpError> {
        log::info!("Tool invoked: azdo_list_work_item_types");
        let types =
            boards::list_work_item_types(&self.client, &args.0.organization, &args.0.project)
                .await
                .map_err(|e| McpError {
                    code: ErrorCode(-32000),
                    message: e.to_string().into(),
                    data: None,
                })?;

        // Extract just the work item type names for compact response
        let type_names: Vec<String> = types.into_iter().map(|wit| wit.name).collect();

        Ok(CallToolResult::success(vec![Content::text(
            compact_llm::to_compact_string(&type_names).unwrap(),
        )]))
    }

    #[tool(description = "List tags")]
    async fn azdo_list_tags(
        &self,
        args: Parameters<ListTagsArgs>,
    ) -> Result<CallToolResult, McpError> {
        log::info!("Tool invoked: azdo_list_tags");
        let tags = tags::list_tags(&self.client, &args.0.organization, &args.0.project)
            .await
            .map_err(|e| McpError {
                code: ErrorCode(-32000),
                message: e.to_string().into(),
                data: None,
            })?;

        // Extract just the tag names for compact response
        let tag_names: Vec<String> = tags.into_iter().map(|tag| tag.name).collect();

        Ok(CallToolResult::success(vec![Content::text(
            compact_llm::to_compact_string(&tag_names).unwrap(),
        )]))
    }

    #[tool(description = "Get current iteration/sprint for team")]
    async fn azdo_get_team_current_iteration(
        &self,
        args: Parameters<GetTeamCurrentIterationArgs>,
    ) -> Result<CallToolResult, McpError> {
        log::info!(
            "Tool invoked: azdo_get_team_current_iteration(team_id={})",
            args.0.team_id
        );

        let iteration = iterations::get_team_current_iteration(
            &self.client,
            &args.0.organization,
            &args.0.project,
            &args.0.team_id,
        )
        .await
        .map_err(|e| McpError {
            code: ErrorCode(-32000),
            message: e.to_string().into(),
            data: None,
        })?;

        // Extract dates without time (just YYYY-MM-DD)
        let start_date = iteration
            .attributes
            .start_date
            .as_ref()
            .and_then(|d| d.split('T').next())
            .unwrap_or("N/A");
        let finish_date = iteration
            .attributes
            .finish_date
            .as_ref()
            .and_then(|d| d.split('T').next())
            .unwrap_or("N/A");

        // Return CSV format: name,start_date,finish_date
        let csv_output = format!("{},{},{}", iteration.name, start_date, finish_date);

        Ok(CallToolResult::success(vec![Content::text(csv_output)]))
    }

    #[tool(description = "Get all iterations/sprints for team")]
    async fn azdo_get_team_iterations(
        &self,
        args: Parameters<GetTeamIterationsArgs>,
    ) -> Result<CallToolResult, McpError> {
        log::info!(
            "Tool invoked: azdo_get_team_iterations(team_id={})",
            args.0.team_id
        );

        let iterations = iterations::get_team_iterations(
            &self.client,
            &args.0.organization,
            &args.0.project,
            &args.0.team_id,
        )
        .await
        .map_err(|e| McpError {
            code: ErrorCode(-32000),
            message: e.to_string().into(),
            data: None,
        })?;

        // Convert to CSV format: name,timeframe,start_date,finish_date
        let mut csv_lines = Vec::new();
        for iteration in iterations {
            let start_date = iteration
                .attributes
                .start_date
                .as_ref()
                .and_then(|d| d.split('T').next())
                .unwrap_or("N/A");
            let finish_date = iteration
                .attributes
                .finish_date
                .as_ref()
                .and_then(|d| d.split('T').next())
                .unwrap_or("N/A");
            let timeframe = iteration.attributes.time_frame.as_deref().unwrap_or("N/A");

            csv_lines.push(format!(
                "{},{},{},{}",
                iteration.name, timeframe, start_date, finish_date
            ));
        }

        let csv_output = csv_lines.join("\n");

        Ok(CallToolResult::success(vec![Content::text(csv_output)]))
    }

    #[tool(description = "List boards")]
    async fn azdo_list_team_boards(
        &self,
        args: Parameters<ListBoardsArgs>,
    ) -> Result<CallToolResult, McpError> {
        log::info!(
            "Tool invoked: azdo_list_team_boards(team_id={})",
            args.0.team_id
        );
        let boards = boards::list_boards(
            &self.client,
            &args.0.organization,
            &args.0.project,
            &args.0.team_id,
        )
        .await
        .map_err(|e| McpError {
            code: ErrorCode(-32000),
            message: e.to_string().into(),
            data: None,
        })?;

        // Extract just the board names for compact response
        let board_names: Vec<String> = boards.into_iter().map(|board| board.name).collect();

        Ok(CallToolResult::success(vec![Content::text(
            compact_llm::to_compact_string(&board_names).unwrap(),
        )]))
    }

    #[tool(description = "Get board details")]
    async fn azdo_get_team_board(
        &self,
        args: Parameters<GetBoardArgs>,
    ) -> Result<CallToolResult, McpError> {
        log::info!(
            "Tool invoked: azdo_get_team_board(team_id={}, board_id={})",
            args.0.team_id,
            args.0.board_id
        );
        let board = boards::get_board(
            &self.client,
            &args.0.organization,
            &args.0.project,
            &args.0.team_id,
            &args.0.board_id,
        )
        .await
        .map_err(|e| McpError {
            code: ErrorCode(-32000),
            message: e.to_string().into(),
            data: None,
        })?;

        Ok(CallToolResult::success(vec![Content::text(
            compact_llm::to_compact_string(&board).unwrap(),
        )]))
    }

    #[tool(description = "Get work item by ID")]
    async fn azdo_get_work_item(
        &self,
        args: Parameters<GetWorkItemArgs>,
    ) -> Result<CallToolResult, McpError> {
        log::info!("Tool invoked: azdo_get_work_item(id={})", args.0.id);
        let work_item = work_items::get_work_item(
            &self.client,
            &args.0.organization,
            &args.0.project,
            args.0.id as u32,
            args.0.include_latest_n_comments,
        )
        .await
        .map_err(|e| McpError {
            code: ErrorCode(-32000),
            message: e.to_string().into(),
            data: None,
        })?;

        // Convert to JSON value, simplify, then convert to CSV
        let mut json_value = serde_json::to_value(&work_item).unwrap();
        simplify_work_item_json(&mut json_value);
        let csv_output = work_items_to_csv(&json_value).map_err(|e| McpError {
            code: ErrorCode(-32000),
            message: format!("Failed to convert to CSV: {}", e).into(),
            data: None,
        })?;

        Ok(CallToolResult::success(vec![Content::text(csv_output)]))
    }

    #[tool(description = "Get multiple work items by IDs")]
    async fn azdo_get_work_items(
        &self,
        args: Parameters<GetWorkItemsArgs>,
    ) -> Result<CallToolResult, McpError> {
        log::info!("Tool invoked: azdo_get_work_items(ids={:?})", args.0.ids);

        if args.0.ids.is_empty() {
            return Ok(CallToolResult::success(vec![Content::text(
                "[]".to_string(),
            )]));
        }

        let ids: Vec<u32> = args.0.ids.iter().map(|&id| id as u32).collect();
        let work_items = work_items::get_work_items(
            &self.client,
            &args.0.organization,
            &args.0.project,
            &ids,
            args.0.include_latest_n_comments,
        )
        .await
        .map_err(|e| McpError {
            code: ErrorCode(-32000),
            message: e.to_string().into(),
            data: None,
        })?;

        // Convert to JSON value, simplify, then convert to CSV
        let mut json_value = serde_json::to_value(&work_items).unwrap();
        simplify_work_item_json(&mut json_value);
        let csv_output = work_items_to_csv(&json_value).map_err(|e| McpError {
            code: ErrorCode(-32000),
            message: format!("Failed to convert to CSV: {}", e).into(),
            data: None,
        })?;

        Ok(CallToolResult::success(vec![Content::text(csv_output)]))
    }

    #[tool(description = "Query work items using WIQL")]
    async fn azdo_query_work_items_by_wiql(
        &self,
        args: Parameters<QueryWorkItemsArgsWiql>,
    ) -> Result<CallToolResult, McpError> {
        log::info!(
            "Tool invoked: azdo_query_work_items_by_wiql(query={})",
            args.0.query
        );
        let items = work_items::query_work_items(
            &self.client,
            &args.0.organization,
            &args.0.project,
            &args.0.query,
            args.0.include_latest_n_comments,
        )
        .await
        .map_err(|e| McpError {
            code: ErrorCode(-32000),
            message: e.to_string().into(),
            data: None,
        })?;

        // Convert to JSON value, simplify, then convert to CSV
        let mut json_value = serde_json::to_value(&items).unwrap();
        simplify_work_item_json(&mut json_value);
        let csv_output = work_items_to_csv(&json_value).map_err(|e| McpError {
            code: ErrorCode(-32000),
            message: format!("Failed to convert to CSV: {}", e).into(),
            data: None,
        })?;

        Ok(CallToolResult::success(vec![Content::text(csv_output)]))
    }

    #[tool(description = "Create work item")]
    async fn azdo_create_work_item(
        &self,
        args: Parameters<CreateWorkItemArgs>,
    ) -> Result<CallToolResult, McpError> {
        log::info!(
            "Tool invoked: azdo_create_work_item(work_item_type={}, title={}, area_path={:?}, iteration_path={:?})",
            args.0.work_item_type,
            args.0.title,
            args.0.area_path,
            args.0.iteration_path,
        );

        // Build the field map
        let mut field_map = serde_json::Map::new();

        // Required fields
        field_map.insert("System.Title".to_string(), serde_json::json!(args.0.title));

        // Core optional fields
        if let Some(desc) = &args.0.description {
            field_map.insert("System.Description".to_string(), serde_json::json!(desc));
        }
        if let Some(assigned_to) = &args.0.assigned_to {
            field_map.insert(
                "System.AssignedTo".to_string(),
                serde_json::json!(assigned_to),
            );
        }
        if let Some(area_path) = &args.0.area_path {
            field_map.insert("System.AreaPath".to_string(), serde_json::json!(area_path));
        }
        if let Some(iteration_path) = &args.0.iteration_path {
            field_map.insert(
                "System.IterationPath".to_string(),
                serde_json::json!(iteration_path),
            );
        }
        if let Some(state) = &args.0.state {
            field_map.insert("System.State".to_string(), serde_json::json!(state));
        }

        // Board placement
        if let Some(board_column) = &args.0.board_column {
            field_map.insert(
                "System.BoardColumn".to_string(),
                serde_json::json!(board_column),
            );
        }
        if let Some(board_row) = &args.0.board_row {
            field_map.insert("System.BoardLane".to_string(), serde_json::json!(board_row));
        }

        // Priority and severity
        if let Some(priority) = args.0.priority {
            field_map.insert(
                "Microsoft.VSTS.Common.Priority".to_string(),
                serde_json::json!(priority),
            );
        }
        if let Some(severity) = &args.0.severity {
            field_map.insert(
                "Microsoft.VSTS.Common.Severity".to_string(),
                serde_json::json!(severity),
            );
        }

        // Effort and planning
        if let Some(story_points) = args.0.story_points {
            field_map.insert(
                "Microsoft.VSTS.Scheduling.StoryPoints".to_string(),
                serde_json::json!(story_points),
            );
        }
        if let Some(effort) = args.0.effort {
            field_map.insert(
                "Microsoft.VSTS.Scheduling.Effort".to_string(),
                serde_json::json!(effort),
            );
        }
        if let Some(remaining_work) = args.0.remaining_work {
            field_map.insert(
                "Microsoft.VSTS.Scheduling.RemainingWork".to_string(),
                serde_json::json!(remaining_work),
            );
        }

        // Categorization
        if let Some(tags) = &args.0.tags {
            field_map.insert("System.Tags".to_string(), serde_json::json!(tags));
        }
        if let Some(activity) = &args.0.activity {
            field_map.insert(
                "Microsoft.VSTS.Common.Activity".to_string(),
                serde_json::json!(activity),
            );
        }

        // Dates
        if let Some(start_date) = &args.0.start_date {
            field_map.insert(
                "Microsoft.VSTS.Scheduling.StartDate".to_string(),
                serde_json::json!(start_date),
            );
        }
        if let Some(target_date) = &args.0.target_date {
            field_map.insert(
                "Microsoft.VSTS.Scheduling.TargetDate".to_string(),
                serde_json::json!(target_date),
            );
        }

        // Additional context
        if let Some(acceptance_criteria) = &args.0.acceptance_criteria {
            field_map.insert(
                "Microsoft.VSTS.Common.AcceptanceCriteria".to_string(),
                serde_json::json!(acceptance_criteria),
            );
        }
        if let Some(repro_steps) = &args.0.repro_steps {
            field_map.insert(
                "Microsoft.VSTS.TCM.ReproSteps".to_string(),
                serde_json::json!(repro_steps),
            );
        }

        // Merge any extra fields supplied as JSON string
        if let Some(extra) = &args.0.fields {
            if let Ok(extra_json) =
                serde_json::from_str::<serde_json::Map<String, serde_json::Value>>(extra)
            {
                for (k, v) in extra_json {
                    field_map.insert(k, v);
                }
            } else {
                return Err(McpError {
                    code: ErrorCode(-32602),
                    message: "Invalid JSON in extra fields".into(),
                    data: None,
                });
            }
        }

        // Convert map to Vec<(&str, Value)>
        let fields_vec: Vec<(&str, serde_json::Value)> = field_map
            .iter()
            .map(|(k, v)| (k.as_str(), v.clone()))
            .collect();

        // Create the work item via Azure API
        let work_item = work_items::create_work_item(
            &self.client,
            &args.0.organization,
            &args.0.project,
            &args.0.work_item_type,
            &fields_vec,
        )
        .await
        .map_err(|e| McpError {
            code: ErrorCode(-32000),
            message: e.to_string().into(),
            data: None,
        })?;

        // If parent_id is provided, create parent-child link
        if let Some(parent_id) = args.0.parent_id {
            log::info!(
                "Creating parent-child link: child={}, parent={}",
                work_item.id,
                parent_id
            );
            work_items::link_work_items(
                &self.client,
                &args.0.organization,
                &args.0.project,
                work_item.id,
                parent_id,
                "System.LinkTypes.Hierarchy-Reverse",
            )
            .await
            .map_err(|e| McpError {
                code: ErrorCode(-32000),
                message: format!("Failed to create parent link: {}", e).into(),
                data: None,
            })?;
        }

        // Convert to JSON value, simplify, then serialize
        let mut json_value = serde_json::to_value(&work_item).unwrap();
        simplify_work_item_json(&mut json_value);

        Ok(CallToolResult::success(vec![Content::text(
            compact_llm::to_compact_string(&json_value).unwrap(),
        )]))
    }

    #[tool(description = "Query work items by filters")]
    async fn azdo_query_work_items(
        &self,
        args: Parameters<QueryWorkItemsArgs>,
    ) -> Result<CallToolResult, McpError> {
        log::info!(
            "Tool invoked: azdo_query_work_items(area_path={:?}, iteration_path={:?}, include_board_column={:?}, exclude_state={:?})",
            args.0.area_path,
            args.0.iteration_path,
            args.0.include_board_column,
            args.0.exclude_state
        );

        // Build WIQL query conditions
        let mut conditions = Vec::new();

        // Area path filter
        if let Some(area_path) = &args.0.area_path {
            conditions.push(format!(
                "[System.AreaPath] UNDER '{}'",
                area_path.replace("'", "''")
            ));
        }

        // Iteration filter
        if let Some(iteration_path) = &args.0.iteration_path {
            conditions.push(format!(
                "[System.IterationPath] UNDER '{}'",
                iteration_path.replace("'", "''")
            ));
        }

        // Date filters
        if let Some(date) = &args.0.created_date_from {
            conditions.push(format!("[System.CreatedDate] >= '{}'", date));
        }
        if let Some(date) = &args.0.created_date_to {
            conditions.push(format!("[System.CreatedDate] <= '{}'", date));
        }
        if let Some(date) = &args.0.modified_date_from {
            conditions.push(format!("[System.ChangedDate] >= '{}'", date));
        }
        if let Some(date) = &args.0.modified_date_to {
            conditions.push(format!("[System.ChangedDate] <= '{}'", date));
        }

        // Include filters (using IN operator)
        if !args.0.include_board_column.is_empty() {
            let values: Vec<String> = args
                .0
                .include_board_column
                .iter()
                .map(|v| format!("'{}'", v.replace("'", "''")))
                .collect();
            conditions.push(format!("[System.BoardColumn] IN ({})", values.join(", ")));
        }

        if !args.0.include_board_row.is_empty() {
            let values: Vec<String> = args
                .0
                .include_board_row
                .iter()
                .map(|v| format!("'{}'", v.replace("'", "''")))
                .collect();
            conditions.push(format!("[System.BoardLane] IN ({})", values.join(", ")));
        }

        if !args.0.include_work_item_type.is_empty() {
            let values: Vec<String> = args
                .0
                .include_work_item_type
                .iter()
                .map(|v| format!("'{}'", v.replace("'", "''")))
                .collect();
            conditions.push(format!("[System.WorkItemType] IN ({})", values.join(", ")));
        }

        if !args.0.include_state.is_empty() {
            let values: Vec<String> = args
                .0
                .include_state
                .iter()
                .map(|v| format!("'{}'", v.replace("'", "''")))
                .collect();
            conditions.push(format!("[System.State] IN ({})", values.join(", ")));
        }

        // Exclude filters (using NOT IN operator)
        if !args.0.exclude_board_column.is_empty() {
            let values: Vec<String> = args
                .0
                .exclude_board_column
                .iter()
                .map(|v| format!("'{}'", v.replace("'", "''")))
                .collect();
            conditions.push(format!(
                "[System.BoardColumn] NOT IN ({})",
                values.join(", ")
            ));
        }

        if !args.0.exclude_board_row.is_empty() {
            let values: Vec<String> = args
                .0
                .exclude_board_row
                .iter()
                .map(|v| format!("'{}'", v.replace("'", "''")))
                .collect();
            conditions.push(format!("[System.BoardLane] NOT IN ({})", values.join(", ")));
        }

        if !args.0.exclude_work_item_type.is_empty() {
            let values: Vec<String> = args
                .0
                .exclude_work_item_type
                .iter()
                .map(|v| format!("'{}'", v.replace("'", "''")))
                .collect();
            conditions.push(format!(
                "[System.WorkItemType] NOT IN ({})",
                values.join(", ")
            ));
        }

        if !args.0.exclude_state.is_empty() {
            let values: Vec<String> = args
                .0
                .exclude_state
                .iter()
                .map(|v| format!("'{}'", v.replace("'", "''")))
                .collect();
            conditions.push(format!("[System.State] NOT IN ({})", values.join(", ")));
        }

        if !args.0.include_assigned_to.is_empty() {
            let values: Vec<String> = args
                .0
                .include_assigned_to
                .iter()
                .map(|v| format!("'{}'", v.replace("'", "''")))
                .collect();
            conditions.push(format!("[System.AssignedTo] IN ({})", values.join(", ")));
        }

        if !args.0.exclude_assigned_to.is_empty() {
            let values: Vec<String> = args
                .0
                .exclude_assigned_to
                .iter()
                .map(|v| format!("'{}'", v.replace("'", "''")))
                .collect();
            conditions.push(format!(
                "[System.AssignedTo] NOT IN ({})",
                values.join(", ")
            ));
        }

        // Tag filters (using CONTAINS operator)
        if !args.0.include_tags.is_empty() {
            for tag in &args.0.include_tags {
                conditions.push(format!(
                    "[System.Tags] CONTAINS '{}'",
                    tag.replace("'", "''")
                ));
            }
        }

        if !args.0.exclude_tags.is_empty() {
            for tag in &args.0.exclude_tags {
                conditions.push(format!(
                    "NOT [System.Tags] CONTAINS '{}'",
                    tag.replace("'", "''")
                ));
            }
        }

        // Build the query
        let query = if conditions.is_empty() {
            // If no filters specified, query all work items in the project
            format!(
                "SELECT [System.Id] FROM WorkItems WHERE [System.TeamProject] = '{}'",
                args.0.project
            )
        } else {
            format!(
                "SELECT [System.Id] FROM WorkItems WHERE {}",
                conditions.join(" AND ")
            )
        };

        log::debug!("Executing WIQL query: {}", query);

        // Execute the query to get work items
        let work_items = work_items::query_work_items(
            &self.client,
            &args.0.organization,
            &args.0.project,
            &query,
            args.0.include_latest_n_comments,
        )
        .await
        .map_err(|e| McpError {
            code: ErrorCode(-32000),
            message: e.to_string().into(),
            data: None,
        })?;

        // Convert to JSON value, simplify, then convert to CSV
        let mut json_value = serde_json::to_value(&work_items).unwrap();
        simplify_work_item_json(&mut json_value);
        let csv_output = work_items_to_csv(&json_value).map_err(|e| McpError {
            code: ErrorCode(-32000),
            message: format!("Failed to convert to CSV: {}", e).into(),
            data: None,
        })?;

        Ok(CallToolResult::success(vec![Content::text(csv_output)]))
    }

    #[tool(description = "Update work item")]
    async fn azdo_update_work_item(
        &self,
        args: Parameters<UpdateWorkItemArgs>,
    ) -> Result<CallToolResult, McpError> {
        log::info!(
            "Tool invoked: azdo_update_work_item(id={}, title={:?}, state={:?})",
            args.0.id,
            args.0.title,
            args.0.state,
        );

        // Build the field map (only include fields that are Some)
        let mut field_map = serde_json::Map::new();

        if let Some(title) = &args.0.title {
            field_map.insert("System.Title".to_string(), serde_json::json!(title));
        }
        if let Some(desc) = &args.0.description {
            field_map.insert("System.Description".to_string(), serde_json::json!(desc));
        }
        if let Some(assigned_to) = &args.0.assigned_to {
            field_map.insert(
                "System.AssignedTo".to_string(),
                serde_json::json!(assigned_to),
            );
        }
        if let Some(area_path) = &args.0.area_path {
            field_map.insert("System.AreaPath".to_string(), serde_json::json!(area_path));
        }
        if let Some(iteration_path) = &args.0.iteration_path {
            field_map.insert(
                "System.IterationPath".to_string(),
                serde_json::json!(iteration_path),
            );
        }
        if let Some(state) = &args.0.state {
            field_map.insert("System.State".to_string(), serde_json::json!(state));
        }
        if let Some(board_column) = &args.0.board_column {
            field_map.insert(
                "System.BoardColumn".to_string(),
                serde_json::json!(board_column),
            );
        }
        if let Some(board_row) = &args.0.board_row {
            field_map.insert("System.BoardLane".to_string(), serde_json::json!(board_row));
        }
        if let Some(priority) = args.0.priority {
            field_map.insert(
                "Microsoft.VSTS.Common.Priority".to_string(),
                serde_json::json!(priority),
            );
        }
        if let Some(severity) = &args.0.severity {
            field_map.insert(
                "Microsoft.VSTS.Common.Severity".to_string(),
                serde_json::json!(severity),
            );
        }
        if let Some(story_points) = args.0.story_points {
            field_map.insert(
                "Microsoft.VSTS.Scheduling.StoryPoints".to_string(),
                serde_json::json!(story_points),
            );
        }
        if let Some(effort) = args.0.effort {
            field_map.insert(
                "Microsoft.VSTS.Scheduling.Effort".to_string(),
                serde_json::json!(effort),
            );
        }
        if let Some(remaining_work) = args.0.remaining_work {
            field_map.insert(
                "Microsoft.VSTS.Scheduling.RemainingWork".to_string(),
                serde_json::json!(remaining_work),
            );
        }
        if let Some(tags) = &args.0.tags {
            field_map.insert("System.Tags".to_string(), serde_json::json!(tags));
        }
        if let Some(activity) = &args.0.activity {
            field_map.insert(
                "Microsoft.VSTS.Common.Activity".to_string(),
                serde_json::json!(activity),
            );
        }
        if let Some(start_date) = &args.0.start_date {
            field_map.insert(
                "Microsoft.VSTS.Scheduling.StartDate".to_string(),
                serde_json::json!(start_date),
            );
        }
        if let Some(target_date) = &args.0.target_date {
            field_map.insert(
                "Microsoft.VSTS.Scheduling.TargetDate".to_string(),
                serde_json::json!(target_date),
            );
        }
        if let Some(acceptance_criteria) = &args.0.acceptance_criteria {
            field_map.insert(
                "Microsoft.VSTS.Common.AcceptanceCriteria".to_string(),
                serde_json::json!(acceptance_criteria),
            );
        }
        if let Some(repro_steps) = &args.0.repro_steps {
            field_map.insert(
                "Microsoft.VSTS.TCM.ReproSteps".to_string(),
                serde_json::json!(repro_steps),
            );
        }

        // Merge any extra fields
        if let Some(extra) = &args.0.fields {
            if let Ok(extra_json) =
                serde_json::from_str::<serde_json::Map<String, serde_json::Value>>(extra)
            {
                for (k, v) in extra_json {
                    field_map.insert(k, v);
                }
            } else {
                return Err(McpError {
                    code: ErrorCode(-32602),
                    message: "Invalid JSON in extra fields".into(),
                    data: None,
                });
            }
        }

        let fields_vec: Vec<(&str, serde_json::Value)> = field_map
            .iter()
            .map(|(k, v)| (k.as_str(), v.clone()))
            .collect();

        let work_item = work_items::update_work_item(
            &self.client,
            &args.0.organization,
            &args.0.project,
            args.0.id,
            &fields_vec,
        )
        .await
        .map_err(|e| McpError {
            code: ErrorCode(-32000),
            message: e.to_string().into(),
            data: None,
        })?;

        // Convert to JSON value, simplify, then serialize
        let mut json_value = serde_json::to_value(&work_item).unwrap();
        simplify_work_item_json(&mut json_value);

        Ok(CallToolResult::success(vec![Content::text(
            compact_llm::to_compact_string(&json_value).unwrap(),
        )]))
    }

    #[tool(description = "Add a comment to a work item")]
    async fn azdo_add_comment(
        &self,
        args: Parameters<AddCommentArgs>,
    ) -> Result<CallToolResult, McpError> {
        log::info!(
            "Tool invoked: azdo_add_comment(work_item_id={}, text_length={})",
            args.0.work_item_id,
            args.0.text.len()
        );

        let result = work_items::add_comment(
            &self.client,
            &args.0.organization,
            &args.0.project,
            args.0.work_item_id,
            &args.0.text,
        )
        .await
        .map_err(|e| McpError {
            code: ErrorCode(-32000),
            message: e.to_string().into(),
            data: None,
        })?;

        Ok(CallToolResult::success(vec![Content::text(
            compact_llm::to_compact_string(&result).unwrap(),
        )]))
    }

    #[tool(description = "Link work items")]
    async fn azdo_link_work_items(
        &self,
        args: Parameters<LinkWorkItemsArgs>,
    ) -> Result<CallToolResult, McpError> {
        log::info!(
            "Tool invoked: azdo_link_work_items(source_id={}, target_id={}, link_type={})",
            args.0.source_id,
            args.0.target_id,
            args.0.link_type
        );

        // Map friendly names to Azure DevOps link type names
        let link_type_ref = match args.0.link_type.to_lowercase().as_str() {
            "parent" => "System.LinkTypes.Hierarchy-Forward",
            "child" => "System.LinkTypes.Hierarchy-Reverse",
            "related" => "System.LinkTypes.Related",
            "duplicate" => "System.LinkTypes.Duplicate-Forward",
            "dependency" => "System.LinkTypes.Dependency-Forward",
            _ => &args.0.link_type, // Use as-is if not a known friendly name
        };

        let result = work_items::link_work_items(
            &self.client,
            &args.0.organization,
            &args.0.project,
            args.0.source_id,
            args.0.target_id,
            link_type_ref,
        )
        .await
        .map_err(|e| McpError {
            code: ErrorCode(-32000),
            message: e.to_string().into(),
            data: None,
        })?;

        Ok(CallToolResult::success(vec![Content::text(
            compact_llm::to_compact_string(&result).unwrap(),
        )]))
    }
}

#[tool_handler]
impl rmcp::ServerHandler for AzureMcpServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            server_info: Implementation {
                name: env!("CARGO_PKG_NAME").into(),
                version: env!("CARGO_PKG_VERSION").into(),
                icons: None,
                title: None,
                website_url: Some(env!("CARGO_PKG_HOMEPAGE").into()),
            },
            instructions: Some(
                "Use this tool to interact with Azure DevOps Boards and Work Items".into(),
            ),
            capabilities: ServerCapabilities::builder().enable_tools().build(),
            ..Default::default()
        }
    }
}
