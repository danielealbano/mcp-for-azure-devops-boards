#[cfg(feature = "test-support")]
mod common;

#[cfg(feature = "test-support")]
mod tests {
    use super::common::assert_tool_output_has_warning;
    use mcp_for_azure_devops_boards::azure::api_trait::MockAzureDevOpsApi;
    use mcp_for_azure_devops_boards::azure::client::AzureError;
    use mcp_for_azure_devops_boards::azure::models::WorkItem;
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
}
