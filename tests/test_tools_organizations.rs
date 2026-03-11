#[cfg(feature = "test-support")]
mod common;

#[cfg(feature = "test-support")]
mod tests {
    use super::common::assert_tool_output_has_warning;
    use mcp_for_azure_devops_boards::azure::api_trait::MockAzureDevOpsApi;
    use mcp_for_azure_devops_boards::azure::organizations::{Organization, Profile};
    use mcp_for_azure_devops_boards::mcp::tools::organizations::{
        GetCurrentUserArgs, ListOrganizationsArgs, get_current_user::get_current_user,
        list_organizations::list_organizations,
    };

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
}
