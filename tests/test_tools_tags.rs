#[cfg(feature = "test-support")]
mod common;

#[cfg(feature = "test-support")]
mod tests {
    use super::common::assert_tool_output_has_warning;
    use mcp_for_azure_devops_boards::azure::api_trait::MockAzureDevOpsApi;
    use mcp_for_azure_devops_boards::azure::tags::TagDefinition;
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
}
