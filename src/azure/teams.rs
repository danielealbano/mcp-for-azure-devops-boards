use crate::azure::client::{AzureDevOpsClient, AzureError};
use reqwest::Method;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct TeamMember {
    pub identity: TeamMemberIdentity,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TeamMemberIdentity {
    #[serde(rename = "displayName")]
    pub display_name: String,
    #[serde(rename = "uniqueName")]
    pub unique_name: String,
    pub id: String,
}

#[derive(Debug, Deserialize)]
struct TeamMembersResponse {
    value: Vec<TeamMember>,
}

impl AzureDevOpsClient {
    pub async fn list_team_members(
        &self,
        organization: &str,
        project: &str,
        team_id: &str,
    ) -> Result<Vec<TeamMember>, AzureError> {
        let path = format!(
            "projects/{}/teams/{}/members?api-version=7.1",
            project, team_id
        );
        let response: TeamMembersResponse = self
            .org_request(organization, Method::GET, &path, None::<&String>)
            .await?;

        Ok(response.value)
    }
}
