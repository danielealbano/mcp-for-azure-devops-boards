use clap::Parser;
use mcp_for_azure_devops_boards::azure::client::AzureDevOpsClient;
use mcp_for_azure_devops_boards::install::{
    InstallError, InstallTarget, install, resolve_config_path,
};
use mcp_for_azure_devops_boards::mcp::server::AzureMcpServer;
use mcp_for_azure_devops_boards::server::http;
use rmcp::ServiceExt;
use rmcp::transport::stdio;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub(crate) struct Args {
    /// Run in server mode
    #[arg(long)]
    server: bool,

    /// Port to run the server on
    #[arg(long, default_value_t = 3000)]
    port: u16,

    /// Install MCP server configuration for the specified client
    #[arg(long, value_enum, conflicts_with = "server")]
    install: Option<InstallTarget>,

    /// Allowed `Host` header value for the HTTP server (repeatable; only valid
    /// with --server). When omitted, only loopback hosts (localhost, 127.0.0.1,
    /// ::1) are accepted, which prevents DNS rebinding attacks. Providing any
    /// value REPLACES the loopback default entirely, so loopback hosts must be
    /// listed explicitly if they still need to be served (e.g. --allowed-host
    /// mcp.example.com --allowed-host 127.0.0.1:3000).
    #[arg(
        long = "allowed-host",
        value_name = "HOST",
        requires = "server",
        conflicts_with = "install",
        value_parser = parse_allowed_host
    )]
    allowed_hosts: Vec<String>,
}

/// Validates a single `--allowed-host` value. Rejects empty / whitespace-only
/// input, which would otherwise replace the secure loopback default with an
/// allow-list that matches no host and rejects every request. The trimmed value
/// is returned so surrounding whitespace never reaches the allow-list.
fn parse_allowed_host(value: &str) -> Result<String, String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err("allowed host must not be empty or whitespace-only".to_string());
    }
    Ok(trimmed.to_string())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();
    let args = Args::parse();

    if let Some(target) = &args.install {
        let binary_path =
            std::env::current_exe().map_err(|e| InstallError::BinaryPathDetection { source: e })?;
        let config_path = resolve_config_path(target)?;
        let message = install(target, &config_path, &binary_path)?;
        println!("{message}");
        return Ok(());
    }

    let client = AzureDevOpsClient::new();
    let mcp_server = AzureMcpServer::new(client);

    if args.server {
        let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{}", args.port)).await?;
        log::warn!(
            "HTTP server binds 0.0.0.0 and is unauthenticated. Host-header validation guards \
             against DNS rebinding but is NOT network access control; deploy behind a firewall \
             or reverse proxy."
        );
        log::info!("Starting web server on {}", listener.local_addr()?);
        http::run_server(mcp_server, listener, args.allowed_hosts).await?;
    } else {
        log::info!("Starting stdio server");
        let service = mcp_server.serve(stdio()).await?;
        service.waiting().await?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;

    #[test]
    fn test_default_args() {
        let args = Args::try_parse_from(["test"]).unwrap();
        assert!(!args.server);
        assert_eq!(args.port, 3000);
    }

    #[test]
    fn test_server_flag() {
        let args = Args::try_parse_from(["test", "--server"]).unwrap();
        assert!(args.server);
        assert_eq!(args.port, 3000);
    }

    #[test]
    fn test_custom_port() {
        let args = Args::try_parse_from(["test", "--port", "8080"]).unwrap();
        assert!(!args.server);
        assert_eq!(args.port, 8080);
    }

    #[test]
    fn test_server_with_port() {
        let args = Args::try_parse_from(["test", "--server", "--port", "8080"]).unwrap();
        assert!(args.server);
        assert_eq!(args.port, 8080);
    }

    #[test]
    fn test_allowed_host_defaults_to_empty() {
        let args = Args::try_parse_from(["test", "--server"]).unwrap();
        assert!(
            args.allowed_hosts.is_empty(),
            "allowed_hosts must be empty by default so the loopback-only secure default applies"
        );
    }

    #[test]
    fn test_allowed_host_single() {
        let args =
            Args::try_parse_from(["test", "--server", "--allowed-host", "example.com"]).unwrap();
        assert_eq!(args.allowed_hosts, vec!["example.com".to_string()]);
    }

    #[test]
    fn test_allowed_host_repeatable() {
        let args = Args::try_parse_from([
            "test",
            "--server",
            "--allowed-host",
            "example.com",
            "--allowed-host",
            "mcp.internal:8443",
        ])
        .unwrap();
        assert_eq!(
            args.allowed_hosts,
            vec!["example.com".to_string(), "mcp.internal:8443".to_string()]
        );
    }

    #[test]
    fn test_allowed_host_requires_server() {
        let err = Args::try_parse_from(["test", "--allowed-host", "example.com"])
            .expect_err("--allowed-host without --server must be rejected");
        assert_eq!(
            err.kind(),
            clap::error::ErrorKind::MissingRequiredArgument,
            "rejection must be due to the missing --server requirement"
        );
        // The same value parses cleanly once --server is present.
        Args::try_parse_from(["test", "--server", "--allowed-host", "example.com"])
            .expect("--allowed-host with --server must parse");
    }

    #[test]
    fn test_allowed_host_conflicts_with_install() {
        // clap conflict rules take precedence over `requires`, so without an
        // explicit conflict the flag would be silently accepted and ignored in
        // install mode. Assert it is rejected instead.
        let err = Args::try_parse_from(["test", "--install", "claude-code", "--allowed-host", "x"])
            .expect_err("--allowed-host with --install must be rejected");
        assert_eq!(
            err.kind(),
            clap::error::ErrorKind::ArgumentConflict,
            "rejection must be due to the --install conflict"
        );
    }

    #[test]
    fn test_allowed_host_rejects_empty_value() {
        for value in ["", "   "] {
            let result = Args::try_parse_from(["test", "--server", "--allowed-host", value]);
            assert!(
                result.is_err(),
                "empty/whitespace --allowed-host '{value}' must be rejected so the loopback default is not replaced by a match-nothing list"
            );
        }
    }

    #[test]
    fn test_install_flag_parsing() {
        let args = Args::try_parse_from(["test", "--install", "claude-code"]).unwrap();
        assert!(args.install.is_some());
        assert!(matches!(args.install.unwrap(), InstallTarget::ClaudeCode));
    }

    #[test]
    fn test_install_conflicts_with_server() {
        let result = Args::try_parse_from(["test", "--install", "claude-code", "--server"]);
        assert!(result.is_err());
    }

    #[test]
    fn test_install_all_targets_parse() {
        let targets = [
            "claude-code",
            "claude-desktop",
            "cursor",
            "vscode",
            "codex",
            "gemini-cli",
        ];
        for target in targets {
            let args = Args::try_parse_from(["test", "--install", target]).unwrap_or_else(|e| {
                panic!("failed to parse --install {target}: {e}");
            });
            assert!(
                args.install.is_some(),
                "install should be Some for {target}"
            );
        }
    }
}
