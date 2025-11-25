use crate::azure::client::AzureDevOpsClient;
use crate::azure::{boards, work_items};
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

/// Recursively simplifies the JSON output to reduce token usage for LLMs.
/// It removes "_links", "url", "descriptor", "imageUrl", "avatar" and simplifies field names.
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
            if let Some(Value::Object(fields_map)) = map.get_mut("fields") {
                let mut new_fields = serde_json::Map::new();

                // Collect keys to remove or rename to avoid borrowing issues
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

                        // Simplify field names
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
                            // Handle dynamic WEF_..._Kanban.Column
                            if key.ends_with(".Done") {
                                "Column.Done".to_string()
                            } else {
                                "Column".to_string()
                            }
                        } else {
                            key
                        };

                        new_fields.insert(new_key, val);
                    }
                }
                *fields_map = new_fields;
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

#[derive(Clone)]
pub struct AzureMcpServer {
    client: Arc<AzureDevOpsClient>,
    tool_router: ToolRouter<Self>,
}

#[derive(Deserialize, JsonSchema)]
struct GetBoardArgs {
    /// Team ID or name
    team_id: String,
    /// Board ID or name
    board_id: String,
}

#[derive(Deserialize, JsonSchema)]
struct ListBoardsArgs {
    /// Team ID or name
    team_id: String,
}

#[derive(Deserialize, JsonSchema)]
struct GetTeamArgs {
    /// Team ID or name
    team_id: String,
}

#[derive(Deserialize, JsonSchema)]
struct GetWorkItemArgs {
    /// Work item ID
    id: i64,
}

#[derive(Deserialize, JsonSchema)]
struct QueryWorkItemsArgs {
    /// WIQL query string (e.g., "SELECT [System.Id] FROM WorkItems WHERE [System.State] = 'Active'")
    query: String,
}

#[derive(Deserialize, JsonSchema)]
struct CreateWorkItemArgs {
    // Required fields
    /// Type of work item (Bug, User Story, Task, Epic, Feature, etc.)
    work_item_type: String,

    /// Work item title
    title: String,

    // Core optional fields
    /// Work item description (HTML supported)
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
    iteration: Option<String>,

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
    /// ID of parent work item (for creating hierarchical relationships)
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
    /// Acceptance criteria (for user stories)
    #[serde(default)]
    acceptance_criteria: Option<String>,

    /// Reproduction steps (for bugs)
    #[serde(default)]
    repro_steps: Option<String>,

    /// Optional extra fields as JSON string (for any additional custom fields)
    #[serde(default)]
    fields: Option<String>,
}

#[derive(Deserialize, JsonSchema)]
struct UpdateWorkItemArgs {
    /// Work item ID to update
    id: u32,

    /// Work item title
    #[serde(default)]
    title: Option<String>,

    /// Work item description (HTML supported)
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
    iteration: Option<String>,

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

    /// Optional extra fields as JSON string (for any additional custom fields)
    #[serde(default)]
    fields: Option<String>,
}

#[derive(Deserialize, JsonSchema)]
struct AddCommentArgs {
    /// Work item ID to add comment to
    work_item_id: u32,
    /// Comment text (supports markdown)
    text: String,
}

#[derive(Deserialize, JsonSchema)]
struct LinkWorkItemsArgs {
    /// Source work item ID
    source_id: u32,
    /// Target work item ID
    target_id: u32,
    /// Link type: "Parent", "Child", "Related", "Duplicate", "Dependency"
    link_type: String,
}

#[derive(Deserialize, JsonSchema)]
struct UploadAttachmentArgs {
    /// File name with extension
    file_name: String,
    /// Base64 encoded file content
    content: String,
}

#[derive(Deserialize, JsonSchema)]
struct DownloadAttachmentArgs {
    /// Attachment ID (GUID)
    id: String,
    /// Optional file name for the downloaded attachment
    file_name: Option<String>,
}

#[derive(Deserialize, JsonSchema)]
struct GetBoardWorkItemsArgs {
    /// Area path to filter by (e.g., "MyProject\\Team1"). Uses UNDER operator to include child paths.
    #[serde(default)]
    area_path: Option<String>,

    /// Iteration path to filter by (e.g., "MyProject\\Sprint 1"). Uses UNDER operator to include child paths.
    #[serde(default)]
    iteration: Option<String>,

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
}

#[tool_router]
impl AzureMcpServer {
    pub fn new(client: AzureDevOpsClient) -> Self {
        Self {
            client: Arc::new(client),
            tool_router: Self::tool_router(),
        }
    }

    #[tool(description = "List all teams in the project")]
    async fn azure_devops_list_teams(&self) -> Result<CallToolResult, McpError> {
        log::info!("Tool invoked: azure_devops_list_teams");
        let teams = boards::list_teams(&self.client)
            .await
            .map_err(|e| McpError {
                code: ErrorCode(-32000),
                message: e.to_string().into(),
                data: None,
            })?;

        Ok(CallToolResult::success(vec![Content::text(
            serde_json::to_string_pretty(&teams).unwrap(),
        )]))
    }

    #[tool(description = "Get details of a specific team")]
    async fn azure_devops_get_team(
        &self,
        args: Parameters<GetTeamArgs>,
    ) -> Result<CallToolResult, McpError> {
        log::info!(
            "Tool invoked: azure_devops_get_team(team_id={})",
            args.0.team_id
        );
        let team = boards::get_team(&self.client, &args.0.team_id)
            .await
            .map_err(|e| McpError {
                code: ErrorCode(-32000),
                message: e.to_string().into(),
                data: None,
            })?;

        Ok(CallToolResult::success(vec![Content::text(
            serde_json::to_string_pretty(&team).unwrap(),
        )]))
    }

    #[tool(description = "List all work item types (Stories, Epics, Features, Bugs, etc.)")]
    async fn azure_devops_list_work_item_types(&self) -> Result<CallToolResult, McpError> {
        log::info!("Tool invoked: azure_devops_list_work_item_types");
        let types = boards::list_work_item_types(&self.client)
            .await
            .map_err(|e| McpError {
                code: ErrorCode(-32000),
                message: e.to_string().into(),
                data: None,
            })?;

        Ok(CallToolResult::success(vec![Content::text(
            serde_json::to_string_pretty(&types).unwrap(),
        )]))
    }

    #[tool(description = "List boards for a specific team (requires team_id)")]
    async fn azure_devops_list_boards(
        &self,
        args: Parameters<ListBoardsArgs>,
    ) -> Result<CallToolResult, McpError> {
        log::info!(
            "Tool invoked: azure_devops_list_boards(team_id={})",
            args.0.team_id
        );
        let boards = boards::list_boards(&self.client, &args.0.team_id)
            .await
            .map_err(|e| McpError {
                code: ErrorCode(-32000),
                message: e.to_string().into(),
                data: None,
            })?;

        Ok(CallToolResult::success(vec![Content::text(
            serde_json::to_string_pretty(&boards).unwrap(),
        )]))
    }

    #[tool(description = "Get details of a specific board (requires team_id)")]
    async fn azure_devops_get_board(
        &self,
        args: Parameters<GetBoardArgs>,
    ) -> Result<CallToolResult, McpError> {
        log::info!(
            "Tool invoked: azure_devops_get_board(team_id={}, board_id={})",
            args.0.team_id,
            args.0.board_id
        );
        let board = boards::get_board(&self.client, &args.0.team_id, &args.0.board_id)
            .await
            .map_err(|e| McpError {
                code: ErrorCode(-32000),
                message: e.to_string().into(),
                data: None,
            })?;

        Ok(CallToolResult::success(vec![Content::text(
            serde_json::to_string_pretty(&board).unwrap(),
        )]))
    }

    #[tool(description = "Get a work item by ID")]
    async fn azure_devops_get_work_item(
        &self,
        args: Parameters<GetWorkItemArgs>,
    ) -> Result<CallToolResult, McpError> {
        log::info!("Tool invoked: azure_devops_get_work_item(id={})", args.0.id);
        let work_item = work_items::get_work_item(&self.client, args.0.id as u32)
            .await
            .map_err(|e| McpError {
                code: ErrorCode(-32000),
                message: e.to_string().into(),
                data: None,
            })?;

        Ok(CallToolResult::success(vec![Content::text(
            serde_json::to_string_pretty(&work_item).unwrap(),
        )]))
    }

    #[tool(description = "Query work items using WIQL (Work Item Query Language)")]
    async fn azure_devops_query_work_items_wiql(
        &self,
        args: Parameters<QueryWorkItemsArgs>,
    ) -> Result<CallToolResult, McpError> {
        log::info!(
            "Tool invoked: azure_devops_query_work_items_wiql(query={})",
            args.0.query
        );
        let items = work_items::query_work_items(&self.client, &args.0.query)
            .await
            .map_err(|e| McpError {
                code: ErrorCode(-32000),
                message: e.to_string().into(),
                data: None,
            })?;

        // Convert to JSON value, simplify, then serialize
        let mut json_value = serde_json::to_value(&items).unwrap();
        simplify_work_item_json(&mut json_value);

        Ok(CallToolResult::success(vec![Content::text(
            serde_json::to_string_pretty(&json_value).unwrap(),
        )]))
    }

    #[tool(
        description = "Create a new work item with comprehensive field support (type, title, description, area path, iteration, priority, tags, parent relationships, etc.)"
    )]
    async fn azure_devops_create_work_item(
        &self,
        args: Parameters<CreateWorkItemArgs>,
    ) -> Result<CallToolResult, McpError> {
        log::info!(
            "Tool invoked: azure_devops_create_work_item(work_item_type={}, title={}, area_path={:?}, iteration={:?})",
            args.0.work_item_type,
            args.0.title,
            args.0.area_path,
            args.0.iteration,
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
        if let Some(iteration) = &args.0.iteration {
            field_map.insert(
                "System.IterationPath".to_string(),
                serde_json::json!(iteration),
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
        let work_item =
            work_items::create_work_item(&self.client, &args.0.work_item_type, &fields_vec)
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

        Ok(CallToolResult::success(vec![Content::text(
            serde_json::to_string_pretty(&work_item).unwrap(),
        )]))
    }

    #[tool(description = "Upload an attachment to Azure DevOps")]
    async fn azure_devops_upload_attachment(
        &self,
        args: Parameters<UploadAttachmentArgs>,
    ) -> Result<CallToolResult, McpError> {
        log::info!(
            "Tool invoked: azure_devops_upload_attachment(file_name={})",
            args.0.file_name
        );
        use base64::{Engine as _, engine::general_purpose};

        let content = general_purpose::STANDARD
            .decode(&args.0.content)
            .map_err(|e| McpError {
                code: ErrorCode(-32602),
                message: format!("Invalid base64 content: {}", e).into(),
                data: None,
            })?;

        let attachment =
            crate::azure::attachments::upload_attachment(&self.client, &args.0.file_name, content)
                .await
                .map_err(|e| McpError {
                    code: ErrorCode(-32000),
                    message: e.to_string().into(),
                    data: None,
                })?;

        Ok(CallToolResult::success(vec![Content::text(
            serde_json::to_string_pretty(&attachment).unwrap(),
        )]))
    }

    #[tool(description = "Download an attachment from Azure DevOps")]
    async fn azure_devops_download_attachment(
        &self,
        args: Parameters<DownloadAttachmentArgs>,
    ) -> Result<CallToolResult, McpError> {
        log::info!(
            "Tool invoked: azure_devops_download_attachment(id={})",
            args.0.id
        );
        use base64::{Engine as _, engine::general_purpose};

        let content = crate::azure::attachments::download_attachment(
            &self.client,
            &args.0.id,
            args.0.file_name.as_deref(),
        )
        .await
        .map_err(|e| McpError {
            code: ErrorCode(-32000),
            message: e.to_string().into(),
            data: None,
        })?;

        let encoded = general_purpose::STANDARD.encode(&content);

        Ok(CallToolResult::success(vec![Content::text(format!(
            r#"{{"content": "{}"}}"#,
            encoded
        ))]))
    }

    #[tool(
        description = "Query work items using field filters (area path, iteration, dates, board columns/rows, work item types, states, tags, assigned to). Supports both include and exclude filters."
    )]
    async fn azure_devops_query_work_items(
        &self,
        args: Parameters<GetBoardWorkItemsArgs>,
    ) -> Result<CallToolResult, McpError> {
        log::info!(
            "Tool invoked: azure_devops_query_work_items(area_path={:?}, iteration={:?}, include_board_column={:?}, exclude_state={:?})",
            args.0.area_path,
            args.0.iteration,
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
        if let Some(iteration) = &args.0.iteration {
            conditions.push(format!(
                "[System.IterationPath] UNDER '{}'",
                iteration.replace("'", "''")
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
                self.client.project
            )
        } else {
            format!(
                "SELECT [System.Id] FROM WorkItems WHERE {}",
                conditions.join(" AND ")
            )
        };

        log::debug!("Executing WIQL query: {}", query);

        // Execute the query to get work items
        let work_items = work_items::query_work_items(&self.client, &query)
            .await
            .map_err(|e| McpError {
                code: ErrorCode(-32000),
                message: e.to_string().into(),
                data: None,
            })?;

        // Convert to JSON value, simplify, then serialize
        let mut json_value = serde_json::to_value(&work_items).unwrap();
        simplify_work_item_json(&mut json_value);

        Ok(CallToolResult::success(vec![Content::text(
            serde_json::to_string_pretty(&json_value).unwrap(),
        )]))
    }

    #[tool(description = "Update an existing work item with comprehensive field support")]
    async fn azure_devops_update_work_item(
        &self,
        args: Parameters<UpdateWorkItemArgs>,
    ) -> Result<CallToolResult, McpError> {
        log::info!(
            "Tool invoked: azure_devops_update_work_item(id={}, title={:?}, state={:?})",
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
        if let Some(iteration) = &args.0.iteration {
            field_map.insert(
                "System.IterationPath".to_string(),
                serde_json::json!(iteration),
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

        let work_item = work_items::update_work_item(&self.client, args.0.id, &fields_vec)
            .await
            .map_err(|e| McpError {
                code: ErrorCode(-32000),
                message: e.to_string().into(),
                data: None,
            })?;

        Ok(CallToolResult::success(vec![Content::text(
            serde_json::to_string_pretty(&work_item).unwrap(),
        )]))
    }

    #[tool(description = "Add a comment to a work item")]
    async fn azure_devops_add_comment(
        &self,
        args: Parameters<AddCommentArgs>,
    ) -> Result<CallToolResult, McpError> {
        log::info!(
            "Tool invoked: azure_devops_add_comment(work_item_id={}, text_length={})",
            args.0.work_item_id,
            args.0.text.len()
        );

        let result = work_items::add_comment(&self.client, args.0.work_item_id, &args.0.text)
            .await
            .map_err(|e| McpError {
                code: ErrorCode(-32000),
                message: e.to_string().into(),
                data: None,
            })?;

        Ok(CallToolResult::success(vec![Content::text(
            serde_json::to_string_pretty(&result).unwrap(),
        )]))
    }

    #[tool(
        description = "Create a link between two work items (Parent, Child, Related, Duplicate, Dependency)"
    )]
    async fn azure_devops_link_work_items(
        &self,
        args: Parameters<LinkWorkItemsArgs>,
    ) -> Result<CallToolResult, McpError> {
        log::info!(
            "Tool invoked: azure_devops_link_work_items(source_id={}, target_id={}, link_type={})",
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
            serde_json::to_string_pretty(&result).unwrap(),
        )]))
    }
}

#[tool_handler]
impl rmcp::ServerHandler for AzureMcpServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            server_info: Implementation {
                name: "azure-devops-boards-mcp-rust".into(),
                version: "0.1.0".into(),
                icons: None,
                title: None,
                website_url: None,
            },
            instructions: Some(
                "Use this tool to interact with Azure DevOps Boards and Work Items".into(),
            ),
            capabilities: ServerCapabilities::builder().enable_tools().build(),
            ..Default::default()
        }
    }
}
