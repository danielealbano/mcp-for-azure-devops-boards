use crate::azure::{client::AzureDevOpsClient, work_items};
use crate::compact_llm;
use crate::mcp::tools::support::{default_comment_format, deserialize_non_empty_string};
use mcp_tools_codegen::mcp_tool;
use rmcp::{
    ErrorData as McpError,
    model::{CallToolResult, Content, ErrorCode},
    schemars::{self, JsonSchema},
    serde::Deserialize,
};

#[derive(Deserialize, JsonSchema)]
pub struct UpdateCommentArgs {
    /// AzDO org name
    #[serde(deserialize_with = "deserialize_non_empty_string")]
    pub organization: String,
    /// AzDO project name
    #[serde(deserialize_with = "deserialize_non_empty_string")]
    pub project: String,
    /// Work item ID the comment belongs to
    pub work_item_id: u32,
    /// Comment ID to update
    pub comment_id: u32,
    /// Updated comment text (use markdown syntax when format is "markdown", HTML tags when format is "html")
    #[serde(deserialize_with = "deserialize_non_empty_string")]
    pub text: String,
    /// Comment format: "markdown" or "html" (default: "markdown")
    #[serde(default = "default_comment_format")]
    pub format: String,
}

#[mcp_tool(
    name = "azdo_update_comment",
    description = "Update a comment on a work item"
)]
pub async fn update_comment(
    client: &AzureDevOpsClient,
    args: UpdateCommentArgs,
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
        "Tool invoked: azdo_update_comment(work_item_id={}, comment_id={}, text_length={}, format={})",
        args.work_item_id,
        args.comment_id,
        args.text.len(),
        format
    );

    let result = work_items::update_comment(
        client,
        &args.organization,
        &args.project,
        args.work_item_id,
        args.comment_id,
        &args.text,
        &format,
    )
    .await
    .map_err(|e| McpError {
        code: ErrorCode(-32000),
        message: e.to_string().into(),
        data: None,
    })?;

    let output = compact_llm::to_compact_string(&result).map_err(|e| McpError {
        code: ErrorCode(-32000),
        message: e.to_string().into(),
        data: None,
    })?;

    Ok(CallToolResult::success(vec![Content::text(output)]))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn deserialize_args(json: &str) -> Result<UpdateCommentArgs, serde_json::Error> {
        serde_json::from_str(json)
    }

    #[test]
    fn test_update_comment_args_format_defaults_to_markdown() {
        let args = deserialize_args(
            r#"{"organization":"org","project":"proj","work_item_id":1,"comment_id":42,"text":"hello"}"#,
        )
        .unwrap();
        assert_eq!(args.format, "markdown");
    }

    #[test]
    fn test_update_comment_args_format_accepts_html() {
        let args = deserialize_args(
            r#"{"organization":"org","project":"proj","work_item_id":1,"comment_id":42,"text":"hello","format":"html"}"#,
        )
        .unwrap();
        assert_eq!(args.format, "html");
    }

    #[test]
    fn test_update_comment_args_format_accepts_markdown() {
        let args = deserialize_args(
            r#"{"organization":"org","project":"proj","work_item_id":1,"comment_id":42,"text":"hello","format":"markdown"}"#,
        )
        .unwrap();
        assert_eq!(args.format, "markdown");
    }

    #[test]
    fn test_update_comment_args_rejects_empty_text() {
        let result = deserialize_args(
            r#"{"organization":"org","project":"proj","work_item_id":1,"comment_id":42,"text":""}"#,
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_update_comment_args_rejects_whitespace_only_text() {
        let result = deserialize_args(
            r#"{"organization":"org","project":"proj","work_item_id":1,"comment_id":42,"text":"   "}"#,
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_update_comment_args_rejects_empty_organization() {
        let result = deserialize_args(
            r#"{"organization":"","project":"proj","work_item_id":1,"comment_id":42,"text":"hello"}"#,
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_update_comment_args_rejects_empty_project() {
        let result = deserialize_args(
            r#"{"organization":"org","project":"","work_item_id":1,"comment_id":42,"text":"hello"}"#,
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_update_comment_args_comment_id_is_u32() {
        let args = deserialize_args(
            r#"{"organization":"org","project":"proj","work_item_id":1,"comment_id":4294967295,"text":"hello"}"#,
        )
        .unwrap();
        assert_eq!(args.comment_id, u32::MAX);
    }

    fn validate_format(format: &str) -> bool {
        let f = format.to_lowercase();
        f == "markdown" || f == "html"
    }

    #[test]
    fn test_update_comment_format_validation_rejects_invalid_values() {
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
    fn test_update_comment_args_invalid_format_passes_deserialization() {
        let args = deserialize_args(
            r#"{"organization":"org","project":"proj","work_item_id":1,"comment_id":42,"text":"hello","format":"xml"}"#,
        )
        .unwrap();
        assert_eq!(args.format, "xml");
    }
}
