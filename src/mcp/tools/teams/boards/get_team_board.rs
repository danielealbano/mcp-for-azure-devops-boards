use crate::azure::{boards, client::AzureDevOpsClient};
use crate::compact_llm;
use crate::mcp::tools::support::deserialize_non_empty_string;
use mcp_tools_codegen::mcp_tool;
use rmcp::{
    ErrorData as McpError,
    model::{CallToolResult, Content, ErrorCode},
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
    client: &AzureDevOpsClient,
    args: GetBoardArgs,
) -> Result<CallToolResult, McpError> {
    log::info!(
        "Tool invoked: azdo_get_team_board(team_id={}, board_id={})",
        args.team_id,
        args.board_id
    );
    let board = boards::get_board(
        client,
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

    Ok(CallToolResult::success(vec![Content::text(
        compact_llm::to_compact_string(&board).unwrap(),
    )]))
}
