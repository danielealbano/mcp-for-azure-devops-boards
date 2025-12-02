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
pub struct GetWorkItemsArgs {
    /// AzDO org name
    #[serde(deserialize_with = "deserialize_non_empty_string")]
    pub organization: String,
    /// AzDO project name
    #[serde(deserialize_with = "deserialize_non_empty_string")]
    pub project: String,
    /// Work item IDs (comma-separated or array)
    pub ids: Vec<i64>,
    /// Include the latest N comments (optional). Set to -1 for all comments.
    #[serde(default)]
    pub include_latest_n_comments: Option<i32>,
}

#[mcp_tool(
    name = "azdo_get_work_items",
    description = "Get multiple work items by IDs"
)]
pub async fn get_work_items(
    client: &AzureDevOpsClient,
    args: GetWorkItemsArgs,
) -> Result<CallToolResult, McpError> {
    log::info!("Tool invoked: azdo_get_work_items(ids={:?})", args.ids);

    if args.ids.is_empty() {
        return Ok(CallToolResult::success(vec![Content::text(
            "No work items found",
        )]));
    }

    let ids: Vec<u32> = args.ids.iter().map(|&id| id as u32).collect();
    let work_items = work_items::get_work_items(
        client,
        &args.organization,
        &args.project,
        &ids,
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
