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
pub struct ListBoardRowsArgs {
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

#[mcp_tool(
    name = "azdo_list_board_rows",
    description = "List board rows (swimlanes)"
)]
pub async fn list_board_rows(
    client: &AzureDevOpsClient,
    args: ListBoardRowsArgs,
) -> Result<CallToolResult, McpError> {
    log::info!(
        "Tool invoked: azdo_list_board_rows(team_id={}, board_id={})",
        args.team_id,
        args.board_id
    );
    let rows = boards::list_board_rows(
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

    // Extract row names into an array
    let row_names: Vec<String> = rows
        .into_iter()
        .map(|row| row.name.unwrap_or_default())
        .collect();

    Ok(CallToolResult::success(vec![Content::text(
        compact_llm::to_compact_string(&row_names).unwrap(),
    )]))
}
