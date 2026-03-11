#[cfg(feature = "test-support")]
mod common;

#[cfg(feature = "test-support")]
mod tests {
    use super::common::{assert_tool_output_has_warning, extract_text_from_result};
    use mcp_for_azure_devops_boards::azure::api_trait::MockAzureDevOpsApi;
    use mcp_for_azure_devops_boards::azure::boards::WorkItemType;
    use mcp_for_azure_devops_boards::azure::client::AzureError;
    use mcp_for_azure_devops_boards::mcp::tools::support::UNTRUSTED_CONTENT_WARNING;
    use mcp_for_azure_devops_boards::mcp::tools::work_item_types::{
        ListWorkItemTypesArgs, list_work_item_types::list_work_item_types,
    };

    #[tokio::test]
    async fn test_list_work_item_types_has_warning() {
        let mut mock = MockAzureDevOpsApi::new();
        mock.expect_list_work_item_types().returning(|_, _| {
            Ok(vec![WorkItemType {
                name: "Bug".to_string(),
                description: None,
                color: None,
                icon: None,
                url: None,
                reference_name: None,
            }])
        });

        let result = list_work_item_types(
            &mock,
            ListWorkItemTypesArgs {
                organization: "org".to_string(),
                project: "proj".to_string(),
            },
        )
        .await
        .unwrap();
        assert_tool_output_has_warning(&result);
    }

    #[tokio::test]
    async fn test_list_work_item_types_api_error_propagates() {
        let mut mock = MockAzureDevOpsApi::new();
        mock.expect_list_work_item_types()
            .returning(|_, _| Err(AzureError::ApiError("test error".to_string())));

        let result = list_work_item_types(
            &mock,
            ListWorkItemTypesArgs {
                organization: "org".to_string(),
                project: "proj".to_string(),
            },
        )
        .await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_list_work_item_types_returns_type_names() {
        let mut mock = MockAzureDevOpsApi::new();
        mock.expect_list_work_item_types().returning(|_, _| {
            Ok(vec![
                WorkItemType {
                    name: "Bug".to_string(),
                    description: None,
                    color: None,
                    icon: None,
                    url: None,
                    reference_name: None,
                },
                WorkItemType {
                    name: "User Story".to_string(),
                    description: None,
                    color: None,
                    icon: None,
                    url: None,
                    reference_name: None,
                },
            ])
        });

        let result = list_work_item_types(
            &mock,
            ListWorkItemTypesArgs {
                organization: "org".to_string(),
                project: "proj".to_string(),
            },
        )
        .await
        .unwrap();

        let text = extract_text_from_result(&result);
        let content = text.strip_prefix(UNTRUSTED_CONTENT_WARNING).unwrap_or(&text);
        assert!(content.contains("Bug"), "Output should contain 'Bug'");
        assert!(
            content.contains("User Story"),
            "Output should contain 'User Story'"
        );
    }
}
