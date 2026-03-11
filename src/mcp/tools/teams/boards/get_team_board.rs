use crate::azure::api_trait::AzureDevOpsApi;
use crate::compact_llm;
use crate::mcp::tools::support::{deserialize_non_empty_string, tool_text_success};
use mcp_tools_codegen::mcp_tool;
use rmcp::{
    ErrorData as McpError,
    model::{CallToolResult, ErrorCode},
    schemars::{self, JsonSchema},
    serde::Deserialize,
};

#[derive(Deserialize, JsonSchema)]
pub struct GetBoardArgs {
    /// AzDO org name
    #[serde(deserialize_with = "deserialize_non_empty_string")]
    pub organization: String,
    /// AzDO project name
    #[serde(deserialize_with = "deserialize_non_empty_string")]
    pub project: String,
    /// Team ID or name
    pub team_id: String,
    /// Board ID or name
    pub board_id: String,
}

#[mcp_tool(name = "azdo_get_team_board", description = "Get board details")]
pub async fn get_team_board(
    client: &(dyn AzureDevOpsApi + Send + Sync),
    args: GetBoardArgs,
) -> Result<CallToolResult, McpError> {
    log::info!(
        "Tool invoked: azdo_get_team_board(team_id={}, board_id={})",
        args.team_id,
        args.board_id
    );
    let board = client
        .get_board(
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

    Ok(tool_text_success(
        compact_llm::to_compact_string(&board).unwrap(),
    ))
}
