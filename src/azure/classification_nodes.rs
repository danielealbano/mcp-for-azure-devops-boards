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
    #[serde(default, rename = "hasChildren")]
    pub has_children: Option<bool>,
}

impl ClassificationNode {
    /// Recursively collect all paths from this node and its children
    pub fn collect_paths(&self, paths: &mut Vec<String>) {
        paths.push(self.path.clone());
        if let Some(children) = &self.children {
            for child in children {
                child.collect_paths(paths);
            }
        }
    }
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
    let path = if let Some(parent) = parent_path {
        // URL encode the parent path to handle special characters
        let encoded_parent = urlencoding::encode(parent);
        format!(
            "wit/classificationnodes/areas/{}?$depth={}&api-version=7.1",
            encoded_parent, depth
        )
    } else {
        format!(
            "wit/classificationnodes/areas?$depth={}&api-version=7.1",
            depth
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
    let path = if let Some(parent) = parent_path {
        // URL encode the parent path to handle special characters
        let encoded_parent = urlencoding::encode(parent);
        format!(
            "wit/classificationnodes/iterations/{}?$depth={}&api-version=7.1",
            encoded_parent, depth
        )
    } else {
        format!(
            "wit/classificationnodes/iterations?$depth={}&api-version=7.1",
            depth
        )
    };

    client.get(organization, project, &path).await
}
