use crate::azure::{client::AzureDevOpsClient, organizations};
use crate::compact_llm;
use mcp_tools_codegen::mcp_tool;
use rmcp::{
    ErrorData as McpError,
    model::{CallToolResult, Content, ErrorCode},
    schemars::{self, JsonSchema},
    serde::Deserialize,
};

#[derive(Deserialize, JsonSchema)]
pub struct ListOrganizationsArgs {}

#[mcp_tool(name = "azdo_list_organizations", description = "List organizations")]
pub async fn list_organizations(
    client: &AzureDevOpsClient,
    _args: ListOrganizationsArgs,
) -> Result<CallToolResult, McpError> {
    log::info!("Tool invoked: azdo_list_organizations");

    // First, get the user's profile to obtain their member ID
    let profile = organizations::get_profile(client)
        .await
        .map_err(|e| McpError {
            code: ErrorCode(-32000),
            message: format!("Failed to get user profile: {}", e).into(),
            data: None,
        })?;

    // Then, list all organizations for this member ID
    let orgs = organizations::list_organizations(client, &profile.id)
        .await
        .map_err(|e| McpError {
            code: ErrorCode(-32000),
            message: e.to_string().into(),
            data: None,
        })?;

    // Extract just the organization names for compact response
    let org_names: Vec<String> = orgs.into_iter().map(|org| org.account_name).collect();

    Ok(CallToolResult::success(vec![Content::text(
        compact_llm::to_compact_string(&org_names).unwrap(),
    )]))
}
