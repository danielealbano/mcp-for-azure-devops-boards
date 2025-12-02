use crate::azure::{client::AzureDevOpsClient, work_items};
use crate::mcp::tools::support::{
    deserialize_non_empty_string, simplify_work_item_json, work_items_to_csv,
};
use mcp_tools_codegen::mcp_tool;
use rmcp::{
    ErrorData as McpError,
    model::{CallToolResult, Content, ErrorCode},
    schemars::{self, JsonSchema},
    serde::Deserialize,
};

#[derive(Deserialize, JsonSchema)]
pub struct QueryWorkItemsArgs {
    /// AzDO org name
    #[serde(deserialize_with = "deserialize_non_empty_string")]
    pub organization: String,
    /// AzDO project name
    #[serde(deserialize_with = "deserialize_non_empty_string")]
    pub project: String,

    /// Area path to filter by (e.g., "MyProject\\Team1"). Uses UNDER operator to include child paths.
    #[serde(default)]
    pub area_path: Option<String>,

    /// Iteration path to filter by (e.g., "MyProject\\Sprint 1"). Uses UNDER operator to include child paths.
    #[serde(default)]
    pub iteration_path: Option<String>,

    /// Filter by creation date (from). Format: YYYY-MM-DD or YYYY-MM-DDTHH:MM:SSZ
    #[serde(default)]
    pub created_date_from: Option<String>,

    /// Filter by creation date (to). Format: YYYY-MM-DD or YYYY-MM-DDTHH:MM:SSZ
    #[serde(default)]
    pub created_date_to: Option<String>,

    /// Filter by modified date (from). Format: YYYY-MM-DD or YYYY-MM-DDTHH:MM:SSZ
    #[serde(default)]
    pub modified_date_from: Option<String>,

    /// Filter by modified date (to). Format: YYYY-MM-DD or YYYY-MM-DDTHH:MM:SSZ
    #[serde(default)]
    pub modified_date_to: Option<String>,

    /// Board columns to include (e.g., ["Active", "Resolved"])
    #[serde(default)]
    pub include_board_column: Vec<String>,

    /// Board rows/swimlanes to include (e.g., ["General", "Scraping Platform"])
    #[serde(default)]
    pub include_board_row: Vec<String>,

    /// Work item types to include (e.g., ["Bug", "User Story"])
    #[serde(default)]
    pub include_work_item_type: Vec<String>,

    /// States to include (e.g., ["Active", "Resolved"])
    #[serde(default)]
    pub include_state: Vec<String>,

    /// Board columns to exclude
    #[serde(default)]
    pub exclude_board_column: Vec<String>,

    /// Board rows/swimlanes to exclude
    #[serde(default)]
    pub exclude_board_row: Vec<String>,

    /// Work item types to exclude
    #[serde(default)]
    pub exclude_work_item_type: Vec<String>,

    /// States to exclude (e.g., ["Closed", "Removed"])
    #[serde(default)]
    pub exclude_state: Vec<String>,

    /// Assignees to include (e.g., ["John Doe", "jane@example.com"])
    #[serde(default)]
    pub include_assigned_to: Vec<String>,

    /// Assignees to exclude
    #[serde(default)]
    pub exclude_assigned_to: Vec<String>,

    /// Tags to include (e.g., ["bug", "critical"])
    #[serde(default)]
    pub include_tags: Vec<String>,

    /// Tags to exclude (e.g., ["wontfix"])
    #[serde(default)]
    pub exclude_tags: Vec<String>,

    /// Include the latest N comments (optional). Set to -1 for all comments.
    #[serde(default)]
    pub include_latest_n_comments: Option<i32>,
}

#[mcp_tool(
    name = "azdo_query_work_items",
    description = "Query work items by filters"
)]
pub async fn query_work_items(
    client: &AzureDevOpsClient,
    args: QueryWorkItemsArgs,
) -> Result<CallToolResult, McpError> {
    log::info!(
        "Tool invoked: azdo_query_work_items(area_path={:?}, iteration_path={:?}, include_board_column={:?}, exclude_state={:?})",
        args.area_path,
        args.iteration_path,
        args.include_board_column,
        args.exclude_state
    );

    // Build WIQL query conditions
    let mut conditions = Vec::new();

    // Area path filter
    if let Some(area_path) = &args.area_path {
        conditions.push(format!(
            "[System.AreaPath] UNDER '{}'",
            area_path.replace("'", "''")
        ));
    }

    // Iteration filter
    if let Some(iteration_path) = &args.iteration_path {
        conditions.push(format!(
            "[System.IterationPath] UNDER '{}'",
            iteration_path.replace("'", "''")
        ));
    }

    // Date filters
    if let Some(date) = &args.created_date_from {
        conditions.push(format!("[System.CreatedDate] >= '{}'", date));
    }
    if let Some(date) = &args.created_date_to {
        conditions.push(format!("[System.CreatedDate] <= '{}'", date));
    }
    if let Some(date) = &args.modified_date_from {
        conditions.push(format!("[System.ChangedDate] >= '{}'", date));
    }
    if let Some(date) = &args.modified_date_to {
        conditions.push(format!("[System.ChangedDate] <= '{}'", date));
    }

    // Include filters (using IN operator)
    if !args.include_board_column.is_empty() {
        let values: Vec<String> = args
            .include_board_column
            .iter()
            .map(|v| format!("'{}'", v.replace("'", "''")))
            .collect();
        conditions.push(format!("[System.BoardColumn] IN ({})", values.join(", ")));
    }

    if !args.include_board_row.is_empty() {
        let values: Vec<String> = args
            .include_board_row
            .iter()
            .map(|v| format!("'{}'", v.replace("'", "''")))
            .collect();
        conditions.push(format!("[System.BoardLane] IN ({})", values.join(", ")));
    }

    if !args.include_work_item_type.is_empty() {
        let values: Vec<String> = args
            .include_work_item_type
            .iter()
            .map(|v| format!("'{}'", v.replace("'", "''")))
            .collect();
        conditions.push(format!("[System.WorkItemType] IN ({})", values.join(", ")));
    }

    if !args.include_state.is_empty() {
        let values: Vec<String> = args
            .include_state
            .iter()
            .map(|v| format!("'{}'", v.replace("'", "''")))
            .collect();
        conditions.push(format!("[System.State] IN ({})", values.join(", ")));
    }

    // Exclude filters (using NOT IN operator)
    if !args.exclude_board_column.is_empty() {
        let values: Vec<String> = args
            .exclude_board_column
            .iter()
            .map(|v| format!("'{}'", v.replace("'", "''")))
            .collect();
        conditions.push(format!(
            "[System.BoardColumn] NOT IN ({})",
            values.join(", ")
        ));
    }

    if !args.exclude_board_row.is_empty() {
        let values: Vec<String> = args
            .exclude_board_row
            .iter()
            .map(|v| format!("'{}'", v.replace("'", "''")))
            .collect();
        conditions.push(format!("[System.BoardLane] NOT IN ({})", values.join(", ")));
    }

    if !args.exclude_work_item_type.is_empty() {
        let values: Vec<String> = args
            .exclude_work_item_type
            .iter()
            .map(|v| format!("'{}'", v.replace("'", "''")))
            .collect();
        conditions.push(format!(
            "[System.WorkItemType] NOT IN ({})",
            values.join(", ")
        ));
    }

    if !args.exclude_state.is_empty() {
        let values: Vec<String> = args
            .exclude_state
            .iter()
            .map(|v| format!("'{}'", v.replace("'", "''")))
            .collect();
        conditions.push(format!("[System.State] NOT IN ({})", values.join(", ")));
    }

    if !args.include_assigned_to.is_empty() {
        let values: Vec<String> = args
            .include_assigned_to
            .iter()
            .map(|v| format!("'{}'", v.replace("'", "''")))
            .collect();
        conditions.push(format!("[System.AssignedTo] IN ({})", values.join(", ")));
    }

    if !args.exclude_assigned_to.is_empty() {
        let values: Vec<String> = args
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
    if !args.include_tags.is_empty() {
        for tag in &args.include_tags {
            conditions.push(format!(
                "[System.Tags] CONTAINS '{}'",
                tag.replace("'", "''")
            ));
        }
    }

    if !args.exclude_tags.is_empty() {
        for tag in &args.exclude_tags {
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
            args.project
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
        client,
        &args.organization,
        &args.project,
        &query,
        args.include_latest_n_comments,
    )
    .await
    .map_err(|e| McpError {
        code: ErrorCode(-32000),
        message: e.to_string().into(),
        data: None,
    })?;

    if work_items.is_empty() {
        return Ok(CallToolResult::success(vec![Content::text(
            "No work items found",
        )]));
    }

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
