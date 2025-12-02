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
) -> Result<Option<TeamSettingsIteration>, AzureError> {
    // API: https://dev.azure.com/{org}/{project}/{team}/_apis/work/teamsettings/iterations?$timeframe=current&api-version=7.1
    let path = "work/teamsettings/iterations?$timeframe=current&api-version=7.1";
    let result: Result<IterationListResponse, AzureError> = client
        .team_request(
            organization,
            project,
            Method::GET,
            team_id,
            path,
            None::<&String>,
        )
        .await;

    match result {
        Ok(response) => {
            // The API returns a list, but with $timeframe=current there should only be one
            Ok(response.value.into_iter().next())
        }
        Err(AzureError::ApiError(msg)) if msg.contains("CurrentIterationDoesNotExistException") => {
            Ok(None)
        }
        Err(e) => Err(e),
    }
}

/// Get all iterations for a team, optionally filtered by timeframe
pub async fn get_team_iterations(
    client: &AzureDevOpsClient,
    organization: &str,
    project: &str,
    team_id: &str,
    timeframe: Option<&str>,
) -> Result<Vec<TeamSettingsIteration>, AzureError> {
    // API: https://dev.azure.com/{org}/{project}/{team}/_apis/work/teamsettings/iterations?api-version=7.1
    let path = if let Some(tf) = timeframe {
        format!(
            "work/teamsettings/iterations?$timeframe={}&api-version=7.1",
            tf
        )
    } else {
        "work/teamsettings/iterations?api-version=7.1".to_string()
    };

    let response: IterationListResponse = client
        .team_request(
            organization,
            project,
            Method::GET,
            team_id,
            &path,
            None::<&String>,
        )
        .await?;

    Ok(response.value)
}
