use crate::azure::{client::AzureDevOpsClient, iterations};
use crate::mcp::tools::support::deserialize_non_empty_string;
use mcp_tools_codegen::mcp_tool;
use rmcp::{
    ErrorData as McpError,
    model::{CallToolResult, Content, ErrorCode},
    schemars::{self, JsonSchema},
    serde::Deserialize,
};

#[derive(Deserialize, JsonSchema)]
pub struct GetTeamCurrentIterationArgs {
    /// AzDO org
    #[serde(deserialize_with = "deserialize_non_empty_string")]
    pub organization: String,
    /// AzDO project
    #[serde(deserialize_with = "deserialize_non_empty_string")]
    pub project: String,
    /// Team ID or name
    pub team_id: String,
}

#[mcp_tool(
    name = "azdo_get_team_current_iteration",
    description = "Get current iteration/sprint for team"
)]
pub async fn get_team_current_iteration(
    client: &AzureDevOpsClient,
    args: GetTeamCurrentIterationArgs,
) -> Result<CallToolResult, McpError> {
    log::info!(
        "Tool invoked: azdo_get_team_current_iteration(team_id={})",
        args.team_id
    );

    let iteration = iterations::get_team_current_iteration(
        client,
        &args.organization,
        &args.project,
        &args.team_id,
    )
    .await
    .map_err(|e| McpError {
        code: ErrorCode(-32000),
        message: e.to_string().into(),
        data: None,
    })?;

    match iteration {
        Some(iteration) => {
            // Extract dates without time (just YYYY-MM-DD)
            let start_date = iteration
                .attributes
                .start_date
                .as_ref()
                .and_then(|d| d.split('T').next())
                .unwrap_or("N/A");
            let finish_date = iteration
                .attributes
                .finish_date
                .as_ref()
                .and_then(|d| d.split('T').next())
                .unwrap_or("N/A");

            // Return CSV format: name,start_date,finish_date
            let csv_output = format!("{},{},{}", iteration.name, start_date, finish_date);
            Ok(CallToolResult::success(vec![Content::text(csv_output)]))
        }
        None => Ok(CallToolResult::success(vec![Content::text(
            "No current iteration found",
        )])),
    }
}
