use std::sync::Arc;

use tokio::net::TcpListener;

use crate::{
    Application,
    logging::LOGGER,
    tcp::{parser::parse_tcp_stream_async, run_with_split_tcp_stream_async},
};

pub(crate) async fn run_http_server_async(app: Arc<Application>) -> std::io::Result<()> {
    futures::future::try_join_all(app.ip.iter().zip(app.http_port.iter()).map(|(ip, port)| {
        let ip = ip.clone();
        let port = port.clone();
        let app = app.clone();
        async move {
            let listener = TcpListener::bind((ip, port)).await?;
            LOGGER::info(format!("HTTP server listening on {}:{}", ip, port));
            loop {
                match listener.accept().await {
                    Ok((tcp_stream, socket_addr)) => {
                        tcp_stream.set_nodelay(true)?;
                        let app = app.clone();
                        tokio::task::spawn(async move {
                            run_with_split_tcp_stream_async(tcp_stream, move |read_half, write_half| {
                                let app = app.clone();
                                async move { parse_tcp_stream_async(app.clone(), socket_addr, read_half, write_half).await }
                            })
                            .await;
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
