use crate::azure::client::{AzureDevOpsClient, AzureError};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct ClassificationNode {
    pub id: i32,
    pub identifier: String,
    pub name: String,
    pub path: String,
    #[serde(rename = "structureType")]
    pub structure_type: String,
    #[serde(default)]
    pub children: Option<Vec<ClassificationNode>>,
    #[serde(default)]
    #[serde(rename = "hasChildren")]
    pub has_children: Option<bool>,
}

/// List area paths for a project
pub async fn list_area_paths(
    client: &AzureDevOpsClient,
    organization: &str,
    project: &str,
    parent_path: Option<&str>,
    depth: i32,
) -> Result<ClassificationNode, AzureError> {
    // API: https://dev.azure.com/{organization}/{project}/_apis/wit/classificationnodes/areas/{path}?$depth={depth}&api-version=7.1
    let path_segment = parent_path.unwrap_or("");
    let path = if path_segment.is_empty() {
        format!(
            "wit/classificationnodes/areas?$depth={}&api-version=7.1",
            depth
        )
    } else {
        format!(
            "wit/classificationnodes/areas/{}?$depth={}&api-version=7.1",
            path_segment, depth
        )
    };

    client.get(organization, project, &path).await
}

/// List iteration paths for a project
pub async fn list_iteration_paths(
    client: &AzureDevOpsClient,
    organization: &str,
    project: &str,
    parent_path: Option<&str>,
    depth: i32,
) -> Result<ClassificationNode, AzureError> {
    // API: https://dev.azure.com/{organization}/{project}/_apis/wit/classificationnodes/iterations/{path}?$depth={depth}&api-version=7.1
    let path_segment = parent_path.unwrap_or("");
    let path = if path_segment.is_empty() {
        format!(
            "wit/classificationnodes/iterations?$depth={}&api-version=7.1",
            depth
        )
    } else {
        format!(
            "wit/classificationnodes/iterations/{}?$depth={}&api-version=7.1",
            path_segment, depth
        )
    };

    client.get(organization, project, &path).await
}
