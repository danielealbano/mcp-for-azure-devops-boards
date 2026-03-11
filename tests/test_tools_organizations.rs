#[cfg(feature = "test-support")]
mod common;

#[cfg(feature = "test-support")]
mod tests {
    use super::common::{assert_tool_output_has_warning, extract_text_from_result};
    use mcp_for_azure_devops_boards::azure::api_trait::MockAzureDevOpsApi;
    use mcp_for_azure_devops_boards::azure::client::AzureError;
    use mcp_for_azure_devops_boards::azure::organizations::{Organization, Profile};
    use mcp_for_azure_devops_boards::mcp::tools::organizations::{
        GetCurrentUserArgs, ListOrganizationsArgs, get_current_user::get_current_user,
        list_organizations::list_organizations,
    };
    use mcp_for_azure_devops_boards::mcp::tools::support::UNTRUSTED_CONTENT_WARNING;

    fn mock_profile() -> Profile {
        Profile {
            id: "member-1".to_string(),
            display_name: "Test User".to_string(),
            email_address: "test@example.com".to_string(),
            public_alias: "testuser".to_string(),
        }
    }

    #[tokio::test]
    async fn test_list_organizations_has_warning() {
        let mut mock = MockAzureDevOpsApi::new();
        mock.expect_get_profile().returning(|| Ok(mock_profile()));
        mock.expect_list_organizations().returning(|_| {
            Ok(vec![Organization {
                account_id: "acc-1".to_string(),
                account_uri: "https://dev.azure.com/org1".to_string(),
                account_name: "org1".to_string(),
            }])
        });

        let result = list_organizations(&mock, ListOrganizationsArgs {})
            .await
            .unwrap();
        assert_tool_output_has_warning(&result);
    }

    #[tokio::test]
    async fn test_get_current_user_has_warning() {
        let mut mock = MockAzureDevOpsApi::new();
        mock.expect_get_profile().returning(|| Ok(mock_profile()));

        let result = get_current_user(&mock, GetCurrentUserArgs {})
            .await
            .unwrap();
        assert_tool_output_has_warning(&result);
    }

    #[tokio::test]
    async fn test_list_organizations_get_profile_error_propagates() {
        let mut mock = MockAzureDevOpsApi::new();
        mock.expect_get_profile()
            .returning(|| Err(AzureError::ApiError("test error".to_string())));

        let result = list_organizations(&mock, ListOrganizationsArgs {}).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_list_organizations_list_orgs_error_propagates() {
        let mut mock = MockAzureDevOpsApi::new();
        mock.expect_get_profile().returning(|| Ok(mock_profile()));
        mock.expect_list_organizations()
            .returning(|_| Err(AzureError::ApiError("test error".to_string())));

        let result = list_organizations(&mock, ListOrganizationsArgs {}).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_get_current_user_api_error_propagates() {
        let mut mock = MockAzureDevOpsApi::new();
        mock.expect_get_profile()
            .returning(|| Err(AzureError::ApiError("test error".to_string())));

        let result = get_current_user(&mock, GetCurrentUserArgs {}).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_list_organizations_returns_org_names() {
        let mut mock = MockAzureDevOpsApi::new();
        mock.expect_get_profile().returning(|| Ok(mock_profile()));
        mock.expect_list_organizations().returning(|_| {
            Ok(vec![
                Organization {
                    account_id: "acc-1".to_string(),
                    account_uri: "https://dev.azure.com/org1".to_string(),
                    account_name: "org1".to_string(),
                },
                Organization {
                    account_id: "acc-2".to_string(),
                    account_uri: "https://dev.azure.com/org2".to_string(),
                    account_name: "org2".to_string(),
                },
            ])
        });

        let result = list_organizations(&mock, ListOrganizationsArgs {})
            .await
            .unwrap();
        let text = extract_text_from_result(&result);
        let content = text.strip_prefix(UNTRUSTED_CONTENT_WARNING).unwrap_or(&text);
        assert!(content.contains("org1"), "Output should contain org1");
        assert!(content.contains("org2"), "Output should contain org2");
    }

    #[tokio::test]
    async fn test_get_current_user_returns_csv_with_profile() {
        let mut mock = MockAzureDevOpsApi::new();
        mock.expect_get_profile().returning(|| Ok(mock_profile()));

        let result = get_current_user(&mock, GetCurrentUserArgs {})
            .await
            .unwrap();
        let text = extract_text_from_result(&result);
        let content = text.strip_prefix(UNTRUSTED_CONTENT_WARNING).unwrap_or(&text);
        assert!(
            content.contains("Test User"),
            "Output should contain display_name"
        );
        assert!(
            content.contains("test@example.com"),
            "Output should contain email_address"
        );
    }
}
