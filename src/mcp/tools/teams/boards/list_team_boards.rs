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
pub struct ListBoardsArgs {
    /// AzDO org name
    #[serde(deserialize_with = "deserialize_non_empty_string")]
    pub organization: String,
    /// AzDO project name
    #[serde(deserialize_with = "deserialize_non_empty_string")]
    pub project: String,
    /// Team ID or name
    pub team_id: String,
}

#[mcp_tool(name = "azdo_list_team_boards", description = "List boards")]
pub async fn list_team_boards(
    client: &AzureDevOpsClient,
    args: ListBoardsArgs,
) -> Result<CallToolResult, McpError> {
    log::info!(
        "Tool invoked: azdo_list_team_boards(team_id={})",
        args.team_id
    );
    let boards = boards::list_boards(client, &args.organization, &args.project, &args.team_id)
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
