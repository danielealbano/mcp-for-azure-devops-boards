#[cfg(feature = "test-support")]
mod common;

#[cfg(feature = "test-support")]
mod tests {
    use super::common::assert_tool_output_has_warning;
    use mcp_for_azure_devops_boards::azure::api_trait::MockAzureDevOpsApi;
    use mcp_for_azure_devops_boards::azure::projects::Project;
    use mcp_for_azure_devops_boards::mcp::tools::projects::{
        ListProjectsArgs, list_projects::list_projects,
    };

    #[tokio::test]
    async fn test_list_projects_has_warning() {
        let mut mock = MockAzureDevOpsApi::new();
        mock.expect_list_projects().returning(|_| {
            Ok(vec![Project {
                id: "proj-1".to_string(),
                name: "TestProject".to_string(),
                description: None,
                url: "https://dev.azure.com/org/TestProject".to_string(),
                state: "wellFormed".to_string(),
                visibility: Some("private".to_string()),
            }])
        });

        let result = list_projects(
            &mock,
            ListProjectsArgs {
                organization: "org".to_string(),
            },
        )
        .await
        .unwrap();
        assert_tool_output_has_warning(&result);
    }
}
