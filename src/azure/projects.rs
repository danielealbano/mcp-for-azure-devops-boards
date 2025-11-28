use crate::azure::client::{AzureDevOpsClient, AzureError};
use reqwest::Method;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct Project {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    pub url: String,
    pub state: String,
    #[serde(default)]
    pub visibility: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ProjectListResponse {
    pub value: Vec<Project>,
}

/// List all projects in an organization
pub async fn list_projects(
    client: &AzureDevOpsClient,
    organization: &str,
) -> Result<Vec<String>, AzureError> {
    let path = "projects?api-version=7.1";
    let response: ProjectListResponse = client
        .org_request(organization, Method::GET, path, None::<&String>)
        .await?;

    // Extract just the project names
    Ok(response
        .value
        .into_iter()
        .map(|project| project.name)
        .collect())
}
