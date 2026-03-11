#[cfg(feature = "test-support")]
mod common;

#[cfg(feature = "test-support")]
mod tests {
    use super::common::{assert_tool_output_has_warning, extract_text_from_result};
    use mcp_for_azure_devops_boards::azure::api_trait::MockAzureDevOpsApi;
    use mcp_for_azure_devops_boards::azure::client::AzureError;
    use mcp_for_azure_devops_boards::azure::tags::TagDefinition;
    use mcp_for_azure_devops_boards::mcp::tools::support::UNTRUSTED_CONTENT_WARNING;
    use mcp_for_azure_devops_boards::mcp::tools::tags::{ListTagsArgs, list_tags::list_tags};

    #[tokio::test]
    async fn test_list_tags_has_warning() {
        let mut mock = MockAzureDevOpsApi::new();
        mock.expect_list_tags().returning(|_, _| {
            Ok(vec![TagDefinition {
                id: "tag-1".to_string(),
                name: "bug".to_string(),
                url: None,
                last_updated: None,
            }])
        });

        let result = list_tags(
            &mock,
            ListTagsArgs {
                organization: "org".to_string(),
                project: "proj".to_string(),
            },
        )
        .await
        .unwrap();
        assert_tool_output_has_warning(&result);
    }

    #[tokio::test]
    async fn test_list_tags_api_error_propagates() {
        let mut mock = MockAzureDevOpsApi::new();
        mock.expect_list_tags()
            .returning(|_, _| Err(AzureError::ApiError("test error".to_string())));

        let result = list_tags(
            &mock,
            ListTagsArgs {
                organization: "org".to_string(),
                project: "proj".to_string(),
            },
        )
        .await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_list_tags_returns_tag_names() {
        let mut mock = MockAzureDevOpsApi::new();
        mock.expect_list_tags().returning(|_, _| {
            Ok(vec![
                TagDefinition {
                    id: "tag-1".to_string(),
                    name: "bug".to_string(),
                    url: None,
                    last_updated: None,
                },
                TagDefinition {
                    id: "tag-2".to_string(),
                    name: "feature".to_string(),
                    url: None,
                    last_updated: None,
                },
            ])
        });

        let result = list_tags(
            &mock,
            ListTagsArgs {
                organization: "org".to_string(),
                project: "proj".to_string(),
            },
        )
        .await
        .unwrap();

        let text = extract_text_from_result(&result);
        let content = text
            .strip_prefix(UNTRUSTED_CONTENT_WARNING)
            .unwrap_or(&text);
        assert!(content.contains("bug"), "Output should contain 'bug'");
        assert!(
            content.contains("feature"),
            "Output should contain 'feature'"
        );
    }
}
