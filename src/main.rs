use clap::Parser;
use mcp_for_azure_devops_boards::azure::client::AzureDevOpsClient;
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
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();
    let args = Args::parse();

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
}
