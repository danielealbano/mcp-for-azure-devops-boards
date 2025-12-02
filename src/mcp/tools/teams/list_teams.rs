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
pub struct ListTeamsArgs {
    /// AzDO org name
    #[serde(deserialize_with = "deserialize_non_empty_string")]
    pub organization: String,
    /// AzDO project name
    #[serde(deserialize_with = "deserialize_non_empty_string")]
    pub project: String,
}

#[mcp_tool(name = "azdo_list_teams", description = "List teams in the project")]
pub async fn list_teams(
    client: &AzureDevOpsClient,
    args: ListTeamsArgs,
) -> Result<CallToolResult, McpError> {
    log::info!("Tool invoked: azdo_list_teams");
    let teams = boards::list_teams(client, &args.organization, &args.project)
        .await
        .map_err(|e| McpError {
            code: ErrorCode(-32000),
            message: e.to_string().into(),
            data: None,
        })?;

    // Extract just the team names for compact response
    let team_names: Vec<String> = teams.into_iter().map(|team| team.name).collect();

    Ok(CallToolResult::success(vec![Content::text(
        compact_llm::to_compact_string(&team_names).unwrap(),
    )]))
}
