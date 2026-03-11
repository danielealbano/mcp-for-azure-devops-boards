use crate::azure::api_trait::AzureDevOpsApi;
use crate::mcp::tools::support::{
    deserialize_non_empty_string, simplify_work_item_json, tool_text_success, work_items_to_csv,
};
use mcp_tools_codegen::mcp_tool;
use rmcp::{
    ErrorData as McpError,
    model::{CallToolResult, ErrorCode},
    schemars::{self, JsonSchema},
    serde::Deserialize,
};

#[derive(Deserialize, JsonSchema)]
pub struct QueryWorkItemsArgsWiql {
    /// AzDO org name
    #[serde(deserialize_with = "deserialize_non_empty_string")]
    pub organization: String,
    /// AzDO project name
    #[serde(deserialize_with = "deserialize_non_empty_string")]
    pub project: String,
    /// WIQL query string (e.g., "SELECT [System.Id] FROM WorkItems WHERE [System.State] = 'Active'")
    pub query: String,
    /// Include the latest N comments (optional). Set to -1 for all comments.
    #[serde(default)]
    pub include_latest_n_comments: Option<i32>,
}

#[mcp_tool(
    name = "azdo_query_work_items_by_wiql",
    description = "Query work items using WIQL"
)]
pub async fn query_work_items_by_wiql(
    client: &(dyn AzureDevOpsApi + Send + Sync),
    args: QueryWorkItemsArgsWiql,
) -> Result<CallToolResult, McpError> {
    log::info!(
        "Tool invoked: azdo_query_work_items_by_wiql(query={})",
        args.query
    );
    let items = client
        .query_work_items(
            &args.organization,
            &args.project,
            &args.query,
            args.include_latest_n_comments,
        )
        .await
        .map_err(|e| McpError {
            code: ErrorCode(-32000),
            message: e.to_string().into(),
            data: None,
        })?;

    if items.is_empty() {
        return Ok(tool_text_success("No work items found"));
    }

    // Convert to JSON value, simplify, then convert to CSV
    let mut json_value = serde_json::to_value(&items).unwrap();
    simplify_work_item_json(&mut json_value);
    let csv_output = work_items_to_csv(&json_value).map_err(|e| McpError {
        code: ErrorCode(-32000),
        message: format!("Failed to convert to CSV: {}", e).into(),
        data: None,
    })?;

    Ok(tool_text_success(csv_output))
}
