use std::sync::Arc;

use rustls::ServerConfig;
use tokio::net::TcpListener;
use tokio_rustls::TlsAcceptor;

use crate::{Application, logging::LOGGER};

pub(crate) async fn hyper_server_tls(app: &Arc<Application>, tls_config: Arc<ServerConfig>) -> std::io::Result<()> {
    let acceptor = TlsAcceptor::from(tls_config);

    // Same Cartesian-product binding strategy as the plain HTTP listener.
    let bindings: Vec<_> = app.ip.iter().flat_map(|ip| app.https_port.iter().map(move |port| (*ip, *port))).collect();

    futures::future::try_join_all(bindings.into_iter().map(|(ip, port)| {
        let acceptor = acceptor.clone();
        async move {
            let listener = TcpListener::bind((ip, port)).await?;
            LOGGER::info(format!("HTTPS server listening on {}:{}", ip, port));
            let routing_service = app.service_provider.get_service::<crate::services::routing::RoutingService>();
            loop {
                match listener.accept().await {
                    Ok((stream, _)) => {
                        let (client_addr, local_addr) = match (stream.peer_addr(), stream.local_addr()) {
                            (Ok(c), Ok(l)) => (c, l),
                            (Err(e), _) | (_, Err(e)) => {
                                LOGGER::warn(format!("Failed to read socket address, dropping connection: {}", e));
                                continue;
                            }
                        };
                        let app = app.clone();
                        let routing_service = routing_service.clone();
                        let acceptor = acceptor.clone();
                        tokio::spawn(async move {
                            // Perform the TLS handshake before handing the connection to hyper.
                            // A failed handshake (bad client, port scan, expired cert on the
                            // client's trust store, etc.) must only drop this one connection,
                            // never crash the accept loop.
                            let tls_stream = match acceptor.accept(stream).await {
                                Ok(s) => s,
                                Err(e) => {
                                    LOGGER::warn(format!("TLS handshake failed with {}: {}", client_addr, e));
                                    return;
                                }
                            };
                            if let Err(e) = crate::http_listener::hyper_service::hyper_service(tls_stream, client_addr, local_addr, app, &routing_service).await {
                                LOGGER::error(format!("Error occurred: {}", e));
                            }
                        });
                    }
                    Err(error) => {
                        match error.kind() {
                            std::io::ErrorKind::ConnectionAborted | std::io::ErrorKind::ConnectionReset | std::io::ErrorKind::BrokenPipe => {
                                LOGGER::warn(format!("TCP connection aborted: {}", error));
                                continue;
                            }
                            _ => {
                                LOGGER::error(format!("Failed to accept TCP connection: {}", error));
                                continue;
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
