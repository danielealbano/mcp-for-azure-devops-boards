#[cfg(feature = "test-support")]
mod tests {
    use mcp_for_azure_devops_boards::azure::api_trait::MockAzureDevOpsApi;
    use mcp_for_azure_devops_boards::mcp::server::AzureMcpServer;
    use mcp_for_azure_devops_boards::server::http;

    #[tokio::test]
    async fn test_http_server_accepts_connection() {
        let mock = MockAzureDevOpsApi::new();
        let server = AzureMcpServer::new_with_api(mock);

        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        let server_handle = tokio::spawn(async move {
            let _ = http::run_server(server, listener, Vec::new()).await;
        });

        tokio::time::sleep(std::time::Duration::from_millis(100)).await;

        let client = reqwest::Client::new();
        let response = client
            .post(format!("http://{}/mcp", addr))
            .header("Content-Type", "application/json")
            .header("Accept", "application/json, text/event-stream")
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
        let server = AzureMcpServer::new_with_api(mock);

        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        let server_handle = tokio::spawn(async move {
            let _ = http::run_server(server, listener, Vec::new()).await;
        });

        tokio::time::sleep(std::time::Duration::from_millis(100)).await;

        let client = reqwest::Client::new();
        let response = client
            .put(format!("http://{}/mcp", addr))
            .send()
            .await
            .unwrap();

        assert!(
            !response.status().is_success(),
            "PUT should not succeed, got {}",
            response.status()
        );

        server_handle.abort();
    }

    #[tokio::test]
    async fn test_http_server_accepts_get_for_sse() {
        let mock = MockAzureDevOpsApi::new();
        let server = AzureMcpServer::new_with_api(mock);

        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        let server_handle = tokio::spawn(async move {
            let _ = http::run_server(server, listener, Vec::new()).await;
        });

        tokio::time::sleep(std::time::Duration::from_millis(100)).await;

        let client = reqwest::Client::new();
        let response = client
            .get(format!("http://{}/mcp", addr))
            .header("Accept", "text/event-stream")
            .send()
            .await
            .unwrap();

        let status = response.status().as_u16();
        assert!(
            status != 405,
            "GET /mcp must be a supported method for SSE streams, got 405 Method Not Allowed",
        );
        assert!(
            status == 200 || status == 400 || status == 401,
            "GET /mcp for SSE should return 200 (stream), 400 (bad request), or 401 (no session), got {}",
            status
        );

        server_handle.abort();
    }

    // Spawns the HTTP server on an ephemeral loopback port with the given
    // allowed-host list and returns its address. The listener is bound (and
    // thus listening) before the accept task starts, so connections queue in
    // the TCP backlog until it runs — no readiness sleep is required.
    async fn spawn_server(allowed_hosts: Vec<String>) -> std::net::SocketAddr {
        let server = AzureMcpServer::new_with_api(MockAzureDevOpsApi::new());
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move {
            let _ = http::run_server(server, listener, allowed_hosts).await;
        });
        addr
    }

    fn initialize_request(
        client: &reqwest::Client,
        addr: std::net::SocketAddr,
        host: &str,
    ) -> reqwest::RequestBuilder {
        client
            .post(format!("http://{}/mcp", addr))
            .header(reqwest::header::HOST, host)
            .header("Content-Type", "application/json")
            .header("Accept", "application/json, text/event-stream")
            .body(r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-03-26","capabilities":{},"clientInfo":{"name":"test","version":"0.1"}}}"#)
    }

    #[tokio::test]
    async fn test_allowed_host_permits_configured_and_rejects_others() {
        let addr = spawn_server(vec!["example.com".to_string()]).await;
        let client = reqwest::Client::new();

        // A request whose Host matches the configured allow-list passes
        // DNS-rebinding validation and reaches MCP handling (a 2xx response,
        // not the 403 that a rejected Host would produce).
        let allowed = initialize_request(&client, addr, "example.com")
            .send()
            .await
            .unwrap();
        assert!(
            allowed.status().is_success(),
            "configured Host 'example.com' must be accepted and reach MCP handling (2xx), got {}",
            allowed.status()
        );

        // A request with a Host outside the allow-list is rejected, even though
        // the underlying socket is the loopback address it connected to.
        let rejected = initialize_request(&client, addr, "attacker.example")
            .send()
            .await
            .unwrap();
        assert_eq!(
            rejected.status().as_u16(),
            403,
            "Host outside the allow-list must be rejected with 403, got {}",
            rejected.status()
        );
    }

    #[tokio::test]
    async fn test_default_rejects_non_loopback_host() {
        // With no allow-list the rmcp secure default permits only loopback
        // hosts, so a non-loopback Host must be rejected with 403.
        let addr = spawn_server(Vec::new()).await;
        let client = reqwest::Client::new();

        let rejected = initialize_request(&client, addr, "attacker.example")
            .send()
            .await
            .unwrap();
        assert_eq!(
            rejected.status().as_u16(),
            403,
            "non-loopback Host must be rejected by the loopback-only default, got {}",
            rejected.status()
        );
    }

    #[tokio::test]
    async fn test_allowed_hosts_multiple_entries_replace_default() {
        // A multi-entry allow-list (including a host:port entry) fully replaces
        // the loopback default: every configured host is accepted, an unlisted
        // host is rejected, and loopback (no longer listed) is rejected too.
        let addr = spawn_server(vec![
            "a.example".to_string(),
            "b.example".to_string(),
            "mcp.internal:8443".to_string(),
        ])
        .await;
        let client = reqwest::Client::new();

        for host in ["a.example", "b.example", "mcp.internal:8443"] {
            let response = initialize_request(&client, addr, host)
                .send()
                .await
                .unwrap();
            assert!(
                response.status().is_success(),
                "configured host '{host}' must be accepted, got {}",
                response.status()
            );
        }

        let unlisted = initialize_request(&client, addr, "attacker.example")
            .send()
            .await
            .unwrap();
        assert_eq!(
            unlisted.status().as_u16(),
            403,
            "unlisted host must be rejected with 403, got {}",
            unlisted.status()
        );

        // The loopback default was replaced, so 127.0.0.1 is no longer allowed.
        let loopback = initialize_request(&client, addr, "127.0.0.1")
            .send()
            .await
            .unwrap();
        assert_eq!(
            loopback.status().as_u16(),
            403,
            "loopback must be rejected once the allow-list replaces the default, got {}",
            loopback.status()
        );
    }
}
