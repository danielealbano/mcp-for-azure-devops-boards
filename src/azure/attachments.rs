use crate::azure::client::{AzureDevOpsClient, AzureError};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct AttachmentReference {
    pub id: String,
    pub url: String,
}

pub async fn upload_attachment(
    client: &AzureDevOpsClient,
    organization: &str,
    project: &str,
    file_name: &str,
    content: Vec<u8>,
) -> Result<AttachmentReference, AzureError> {
    let path = format!("wit/attachments?fileName={}&api-version=7.1", file_name);
    client
        .post_binary(organization, project, &path, content)
        .await
}

pub async fn download_attachment(
    client: &AzureDevOpsClient,
    organization: &str,
    project: &str,
    id: &str,
    file_name: Option<&str>,
) -> Result<Vec<u8>, AzureError> {
    let mut path = format!("wit/attachments/{}?api-version=7.1", id);
    if let Some(name) = file_name {
        path.push_str(&format!("&fileName={}", name));
    }
    client.get_binary(organization, project, &path).await
}
