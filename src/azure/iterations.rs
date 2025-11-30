use crate::azure::client::{AzureDevOpsClient, AzureError};
use reqwest::Method;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct IterationAttributes {
    #[serde(rename = "startDate")]
    pub start_date: Option<String>,
    #[serde(rename = "finishDate")]
    pub finish_date: Option<String>,
    #[serde(rename = "timeFrame")]
    pub time_frame: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TeamSettingsIteration {
    pub id: String,
    pub name: String,
    pub path: String,
    pub attributes: IterationAttributes,
    pub url: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct IterationListResponse {
    pub value: Vec<TeamSettingsIteration>,
}

/// Get the current iteration for a team
pub async fn get_team_current_iteration(
    client: &AzureDevOpsClient,
    organization: &str,
    project: &str,
    team_id: &str,
) -> Result<TeamSettingsIteration, AzureError> {
    // API: https://dev.azure.com/{org}/{project}/{team}/_apis/work/teamsettings/iterations?$timeframe=current&api-version=7.1
    let path = "work/teamsettings/iterations?$timeframe=current&api-version=7.1";
    let response: IterationListResponse = client
        .team_request(
            organization,
            project,
            Method::GET,
            team_id,
            path,
            None::<&String>,
        )
        .await?;

    // The API returns a list, but with $timeframe=current there should only be one
    response
        .value
        .into_iter()
        .next()
        .ok_or_else(|| AzureError::ApiError("No current iteration found for this team".to_string()))
}

/// Get all iterations for a team
pub async fn get_team_iterations(
    client: &AzureDevOpsClient,
    organization: &str,
    project: &str,
    team_id: &str,
) -> Result<Vec<TeamSettingsIteration>, AzureError> {
    // API: https://dev.azure.com/{org}/{project}/{team}/_apis/work/teamsettings/iterations?api-version=7.1
    let path = "work/teamsettings/iterations?api-version=7.1";
    let response: IterationListResponse = client
        .team_request(
            organization,
            project,
            Method::GET,
            team_id,
            path,
            None::<&String>,
        )
        .await?;

    Ok(response.value)
}
