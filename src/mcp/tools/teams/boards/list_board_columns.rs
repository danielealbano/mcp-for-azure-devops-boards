use crate::azure::api_trait::AzureDevOpsApi;
use crate::mcp::tools::support::{
    board_columns_to_csv, deserialize_non_empty_string, tool_text_success,
};
use mcp_tools_codegen::mcp_tool;
use rmcp::{
    ErrorData as McpError,
    model::{CallToolResult, ErrorCode},
    schemars::{self, JsonSchema},
    serde::Deserialize,
};

#[derive(Deserialize, JsonSchema)]
pub struct ListBoardColumnsArgs {
    /// AzDO org name
    #[serde(deserialize_with = "deserialize_non_empty_string")]
    pub organization: String,
    /// AzDO project name
    #[serde(deserialize_with = "deserialize_non_empty_string")]
    pub project: String,
    /// Team ID or name
    #[serde(deserialize_with = "deserialize_non_empty_string")]
    pub team_id: String,
    /// Board ID or name
    #[serde(deserialize_with = "deserialize_non_empty_string")]
    pub board_id: String,
}

#[mcp_tool(name = "azdo_list_board_columns", description = "List board columns")]
pub async fn list_board_columns(
    client: &(dyn AzureDevOpsApi + Send + Sync),
    args: ListBoardColumnsArgs,
) -> Result<CallToolResult, McpError> {
    log::info!(
        "Tool invoked: azdo_list_board_columns(team_id={}, board_id={})",
        args.team_id,
        args.board_id
    );
    let columns = client
        .list_board_columns(
            &args.organization,
            &args.project,
            &args.team_id,
            &args.board_id,
        )
        .await
        .map_err(|e| McpError {
            code: ErrorCode(-32000),
            message: e.to_string().into(),
            data: None,
        })?;

    let csv_data = board_columns_to_csv(&columns).map_err(|e| McpError {
        code: ErrorCode(-32000),
        message: e.into(),
        data: None,
    })?;

    Ok(tool_text_success(csv_data))
}
