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
pub struct LinkWorkItemsArgs {
    /// AzDO org name
    #[serde(deserialize_with = "deserialize_non_empty_string")]
    pub organization: String,
    /// AzDO project name
    #[serde(deserialize_with = "deserialize_non_empty_string")]
    pub project: String,
    /// Source work item ID
    pub source_id: u32,
    /// Target work item ID
    pub target_id: u32,
    /// Link type: "Parent", "Child", "Related", "Duplicate", "Dependency"
    pub link_type: String,
}

#[mcp_tool(name = "azdo_link_work_items", description = "Link work items")]
pub async fn link_work_items(
    client: &AzureDevOpsClient,
    args: LinkWorkItemsArgs,
) -> Result<CallToolResult, McpError> {
    log::info!(
        "Tool invoked: azdo_link_work_items(source_id={}, target_id={}, link_type={})",
        args.source_id,
        args.target_id,
        args.link_type
    );

    // Map friendly names to Azure DevOps link type names
    let link_type_ref = match args.link_type.to_lowercase().as_str() {
        "parent" => "System.LinkTypes.Hierarchy-Forward",
        "child" => "System.LinkTypes.Hierarchy-Reverse",
        "related" => "System.LinkTypes.Related",
        "duplicate" => "System.LinkTypes.Duplicate-Forward",
        "dependency" => "System.LinkTypes.Dependency-Forward",
        _ => &args.link_type, // Use as-is if not a known friendly name
    };

    let result = work_items::link_work_items(
        client,
        &args.organization,
        &args.project,
        args.source_id,
        args.target_id,
        link_type_ref,
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
