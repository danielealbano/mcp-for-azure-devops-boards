use crate::azure::{boards, client::AzureDevOpsClient};

use crate::mcp::tools::support::{deserialize_non_empty_string, tool_text_success};
use mcp_tools_codegen::mcp_tool;
use rmcp::{
    ErrorData as McpError,
    model::{CallToolResult, ErrorCode},
    schemars::{self, JsonSchema},
    serde::Deserialize,
};

#[derive(Deserialize, JsonSchema)]
pub struct GetTeamArgs {
    /// AzDO org name
    #[serde(deserialize_with = "deserialize_non_empty_string")]
    pub organization: String,
    /// AzDO project name
    #[serde(deserialize_with = "deserialize_non_empty_string")]
    pub project: String,
    /// Team ID or name
    pub team_id: String,
}

#[mcp_tool(name = "azdo_get_team", description = "Get team details")]
pub async fn get_team(
    client: &AzureDevOpsClient,
    args: GetTeamArgs,
) -> Result<CallToolResult, McpError> {
    log::info!("Tool invoked: azdo_get_team(team_id={})", args.team_id);
    let team = boards::get_team(client, &args.organization, &args.project, &args.team_id)
        .await
        .map_err(|e| McpError {
            code: ErrorCode(-32000),
            message: e.to_string().into(),
            data: None,
        })?;

    let description = team.description.unwrap_or_default();
    let output = format!("{},{}", team.name, description);

    Ok(tool_text_success(output))
}
