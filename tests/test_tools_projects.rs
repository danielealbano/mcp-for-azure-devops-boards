#[cfg(feature = "test-support")]
mod common;

#[cfg(feature = "test-support")]
mod tests {
    use super::common::{assert_tool_output_has_warning, extract_text_from_result};
    use mcp_for_azure_devops_boards::azure::api_trait::MockAzureDevOpsApi;
    use mcp_for_azure_devops_boards::azure::client::AzureError;
    use mcp_for_azure_devops_boards::azure::projects::Project;
    use mcp_for_azure_devops_boards::mcp::tools::projects::{
        ListProjectsArgs, list_projects::list_projects,
    };
    use mcp_for_azure_devops_boards::mcp::tools::support::UNTRUSTED_CONTENT_WARNING;

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

    #[tokio::test]
    async fn test_list_projects_api_error_propagates() {
        let mut mock = MockAzureDevOpsApi::new();
        mock.expect_list_projects()
            .returning(|_| Err(AzureError::ApiError("test error".to_string())));

        let result = list_projects(
            &mock,
            ListProjectsArgs {
                organization: "org".to_string(),
            },
        )
        .await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_list_projects_returns_project_names() {
        let mut mock = MockAzureDevOpsApi::new();
        mock.expect_list_projects().returning(|_| {
            Ok(vec![
                Project {
                    id: "proj-1".to_string(),
                    name: "ProjectAlpha".to_string(),
                    description: None,
                    url: "https://dev.azure.com/org/ProjectAlpha".to_string(),
                    state: "wellFormed".to_string(),
                    visibility: Some("private".to_string()),
                },
                Project {
                    id: "proj-2".to_string(),
                    name: "ProjectBeta".to_string(),
                    description: Some("Second project".to_string()),
                    url: "https://dev.azure.com/org/ProjectBeta".to_string(),
                    state: "wellFormed".to_string(),
                    visibility: Some("private".to_string()),
                },
            ])
        });

        let result = list_projects(
            &mock,
            ListProjectsArgs {
                organization: "org".to_string(),
            },
        )
        .await
        .unwrap();
        let text = extract_text_from_result(&result);
        let content = text.strip_prefix(UNTRUSTED_CONTENT_WARNING).unwrap_or(&text);
        assert!(
            content.contains("ProjectAlpha"),
            "Output should contain ProjectAlpha"
        );
        assert!(
            content.contains("ProjectBeta"),
            "Output should contain ProjectBeta"
        );
    }
}
