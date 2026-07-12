use azure_core::credentials::{Secret, TokenCredential};
use azure_identity::{
    AzureCliCredential, AzureDeveloperCliCredential, ClientSecretCredential,
    ManagedIdentityCredential,
};
use reqwest::{Client, Method};
use serde::Serialize;
use serde::de::DeserializeOwned;
use std::sync::Arc;
use std::time::Duration;
use thiserror::Error;

/// OAuth2 v2.0 scope for the Azure DevOps REST API: the Azure DevOps resource
/// application ID (`499b84ac-1321-427f-aa17-267ca6975798`) with the required
/// `/.default` suffix. The bare resource GUID is a v1.0 "resource" value and is
/// rejected by the Azure CLI (`az account get-access-token --scope`) with an
/// `AADSTS65002` consent error; managed identity strips `/.default` back to the
/// v1.0 resource itself, so this form is correct for every credential source.
const AZURE_DEVOPS_SCOPE: &str = "499b84ac-1321-427f-aa17-267ca6975798/.default";

/// Environment variables that configure the "environment" (client-secret)
/// credential. All three MUST be present for that credential to be used.
const ENV_TENANT_ID: &str = "AZURE_TENANT_ID";
const ENV_CLIENT_ID: &str = "AZURE_CLIENT_ID";
const ENV_CLIENT_SECRET: &str = "AZURE_CLIENT_SECRET";

/// Hard upper bound on a single managed-identity token attempt. Off Azure the
/// IMDS endpoint (`169.254.169.254`) is unreachable and the probe would
/// otherwise block on a long TCP timeout; bounding it lets the chain fail fast
/// and fall through to the remaining sources.
const MANAGED_IDENTITY_TIMEOUT: Duration = Duration::from_secs(2);

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
    credentials: Vec<CredentialSource>,
}

/// A single entry in the ordered credential fallback chain: the credential, a
/// human-readable `label` used in logs, and an optional per-attempt `timeout`
/// applied at token-acquisition time.
struct CredentialSource {
    label: &'static str,
    credential: Arc<dyn TokenCredential>,
    timeout: Option<Duration>,
}

/// Builds the ordered credential fallback chain that replaces the
/// `DefaultAzureCredential` removed in azure_identity 1.0. Sources are tried in
/// order at token-acquisition time:
///
/// 1. the environment (client-secret) credential, used only when
///    `AZURE_TENANT_ID`, `AZURE_CLIENT_ID`, and `AZURE_CLIENT_SECRET` are all set,
/// 2. the Azure CLI,
/// 3. the Azure Developer CLI,
/// 4. managed identity (production / Azure-hosted deployments), tried last and
///    bounded by [`MANAGED_IDENTITY_TIMEOUT`] so its IMDS probe fails fast off
///    Azure instead of blocking the whole chain.
///
/// A source that fails to initialize is skipped so the remaining sources can
/// still be used.
fn build_credential_chain() -> Vec<CredentialSource> {
    build_credential_chain_with_env(|key| std::env::var(key).ok())
}

/// Same as [`build_credential_chain`] but with an injectable environment reader,
/// so the chain's composition can be unit-tested without mutating process state.
fn build_credential_chain_with_env(env: impl Fn(&str) -> Option<String>) -> Vec<CredentialSource> {
    let mut sources: Vec<CredentialSource> = Vec::new();

    if let Some(credential) = env_client_secret_credential(&env) {
        sources.push(CredentialSource {
            label: "environment (client secret)",
            credential,
            timeout: None,
        });
    }
    if let Ok(credential) = AzureCliCredential::new(None) {
        sources.push(CredentialSource {
            label: "Azure CLI",
            credential,
            timeout: None,
        });
    }
    if let Ok(credential) = AzureDeveloperCliCredential::new(None) {
        sources.push(CredentialSource {
            label: "Azure Developer CLI",
            credential,
            timeout: None,
        });
    }
    if let Ok(credential) = ManagedIdentityCredential::new(None) {
        sources.push(CredentialSource {
            label: "managed identity",
            credential,
            timeout: Some(MANAGED_IDENTITY_TIMEOUT),
        });
    }

    sources
}

/// Builds the environment (client-secret) credential from `AZURE_TENANT_ID`,
/// `AZURE_CLIENT_ID`, and `AZURE_CLIENT_SECRET`. Returns `None` when none of the
/// three are set (the credential is simply not configured). When only some are
/// set — or construction fails — it logs a warning and returns `None`, so a
/// misconfiguration is surfaced instead of being silently ignored. The values
/// themselves are never logged.
fn env_client_secret_credential(
    env: &impl Fn(&str) -> Option<String>,
) -> Option<Arc<dyn TokenCredential>> {
    match (
        env(ENV_TENANT_ID),
        env(ENV_CLIENT_ID),
        env(ENV_CLIENT_SECRET),
    ) {
        (None, None, None) => None,
        (Some(tenant_id), Some(client_id), Some(client_secret)) => {
            match ClientSecretCredential::new(
                &tenant_id,
                client_id,
                Secret::from(client_secret),
                None,
            ) {
                Ok(credential) => Some(credential),
                Err(error) => {
                    log::warn!(
                        "{ENV_TENANT_ID}, {ENV_CLIENT_ID}, and {ENV_CLIENT_SECRET} are set but \
                         building the environment credential failed: {error}; skipping it"
                    );
                    None
                }
            }
        }
        _ => {
            log::warn!(
                "Azure environment credential is partially configured (some but not all of \
                 {ENV_TENANT_ID}, {ENV_CLIENT_ID}, {ENV_CLIENT_SECRET} are set); skipping it"
            );
            None
        }
    }
}

impl Default for AzureDevOpsClient {
    fn default() -> Self {
        Self::new()
    }
}

impl AzureDevOpsClient {
    pub fn new() -> Self {
        let client = Client::new();
        Self {
            client,
            credentials: build_credential_chain(),
        }
    }

    async fn get_token(&self) -> Result<String, AzureError> {
        if self.credentials.is_empty() {
            return Err(AzureError::ApiError(
                "no Azure credential sources could be initialized".to_string(),
            ));
        }

        // Every source is tried in order; the first success returns. If they
        // all fail, each failure is aggregated into the error so no meaningful
        // failure (e.g. an Azure CLI consent error) is masked by a later,
        // less-useful one (e.g. "azd not found on PATH").
        let mut failures: Vec<String> = Vec::new();
        for source in &self.credentials {
            let attempt = source.credential.get_token(&[AZURE_DEVOPS_SCOPE], None);
            let outcome = match source.timeout {
                Some(limit) => match tokio::time::timeout(limit, attempt).await {
                    Ok(outcome) => outcome,
                    Err(_elapsed) => {
                        let detail = format!("timed out after {limit:?}");
                        log::warn!(
                            "{} credential {detail}; trying the next source",
                            source.label
                        );
                        failures.push(format!("{}: {detail}", source.label));
                        continue;
                    }
                },
                None => attempt.await,
            };

            match outcome {
                Ok(token) => return Ok(token.token.secret().to_string()),
                Err(error) => {
                    log::debug!("{} credential failed: {error}", source.label);
                    failures.push(format!("{}: {error}", source.label));
                }
            }
        }

        Err(AzureError::ApiError(format!(
            "all Azure credential sources failed: [{}]",
            failures.join("; ")
        )))
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
            urlencoding::encode(organization),
            urlencoding::encode(project),
            path
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
        let url = format!(
            "https://dev.azure.com/{}/_apis/{}",
            urlencoding::encode(organization),
            path
        );

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
            urlencoding::encode(organization),
            urlencoding::encode(project),
            urlencoding::encode(team),
            path
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
            urlencoding::encode(organization),
            urlencoding::encode(project),
            path
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
            urlencoding::encode(organization),
            urlencoding::encode(project),
            path
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
            urlencoding::encode(organization),
            urlencoding::encode(project),
            path
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

#[cfg(test)]
impl AzureDevOpsClient {
    /// Test-only constructor injecting a pre-built credential chain, so
    /// `get_token`'s ordering and timeout behavior can be exercised
    /// deterministically without real credentials or network access.
    fn with_credentials(credentials: Vec<CredentialSource>) -> Self {
        Self {
            client: Client::new(),
            credentials,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use azure_core::credentials::{AccessToken, TokenRequestOptions};
    use std::collections::HashMap;

    const FAKE_TENANT_ID: &str = "00000000-0000-0000-0000-000000000000";

    /// Builds an environment reader over a fixed set of pairs.
    fn env_from(pairs: &[(&str, &str)]) -> impl Fn(&str) -> Option<String> {
        let map: HashMap<String, String> = pairs
            .iter()
            .map(|(key, value)| ((*key).to_string(), (*value).to_string()))
            .collect();
        move |key: &str| map.get(key).cloned()
    }

    fn labels(chain: &[CredentialSource]) -> Vec<&'static str> {
        chain.iter().map(|source| source.label).collect()
    }

    /// A controllable credential: optionally delays, then returns a token or fails.
    #[derive(Debug)]
    struct MockCredential {
        delay: Option<Duration>,
        result: Result<&'static str, ()>,
    }

    #[async_trait::async_trait]
    impl TokenCredential for MockCredential {
        async fn get_token(
            &self,
            _scopes: &[&str],
            _options: Option<TokenRequestOptions<'_>>,
        ) -> azure_core::Result<AccessToken> {
            if let Some(delay) = self.delay {
                tokio::time::sleep(delay).await;
            }
            match self.result {
                Ok(token) => Ok(AccessToken::new(
                    token,
                    azure_core::time::OffsetDateTime::now_utc(),
                )),
                Err(()) => Err(azure_core::Error::with_message(
                    azure_core::error::ErrorKind::Credential,
                    "mock credential failure",
                )),
            }
        }
    }

    fn source(
        label: &'static str,
        delay: Option<Duration>,
        result: Result<&'static str, ()>,
        timeout: Option<Duration>,
    ) -> CredentialSource {
        CredentialSource {
            label,
            credential: Arc::new(MockCredential { delay, result }),
            timeout,
        }
    }

    #[test]
    fn test_build_credential_chain_is_not_empty() {
        // Credential construction is environment-independent (network calls
        // happen only at get_token time), so the chain must always expose at
        // least one source for get_token to try.
        let chain = build_credential_chain();
        assert!(
            !chain.is_empty(),
            "credential chain must contain at least one source"
        );
    }

    #[test]
    fn test_credential_chain_order_and_managed_identity_timeout() {
        // With all environment variables present, the environment credential
        // leads, managed identity is last, and only managed identity is
        // time-bounded.
        let chain = build_credential_chain_with_env(env_from(&[
            (ENV_TENANT_ID, FAKE_TENANT_ID),
            (ENV_CLIENT_ID, "client-id"),
            (ENV_CLIENT_SECRET, "client-secret"),
        ]));
        assert_eq!(
            labels(&chain),
            vec![
                "environment (client secret)",
                "Azure CLI",
                "Azure Developer CLI",
                "managed identity",
            ],
            "environment credential must lead and managed identity must be last"
        );
        for source in &chain {
            let expected = if source.label == "managed identity" {
                Some(MANAGED_IDENTITY_TIMEOUT)
            } else {
                None
            };
            assert_eq!(
                source.timeout, expected,
                "unexpected timeout for source '{}'",
                source.label
            );
        }
    }

    #[test]
    fn test_environment_credential_presence_variants() {
        let cases = vec![
            (
                "all set -> present",
                vec![
                    (ENV_TENANT_ID, FAKE_TENANT_ID),
                    (ENV_CLIENT_ID, "client-id"),
                    (ENV_CLIENT_SECRET, "client-secret"),
                ],
                true,
            ),
            ("none set -> absent", vec![], false),
            (
                "partial (tenant+client) -> skipped",
                vec![
                    (ENV_TENANT_ID, FAKE_TENANT_ID),
                    (ENV_CLIENT_ID, "client-id"),
                ],
                false,
            ),
            (
                "partial (secret only) -> skipped",
                vec![(ENV_CLIENT_SECRET, "client-secret")],
                false,
            ),
            (
                "all set but invalid tenant -> construction fails, skipped",
                vec![
                    (ENV_TENANT_ID, "invalid tenant!"),
                    (ENV_CLIENT_ID, "client-id"),
                    (ENV_CLIENT_SECRET, "client-secret"),
                ],
                false,
            ),
        ];
        for (name, pairs, want_env_first) in cases {
            let chain = build_credential_chain_with_env(env_from(&pairs));
            let has_env = chain
                .first()
                .is_some_and(|source| source.label == "environment (client secret)");
            assert_eq!(has_env, want_env_first, "case '{name}'");
        }
    }

    #[tokio::test]
    async fn test_get_token_falls_through_on_timeout() {
        // A slow source that exceeds its own timeout must be skipped, and the
        // next (fast) source must satisfy the request.
        let client = AzureDevOpsClient::with_credentials(vec![
            source(
                "slow",
                Some(Duration::from_secs(30)),
                Ok("unused"),
                Some(Duration::from_millis(50)),
            ),
            source("fast", None, Ok("good-token"), None),
        ]);
        let token = client.get_token().await.expect("token from fast source");
        assert_eq!(token, "good-token");
    }

    #[tokio::test]
    async fn test_get_token_tries_sources_in_order() {
        // The first successful source wins; a preceding failing source is
        // skipped and later sources are not consulted.
        let client = AzureDevOpsClient::with_credentials(vec![
            source("first", None, Err(()), None),
            source("second", None, Ok("second-token"), None),
            source("third", None, Ok("third-token"), None),
        ]);
        let token = client.get_token().await.expect("token from second source");
        assert_eq!(token, "second-token");
    }

    #[tokio::test]
    async fn test_get_token_all_sources_error_aggregates_failures() {
        // When every source fails, the error aggregates each labeled failure so
        // none is masked by a later, less-useful one.
        let client = AzureDevOpsClient::with_credentials(vec![
            source("first", None, Err(()), None),
            source("second", None, Err(()), None),
        ]);
        let error = client
            .get_token()
            .await
            .expect_err("all-error chain must error");
        let AzureError::ApiError(message) = error else {
            panic!("all-error chain must yield ApiError, got {error:?}");
        };
        assert!(
            message.contains("first:") && message.contains("second:"),
            "aggregated error must name every failed source, got: {message}"
        );
    }

    #[tokio::test]
    async fn test_get_token_empty_chain_errors() {
        let client = AzureDevOpsClient::with_credentials(vec![]);
        let error = client
            .get_token()
            .await
            .expect_err("empty chain must error");
        assert!(
            matches!(error, AzureError::ApiError(_)),
            "empty chain must yield ApiError, got {error:?}"
        );
    }

    #[tokio::test]
    async fn test_get_token_all_sources_timeout_errors() {
        // When the only source times out, get_token must return an error rather
        // than hang or claim no sources were initialized.
        let client = AzureDevOpsClient::with_credentials(vec![source(
            "slow",
            Some(Duration::from_secs(30)),
            Ok("unused"),
            Some(Duration::from_millis(50)),
        )]);
        let error = client
            .get_token()
            .await
            .expect_err("all-timeout must error");
        let AzureError::ApiError(message) = error else {
            panic!("all-timeout must yield ApiError, got {error:?}");
        };
        assert!(
            message.contains("slow:") && message.contains("timed out"),
            "aggregated error must report the timed-out source, got: {message}"
        );
    }
}
