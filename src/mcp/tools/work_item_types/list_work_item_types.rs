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
pub struct ListWorkItemTypesArgs {
    /// AzDO org name
    #[serde(deserialize_with = "deserialize_non_empty_string")]
    pub organization: String,
    /// AzDO project name
    #[serde(deserialize_with = "deserialize_non_empty_string")]
    pub project: String,
}

#[mcp_tool(
    name = "azdo_list_work_item_types",
    description = "List work item types"
)]
pub async fn list_work_item_types(
    client: &(dyn AzureDevOpsApi + Send + Sync),
    args: ListWorkItemTypesArgs,
) -> Result<CallToolResult, McpError> {
    log::info!("Tool invoked: azdo_list_work_item_types");
    let types = client
        .list_work_item_types(&args.organization, &args.project)
        .await
        .map_err(|e| McpError {
            code: ErrorCode(-32000),
            message: e.to_string().into(),
            data: None,
        })?;

    // Extract just the work item type names for compact response
    let type_names: Vec<String> = types.into_iter().map(|wit| wit.name).collect();

    let output = compact_llm::to_compact_string(&type_names).map_err(|e| McpError {
        code: ErrorCode(-32000),
        message: format!("Failed to serialize response: {}", e).into(),
        data: None,
    })?;

    Ok(tool_text_success(output))
}
