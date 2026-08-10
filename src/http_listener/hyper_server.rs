use crate::{
    Application,
    logging::LOGGER,
    services::{configuration::ConfigurationService, routing::RoutingService},
};
use std::{
    net::{IpAddr, Ipv4Addr, Ipv6Addr},
    sync::Arc,
};
use tokio::net::TcpListener;

pub(crate) async fn hyper_server(app: &Arc<Application>) -> std::io::Result<()> {
    // Every ip is served on every port. The order is fixed (IPv4 first), so the result never depends on HashSet order.
    let mut ips: Vec<IpAddr> = app.ip.iter().copied().collect();
    ips.sort_by_key(|ip| (ip.is_ipv6(), *ip));
    let mut ports: Vec<u16> = app.http_port.iter().copied().collect();
    ports.sort_unstable();
    let ipv4_any_is_served = app.ip.contains(&IpAddr::V4(Ipv4Addr::UNSPECIFIED));

    // Bind one after another: on a dual-stack host `[::]` also takes the IPv4 port, so `0.0.0.0` has to be bound first
    let mut listeners = Vec::new();
    for ip in ips {
        for port in &ports {
            let port = *port;
            match TcpListener::bind((ip, port)).await {
                Ok(listener) => {
                    LOGGER::info(format!("HTTP server listening on {}:{}", ip, port));
                    listeners.push(listener);
                }
                // `[::]` is optional when `0.0.0.0` is served: the host has no IPv6, or `[::]` is the same socket as `0.0.0.0`
                Err(error) if ip == IpAddr::V6(Ipv6Addr::UNSPECIFIED) && ipv4_any_is_served => {
                    LOGGER::warn(format!("Skip listening on {}:{}: {}", ip, port, error));
                }
                Err(error) => return Err(error),
            }
        }
    }

    let routing_service = app.service_provider.get_service::<RoutingService>();
    let configuration_service = app.service_provider.get_service::<ConfigurationService>();
    let hypercfg = configuration_service.get::<crate::configuration::HyperConfig>().unwrap_or_default();
    futures::future::try_join_all(listeners.into_iter().map(|listener| {
        let routing_service = routing_service.clone();
        let hypercfg = hypercfg.clone();
        async move {
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
                            }
                            _ => {
                                LOGGER::error(format!("Failed to accept TCP connection: {}", error));
                            }
                        }
                        continue; // log the error and continue accepting new connections
                    }
                }
            }
            #[allow(unreachable_code)]
            Ok::<(), std::io::Error>(())
        }
    }))
    .await?;
    Ok(())
}
