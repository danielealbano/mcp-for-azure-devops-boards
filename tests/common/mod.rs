use mcp_for_azure_devops_boards::mcp::tools::support::UNTRUSTED_CONTENT_WARNING;
use rmcp::model::CallToolResult;

pub fn assert_tool_output_has_warning(result: &CallToolResult) {
    assert!(!result.content.is_empty(), "Tool output must have content");
    let text = extract_text_from_result(result);
    assert!(
        text.starts_with(UNTRUSTED_CONTENT_WARNING),
        "Tool output must start with anti-prompt-injection warning.\nGot: {}",
        &text[..std::cmp::min(text.len(), 120)]
    );
}

pub fn extract_text_from_result(result: &CallToolResult) -> String {
    let content = &result.content[0];
    content
        .raw
        .as_text()
        .expect("Content should be a text variant")
        .text
        .clone()
}
