use crate::azure::{client::AzureDevOpsClient, tags};
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
pub struct ListTagsArgs {
    /// AzDO org name
    #[serde(deserialize_with = "deserialize_non_empty_string")]
    pub organization: String,
    /// AzDO project name
    #[serde(deserialize_with = "deserialize_non_empty_string")]
    pub project: String,
}

#[mcp_tool(name = "azdo_list_tags", description = "List tags")]
pub async fn list_tags(
    client: &AzureDevOpsClient,
    args: ListTagsArgs,
) -> Result<CallToolResult, McpError> {
    log::info!("Tool invoked: azdo_list_tags");
    let tags = tags::list_tags(client, &args.organization, &args.project)
        .await
        .map_err(|e| McpError {
            code: ErrorCode(-32000),
            message: e.to_string().into(),
            data: None,
        })?;

    // Extract just the tag names for compact response
    let tag_names: Vec<String> = tags.into_iter().map(|tag| tag.name).collect();

    Ok(CallToolResult::success(vec![Content::text(
        compact_llm::to_compact_string(&tag_names).unwrap(),
    )]))
}
