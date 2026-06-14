use std::sync::Arc;

use tokio::net::TcpListener;

use crate::{Application, logging::LOGGER};


pub(crate) async fn hyper_server(app: Arc<Application>) -> std::io::Result<()> {
    futures::future::try_join_all(app.ip.iter().zip(app.http_port.iter()).map(|(ip, port)| {
        let ip = ip.clone();
        let port = port.clone();
        let app = app.clone();
        async move {
            let listener = TcpListener::bind((ip, port)).await?;
            LOGGER::info(format!("HTTP server listening on {}:{}", ip, port));
            loop {
                match listener.accept().await {
                    Ok((stream, _)) => {
                        let app = app.clone();
                        tokio::spawn(async move {
                            if let Err(e) = crate::http_listener::hyper_service::hyper_service(stream, app).await {
                                LOGGER::error(format!("Error occurred: {}", e));
                            }
                        });
                    }
                    Err(error) => {
                        match error.kind() {
                            std::io::ErrorKind::ConnectionAborted | std::io::ErrorKind::ConnectionReset | std::io::ErrorKind::BrokenPipe => {
                                LOGGER::warn(format!("TCP connection aborted: {}", error));
                                continue; // skip this error and continue accepting new connections
                            }
                            _ => {
                                LOGGER::error(format!("Failed to accept TCP connection: {}", error));
                                continue; // log the error and continue accepting new connections
                            }
                        }
                        #[allow(unreachable_code)]
                        return Err::<(), std::io::Error>(error);
                    }
                }
            }
        }
        }))
    .await?;
    Ok(())
}