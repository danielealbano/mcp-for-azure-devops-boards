use crate::mcp::server::AzureMcpServer;
use hyper_util::{
    rt::{TokioExecutor, TokioIo},
    server::conn::auto::Builder,
    service::TowerToHyperService,
};
use rmcp::transport::streamable_http_server::{
    StreamableHttpService, session::local::LocalSessionManager,
};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Semaphore;

const MAX_CONNECTIONS: usize = 256;
const CONNECTION_TIMEOUT: Duration = Duration::from_secs(60);

pub async fn run_server(
    server: AzureMcpServer,
    listener: tokio::net::TcpListener,
) -> std::io::Result<()> {
    let service = TowerToHyperService::new(StreamableHttpService::new(
        move || Ok(server.clone()),
        LocalSessionManager::default().into(),
        Default::default(),
    ));

    let semaphore = Arc::new(Semaphore::new(MAX_CONNECTIONS));

    loop {
        let (stream, _) = listener.accept().await?;
        let permit = match semaphore.clone().acquire_owned().await {
            Ok(permit) => permit,
            Err(e) => {
                log::error!("Failed to acquire connection permit: {:?}", e);
                continue;
            }
        };
        let io = TokioIo::new(stream);
        let service = service.clone();

        tokio::spawn(async move {
            let result = tokio::time::timeout(
                CONNECTION_TIMEOUT,
                Builder::new(TokioExecutor::default()).serve_connection(io, service),
            )
            .await;

            match result {
                Ok(Ok(())) => {}
                Ok(Err(err)) => log::error!("Error serving connection: {:?}", err),
                Err(_) => log::warn!("Connection timed out after {:?}", CONNECTION_TIMEOUT),
            }

            drop(permit);
        });
    }
}
