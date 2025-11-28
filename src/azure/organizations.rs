use crate::azure::client::{AzureDevOpsClient, AzureError};
use reqwest::Method;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct Profile {
    pub id: String,
    #[serde(rename = "displayName")]
    pub display_name: String,
    #[serde(rename = "emailAddress")]
    pub email_address: String,
    #[serde(rename = "publicAlias")]
    pub public_alias: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Organization {
    #[serde(rename = "accountId")]
    pub account_id: String,
    #[serde(rename = "accountUri")]
    pub account_uri: String,
    #[serde(rename = "accountName")]
    pub account_name: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct OrganizationListResponse {
    pub value: Vec<Organization>,
}

/// Get the current user's profile
pub async fn get_profile(client: &AzureDevOpsClient) -> Result<Profile, AzureError> {
    let path = "profile/profiles/me?api-version=7.1";
    client
        .vssps_request(Method::GET, path, None::<&String>)
        .await
}

/// List all organizations the current user has access to
pub async fn list_organizations(
    client: &AzureDevOpsClient,
    member_id: &str,
) -> Result<Vec<Organization>, AzureError> {
    let path = format!("accounts?memberId={}&api-version=7.1", member_id);
    let response: OrganizationListResponse = client
        .vssps_request(Method::GET, &path, None::<&String>)
        .await?;
    Ok(response.value)
}
