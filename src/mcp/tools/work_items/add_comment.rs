use crate::azure::{client::AzureDevOpsClient, work_items};
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
pub struct AddCommentArgs {
    /// AzDO org name
    #[serde(deserialize_with = "deserialize_non_empty_string")]
    pub organization: String,
    /// AzDO project name
    #[serde(deserialize_with = "deserialize_non_empty_string")]
    pub project: String,
    /// Work item ID to add comment to
    pub work_item_id: u32,
    /// Comment text (supports markdown)
    pub text: String,
}

#[mcp_tool(
    name = "azdo_add_comment",
    description = "Add a comment to a work item"
)]
pub async fn add_comment(
    client: &AzureDevOpsClient,
    args: AddCommentArgs,
) -> Result<CallToolResult, McpError> {
    log::info!(
        "Tool invoked: azdo_add_comment(work_item_id={}, text_length={})",
        args.work_item_id,
        args.text.len()
    );

    let result = work_items::add_comment(
        client,
        &args.organization,
        &args.project,
        args.work_item_id,
        &args.text,
    )
    .await
    .map_err(|e| McpError {
        code: ErrorCode(-32000),
        message: e.to_string().into(),
        data: None,
    })?;

    Ok(CallToolResult::success(vec![Content::text(
        compact_llm::to_compact_string(&result).unwrap(),
    )]))
}
