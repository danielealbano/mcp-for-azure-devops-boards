#[cfg(feature = "test-support")]
mod tests {
    use mcp_for_azure_devops_boards::azure::api_trait::MockAzureDevOpsApi;
    use mcp_for_azure_devops_boards::mcp::server::AzureMcpServer;
    use mcp_for_azure_devops_boards::server::http;

    #[tokio::test]
    async fn test_http_server_accepts_connection() {
        let mock = MockAzureDevOpsApi::new();
        let server = AzureMcpServer::new(mock);

        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        let server_handle = tokio::spawn(async move {
            let _ = http::run_server(server, listener).await;
        });

        tokio::time::sleep(std::time::Duration::from_millis(100)).await;

        let client = reqwest::Client::new();
        let response = client
            .post(format!("http://{}/mcp", addr))
            .header("Content-Type", "application/json")
            .body(r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-03-26","capabilities":{},"clientInfo":{"name":"test","version":"0.1"}}}"#)
            .send()
            .await
            .unwrap();

        assert!(
            response.status().is_success() || response.status().as_u16() == 400,
            "Expected 2xx or 400, got {}",
            response.status()
        );

        server_handle.abort();
    }

    #[tokio::test]
    async fn test_http_server_rejects_invalid_method() {
        let mock = MockAzureDevOpsApi::new();
        let server = AzureMcpServer::new(mock);

        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        let server_handle = tokio::spawn(async move {
            let _ = http::run_server(server, listener).await;
        });

        tokio::time::sleep(std::time::Duration::from_millis(100)).await;

        let client = reqwest::Client::new();
        let response = client
            .get(format!("http://{}/mcp", addr))
            .send()
            .await
            .unwrap();

        assert!(
            !response.status().is_success() || response.status().as_u16() == 405,
            "GET should not succeed with 2xx (or should be 405), got {}",
            response.status()
        );

        server_handle.abort();
    }
}
