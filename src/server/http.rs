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
use tokio::sync::Semaphore;

const MAX_CONNECTIONS: usize = 256;

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
            if let Err(err) = Builder::new(TokioExecutor::default())
                .serve_connection(io, service)
                .await
            {
                log::error!("Error serving connection: {:?}", err);
            }

            drop(permit);
        });
    }
}
