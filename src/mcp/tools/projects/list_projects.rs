use crate::azure::{client::AzureDevOpsClient, projects};
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
    client: &AzureDevOpsClient,
    args: ListProjectsArgs,
) -> Result<CallToolResult, McpError> {
    log::info!("Tool invoked: azdo_list_projects");
    let projects = projects::list_projects(client, &args.organization)
        .await
        .map_err(|e| McpError {
            code: ErrorCode(-32000),
            message: e.to_string().into(),
            data: None,
        })?;

    // Extract just the project names for compact response
    let project_names: Vec<String> = projects.into_iter().map(|project| project.name).collect();

    Ok(CallToolResult::success(vec![Content::text(
        compact_llm::to_compact_string(&project_names).unwrap(),
    )]))
}
