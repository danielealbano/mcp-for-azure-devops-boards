use crate::azure::api_trait::AzureDevOpsApi;

use crate::mcp::tools::support::{deserialize_non_empty_string, tool_text_success};
use mcp_tools_codegen::mcp_tool;
use rmcp::{
    ErrorData as McpError,
    model::{CallToolResult, ErrorCode},
    schemars::{self, JsonSchema},
    serde::Deserialize,
};

#[derive(Deserialize, JsonSchema)]
pub struct ListProjectsArgs {
    /// AzDO org name
    #[serde(deserialize_with = "deserialize_non_empty_string")]
    pub organization: String,
}

#[mcp_tool(
    name = "azdo_list_projects",
    description = "List projects in an organization"
)]
pub async fn list_projects(
    client: &(dyn AzureDevOpsApi + Send + Sync),
    args: ListProjectsArgs,
) -> Result<CallToolResult, McpError> {
    log::info!("Tool invoked: azdo_list_projects");
    let projects = client
        .list_projects(&args.organization)
        .await
        .map_err(|e| McpError {
            code: ErrorCode(-32000),
            message: e.to_string().into(),
            data: None,
        })?;

    // Extract just the project names for compact response
    let project_names: Vec<String> = projects.into_iter().map(|project| project.name).collect();

    Ok(tool_text_success(project_names.join(",")))
}
