#[cfg(feature = "test-support")]
mod common;

#[cfg(feature = "test-support")]
mod tests {
    use super::common::assert_tool_output_has_warning;
    use mcp_for_azure_devops_boards::azure::api_trait::MockAzureDevOpsApi;
    use mcp_for_azure_devops_boards::azure::boards::WorkItemType;
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
}
