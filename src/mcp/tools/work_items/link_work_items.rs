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
    #[serde(deserialize_with = "deserialize_non_empty_string")]
    pub link_type: String,
}

#[mcp_tool(name = "azdo_link_work_items", description = "Link work items")]
pub async fn link_work_items(
    client: &(dyn AzureDevOpsApi + Send + Sync),
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

    let result = client
        .link_work_items(
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

    let output = compact_llm::to_compact_string(&result).map_err(|e| McpError {
        code: ErrorCode(-32000),
        message: format!("Failed to serialize response: {}", e).into(),
        data: None,
    })?;

    Ok(tool_text_success(output))
}
