use crate::azure::{classification_nodes, client::AzureDevOpsClient};
use crate::mcp::tools::support::deserialize_non_empty_string;
use mcp_tools_codegen::mcp_tool;
use rmcp::{
    ErrorData as McpError,
    model::{CallToolResult, Content, ErrorCode},
    schemars::{self, JsonSchema},
    serde::Deserialize,
};

#[derive(Deserialize, JsonSchema)]
pub struct ListAreaPathsArgs {
    /// AzDO org name
    #[serde(deserialize_with = "deserialize_non_empty_string")]
    pub organization: String,
    /// AzDO project name
    #[serde(deserialize_with = "deserialize_non_empty_string")]
    pub project: String,
    /// Optional parent path to traverse the tree (e.g., "Area1\\SubArea1")
    #[serde(default)]
    pub parent_path: Option<String>,
}

#[mcp_tool(
    name = "azdo_list_area_paths",
    description = "List area paths for a project"
)]
pub async fn list_area_paths(
    client: &AzureDevOpsClient,
    args: ListAreaPathsArgs,
) -> Result<CallToolResult, McpError> {
    log::info!("Tool invoked: azdo_list_area_paths");

    let root_node = classification_nodes::list_area_paths(
        client,
        &args.organization,
        &args.project,
        args.parent_path.as_deref(),
        10, // depth
    )
    .await
    .map_err(|e| McpError {
        code: ErrorCode(-32000),
        message: e.to_string().into(),
        data: None,
    })?;

    // Flatten the tree into a list of paths
    fn collect_paths(node: &classification_nodes::ClassificationNode, paths: &mut Vec<String>) {
        paths.push(node.path.clone());
        if let Some(children) = &node.children {
            for child in children {
                collect_paths(child, paths);
            }
        }
    }

    let mut paths = Vec::new();
    collect_paths(&root_node, &mut paths);

    // Return as comma-separated list
    Ok(CallToolResult::success(vec![Content::text(
        paths.join(","),
    )]))
}
