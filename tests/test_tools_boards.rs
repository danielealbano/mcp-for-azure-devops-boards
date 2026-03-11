#[cfg(feature = "test-support")]
mod common;

#[cfg(feature = "test-support")]
mod tests {
    use super::common::{assert_tool_output_has_warning, extract_text_from_result};
    use mcp_for_azure_devops_boards::azure::api_trait::MockAzureDevOpsApi;
    use mcp_for_azure_devops_boards::azure::boards::{
        BoardColumn, BoardDetail, BoardField, BoardFields, BoardRow, BoardSummary,
    };
    use mcp_for_azure_devops_boards::azure::client::AzureError;
    use mcp_for_azure_devops_boards::mcp::tools::support::UNTRUSTED_CONTENT_WARNING;
    use mcp_for_azure_devops_boards::mcp::tools::teams::boards::{
        GetBoardArgs, ListBoardColumnsArgs, ListBoardRowsArgs, ListBoardsArgs,
        get_team_board::get_team_board, list_board_columns::list_board_columns,
        list_board_rows::list_board_rows, list_team_boards::list_team_boards,
    };

    #[tokio::test]
    async fn test_list_team_boards_has_warning() {
        let mut mock = MockAzureDevOpsApi::new();
        mock.expect_list_boards().returning(|_, _, _| {
            Ok(vec![BoardSummary {
                id: "board-1".to_string(),
                name: "TestBoard".to_string(),
                url: "https://dev.azure.com/org/proj/_apis/work/boards/board-1".to_string(),
            }])
        });

        let result = list_team_boards(
            &mock,
            ListBoardsArgs {
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
    async fn test_get_team_board_has_warning() {
        let mut mock = MockAzureDevOpsApi::new();
        mock.expect_get_board().returning(|_, _, _, _| {
            Ok(BoardDetail {
                id: "board-1".to_string(),
                name: "TestBoard".to_string(),
                url: "https://dev.azure.com/org/proj/_apis/work/boards/board-1".to_string(),
                revision: Some(1),
                columns: Some(vec![]),
                rows: Some(vec![]),
                is_valid: Some(true),
                allowed_mappings: None,
                can_edit: Some(true),
                fields: Some(BoardFields {
                    column_field: BoardField {
                        reference_name: "System.BoardColumn".to_string(),
                        url: "https://example.com".to_string(),
                    },
                    row_field: BoardField {
                        reference_name: "System.BoardLane".to_string(),
                        url: "https://example.com".to_string(),
                    },
                    done_field: BoardField {
                        reference_name: "System.BoardColumnDone".to_string(),
                        url: "https://example.com".to_string(),
                    },
                }),
            })
        });

        let result = get_team_board(
            &mock,
            GetBoardArgs {
                organization: "org".to_string(),
                project: "proj".to_string(),
                team_id: "team-1".to_string(),
                board_id: "board-1".to_string(),
            },
        )
        .await
        .unwrap();
        assert_tool_output_has_warning(&result);
    }

    #[tokio::test]
    async fn test_list_board_columns_has_warning() {
        let mut mock = MockAzureDevOpsApi::new();
        mock.expect_list_board_columns().returning(|_, _, _, _| {
            Ok(vec![BoardColumn {
                id: "col-1".to_string(),
                name: "To Do".to_string(),
                item_limit: 5,
                state_mappings: serde_json::json!({"Bug": "New"}),
                column_type: "inProgress".to_string(),
                is_split: Some(false),
                description: None,
            }])
        });

        let result = list_board_columns(
            &mock,
            ListBoardColumnsArgs {
                organization: "org".to_string(),
                project: "proj".to_string(),
                team_id: "team-1".to_string(),
                board_id: "board-1".to_string(),
            },
        )
        .await
        .unwrap();
        assert_tool_output_has_warning(&result);
    }

    #[tokio::test]
    async fn test_list_board_rows_has_warning() {
        let mut mock = MockAzureDevOpsApi::new();
        mock.expect_list_board_rows().returning(|_, _, _, _| {
            Ok(vec![BoardRow {
                id: "row-1".to_string(),
                name: Some("Default".to_string()),
                color: None,
            }])
        });

        let result = list_board_rows(
            &mock,
            ListBoardRowsArgs {
                organization: "org".to_string(),
                project: "proj".to_string(),
                team_id: "team-1".to_string(),
                board_id: "board-1".to_string(),
            },
        )
        .await
        .unwrap();
        assert_tool_output_has_warning(&result);
    }

    #[tokio::test]
    async fn test_list_team_boards_api_error_propagates() {
        let mut mock = MockAzureDevOpsApi::new();
        mock.expect_list_boards()
            .returning(|_, _, _| Err(AzureError::ApiError("test error".to_string())));

        let result = list_team_boards(
            &mock,
            ListBoardsArgs {
                organization: "org".to_string(),
                project: "proj".to_string(),
                team_id: "team-1".to_string(),
            },
        )
        .await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_get_team_board_api_error_propagates() {
        let mut mock = MockAzureDevOpsApi::new();
        mock.expect_get_board()
            .returning(|_, _, _, _| Err(AzureError::ApiError("test error".to_string())));

        let result = get_team_board(
            &mock,
            GetBoardArgs {
                organization: "org".to_string(),
                project: "proj".to_string(),
                team_id: "team-1".to_string(),
                board_id: "board-1".to_string(),
            },
        )
        .await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_list_board_columns_api_error_propagates() {
        let mut mock = MockAzureDevOpsApi::new();
        mock.expect_list_board_columns()
            .returning(|_, _, _, _| Err(AzureError::ApiError("test error".to_string())));

        let result = list_board_columns(
            &mock,
            ListBoardColumnsArgs {
                organization: "org".to_string(),
                project: "proj".to_string(),
                team_id: "team-1".to_string(),
                board_id: "board-1".to_string(),
            },
        )
        .await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_list_board_rows_api_error_propagates() {
        let mut mock = MockAzureDevOpsApi::new();
        mock.expect_list_board_rows()
            .returning(|_, _, _, _| Err(AzureError::ApiError("test error".to_string())));

        let result = list_board_rows(
            &mock,
            ListBoardRowsArgs {
                organization: "org".to_string(),
                project: "proj".to_string(),
                team_id: "team-1".to_string(),
                board_id: "board-1".to_string(),
            },
        )
        .await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_list_team_boards_returns_board_names() {
        let mut mock = MockAzureDevOpsApi::new();
        mock.expect_list_boards().returning(|_, _, _| {
            Ok(vec![BoardSummary {
                id: "board-1".to_string(),
                name: "SprintBoard".to_string(),
                url: "https://dev.azure.com/org/proj/_apis/work/boards/board-1".to_string(),
            }])
        });

        let result = list_team_boards(
            &mock,
            ListBoardsArgs {
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
            content.contains("SprintBoard"),
            "Output should contain board name"
        );
    }

    #[tokio::test]
    async fn test_get_team_board_returns_compact_json() {
        let mut mock = MockAzureDevOpsApi::new();
        mock.expect_get_board().returning(|_, _, _, _| {
            Ok(BoardDetail {
                id: "board-42".to_string(),
                name: "MyBoard".to_string(),
                url: "https://dev.azure.com/org/proj/_apis/work/boards/board-42".to_string(),
                revision: Some(1),
                columns: Some(vec![]),
                rows: Some(vec![]),
                is_valid: Some(true),
                allowed_mappings: None,
                can_edit: Some(true),
                fields: Some(BoardFields {
                    column_field: BoardField {
                        reference_name: "System.BoardColumn".to_string(),
                        url: "https://example.com".to_string(),
                    },
                    row_field: BoardField {
                        reference_name: "System.BoardLane".to_string(),
                        url: "https://example.com".to_string(),
                    },
                    done_field: BoardField {
                        reference_name: "System.BoardColumnDone".to_string(),
                        url: "https://example.com".to_string(),
                    },
                }),
            })
        });

        let result = get_team_board(
            &mock,
            GetBoardArgs {
                organization: "org".to_string(),
                project: "proj".to_string(),
                team_id: "team-1".to_string(),
                board_id: "board-42".to_string(),
            },
        )
        .await
        .unwrap();
        let text = extract_text_from_result(&result);
        let content = text.strip_prefix(UNTRUSTED_CONTENT_WARNING).unwrap_or(&text);
        assert!(
            content.contains("board-42"),
            "Output should contain board id"
        );
        assert!(
            content.contains("MyBoard"),
            "Output should contain board name"
        );
    }

    #[tokio::test]
    async fn test_list_board_columns_returns_csv() {
        let mut mock = MockAzureDevOpsApi::new();
        mock.expect_list_board_columns().returning(|_, _, _, _| {
            Ok(vec![BoardColumn {
                id: "col-1".to_string(),
                name: "Doing".to_string(),
                item_limit: 10,
                state_mappings: serde_json::json!({"Bug": "Active"}),
                column_type: "inProgress".to_string(),
                is_split: Some(true),
                description: None,
            }])
        });

        let result = list_board_columns(
            &mock,
            ListBoardColumnsArgs {
                organization: "org".to_string(),
                project: "proj".to_string(),
                team_id: "team-1".to_string(),
                board_id: "board-1".to_string(),
            },
        )
        .await
        .unwrap();
        let text = extract_text_from_result(&result);
        let content = text.strip_prefix(UNTRUSTED_CONTENT_WARNING).unwrap_or(&text);
        assert!(content.contains("name"), "CSV should contain name header");
        assert!(
            content.contains("item_limit"),
            "CSV should contain item_limit header"
        );
        assert!(
            content.contains("is_split"),
            "CSV should contain is_split header"
        );
        assert!(
            content.contains("column_type"),
            "CSV should contain column_type header"
        );
        assert!(
            content.contains("Doing"),
            "CSV should contain column name value"
        );
    }

    #[tokio::test]
    async fn test_list_board_rows_returns_row_names() {
        let mut mock = MockAzureDevOpsApi::new();
        mock.expect_list_board_rows().returning(|_, _, _, _| {
            Ok(vec![BoardRow {
                id: "row-1".to_string(),
                name: Some("Expedite".to_string()),
                color: None,
            }])
        });

        let result = list_board_rows(
            &mock,
            ListBoardRowsArgs {
                organization: "org".to_string(),
                project: "proj".to_string(),
                team_id: "team-1".to_string(),
                board_id: "board-1".to_string(),
            },
        )
        .await
        .unwrap();
        let text = extract_text_from_result(&result);
        let content = text.strip_prefix(UNTRUSTED_CONTENT_WARNING).unwrap_or(&text);
        assert!(
            content.contains("Expedite"),
            "Output should contain row name"
        );
    }
}
