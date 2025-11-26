use azure_core::auth::TokenCredential;
use azure_identity::DefaultAzureCredential;
use reqwest::{Client, Method};
use serde::Serialize;
use serde::de::DeserializeOwned;
use std::sync::Arc;
use thiserror::Error;

const AZURE_DEVOPS_SCOPE: &str = "499b84ac-1321-427f-aa17-267ca6975798";

#[derive(Error, Debug)]
pub enum AzureError {
    #[error("Authentication failed: {0}")]
    AuthError(#[from] azure_core::Error),
    #[error("HTTP request failed: {0}")]
    HttpError(#[from] reqwest::Error),
    #[error("JSON parsing failed: {0}")]
    SerdeJson(#[from] serde_json::Error),
    #[error("API error: {0}")]
    ApiError(String),
}

pub struct AzureDevOpsClient {
    client: Client,
    credential: Arc<DefaultAzureCredential>,
}

impl Default for AzureDevOpsClient {
    fn default() -> Self {
        Self::new()
    }
}

impl AzureDevOpsClient {
    pub fn new() -> Self {
        let credential = Arc::new(DefaultAzureCredential::default());
        let client = Client::new();
        Self { client, credential }
    }

    async fn get_token(&self) -> Result<String, AzureError> {
        let token_response = self.credential.get_token(AZURE_DEVOPS_SCOPE).await?;
        Ok(token_response.token.secret().to_string())
    }

    pub async fn request_with_content_type<T: DeserializeOwned>(
        &self,
        organization: &str,
        project: &str,
        method: Method,
        path: &str,
        body: Option<&(impl Serialize + ?Sized)>,
        content_type: &str,
    ) -> Result<T, AzureError> {
        let token = self.get_token().await?;
        let url = format!(
            "https://dev.azure.com/{}/{}/_apis/{}",
            organization, project, path
        );

        log::debug!("Request: {} {}", method, url);
        if let Some(b) = &body
            && let Ok(json) = serde_json::to_string_pretty(b)
        {
            log::debug!("Request body: {}", json);
        }

        let mut request = self
            .client
            .request(method, &url)
            .bearer_auth(token)
            .header("Content-Type", content_type);

        if let Some(b) = body {
            request = request.json(b);
        }

        let response = request.send().await?;
        let status = response.status();

        log::debug!("Response status: {}", status);

        if !response.status().is_success() {
            let error_text = response.text().await?;
            log::debug!("Error response: {}", error_text);
            return Err(AzureError::ApiError(error_text));
        }

        let response_text = response.text().await?;
        log::debug!("Response body: {}", response_text);

        let data = serde_json::from_str(&response_text)?;
        Ok(data)
    }

    /// Make a request at the organization level (not project-scoped)
    pub async fn org_request<T: DeserializeOwned>(
        &self,
        organization: &str,
        method: Method,
        path: &str,
        body: Option<&(impl Serialize + ?Sized)>,
    ) -> Result<T, AzureError> {
        let token = self.get_token().await?;
        let url = format!("https://dev.azure.com/{}/_apis/{}", organization, path);

        log::debug!("ORG Request: {} {}", method, url);
        if let Some(b) = &body
            && let Ok(json) = serde_json::to_string_pretty(b)
        {
            log::debug!("Request body: {}", json);
        }

        let mut request = self
            .client
            .request(method, &url)
            .bearer_auth(token)
            .header("Content-Type", "application/json");

        if let Some(b) = body {
            request = request.json(b);
        }

        let response = request.send().await?;
        let status = response.status();

        log::debug!("Response status: {}", status);

        if !response.status().is_success() {
            let error_text = response.text().await?;
            log::debug!("Error response: {}", error_text);
            return Err(AzureError::ApiError(error_text));
        }

        let response_text = response.text().await?;
        log::debug!("Response body: {}", response_text);

        let data = serde_json::from_str(&response_text)?;
        Ok(data)
    }

    /// Make a request to the VSSPS API (Visual Studio Services Platform Services)
    /// URL format: https://app.vssps.visualstudio.com/_apis/{path}
    pub async fn vssps_request<T: DeserializeOwned>(
        &self,
        method: Method,
        path: &str,
        body: Option<&(impl Serialize + ?Sized)>,
    ) -> Result<T, AzureError> {
        let token = self.get_token().await?;
        let url = format!("https://app.vssps.visualstudio.com/_apis/{}", path);

        log::debug!("VSSPS Request: {} {}", method, url);
        if let Some(b) = &body
            && let Ok(json) = serde_json::to_string_pretty(b)
        {
            log::debug!("Request body: {}", json);
        }

        let mut request = self
            .client
            .request(method, &url)
            .bearer_auth(token)
            .header("Content-Type", "application/json");

        if let Some(b) = body {
            request = request.json(b);
        }

        let response = request.send().await?;
        let status = response.status();

        log::debug!("Response status: {}", status);

        if !response.status().is_success() {
            let error_text = response.text().await?;
            log::debug!("Error response: {}", error_text);
            return Err(AzureError::ApiError(error_text));
        }

        let response_text = response.text().await?;
        log::debug!("Response body: {}", response_text);

        let data = serde_json::from_str(&response_text)?;
        Ok(data)
    }

    /// Make a request at the team level (team-scoped)
    /// URL format: https://dev.azure.com/{organization}/{project}/{team}/_apis/{path}
    pub async fn team_request<T: DeserializeOwned>(
        &self,
        organization: &str,
        project: &str,
        method: Method,
        team: &str,
        path: &str,
        body: Option<&(impl Serialize + ?Sized)>,
    ) -> Result<T, AzureError> {
        let token = self.get_token().await?;
        let url = format!(
            "https://dev.azure.com/{}/{}/{}/_apis/{}",
            organization, project, team, path
        );

        log::debug!("TEAM Request: {} {}", method, url);
        if let Some(b) = &body
            && let Ok(json) = serde_json::to_string_pretty(b)
        {
            log::debug!("Request body: {}", json);
        }

        let mut request = self
            .client
            .request(method, &url)
            .bearer_auth(token)
            .header("Content-Type", "application/json");

        if let Some(b) = body {
            request = request.json(b);
        }

        let response = request.send().await?;
        let status = response.status();

        log::debug!("Response status: {}", status);

        if !response.status().is_success() {
            let error_text = response.text().await?;
            log::debug!("Error response: {}", error_text);
            return Err(AzureError::ApiError(error_text));
        }

        let response_text = response.text().await?;
        log::debug!("Response body: {}", response_text);

        let data = serde_json::from_str(&response_text)?;
        Ok(data)
    }

    pub async fn request<T: DeserializeOwned>(
        &self,
        organization: &str,
        project: &str,
        method: Method,
        path: &str,
        body: Option<&(impl Serialize + ?Sized)>,
    ) -> Result<T, AzureError> {
        self.request_with_content_type(
            organization,
            project,
            method,
            path,
            body,
            "application/json",
        )
        .await
    }

    pub async fn get<T: DeserializeOwned>(
        &self,
        organization: &str,
        project: &str,
        path: &str,
    ) -> Result<T, AzureError> {
        self.request(organization, project, Method::GET, path, None::<&String>)
            .await
    }

    /// GET request that returns both the response body and headers
    pub async fn get_with_headers<T: DeserializeOwned>(
        &self,
        organization: &str,
        project: &str,
        path: &str,
    ) -> Result<(T, reqwest::header::HeaderMap), AzureError> {
        let token = self.get_token().await?;
        let url = format!(
            "https://dev.azure.com/{}/{}/_apis/{}",
            organization, project, path
        );

        log::debug!("Request: GET {}", url);

        let request = self
            .client
            .get(&url)
            .bearer_auth(token)
            .header("Content-Type", "application/json");

        let response = request.send().await?;
        let status = response.status();
        let headers = response.headers().clone();

        log::debug!("Response status: {}", status);

        if !status.is_success() {
            let error_text = response.text().await?;
            log::debug!("Error response: {}", error_text);
            return Err(AzureError::ApiError(error_text));
        }

        let response_text = response.text().await?;
        log::debug!("Response body: {}", response_text);

        let data = serde_json::from_str(&response_text)?;
        Ok((data, headers))
    }

    pub async fn post<T: DeserializeOwned>(
        &self,
        organization: &str,
        project: &str,
        path: &str,
        body: &(impl Serialize + ?Sized),
    ) -> Result<T, AzureError> {
        self.request(organization, project, Method::POST, path, Some(body))
            .await
    }

    pub async fn patch<T: DeserializeOwned>(
        &self,
        organization: &str,
        project: &str,
        path: &str,
        body: &(impl Serialize + ?Sized),
    ) -> Result<T, AzureError> {
        self.request(organization, project, Method::PATCH, path, Some(body))
            .await
    }

    pub async fn post_patch<T: DeserializeOwned>(
        &self,
        organization: &str,
        project: &str,
        path: &str,
        body: &(impl Serialize + ?Sized),
    ) -> Result<T, AzureError> {
        self.request_with_content_type(
            organization,
            project,
            Method::POST,
            path,
            Some(body),
            "application/json-patch+json",
        )
        .await
    }

    pub async fn patch_patch<T: DeserializeOwned>(
        &self,
        organization: &str,
        project: &str,
        path: &str,
        body: &(impl Serialize + ?Sized),
    ) -> Result<T, AzureError> {
        self.request_with_content_type(
            organization,
            project,
            Method::PATCH,
            path,
            Some(body),
            "application/json-patch+json",
        )
        .await
    }

    pub async fn post_binary<T: DeserializeOwned>(
        &self,
        organization: &str,
        project: &str,
        path: &str,
        body: Vec<u8>,
    ) -> Result<T, AzureError> {
        let token = self.get_token().await?;
        let url = format!(
            "https://dev.azure.com/{}/{}/_apis/{}",
            organization, project, path
        );

        let response = self
            .client
            .post(&url)
            .bearer_auth(token)
            .header("Content-Type", "application/octet-stream")
            .body(body)
            .send()
            .await?;

        if !response.status().is_success() {
            let error_text = response.text().await?;
            return Err(AzureError::ApiError(error_text));
        }

        let data = response.json::<T>().await?;
        Ok(data)
    }

    pub async fn get_binary(
        &self,
        organization: &str,
        project: &str,
        path: &str,
    ) -> Result<Vec<u8>, AzureError> {
        let token = self.get_token().await?;
        let url = format!(
            "https://dev.azure.com/{}/{}/_apis/{}",
            organization, project, path
        );

        let response = self.client.get(&url).bearer_auth(token).send().await?;

        if !response.status().is_success() {
            let error_text = response.text().await?;
            return Err(AzureError::ApiError(error_text));
        }

        let bytes = response.bytes().await?;
        Ok(bytes.to_vec())
    }
}
