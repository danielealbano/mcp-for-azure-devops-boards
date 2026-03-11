#[cfg(feature = "test-support")]
mod common;

#[cfg(feature = "test-support")]
mod tests {
    use super::common::assert_tool_output_has_warning;
    use mcp_for_azure_devops_boards::azure::api_trait::MockAzureDevOpsApi;
    use mcp_for_azure_devops_boards::azure::classification_nodes::ClassificationNode;
    use mcp_for_azure_devops_boards::mcp::tools::classification_nodes::{
        ListAreaPathsArgs, ListIterationPathsArgs, list_area_paths::list_area_paths,
        list_iteration_paths::list_iteration_paths,
    };

    fn mock_classification_node() -> ClassificationNode {
        ClassificationNode {
            id: 1,
            identifier: "node-1".to_string(),
            name: "TestProject".to_string(),
            path: "\\TestProject\\Area".to_string(),
            structure_type: "area".to_string(),
            children: None,
            has_children: Some(false),
        }
    }

    #[tokio::test]
    async fn test_list_area_paths_has_warning() {
        let mut mock = MockAzureDevOpsApi::new();
        mock.expect_list_area_paths()
            .withf(|_, _, _, _| true)
            .returning(|_, _, _, _| Ok(mock_classification_node()));

        let result = list_area_paths(
            &mock,
            ListAreaPathsArgs {
                organization: "org".to_string(),
                project: "proj".to_string(),
                parent_path: None,
            },
        )
        .await
        .unwrap();
        assert_tool_output_has_warning(&result);
    }

    #[tokio::test]
    async fn test_list_iteration_paths_has_warning() {
        let mut mock = MockAzureDevOpsApi::new();
        mock.expect_list_iteration_paths()
            .withf(|_, _, _, _| true)
            .returning(|_, _, _, _| {
                Ok(ClassificationNode {
                    id: 2,
                    identifier: "node-2".to_string(),
                    name: "TestProject".to_string(),
                    path: "\\TestProject\\Iteration".to_string(),
                    structure_type: "iteration".to_string(),
                    children: None,
                    has_children: Some(false),
                })
            });

        let result = list_iteration_paths(
            &mock,
            ListIterationPathsArgs {
                organization: "org".to_string(),
                project: "proj".to_string(),
                team_id: None,
                timeframe: None,
            },
        )
        .await
        .unwrap();
        assert_tool_output_has_warning(&result);
    }
}
