use crate::{
    middleware::MiddlewareNext,
    logging::LOGGER,
    services::{configuration::ConfigurationService, routing::RoutingService, service_provider::ServiceProviderScope},
};
use rustls::ServerConfig;
use std::{collections::HashSet, net::IpAddr, sync::Arc};
use tokio::net::TcpListener;
use tokio_rustls::TlsAcceptor;

pub(crate) async fn hyper_server_tls(ips: &HashSet<IpAddr>, https_ports: HashSet<u16>, app_service_provider: Arc<ServiceProviderScope>, middleware_pipeline: MiddlewareNext, tls_config: Arc<ServerConfig>) -> std::io::Result<()> {
    let acceptor = TlsAcceptor::from(tls_config);

    // Same Cartesian-product binding strategy as the plain HTTP listener.
    let bindings: Vec<_> = ips.iter().flat_map(|ip| https_ports.iter().map(move |port| (*ip, *port))).collect();

    let routing_service = app_service_provider.get_service::<RoutingService>().await;
    let configuration_service = app_service_provider.get_service::<ConfigurationService>().await;
    let hypercfg = configuration_service.get::<crate::configuration::HyperConfig>();
    futures::future::try_join_all(bindings.into_iter().map(|(ip, port)| {
        let acceptor = acceptor.clone();
        let routing_service = routing_service.clone();
        let hypercfg = hypercfg.clone();
        let app_service_provider = app_service_provider.clone();
        let middleware_pipeline = middleware_pipeline.clone();
        async move {
            let listener = TcpListener::bind((ip, port)).await?;
            LOGGER::info(format!("HTTPS server listening on {}:{}", if ip.is_ipv4() { "IPv4" } else { "IPv6" }, port));
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
                        let app_service_provider = app_service_provider.clone();
                        let middleware_pipeline = middleware_pipeline.clone();
                        let routing_service = routing_service.clone();
                        let hypercfg = hypercfg.capture().unwrap_or_default();
                        let acceptor = acceptor.clone();
                        tokio::spawn(async move {
                            // Perform the TLS handshake before handing the connection to hyper.
                            // A failed handshake (bad client, port scan, expired cert on the
                            // client's trust store, etc.) must only drop this one connection,
                            // never crash the accept loop.
                            match acceptor.accept(stream).await {
                                Ok(tls_stream) => {
                                    if let Err(e) = crate::http_listener::hyper_service::hyper_service(tls_stream, client_addr, local_addr, app_service_provider, middleware_pipeline, routing_service, hypercfg).await {
                                        LOGGER::error(format!("Error occurred: {}", e));
                                    }
                                }
                                Err(e) => {
                                    LOGGER::warn(format!("TLS handshake failed with {}: {}", client_addr, e));
                                }
                            };
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
