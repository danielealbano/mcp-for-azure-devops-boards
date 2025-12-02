use crate::azure::{client::AzureDevOpsClient, work_items};
use crate::compact_llm;
use crate::mcp::tools::support::{deserialize_non_empty_string, simplify_work_item_json};
use mcp_tools_codegen::mcp_tool;
use rmcp::{
    ErrorData as McpError,
    model::{CallToolResult, Content, ErrorCode},
    schemars::{self, JsonSchema},
    serde::Deserialize,
};

#[derive(Deserialize, JsonSchema)]
pub struct UpdateWorkItemArgs {
    /// AzDO org name
    #[serde(deserialize_with = "deserialize_non_empty_string")]
    pub organization: String,
    /// AzDO project name
    #[serde(deserialize_with = "deserialize_non_empty_string")]
    pub project: String,
    /// Work item ID to update
    pub id: u32,

    /// Work item title
    #[serde(default)]
    pub title: Option<String>,

    /// Work item description (Basic HTML supported)
    #[serde(default)]
    pub description: Option<String>,

    /// User to assign the work item to (email or display name)
    #[serde(default)]
    pub assigned_to: Option<String>,

    /// Area path (e.g., "MyProject\\Team1")
    #[serde(default)]
    pub area_path: Option<String>,

    /// Iteration path (e.g., "MyProject\\Sprint 1")
    #[serde(default)]
    pub iteration_path: Option<String>,

    /// State (New, Active, Resolved, Closed, etc.)
    #[serde(default)]
    pub state: Option<String>,

    /// Board column to place the work item in
    #[serde(default)]
    pub board_column: Option<String>,

    /// Board row/swimlane to place the work item in
    #[serde(default)]
    pub board_row: Option<String>,

    /// Priority (1-4, where 1 is highest)
    #[serde(default)]
    pub priority: Option<u32>,

    /// Severity for bugs (Critical, High, Medium, Low)
    #[serde(default)]
    pub severity: Option<String>,

    /// Story points for estimation
    #[serde(default)]
    pub story_points: Option<f64>,

    /// Effort estimate in hours
    #[serde(default)]
    pub effort: Option<f64>,

    /// Remaining work in hours
    #[serde(default)]
    pub remaining_work: Option<f64>,

    /// Comma-separated tags (e.g., "bug, critical, ui")
    #[serde(default)]
    pub tags: Option<String>,

    /// Activity type (Development, Testing, Documentation, etc.)
    #[serde(default)]
    pub activity: Option<String>,

    /// Start date (YYYY-MM-DD)
    #[serde(default)]
    pub start_date: Option<String>,

    /// Target/due date (YYYY-MM-DD)
    #[serde(default)]
    pub target_date: Option<String>,

    /// Acceptance criteria (for user stories)
    #[serde(default)]
    pub acceptance_criteria: Option<String>,

    /// Reproduction steps (for bugs)
    #[serde(default)]
    pub repro_steps: Option<String>,

    /// Optional extra fields as JSON string (for custom fields)
    #[serde(default)]
    pub fields: Option<String>,
}

#[mcp_tool(name = "azdo_update_work_item", description = "Update work item")]
pub async fn update_work_item(
    client: &AzureDevOpsClient,
    args: UpdateWorkItemArgs,
) -> Result<CallToolResult, McpError> {
    log::info!(
        "Tool invoked: azdo_update_work_item(id={}, title={:?}, state={:?})",
        args.id,
        args.title,
        args.state,
    );

    // Build the field map (only include fields that are Some)
    let mut field_map = serde_json::Map::new();

    if let Some(title) = &args.title {
        field_map.insert("System.Title".to_string(), serde_json::json!(title));
    }
    if let Some(desc) = &args.description {
        field_map.insert("System.Description".to_string(), serde_json::json!(desc));
    }
    if let Some(assigned_to) = &args.assigned_to {
        field_map.insert(
            "System.AssignedTo".to_string(),
            serde_json::json!(assigned_to),
        );
    }
    if let Some(area_path) = &args.area_path {
        field_map.insert("System.AreaPath".to_string(), serde_json::json!(area_path));
    }
    if let Some(iteration_path) = &args.iteration_path {
        field_map.insert(
            "System.IterationPath".to_string(),
            serde_json::json!(iteration_path),
        );
    }
    if let Some(state) = &args.state {
        field_map.insert("System.State".to_string(), serde_json::json!(state));
    }
    if let Some(board_column) = &args.board_column {
        field_map.insert(
            "System.BoardColumn".to_string(),
            serde_json::json!(board_column),
        );
    }
    if let Some(board_row) = &args.board_row {
        field_map.insert("System.BoardLane".to_string(), serde_json::json!(board_row));
    }
    if let Some(priority) = args.priority {
        field_map.insert(
            "Microsoft.VSTS.Common.Priority".to_string(),
            serde_json::json!(priority),
        );
    }
    if let Some(severity) = &args.severity {
        field_map.insert(
            "Microsoft.VSTS.Common.Severity".to_string(),
            serde_json::json!(severity),
        );
    }
    if let Some(story_points) = args.story_points {
        field_map.insert(
            "Microsoft.VSTS.Scheduling.StoryPoints".to_string(),
            serde_json::json!(story_points),
        );
    }
    if let Some(effort) = args.effort {
        field_map.insert(
            "Microsoft.VSTS.Scheduling.Effort".to_string(),
            serde_json::json!(effort),
        );
    }
    if let Some(remaining_work) = args.remaining_work {
        field_map.insert(
            "Microsoft.VSTS.Scheduling.RemainingWork".to_string(),
            serde_json::json!(remaining_work),
        );
    }
    if let Some(tags) = &args.tags {
        field_map.insert("System.Tags".to_string(), serde_json::json!(tags));
    }
    if let Some(activity) = &args.activity {
        field_map.insert(
            "Microsoft.VSTS.Common.Activity".to_string(),
            serde_json::json!(activity),
        );
    }
    if let Some(start_date) = &args.start_date {
        field_map.insert(
            "Microsoft.VSTS.Scheduling.StartDate".to_string(),
            serde_json::json!(start_date),
        );
    }
    if let Some(target_date) = &args.target_date {
        field_map.insert(
            "Microsoft.VSTS.Scheduling.TargetDate".to_string(),
            serde_json::json!(target_date),
        );
    }
    if let Some(acceptance_criteria) = &args.acceptance_criteria {
        field_map.insert(
            "Microsoft.VSTS.Common.AcceptanceCriteria".to_string(),
            serde_json::json!(acceptance_criteria),
        );
    }
    if let Some(repro_steps) = &args.repro_steps {
        field_map.insert(
            "Microsoft.VSTS.TCM.ReproSteps".to_string(),
            serde_json::json!(repro_steps),
        );
    }

    // Merge any extra fields
    if let Some(extra) = &args.fields {
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
        client,
        &args.organization,
        &args.project,
        args.id,
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
