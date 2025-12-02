use crate::azure::client::AzureDevOpsClient;
use crate::mcp::tools::support::deserialize_non_empty_string;
use mcp_tools_codegen::mcp_tool;
use rmcp::{
    ErrorData as McpError,
    model::{CallToolResult, Content, ErrorCode},
    schemars::{self, JsonSchema},
    serde::Deserialize,
};

#[derive(Deserialize, JsonSchema)]
pub struct ListTeamMembersArgs {
    /// AzDO org name
    #[serde(deserialize_with = "deserialize_non_empty_string")]
    pub organization: String,
    /// AzDO project name
    #[serde(deserialize_with = "deserialize_non_empty_string")]
    pub project: String,
    /// Team ID or name
    pub team_id: String,
}

#[mcp_tool(name = "azdo_list_team_members", description = "List team members")]
pub async fn list_team_members(
    client: &AzureDevOpsClient,
    args: ListTeamMembersArgs,
) -> Result<CallToolResult, McpError> {
    log::info!("Tool invoked: azdo_list_team_members");
    let members = client
        .list_team_members(&args.organization, &args.project, &args.team_id)
        .await
        .map_err(|e| McpError {
            code: ErrorCode(-32000),
            message: e.to_string().into(),
            data: None,
        })?;

    let mut wtr = csv::WriterBuilder::new()
        .has_headers(false)
        .from_writer(vec![]);

    for member in members {
        wtr.write_record(&[member.identity.display_name, member.identity.unique_name])
            .map_err(|e| McpError {
                code: ErrorCode(-32000),
                message: format!("Failed to write CSV: {}", e).into(),
                data: None,
            })?;
    }

    wtr.flush().map_err(|e| McpError {
        code: ErrorCode(-32000),
        message: format!("Failed to flush CSV: {}", e).into(),
        data: None,
    })?;

    let csv_bytes = wtr.into_inner().map_err(|e| McpError {
        code: ErrorCode(-32000),
        message: format!("Failed to get CSV bytes: {}", e).into(),
        data: None,
    })?;

    let data = String::from_utf8(csv_bytes).map_err(|e| McpError {
        code: ErrorCode(-32000),
        message: format!("Failed to convert CSV to string: {}", e).into(),
        data: None,
    })?;

    Ok(CallToolResult::success(vec![Content::text(data)]))
}
