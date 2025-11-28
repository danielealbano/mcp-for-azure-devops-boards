use crate::azure::client::{AzureDevOpsClient, AzureError};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct TagDefinition {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub url: Option<String>,
    #[serde(rename = "lastUpdated", default)]
    pub last_updated: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TagListResponse {
    pub count: u32,
    pub value: Vec<TagDefinition>,
}

/// Get all tags in a project
pub async fn list_tags(
    client: &AzureDevOpsClient,
    organization: &str,
    project: &str,
) -> Result<Vec<TagDefinition>, AzureError> {
    let path = "wit/tags?api-version=7.1";
    let response: TagListResponse = client.get(organization, project, path).await?;
    Ok(response.value)
}
