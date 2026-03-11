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
pub struct ListBoardsArgs {
    /// AzDO org name
    #[serde(deserialize_with = "deserialize_non_empty_string")]
    pub organization: String,
    /// AzDO project name
    #[serde(deserialize_with = "deserialize_non_empty_string")]
    pub project: String,
    /// Team ID or name
    #[serde(deserialize_with = "deserialize_non_empty_string")]
    pub team_id: String,
}

#[mcp_tool(name = "azdo_list_team_boards", description = "List boards")]
pub async fn list_team_boards(
    client: &(dyn AzureDevOpsApi + Send + Sync),
    args: ListBoardsArgs,
) -> Result<CallToolResult, McpError> {
    log::info!(
        "Tool invoked: azdo_list_team_boards(team_id={})",
        args.team_id
    );
    let boards = client
        .list_boards(&args.organization, &args.project, &args.team_id)
        .await
        .map_err(|e| McpError {
            code: ErrorCode(-32000),
            message: e.to_string().into(),
            data: None,
        })?;

    // Extract just the board names for compact response
    let board_names: Vec<String> = boards.into_iter().map(|board| board.name).collect();

    let output = compact_llm::to_compact_string(&board_names).map_err(|e| McpError {
        code: ErrorCode(-32000),
        message: format!("Failed to serialize response: {}", e).into(),
        data: None,
    })?;

    Ok(tool_text_success(output))
}
