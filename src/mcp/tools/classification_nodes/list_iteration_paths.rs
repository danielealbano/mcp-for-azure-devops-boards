use crate::azure::{classification_nodes, client::AzureDevOpsClient, iterations};
use crate::mcp::tools::support::deserialize_non_empty_string;
use mcp_tools_codegen::mcp_tool;
use rmcp::{
    ErrorData as McpError,
    model::{CallToolResult, Content, ErrorCode},
    schemars::{self, JsonSchema},
    serde::Deserialize,
};

#[derive(Deserialize, JsonSchema)]
pub struct ListIterationPathsArgs {
    /// AzDO org name
    #[serde(deserialize_with = "deserialize_non_empty_string")]
    pub organization: String,
    /// AzDO project name
    #[serde(deserialize_with = "deserialize_non_empty_string")]
    pub project: String,
    /// Optional team ID or name to get team-specific iterations
    #[serde(default)]
    pub team_id: Option<String>,
    /// Optional timeframe filter: "current", "past", or "future" (only applies when team_id is provided)
    #[serde(default)]
    pub timeframe: Option<String>,
}

#[mcp_tool(
    name = "azdo_list_iteration_paths",
    description = "List iteration paths for a project or team"
)]
pub async fn list_iteration_paths(
    client: &AzureDevOpsClient,
    args: ListIterationPathsArgs,
) -> Result<CallToolResult, McpError> {
    log::info!("Tool invoked: azdo_list_iteration_paths");

    // Validate timeframe if provided
    if let Some(ref timeframe) = args.timeframe {
        if !matches!(timeframe.as_str(), "current" | "past" | "future") {
            return Err(McpError {
                code: ErrorCode(-32602),
                message: format!(
                    "Invalid timeframe '{}'. Valid values are: 'current', 'past', 'future'",
                    timeframe
                )
                .into(),
                data: None,
            });
        }
    }

    // If team_id is provided, use team-specific iterations
    if let Some(team_id) = &args.team_id {
        let mut iterations = iterations::get_team_iterations(
            client,
            &args.organization,
            &args.project,
            team_id,
            None, // Get all iterations first
        )
        .await
        .map_err(|e| McpError {
            code: ErrorCode(-32000),
            message: e.to_string().into(),
            data: None,
        })?;

        // Filter by timeframe if provided (post-acquisition filtering)
        if let Some(ref timeframe) = args.timeframe {
            iterations.retain(|iter| {
                iter.attributes
                    .time_frame
                    .as_ref()
                    .map(|tf| tf.eq_ignore_ascii_case(timeframe))
                    .unwrap_or(false)
            });
        }

        // Convert to CSV format: name,timeframe,start_date,finish_date
        let mut csv_lines = Vec::new();
        for iteration in iterations {
            let start_date = iteration
                .attributes
                .start_date
                .as_ref()
                .and_then(|d| d.split('T').next())
                .unwrap_or("N/A");
            let finish_date = iteration
                .attributes
                .finish_date
                .as_ref()
                .and_then(|d| d.split('T').next())
                .unwrap_or("N/A");
            let timeframe = iteration.attributes.time_frame.as_deref().unwrap_or("N/A");

            csv_lines.push(format!(
                "{},{},{},{}",
                iteration.name, timeframe, start_date, finish_date
            ));
        }

        if csv_lines.is_empty() {
            Ok(CallToolResult::success(vec![Content::text(
                "No iterations found",
            )]))
        } else {
            Ok(CallToolResult::success(vec![Content::text(
                csv_lines.join(","),
            )]))
        }
    } else {
        // Use project-level classification nodes
        let root_node = classification_nodes::list_iteration_paths(
            client,
            &args.organization,
            &args.project,
            None,
            10, // depth
        )
        .await
        .map_err(|e| McpError {
            code: ErrorCode(-32000),
            message: e.to_string().into(),
            data: None,
        })?;

        // Flatten the tree into a list of paths and return as CSV
        let mut paths = Vec::new();
        root_node.collect_paths(&mut paths);

        // Return as CSV format: path (single column for consistency)
        if paths.is_empty() {
            Ok(CallToolResult::success(vec![Content::text(
                "No iterations found",
            )]))
        } else {
            Ok(CallToolResult::success(vec![Content::text(
                paths.join(","),
            )]))
        }
    }
}
