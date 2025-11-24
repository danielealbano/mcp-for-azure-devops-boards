use azure_devops_mcp::azure::client::AzureDevOpsClient;
use azure_devops_mcp::mcp::server::AzureMcpServer;
use azure_devops_mcp::server::http;
use clap::Parser;
use rmcp::ServiceExt;
use rmcp::transport::stdio;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Run in server mode
    #[arg(long)]
    server: bool,

    /// Port to run the server on
    #[arg(long, default_value_t = 3000)]
    port: u16,

    /// Azure DevOps Organization
    #[arg(long, env = "AZDO_ORGANIZATION")]
    organization: String,

    /// Azure DevOps Project
    #[arg(long, env = "AZDO_PROJECT")]
    project: String,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logger - set RUST_LOG=debug to see debug logs
    env_logger::init();
    let args = Args::parse();

    let client = AzureDevOpsClient::new(args.organization, args.project);
    let mcp_server = AzureMcpServer::new(client);

    if args.server {
        log::info!("Starting web server on port {}", args.port);
        http::run_server(mcp_server, args.port).await?;
    } else {
        log::info!("Starting stdio server");
        let service = mcp_server.serve(stdio()).await?;
        service.waiting().await?;
    }

    Ok(())
}
