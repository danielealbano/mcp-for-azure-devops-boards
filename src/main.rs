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
        log::info!("Starting web server on {}", listener.local_addr()?);
        http::run_server(mcp_server, listener).await?;
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
