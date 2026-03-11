#[cfg(feature = "test-support")]
mod common;

#[cfg(feature = "test-support")]
mod tests {
    use super::common::{assert_tool_output_has_warning, extract_text_from_result};
    use mcp_for_azure_devops_boards::azure::api_trait::MockAzureDevOpsApi;
    use mcp_for_azure_devops_boards::azure::boards::Team;
    use mcp_for_azure_devops_boards::azure::client::AzureError;
    use mcp_for_azure_devops_boards::azure::iterations::{
        IterationAttributes, TeamSettingsIteration,
    };
    use mcp_for_azure_devops_boards::azure::teams::{TeamMember, TeamMemberIdentity};
    use mcp_for_azure_devops_boards::mcp::tools::support::UNTRUSTED_CONTENT_WARNING;
    use mcp_for_azure_devops_boards::mcp::tools::teams::{
        GetTeamArgs, GetTeamCurrentIterationArgs, ListTeamMembersArgs, ListTeamsArgs,
        get_team::get_team, get_team_current_iteration::get_team_current_iteration,
        list_team_members::list_team_members, list_teams::list_teams,
    };

    fn mock_team() -> Team {
        Team {
            id: "team-1".to_string(),
            name: "TestTeam".to_string(),
            url: "https://dev.azure.com/org/proj/_apis/projects/proj/teams/team-1".to_string(),
            description: Some("A test team".to_string()),
            default_value: None,
        }
    }

    #[tokio::test]
    async fn test_list_teams_has_warning() {
        let mut mock = MockAzureDevOpsApi::new();
        mock.expect_list_teams()
            .returning(|_, _| Ok(vec![mock_team()]));

        let result = list_teams(
            &mock,
            ListTeamsArgs {
                organization: "org".to_string(),
                project: "proj".to_string(),
            },
        )
        .await
        .unwrap();
        assert_tool_output_has_warning(&result);
    }

    #[tokio::test]
    async fn test_get_team_has_warning() {
        let mut mock = MockAzureDevOpsApi::new();
        mock.expect_get_team().returning(|_, _, _| Ok(mock_team()));

        let result = get_team(
            &mock,
            GetTeamArgs {
                organization: "org".to_string(),
                project: "proj".to_string(),
                team_id: "team-1".to_string(),
            },
        )
        .await
        .unwrap();
        assert_tool_output_has_warning(&result);
    }

    #[tokio::test]
    async fn test_list_team_members_has_warning() {
        let mut mock = MockAzureDevOpsApi::new();
        mock.expect_list_team_members().returning(|_, _, _| {
            Ok(vec![TeamMember {
                identity: TeamMemberIdentity {
                    display_name: "Test User".to_string(),
                    unique_name: "test@example.com".to_string(),
                    id: "user-1".to_string(),
                },
            }])
        });

        let result = list_team_members(
            &mock,
            ListTeamMembersArgs {
                organization: "org".to_string(),
                project: "proj".to_string(),
                team_id: "team-1".to_string(),
            },
        )
        .await
        .unwrap();
        assert_tool_output_has_warning(&result);
    }

    #[tokio::test]
    async fn test_get_team_current_iteration_has_warning() {
        let mut mock = MockAzureDevOpsApi::new();
        mock.expect_get_team_current_iteration()
            .returning(|_, _, _| {
                Ok(Some(TeamSettingsIteration {
                    id: "iter-1".to_string(),
                    name: "Sprint 1".to_string(),
                    path: "proj\\Sprint 1".to_string(),
                    attributes: IterationAttributes {
                        start_date: Some("2026-01-01T00:00:00Z".to_string()),
                        finish_date: Some("2026-01-14T00:00:00Z".to_string()),
                        time_frame: Some("current".to_string()),
                    },
                    url: "https://dev.azure.com/org/proj/_apis/work/teamsettings/iterations/iter-1"
                        .to_string(),
                }))
            });

        let result = get_team_current_iteration(
            &mock,
            GetTeamCurrentIterationArgs {
                organization: "org".to_string(),
                project: "proj".to_string(),
                team_id: "team-1".to_string(),
            },
        )
        .await
        .unwrap();
        assert_tool_output_has_warning(&result);
    }

    #[tokio::test]
    async fn test_list_teams_api_error_propagates() {
        let mut mock = MockAzureDevOpsApi::new();
        mock.expect_list_teams()
            .returning(|_, _| Err(AzureError::ApiError("test error".to_string())));

        let result = list_teams(
            &mock,
            ListTeamsArgs {
                organization: "org".to_string(),
                project: "proj".to_string(),
            },
        )
        .await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_get_team_api_error_propagates() {
        let mut mock = MockAzureDevOpsApi::new();
        mock.expect_get_team()
            .returning(|_, _, _| Err(AzureError::ApiError("test error".to_string())));

        let result = get_team(
            &mock,
            GetTeamArgs {
                organization: "org".to_string(),
                project: "proj".to_string(),
                team_id: "team-1".to_string(),
            },
        )
        .await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_list_team_members_api_error_propagates() {
        let mut mock = MockAzureDevOpsApi::new();
        mock.expect_list_team_members()
            .returning(|_, _, _| Err(AzureError::ApiError("test error".to_string())));

        let result = list_team_members(
            &mock,
            ListTeamMembersArgs {
                organization: "org".to_string(),
                project: "proj".to_string(),
                team_id: "team-1".to_string(),
            },
        )
        .await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_get_team_current_iteration_api_error_propagates() {
        let mut mock = MockAzureDevOpsApi::new();
        mock.expect_get_team_current_iteration()
            .returning(|_, _, _| Err(AzureError::ApiError("test error".to_string())));

        let result = get_team_current_iteration(
            &mock,
            GetTeamCurrentIterationArgs {
                organization: "org".to_string(),
                project: "proj".to_string(),
                team_id: "team-1".to_string(),
            },
        )
        .await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_list_teams_returns_team_names() {
        let mut mock = MockAzureDevOpsApi::new();
        mock.expect_list_teams().returning(|_, _| {
            Ok(vec![
                Team {
                    id: "team-1".to_string(),
                    name: "AlphaTeam".to_string(),
                    url: "https://dev.azure.com/org/proj/_apis/projects/proj/teams/team-1"
                        .to_string(),
                    description: Some("First team".to_string()),
                    default_value: None,
                },
                Team {
                    id: "team-2".to_string(),
                    name: "BetaTeam".to_string(),
                    url: "https://dev.azure.com/org/proj/_apis/projects/proj/teams/team-2"
                        .to_string(),
                    description: Some("Second team".to_string()),
                    default_value: None,
                },
            ])
        });

        let result = list_teams(
            &mock,
            ListTeamsArgs {
                organization: "org".to_string(),
                project: "proj".to_string(),
            },
        )
        .await
        .unwrap();
        let text = extract_text_from_result(&result);
        let content = text.strip_prefix(UNTRUSTED_CONTENT_WARNING).unwrap_or(&text);
        assert!(
            content.contains("AlphaTeam"),
            "Output should contain AlphaTeam"
        );
        assert!(
            content.contains("BetaTeam"),
            "Output should contain BetaTeam"
        );
    }

    #[tokio::test]
    async fn test_get_team_returns_team_details() {
        let mut mock = MockAzureDevOpsApi::new();
        mock.expect_get_team().returning(|_, _, _| {
            Ok(Team {
                id: "team-1".to_string(),
                name: "AlphaTeam".to_string(),
                url: "https://dev.azure.com/org/proj/_apis/projects/proj/teams/team-1".to_string(),
                description: Some("The alpha team".to_string()),
                default_value: None,
            })
        });

        let result = get_team(
            &mock,
            GetTeamArgs {
                organization: "org".to_string(),
                project: "proj".to_string(),
                team_id: "team-1".to_string(),
            },
        )
        .await
        .unwrap();
        let text = extract_text_from_result(&result);
        let content = text.strip_prefix(UNTRUSTED_CONTENT_WARNING).unwrap_or(&text);
        assert!(
            content.contains("AlphaTeam"),
            "Output should contain team name"
        );
        assert!(
            content.contains("The alpha team"),
            "Output should contain team description"
        );
    }

    #[tokio::test]
    async fn test_list_team_members_returns_csv() {
        let mut mock = MockAzureDevOpsApi::new();
        mock.expect_list_team_members().returning(|_, _, _| {
            Ok(vec![
                TeamMember {
                    identity: TeamMemberIdentity {
                        display_name: "Alice Smith".to_string(),
                        unique_name: "alice@example.com".to_string(),
                        id: "user-1".to_string(),
                    },
                },
                TeamMember {
                    identity: TeamMemberIdentity {
                        display_name: "Bob Jones".to_string(),
                        unique_name: "bob@example.com".to_string(),
                        id: "user-2".to_string(),
                    },
                },
            ])
        });

        let result = list_team_members(
            &mock,
            ListTeamMembersArgs {
                organization: "org".to_string(),
                project: "proj".to_string(),
                team_id: "team-1".to_string(),
            },
        )
        .await
        .unwrap();
        let text = extract_text_from_result(&result);
        let content = text.strip_prefix(UNTRUSTED_CONTENT_WARNING).unwrap_or(&text);
        assert!(
            content.contains("Alice Smith"),
            "Output should contain display_name"
        );
        assert!(
            content.contains("alice@example.com"),
            "Output should contain unique_name"
        );
        assert!(
            content.contains("Bob Jones"),
            "Output should contain second member display_name"
        );
        assert!(
            content.contains("bob@example.com"),
            "Output should contain second member unique_name"
        );
    }

    #[tokio::test]
    async fn test_get_team_current_iteration_no_iteration() {
        let mut mock = MockAzureDevOpsApi::new();
        mock.expect_get_team_current_iteration()
            .returning(|_, _, _| Ok(None));

        let result = get_team_current_iteration(
            &mock,
            GetTeamCurrentIterationArgs {
                organization: "org".to_string(),
                project: "proj".to_string(),
                team_id: "team-1".to_string(),
            },
        )
        .await
        .unwrap();
        let text = extract_text_from_result(&result);
        let content = text.strip_prefix(UNTRUSTED_CONTENT_WARNING).unwrap_or(&text);
        assert!(
            content.contains("No current iteration found"),
            "Output should indicate no current iteration"
        );
    }
}
