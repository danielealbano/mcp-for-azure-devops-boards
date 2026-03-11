#[cfg(feature = "test-support")]
mod common;

#[cfg(feature = "test-support")]
mod tests {
    use super::common::{assert_tool_output_has_warning, extract_text_from_result};
    use mcp_for_azure_devops_boards::azure::api_trait::MockAzureDevOpsApi;
    use mcp_for_azure_devops_boards::azure::client::AzureError;
    use mcp_for_azure_devops_boards::azure::models::WorkItem;
    use mcp_for_azure_devops_boards::mcp::tools::support::UNTRUSTED_CONTENT_WARNING;
    use mcp_for_azure_devops_boards::mcp::tools::work_items::{
        AddCommentArgs, CreateWorkItemArgs, GetWorkItemArgs, GetWorkItemsArgs, LinkWorkItemsArgs,
        QueryWorkItemsArgs, QueryWorkItemsArgsWiql, UpdateCommentArgs, UpdateWorkItemArgs,
        add_comment::add_comment, create_work_item::create_work_item, get_work_item::get_work_item,
        get_work_items::get_work_items, link_work_items::link_work_items,
        query_work_items::query_work_items, query_work_items_by_wiql::query_work_items_by_wiql,
        update_comment::update_comment, update_work_item::update_work_item,
    };
    use std::collections::HashMap;

    fn mock_work_item() -> WorkItem {
        let mut fields = HashMap::new();
        fields.insert(
            "System.Title".to_string(),
            serde_json::json!("Test Work Item"),
        );
        fields.insert("System.State".to_string(), serde_json::json!("New"));
        fields.insert("System.WorkItemType".to_string(), serde_json::json!("Bug"));
        WorkItem {
            id: 42,
            fields,
            url: Some("https://dev.azure.com/org/proj/_apis/wit/workitems/42".to_string()),
            comments: None,
        }
    }

    #[tokio::test]
    async fn test_get_work_item_has_warning() {
        let mut mock = MockAzureDevOpsApi::new();
        mock.expect_get_work_item()
            .returning(|_, _, _, _| Ok(Some(mock_work_item())));

        let result = get_work_item(
            &mock,
            GetWorkItemArgs {
                organization: "org".to_string(),
                project: "proj".to_string(),
                id: 42,
                include_latest_n_comments: None,
            },
        )
        .await
        .unwrap();
        assert_tool_output_has_warning(&result);
    }

    #[tokio::test]
    async fn test_get_work_items_has_warning() {
        let mut mock = MockAzureDevOpsApi::new();
        mock.expect_get_work_items()
            .returning(|_, _, _, _| Ok(vec![mock_work_item()]));

        let result = get_work_items(
            &mock,
            GetWorkItemsArgs {
                organization: "org".to_string(),
                project: "proj".to_string(),
                ids: vec![42],
                include_latest_n_comments: None,
            },
        )
        .await
        .unwrap();
        assert_tool_output_has_warning(&result);
    }

    #[tokio::test]
    async fn test_create_work_item_has_warning() {
        let mut mock = MockAzureDevOpsApi::new();
        mock.expect_create_work_item()
            .returning(|_, _, _, _, _| Ok(mock_work_item()));

        let result = create_work_item(
            &mock,
            CreateWorkItemArgs {
                organization: "org".to_string(),
                project: "proj".to_string(),
                work_item_type: "Bug".to_string(),
                title: "Test Bug".to_string(),
                format: "markdown".to_string(),
                description: None,
                assigned_to: None,
                area_path: None,
                iteration_path: None,
                state: None,
                board_column: None,
                board_row: None,
                priority: None,
                severity: None,
                story_points: None,
                effort: None,
                remaining_work: None,
                tags: None,
                activity: None,
                parent_id: None,
                start_date: None,
                target_date: None,
                acceptance_criteria: None,
                repro_steps: None,
                fields: None,
            },
        )
        .await
        .unwrap();
        assert_tool_output_has_warning(&result);
    }

    #[tokio::test]
    async fn test_update_work_item_has_warning() {
        let mut mock = MockAzureDevOpsApi::new();
        mock.expect_update_work_item()
            .returning(|_, _, _, _, _| Ok(mock_work_item()));

        let result = update_work_item(
            &mock,
            UpdateWorkItemArgs {
                organization: "org".to_string(),
                project: "proj".to_string(),
                id: 42,
                format: "markdown".to_string(),
                title: Some("Updated Title".to_string()),
                description: None,
                assigned_to: None,
                area_path: None,
                iteration_path: None,
                state: None,
                board_column: None,
                board_row: None,
                priority: None,
                severity: None,
                story_points: None,
                effort: None,
                remaining_work: None,
                tags: None,
                activity: None,
                start_date: None,
                target_date: None,
                acceptance_criteria: None,
                repro_steps: None,
                fields: None,
            },
        )
        .await
        .unwrap();
        assert_tool_output_has_warning(&result);
    }

    #[tokio::test]
    async fn test_query_work_items_has_warning() {
        let mut mock = MockAzureDevOpsApi::new();
        mock.expect_query_work_items()
            .returning(|_, _, _, _| Ok(vec![mock_work_item()]));

        let result = query_work_items(
            &mock,
            QueryWorkItemsArgs {
                organization: "org".to_string(),
                project: "proj".to_string(),
                area_path: None,
                iteration_path: None,
                created_date_from: None,
                created_date_to: None,
                state_change_date_from: None,
                state_change_date_to: None,
                changed_date_from: None,
                changed_date_to: None,
                include_board_column: vec![],
                include_board_row: vec![],
                include_work_item_type: vec![],
                include_state: vec!["New".to_string()],
                exclude_board_column: vec![],
                exclude_board_row: vec![],
                exclude_work_item_type: vec![],
                exclude_state: vec![],
                include_assigned_to: vec![],
                exclude_assigned_to: vec![],
                include_changed_by: vec![],
                exclude_changed_by: vec![],
                include_tags: vec![],
                exclude_tags: vec![],
                include_latest_n_comments: None,
            },
        )
        .await
        .unwrap();
        assert_tool_output_has_warning(&result);
    }

    #[tokio::test]
    async fn test_query_work_items_by_wiql_has_warning() {
        let mut mock = MockAzureDevOpsApi::new();
        mock.expect_query_work_items()
            .returning(|_, _, _, _| Ok(vec![mock_work_item()]));

        let result = query_work_items_by_wiql(
            &mock,
            QueryWorkItemsArgsWiql {
                organization: "org".to_string(),
                project: "proj".to_string(),
                query: "SELECT [System.Id] FROM WorkItems WHERE [System.State] = 'New'".to_string(),
                include_latest_n_comments: None,
            },
        )
        .await
        .unwrap();
        assert_tool_output_has_warning(&result);
    }

    #[tokio::test]
    async fn test_link_work_items_has_warning() {
        let mut mock = MockAzureDevOpsApi::new();
        mock.expect_link_work_items()
            .returning(|_, _, _, _, _| Ok(serde_json::json!({"id": 42})));

        let result = link_work_items(
            &mock,
            LinkWorkItemsArgs {
                organization: "org".to_string(),
                project: "proj".to_string(),
                source_id: 42,
                target_id: 43,
                link_type: "Related".to_string(),
            },
        )
        .await
        .unwrap();
        assert_tool_output_has_warning(&result);
    }

    #[tokio::test]
    async fn test_add_comment_has_warning() {
        let mut mock = MockAzureDevOpsApi::new();
        mock.expect_add_comment()
            .returning(|_, _, _, _, _| Ok(serde_json::json!({"id": 1, "text": "test comment"})));

        let result = add_comment(
            &mock,
            AddCommentArgs {
                organization: "org".to_string(),
                project: "proj".to_string(),
                work_item_id: 42,
                text: "A test comment".to_string(),
                format: "markdown".to_string(),
            },
        )
        .await
        .unwrap();
        assert_tool_output_has_warning(&result);
    }

    #[tokio::test]
    async fn test_update_comment_has_warning() {
        let mut mock = MockAzureDevOpsApi::new();
        mock.expect_update_comment().returning(|_, _, _, _, _, _| {
            Ok(serde_json::json!({"id": 1, "text": "updated comment"}))
        });

        let result = update_comment(
            &mock,
            UpdateCommentArgs {
                organization: "org".to_string(),
                project: "proj".to_string(),
                work_item_id: 42,
                comment_id: 1,
                text: "Updated comment".to_string(),
                format: "markdown".to_string(),
            },
        )
        .await
        .unwrap();
        assert_tool_output_has_warning(&result);
    }

    #[tokio::test]
    async fn test_get_work_item_api_error_propagates() {
        let mut mock = MockAzureDevOpsApi::new();
        mock.expect_get_work_item().returning(|_, _, _, _| {
            Err(AzureError::ApiError(
                "TF401232: Work item 999 does not exist".to_string(),
            ))
        });

        let result = get_work_item(
            &mock,
            GetWorkItemArgs {
                organization: "org".to_string(),
                project: "proj".to_string(),
                id: 999,
                include_latest_n_comments: None,
            },
        )
        .await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_get_work_items_api_error_propagates() {
        let mut mock = MockAzureDevOpsApi::new();
        mock.expect_get_work_items()
            .returning(|_, _, _, _| Err(AzureError::ApiError("test error".to_string())));

        let result = get_work_items(
            &mock,
            GetWorkItemsArgs {
                organization: "org".to_string(),
                project: "proj".to_string(),
                ids: vec![42],
                include_latest_n_comments: None,
            },
        )
        .await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_create_work_item_api_error_propagates() {
        let mut mock = MockAzureDevOpsApi::new();
        mock.expect_create_work_item()
            .returning(|_, _, _, _, _| Err(AzureError::ApiError("test error".to_string())));

        let result = create_work_item(
            &mock,
            CreateWorkItemArgs {
                organization: "org".to_string(),
                project: "proj".to_string(),
                work_item_type: "Bug".to_string(),
                title: "Test Bug".to_string(),
                format: "markdown".to_string(),
                description: None,
                assigned_to: None,
                area_path: None,
                iteration_path: None,
                state: None,
                board_column: None,
                board_row: None,
                priority: None,
                severity: None,
                story_points: None,
                effort: None,
                remaining_work: None,
                tags: None,
                activity: None,
                parent_id: None,
                start_date: None,
                target_date: None,
                acceptance_criteria: None,
                repro_steps: None,
                fields: None,
            },
        )
        .await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_update_work_item_api_error_propagates() {
        let mut mock = MockAzureDevOpsApi::new();
        mock.expect_update_work_item()
            .returning(|_, _, _, _, _| Err(AzureError::ApiError("test error".to_string())));

        let result = update_work_item(
            &mock,
            UpdateWorkItemArgs {
                organization: "org".to_string(),
                project: "proj".to_string(),
                id: 42,
                format: "markdown".to_string(),
                title: Some("Updated Title".to_string()),
                description: None,
                assigned_to: None,
                area_path: None,
                iteration_path: None,
                state: None,
                board_column: None,
                board_row: None,
                priority: None,
                severity: None,
                story_points: None,
                effort: None,
                remaining_work: None,
                tags: None,
                activity: None,
                start_date: None,
                target_date: None,
                acceptance_criteria: None,
                repro_steps: None,
                fields: None,
            },
        )
        .await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_query_work_items_api_error_propagates() {
        let mut mock = MockAzureDevOpsApi::new();
        mock.expect_query_work_items()
            .returning(|_, _, _, _| Err(AzureError::ApiError("test error".to_string())));

        let result = query_work_items(
            &mock,
            QueryWorkItemsArgs {
                organization: "org".to_string(),
                project: "proj".to_string(),
                area_path: None,
                iteration_path: None,
                created_date_from: None,
                created_date_to: None,
                state_change_date_from: None,
                state_change_date_to: None,
                changed_date_from: None,
                changed_date_to: None,
                include_board_column: vec![],
                include_board_row: vec![],
                include_work_item_type: vec![],
                include_state: vec![],
                exclude_board_column: vec![],
                exclude_board_row: vec![],
                exclude_work_item_type: vec![],
                exclude_state: vec![],
                include_assigned_to: vec![],
                exclude_assigned_to: vec![],
                include_changed_by: vec![],
                exclude_changed_by: vec![],
                include_tags: vec![],
                exclude_tags: vec![],
                include_latest_n_comments: None,
            },
        )
        .await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_query_work_items_by_wiql_api_error_propagates() {
        let mut mock = MockAzureDevOpsApi::new();
        mock.expect_query_work_items()
            .returning(|_, _, _, _| Err(AzureError::ApiError("test error".to_string())));

        let result = query_work_items_by_wiql(
            &mock,
            QueryWorkItemsArgsWiql {
                organization: "org".to_string(),
                project: "proj".to_string(),
                query: "SELECT [System.Id] FROM WorkItems WHERE [System.State] = 'New'"
                    .to_string(),
                include_latest_n_comments: None,
            },
        )
        .await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_link_work_items_api_error_propagates() {
        let mut mock = MockAzureDevOpsApi::new();
        mock.expect_link_work_items()
            .returning(|_, _, _, _, _| Err(AzureError::ApiError("test error".to_string())));

        let result = link_work_items(
            &mock,
            LinkWorkItemsArgs {
                organization: "org".to_string(),
                project: "proj".to_string(),
                source_id: 42,
                target_id: 43,
                link_type: "Related".to_string(),
            },
        )
        .await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_add_comment_api_error_propagates() {
        let mut mock = MockAzureDevOpsApi::new();
        mock.expect_add_comment()
            .returning(|_, _, _, _, _| Err(AzureError::ApiError("test error".to_string())));

        let result = add_comment(
            &mock,
            AddCommentArgs {
                organization: "org".to_string(),
                project: "proj".to_string(),
                work_item_id: 42,
                text: "A test comment".to_string(),
                format: "markdown".to_string(),
            },
        )
        .await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_update_comment_api_error_propagates() {
        let mut mock = MockAzureDevOpsApi::new();
        mock.expect_update_comment()
            .returning(|_, _, _, _, _, _| Err(AzureError::ApiError("test error".to_string())));

        let result = update_comment(
            &mock,
            UpdateCommentArgs {
                organization: "org".to_string(),
                project: "proj".to_string(),
                work_item_id: 42,
                comment_id: 1,
                text: "Updated comment".to_string(),
                format: "markdown".to_string(),
            },
        )
        .await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_get_work_item_returns_csv_with_fields() {
        let mut mock = MockAzureDevOpsApi::new();
        mock.expect_get_work_item()
            .returning(|_, _, _, _| Ok(Some(mock_work_item())));

        let result = get_work_item(
            &mock,
            GetWorkItemArgs {
                organization: "org".to_string(),
                project: "proj".to_string(),
                id: 42,
                include_latest_n_comments: None,
            },
        )
        .await
        .unwrap();

        let text = extract_text_from_result(&result);
        let content = text.strip_prefix(UNTRUSTED_CONTENT_WARNING).unwrap_or(&text);
        assert!(content.contains("42"), "Output should contain work item id");
        assert!(content.contains("Bug"), "Output should contain Type");
        assert!(
            content.contains("Test Work Item"),
            "Output should contain Title"
        );
    }

    #[tokio::test]
    async fn test_get_work_item_not_found_returns_message() {
        let mut mock = MockAzureDevOpsApi::new();
        mock.expect_get_work_item()
            .returning(|_, _, _, _| Ok(None));

        let result = get_work_item(
            &mock,
            GetWorkItemArgs {
                organization: "org".to_string(),
                project: "proj".to_string(),
                id: 999,
                include_latest_n_comments: None,
            },
        )
        .await
        .unwrap();

        let text = extract_text_from_result(&result);
        let content = text.strip_prefix(UNTRUSTED_CONTENT_WARNING).unwrap_or(&text);
        assert!(
            content.contains("Work item not found"),
            "Output should contain 'Work item not found'"
        );
    }

    #[tokio::test]
    async fn test_get_work_items_returns_csv() {
        let mut mock = MockAzureDevOpsApi::new();
        mock.expect_get_work_items().returning(|_, _, _, _| {
            let mut fields1 = HashMap::new();
            fields1.insert("System.Title".to_string(), serde_json::json!("Item One"));
            fields1.insert("System.State".to_string(), serde_json::json!("New"));
            fields1.insert(
                "System.WorkItemType".to_string(),
                serde_json::json!("Bug"),
            );

            let mut fields2 = HashMap::new();
            fields2.insert("System.Title".to_string(), serde_json::json!("Item Two"));
            fields2.insert("System.State".to_string(), serde_json::json!("Active"));
            fields2.insert(
                "System.WorkItemType".to_string(),
                serde_json::json!("User Story"),
            );

            Ok(vec![
                WorkItem {
                    id: 1,
                    fields: fields1,
                    url: None,
                    comments: None,
                },
                WorkItem {
                    id: 2,
                    fields: fields2,
                    url: None,
                    comments: None,
                },
            ])
        });

        let result = get_work_items(
            &mock,
            GetWorkItemsArgs {
                organization: "org".to_string(),
                project: "proj".to_string(),
                ids: vec![1, 2],
                include_latest_n_comments: None,
            },
        )
        .await
        .unwrap();

        let text = extract_text_from_result(&result);
        let content = text.strip_prefix(UNTRUSTED_CONTENT_WARNING).unwrap_or(&text);
        assert!(content.contains("Item One"), "Output should contain 'Item One'");
        assert!(content.contains("Item Two"), "Output should contain 'Item Two'");
    }

    #[tokio::test]
    async fn test_get_work_items_empty_ids_returns_message() {
        let mock = MockAzureDevOpsApi::new();

        let result = get_work_items(
            &mock,
            GetWorkItemsArgs {
                organization: "org".to_string(),
                project: "proj".to_string(),
                ids: vec![],
                include_latest_n_comments: None,
            },
        )
        .await
        .unwrap();

        let text = extract_text_from_result(&result);
        let content = text.strip_prefix(UNTRUSTED_CONTENT_WARNING).unwrap_or(&text);
        assert!(
            content.contains("No work items found"),
            "Output should contain 'No work items found'"
        );
    }

    #[tokio::test]
    async fn test_create_work_item_returns_compact_json() {
        let mut mock = MockAzureDevOpsApi::new();
        mock.expect_create_work_item()
            .returning(|_, _, _, _, _| Ok(mock_work_item()));

        let result = create_work_item(
            &mock,
            CreateWorkItemArgs {
                organization: "org".to_string(),
                project: "proj".to_string(),
                work_item_type: "Bug".to_string(),
                title: "Test Bug".to_string(),
                format: "markdown".to_string(),
                description: None,
                assigned_to: None,
                area_path: None,
                iteration_path: None,
                state: None,
                board_column: None,
                board_row: None,
                priority: None,
                severity: None,
                story_points: None,
                effort: None,
                remaining_work: None,
                tags: None,
                activity: None,
                parent_id: None,
                start_date: None,
                target_date: None,
                acceptance_criteria: None,
                repro_steps: None,
                fields: None,
            },
        )
        .await
        .unwrap();

        let text = extract_text_from_result(&result);
        let content = text.strip_prefix(UNTRUSTED_CONTENT_WARNING).unwrap_or(&text);
        assert!(content.contains("42"), "Output should contain work item id");
    }

    #[tokio::test]
    async fn test_update_work_item_returns_compact_json() {
        let mut mock = MockAzureDevOpsApi::new();
        mock.expect_update_work_item()
            .returning(|_, _, _, _, _| Ok(mock_work_item()));

        let result = update_work_item(
            &mock,
            UpdateWorkItemArgs {
                organization: "org".to_string(),
                project: "proj".to_string(),
                id: 42,
                format: "markdown".to_string(),
                title: Some("Updated Title".to_string()),
                description: None,
                assigned_to: None,
                area_path: None,
                iteration_path: None,
                state: None,
                board_column: None,
                board_row: None,
                priority: None,
                severity: None,
                story_points: None,
                effort: None,
                remaining_work: None,
                tags: None,
                activity: None,
                start_date: None,
                target_date: None,
                acceptance_criteria: None,
                repro_steps: None,
                fields: None,
            },
        )
        .await
        .unwrap();

        let text = extract_text_from_result(&result);
        let content = text.strip_prefix(UNTRUSTED_CONTENT_WARNING).unwrap_or(&text);
        assert!(content.contains("42"), "Output should contain work item id");
    }

    #[tokio::test]
    async fn test_query_work_items_returns_csv() {
        let mut mock = MockAzureDevOpsApi::new();
        mock.expect_query_work_items()
            .returning(|_, _, _, _| Ok(vec![mock_work_item()]));

        let result = query_work_items(
            &mock,
            QueryWorkItemsArgs {
                organization: "org".to_string(),
                project: "proj".to_string(),
                area_path: None,
                iteration_path: None,
                created_date_from: None,
                created_date_to: None,
                state_change_date_from: None,
                state_change_date_to: None,
                changed_date_from: None,
                changed_date_to: None,
                include_board_column: vec![],
                include_board_row: vec![],
                include_work_item_type: vec![],
                include_state: vec![],
                exclude_board_column: vec![],
                exclude_board_row: vec![],
                exclude_work_item_type: vec![],
                exclude_state: vec![],
                include_assigned_to: vec![],
                exclude_assigned_to: vec![],
                include_changed_by: vec![],
                exclude_changed_by: vec![],
                include_tags: vec![],
                exclude_tags: vec![],
                include_latest_n_comments: None,
            },
        )
        .await
        .unwrap();

        let text = extract_text_from_result(&result);
        let content = text.strip_prefix(UNTRUSTED_CONTENT_WARNING).unwrap_or(&text);
        assert!(content.contains("42"), "Output should contain work item id");
        assert!(
            content.contains("Test Work Item"),
            "Output should contain work item title"
        );
    }

    #[tokio::test]
    async fn test_query_work_items_empty_returns_message() {
        let mut mock = MockAzureDevOpsApi::new();
        mock.expect_query_work_items()
            .returning(|_, _, _, _| Ok(vec![]));

        let result = query_work_items(
            &mock,
            QueryWorkItemsArgs {
                organization: "org".to_string(),
                project: "proj".to_string(),
                area_path: None,
                iteration_path: None,
                created_date_from: None,
                created_date_to: None,
                state_change_date_from: None,
                state_change_date_to: None,
                changed_date_from: None,
                changed_date_to: None,
                include_board_column: vec![],
                include_board_row: vec![],
                include_work_item_type: vec![],
                include_state: vec![],
                exclude_board_column: vec![],
                exclude_board_row: vec![],
                exclude_work_item_type: vec![],
                exclude_state: vec![],
                include_assigned_to: vec![],
                exclude_assigned_to: vec![],
                include_changed_by: vec![],
                exclude_changed_by: vec![],
                include_tags: vec![],
                exclude_tags: vec![],
                include_latest_n_comments: None,
            },
        )
        .await
        .unwrap();

        let text = extract_text_from_result(&result);
        let content = text.strip_prefix(UNTRUSTED_CONTENT_WARNING).unwrap_or(&text);
        assert!(
            content.contains("No work items found"),
            "Output should contain 'No work items found'"
        );
    }

    #[tokio::test]
    async fn test_query_work_items_by_wiql_returns_csv() {
        let mut mock = MockAzureDevOpsApi::new();
        mock.expect_query_work_items()
            .returning(|_, _, _, _| Ok(vec![mock_work_item()]));

        let result = query_work_items_by_wiql(
            &mock,
            QueryWorkItemsArgsWiql {
                organization: "org".to_string(),
                project: "proj".to_string(),
                query: "SELECT [System.Id] FROM WorkItems WHERE [System.State] = 'New'"
                    .to_string(),
                include_latest_n_comments: None,
            },
        )
        .await
        .unwrap();

        let text = extract_text_from_result(&result);
        let content = text.strip_prefix(UNTRUSTED_CONTENT_WARNING).unwrap_or(&text);
        assert!(content.contains("42"), "Output should contain work item id");
        assert!(
            content.contains("Test Work Item"),
            "Output should contain work item title"
        );
    }

    #[tokio::test]
    async fn test_link_work_items_returns_compact_json() {
        let mut mock = MockAzureDevOpsApi::new();
        mock.expect_link_work_items()
            .returning(|_, _, _, _, _| Ok(serde_json::json!({"id": 42})));

        let result = link_work_items(
            &mock,
            LinkWorkItemsArgs {
                organization: "org".to_string(),
                project: "proj".to_string(),
                source_id: 42,
                target_id: 43,
                link_type: "Related".to_string(),
            },
        )
        .await
        .unwrap();

        let text = extract_text_from_result(&result);
        let content = text.strip_prefix(UNTRUSTED_CONTENT_WARNING).unwrap_or(&text);
        assert!(content.contains("42"), "Output should contain result data");
    }

    #[tokio::test]
    async fn test_add_comment_returns_compact_json() {
        let mut mock = MockAzureDevOpsApi::new();
        mock.expect_add_comment()
            .returning(|_, _, _, _, _| Ok(serde_json::json!({"id": 1, "text": "test comment"})));

        let result = add_comment(
            &mock,
            AddCommentArgs {
                organization: "org".to_string(),
                project: "proj".to_string(),
                work_item_id: 42,
                text: "A test comment".to_string(),
                format: "markdown".to_string(),
            },
        )
        .await
        .unwrap();

        let text = extract_text_from_result(&result);
        let content = text.strip_prefix(UNTRUSTED_CONTENT_WARNING).unwrap_or(&text);
        assert!(content.contains("test comment"), "Output should contain comment text");
    }

    #[tokio::test]
    async fn test_update_comment_returns_compact_json() {
        let mut mock = MockAzureDevOpsApi::new();
        mock.expect_update_comment().returning(|_, _, _, _, _, _| {
            Ok(serde_json::json!({"id": 1, "text": "updated comment"}))
        });

        let result = update_comment(
            &mock,
            UpdateCommentArgs {
                organization: "org".to_string(),
                project: "proj".to_string(),
                work_item_id: 42,
                comment_id: 1,
                text: "Updated comment".to_string(),
                format: "markdown".to_string(),
            },
        )
        .await
        .unwrap();

        let text = extract_text_from_result(&result);
        let content = text.strip_prefix(UNTRUSTED_CONTENT_WARNING).unwrap_or(&text);
        assert!(
            content.contains("updated comment"),
            "Output should contain updated comment text"
        );
    }
}
