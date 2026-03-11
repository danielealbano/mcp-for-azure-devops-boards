use crate::azure::api_trait::AzureDevOpsApi;
use crate::compact_llm;
use crate::mcp::tools::support::{
    default_text_format, deserialize_non_empty_string, simplify_work_item_json, tool_text_success,
};
use mcp_tools_codegen::mcp_tool;
use rmcp::{
    ErrorData as McpError,
    model::{CallToolResult, ErrorCode},
    schemars::{self, JsonSchema},
    serde::Deserialize,
};

#[derive(Deserialize, JsonSchema)]
pub struct UpdateWorkItemArgs {
    /// AzDO org name
    #[serde(deserialize_with = "deserialize_non_empty_string")]
    pub organization: String,
    /// AzDO project name
    #[serde(deserialize_with = "deserialize_non_empty_string")]
    pub project: String,
    /// Work item ID to update
    pub id: u32,

    /// Format for large text fields (description, acceptance criteria, repro steps): "markdown" or "html" (default: "markdown")
    #[serde(default = "default_text_format")]
    pub format: String,

    /// Work item title
    #[serde(default)]
    pub title: Option<String>,

    /// Work item description (use markdown syntax when format is "markdown", HTML tags when format is "html")
    #[serde(default)]
    pub description: Option<String>,

    /// User to assign the work item to (email or display name)
    #[serde(default)]
    pub assigned_to: Option<String>,

    /// Area path (e.g., "MyProject\\Team1")
    #[serde(default)]
    pub area_path: Option<String>,

    /// Iteration path (e.g., "MyProject\\Sprint 1")
    #[serde(default)]
    pub iteration_path: Option<String>,

    /// State (New, Active, Resolved, Closed, etc.)
    #[serde(default)]
    pub state: Option<String>,

    /// Board column to place the work item in
    #[serde(default)]
    pub board_column: Option<String>,

    /// Board row/swimlane to place the work item in
    #[serde(default)]
    pub board_row: Option<String>,

    /// Priority (1-4, where 1 is highest)
    #[serde(default)]
    pub priority: Option<u32>,

    /// Severity for bugs (Critical, High, Medium, Low)
    #[serde(default)]
    pub severity: Option<String>,

    /// Story points for estimation
    #[serde(default)]
    pub story_points: Option<f64>,

    /// Effort estimate in hours
    #[serde(default)]
    pub effort: Option<f64>,

    /// Remaining work in hours
    #[serde(default)]
    pub remaining_work: Option<f64>,

    /// Comma-separated tags (e.g., "bug, critical, ui")
    #[serde(default)]
    pub tags: Option<String>,

    /// Activity type (Development, Testing, Documentation, etc.)
    #[serde(default)]
    pub activity: Option<String>,

    /// Start date (YYYY-MM-DD)
    #[serde(default)]
    pub start_date: Option<String>,

    /// Target/due date (YYYY-MM-DD)
    #[serde(default)]
    pub target_date: Option<String>,

    /// Acceptance criteria (use markdown syntax when format is "markdown", HTML tags when format is "html")
    #[serde(default)]
    pub acceptance_criteria: Option<String>,

    /// Reproduction steps (use markdown syntax when format is "markdown", HTML tags when format is "html")
    #[serde(default)]
    pub repro_steps: Option<String>,

    /// Optional extra fields as JSON string (for custom fields)
    #[serde(default)]
    pub fields: Option<String>,
}

#[mcp_tool(name = "azdo_update_work_item", description = "Update work item")]
pub async fn update_work_item(
    client: &(dyn AzureDevOpsApi + Send + Sync),
    args: UpdateWorkItemArgs,
) -> Result<CallToolResult, McpError> {
    let format = args.format.to_lowercase();
    if format != "markdown" && format != "html" {
        return Err(McpError {
            code: ErrorCode(-32602),
            message: "Invalid format: must be \"markdown\" or \"html\"".into(),
            data: None,
        });
    }

    log::info!(
        "Tool invoked: azdo_update_work_item(id={}, title={:?}, state={:?}, format={})",
        args.id,
        args.title,
        args.state,
        format,
    );

    // Build multiline fields format list for large text fields
    let mut multiline_formats: Vec<(String, String)> = Vec::new();
    if format == "markdown" {
        if args.description.is_some() {
            multiline_formats.push(("System.Description".to_string(), "Markdown".to_string()));
        }
        if args.acceptance_criteria.is_some() {
            multiline_formats.push((
                "Microsoft.VSTS.Common.AcceptanceCriteria".to_string(),
                "Markdown".to_string(),
            ));
        }
        if args.repro_steps.is_some() {
            multiline_formats.push((
                "Microsoft.VSTS.TCM.ReproSteps".to_string(),
                "Markdown".to_string(),
            ));
        }
    }

    // Build the field map (only include fields that are Some)
    let mut field_map = serde_json::Map::new();

    if let Some(title) = &args.title {
        field_map.insert("System.Title".to_string(), serde_json::json!(title));
    }
    if let Some(desc) = &args.description {
        field_map.insert("System.Description".to_string(), serde_json::json!(desc));
    }
    if let Some(assigned_to) = &args.assigned_to {
        field_map.insert(
            "System.AssignedTo".to_string(),
            serde_json::json!(assigned_to),
        );
    }
    if let Some(area_path) = &args.area_path {
        field_map.insert("System.AreaPath".to_string(), serde_json::json!(area_path));
    }
    if let Some(iteration_path) = &args.iteration_path {
        field_map.insert(
            "System.IterationPath".to_string(),
            serde_json::json!(iteration_path),
        );
    }
    if let Some(state) = &args.state {
        field_map.insert("System.State".to_string(), serde_json::json!(state));
    }
    if let Some(board_column) = &args.board_column {
        field_map.insert(
            "System.BoardColumn".to_string(),
            serde_json::json!(board_column),
        );
    }
    if let Some(board_row) = &args.board_row {
        field_map.insert("System.BoardLane".to_string(), serde_json::json!(board_row));
    }
    if let Some(priority) = args.priority {
        field_map.insert(
            "Microsoft.VSTS.Common.Priority".to_string(),
            serde_json::json!(priority),
        );
    }
    if let Some(severity) = &args.severity {
        field_map.insert(
            "Microsoft.VSTS.Common.Severity".to_string(),
            serde_json::json!(severity),
        );
    }
    if let Some(story_points) = args.story_points {
        field_map.insert(
            "Microsoft.VSTS.Scheduling.StoryPoints".to_string(),
            serde_json::json!(story_points),
        );
    }
    if let Some(effort) = args.effort {
        field_map.insert(
            "Microsoft.VSTS.Scheduling.Effort".to_string(),
            serde_json::json!(effort),
        );
    }
    if let Some(remaining_work) = args.remaining_work {
        field_map.insert(
            "Microsoft.VSTS.Scheduling.RemainingWork".to_string(),
            serde_json::json!(remaining_work),
        );
    }
    if let Some(tags) = &args.tags {
        field_map.insert("System.Tags".to_string(), serde_json::json!(tags));
    }
    if let Some(activity) = &args.activity {
        field_map.insert(
            "Microsoft.VSTS.Common.Activity".to_string(),
            serde_json::json!(activity),
        );
    }
    if let Some(start_date) = &args.start_date {
        field_map.insert(
            "Microsoft.VSTS.Scheduling.StartDate".to_string(),
            serde_json::json!(start_date),
        );
    }
    if let Some(target_date) = &args.target_date {
        field_map.insert(
            "Microsoft.VSTS.Scheduling.TargetDate".to_string(),
            serde_json::json!(target_date),
        );
    }
    if let Some(acceptance_criteria) = &args.acceptance_criteria {
        field_map.insert(
            "Microsoft.VSTS.Common.AcceptanceCriteria".to_string(),
            serde_json::json!(acceptance_criteria),
        );
    }
    if let Some(repro_steps) = &args.repro_steps {
        field_map.insert(
            "Microsoft.VSTS.TCM.ReproSteps".to_string(),
            serde_json::json!(repro_steps),
        );
    }

    // Merge any extra fields
    if let Some(extra) = &args.fields {
        if let Ok(extra_json) =
            serde_json::from_str::<serde_json::Map<String, serde_json::Value>>(extra)
        {
            for (k, v) in extra_json {
                field_map.insert(k, v);
            }
        } else {
            return Err(McpError {
                code: ErrorCode(-32602),
                message: "Invalid JSON in extra fields".into(),
                data: None,
            });
        }
    }

    let fields_vec: Vec<(String, serde_json::Value)> = field_map.into_iter().collect();

    let work_item = client
        .update_work_item(
            &args.organization,
            &args.project,
            args.id,
            &fields_vec,
            &multiline_formats,
        )
        .await
        .map_err(|e| McpError {
            code: ErrorCode(-32000),
            message: e.to_string().into(),
            data: None,
        })?;

    // Convert to JSON value, simplify, then serialize
    let mut json_value = serde_json::to_value(&work_item).unwrap();
    simplify_work_item_json(&mut json_value);

    Ok(tool_text_success(
        compact_llm::to_compact_string(&json_value).unwrap(),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn deserialize_args(json: &str) -> Result<UpdateWorkItemArgs, serde_json::Error> {
        serde_json::from_str(json)
    }

    #[test]
    fn test_update_work_item_args_format_defaults_to_markdown() {
        let args = deserialize_args(r#"{"organization":"org","project":"proj","id":1}"#).unwrap();
        assert_eq!(args.format, "markdown");
    }

    #[test]
    fn test_update_work_item_args_format_accepts_html() {
        let args =
            deserialize_args(r#"{"organization":"org","project":"proj","id":1,"format":"html"}"#)
                .unwrap();
        assert_eq!(args.format, "html");
    }

    #[test]
    fn test_update_work_item_args_format_accepts_markdown() {
        let args = deserialize_args(
            r#"{"organization":"org","project":"proj","id":1,"format":"markdown"}"#,
        )
        .unwrap();
        assert_eq!(args.format, "markdown");
    }

    #[test]
    fn test_update_work_item_args_invalid_format_passes_deserialization() {
        let args =
            deserialize_args(r#"{"organization":"org","project":"proj","id":1,"format":"xml"}"#)
                .unwrap();
        assert_eq!(args.format, "xml");
    }

    #[test]
    fn test_update_work_item_args_rejects_empty_organization() {
        let result = deserialize_args(r#"{"organization":"","project":"proj","id":1}"#);
        assert!(result.is_err());
    }

    #[test]
    fn test_update_work_item_args_rejects_empty_project() {
        let result = deserialize_args(r#"{"organization":"org","project":"","id":1}"#);
        assert!(result.is_err());
    }

    fn validate_format(format: &str) -> bool {
        let f = format.to_lowercase();
        f == "markdown" || f == "html"
    }

    #[test]
    fn test_update_work_item_format_validation_rejects_invalid_values() {
        let cases = vec![
            ("xml", false),
            ("plaintext", false),
            ("", false),
            ("MARKDOWN", true),
            ("Html", true),
            ("markdown", true),
            ("html", true),
        ];

        for (format, expected_valid) in cases {
            assert_eq!(
                validate_format(format),
                expected_valid,
                "format '{}' should be {}",
                format,
                if expected_valid { "valid" } else { "invalid" }
            );
        }
    }

    #[test]
    fn test_update_work_item_multiline_formats_built_for_markdown() {
        let args = deserialize_args(
            r#"{"organization":"org","project":"proj","id":1,"description":"desc","acceptance_criteria":"ac","repro_steps":"steps"}"#,
        )
        .unwrap();

        let format = args.format.to_lowercase();
        let mut multiline_formats: Vec<(String, String)> = Vec::new();
        if format == "markdown" {
            if args.description.is_some() {
                multiline_formats.push(("System.Description".to_string(), "Markdown".to_string()));
            }
            if args.acceptance_criteria.is_some() {
                multiline_formats.push((
                    "Microsoft.VSTS.Common.AcceptanceCriteria".to_string(),
                    "Markdown".to_string(),
                ));
            }
            if args.repro_steps.is_some() {
                multiline_formats.push((
                    "Microsoft.VSTS.TCM.ReproSteps".to_string(),
                    "Markdown".to_string(),
                ));
            }
        }

        assert_eq!(multiline_formats.len(), 3);
        assert_eq!(
            multiline_formats[0],
            ("System.Description".to_string(), "Markdown".to_string())
        );
        assert_eq!(
            multiline_formats[1],
            (
                "Microsoft.VSTS.Common.AcceptanceCriteria".to_string(),
                "Markdown".to_string()
            )
        );
        assert_eq!(
            multiline_formats[2],
            (
                "Microsoft.VSTS.TCM.ReproSteps".to_string(),
                "Markdown".to_string()
            )
        );
    }

    #[test]
    fn test_update_work_item_multiline_formats_empty_for_html() {
        let args = deserialize_args(
            r#"{"organization":"org","project":"proj","id":1,"format":"html","description":"desc","acceptance_criteria":"ac","repro_steps":"steps"}"#,
        )
        .unwrap();

        let format = args.format.to_lowercase();
        let mut multiline_formats: Vec<(String, String)> = Vec::new();
        if format == "markdown" {
            if args.description.is_some() {
                multiline_formats.push(("System.Description".to_string(), "Markdown".to_string()));
            }
            if args.acceptance_criteria.is_some() {
                multiline_formats.push((
                    "Microsoft.VSTS.Common.AcceptanceCriteria".to_string(),
                    "Markdown".to_string(),
                ));
            }
            if args.repro_steps.is_some() {
                multiline_formats.push((
                    "Microsoft.VSTS.TCM.ReproSteps".to_string(),
                    "Markdown".to_string(),
                ));
            }
        }

        assert!(multiline_formats.is_empty());
    }

    #[test]
    fn test_update_work_item_multiline_formats_only_for_provided_fields() {
        let args = deserialize_args(
            r#"{"organization":"org","project":"proj","id":1,"repro_steps":"steps"}"#,
        )
        .unwrap();

        let format = args.format.to_lowercase();
        let mut multiline_formats: Vec<(String, String)> = Vec::new();
        if format == "markdown" {
            if args.description.is_some() {
                multiline_formats.push(("System.Description".to_string(), "Markdown".to_string()));
            }
            if args.acceptance_criteria.is_some() {
                multiline_formats.push((
                    "Microsoft.VSTS.Common.AcceptanceCriteria".to_string(),
                    "Markdown".to_string(),
                ));
            }
            if args.repro_steps.is_some() {
                multiline_formats.push((
                    "Microsoft.VSTS.TCM.ReproSteps".to_string(),
                    "Markdown".to_string(),
                ));
            }
        }

        assert_eq!(multiline_formats.len(), 1);
        assert_eq!(
            multiline_formats[0],
            (
                "Microsoft.VSTS.TCM.ReproSteps".to_string(),
                "Markdown".to_string()
            )
        );
    }
}
