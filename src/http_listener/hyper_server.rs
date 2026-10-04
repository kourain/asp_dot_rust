use crate::{
    Application,
    logging::LOGGER,
    services::{configuration::ConfigurationService, routing::RoutingService},
};
use std::sync::Arc;
use tokio::net::TcpListener;

pub(crate) async fn hyper_server(app: &Arc<Application>) -> std::io::Result<()> {
    // NOTE: iterate every (ip, port) combination (Cartesian product), not a
    // pairwise zip — with_any_ip() registers 2 IPs, and zip() would silently
    // drop the second one if only one port were configured.
    let bindings: Vec<_> = app.ip.iter().flat_map(|ip| app.http_port.iter().map(move |port| (*ip, *port))).collect();
    futures::future::try_join_all(bindings.into_iter().map(|(ip, port)| {
        let routing_service = app.service_provider.get_service::<RoutingService>();
        let configuration_service = app.service_provider.get_service::<ConfigurationService>();
        let hypercfg = configuration_service.get::<crate::configuration::HyperConfig>().unwrap_or_default();
        async move {
            let listener = TcpListener::bind((ip, port)).await?;
            LOGGER::info(format!("HTTP server listening on {}:{}", if ip.is_ipv4() { "IPv4" } else { "IPv6" }, port));
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
                        let hypercfg = hypercfg.clone();
                        tokio::spawn(async move {
                            if let Err(e) = crate::http_listener::hyper_service::hyper_service(stream, client_addr, local_addr, app, routing_service, hypercfg).await {
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
