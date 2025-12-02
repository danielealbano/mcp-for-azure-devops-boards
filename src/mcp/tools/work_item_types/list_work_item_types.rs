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
    client: &AzureDevOpsClient,
    args: ListWorkItemTypesArgs,
) -> Result<CallToolResult, McpError> {
    log::info!("Tool invoked: azdo_list_work_item_types");
    let types = boards::list_work_item_types(client, &args.organization, &args.project)
        .await
        .map_err(|e| McpError {
            code: ErrorCode(-32000),
            message: e.to_string().into(),
            data: None,
        })?;

    // Extract just the work item type names for compact response
    let type_names: Vec<String> = types.into_iter().map(|wit| wit.name).collect();

    Ok(CallToolResult::success(vec![Content::text(
        compact_llm::to_compact_string(&type_names).unwrap(),
    )]))
}
