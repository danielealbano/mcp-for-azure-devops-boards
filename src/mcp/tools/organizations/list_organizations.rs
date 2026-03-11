use crate::azure::api_trait::AzureDevOpsApi;
use crate::mcp::tools::support::tool_text_success;

use mcp_tools_codegen::mcp_tool;
use rmcp::{
    ErrorData as McpError,
    model::{CallToolResult, ErrorCode},
    schemars::{self, JsonSchema},
    serde::Deserialize,
};

#[derive(Deserialize, JsonSchema)]
pub struct ListOrganizationsArgs {}

#[mcp_tool(name = "azdo_list_organizations", description = "List organizations")]
pub async fn list_organizations(
    client: &(dyn AzureDevOpsApi + Send + Sync),
    _args: ListOrganizationsArgs,
) -> Result<CallToolResult, McpError> {
    log::info!("Tool invoked: azdo_list_organizations");

    // First, get the user's profile to obtain their member ID
    let profile = client.get_profile().await.map_err(|e| McpError {
        code: ErrorCode(-32000),
        message: format!("Failed to get user profile: {}", e).into(),
        data: None,
    })?;

    // Then, list all organizations for this member ID
    let orgs = client
        .list_organizations(&profile.id)
        .await
        .map_err(|e| McpError {
            code: ErrorCode(-32000),
            message: e.to_string().into(),
            data: None,
        })?;

    // Extract just the organization names for compact response
    let org_names: Vec<String> = orgs.into_iter().map(|org| org.account_name).collect();

    Ok(tool_text_success(org_names.join(",")))
}
