use crate::{
    logging::LOGGER,
    middleware::MiddlewareNext,
    services::{configuration::ConfigurationService, routing::RoutingService, service_provider::ServiceProviderScope},
};
use std::{
    collections::HashSet,
    net::{IpAddr, Ipv4Addr, Ipv6Addr},
    sync::Arc,
};
use tokio::net::TcpListener;

pub(crate) async fn hyper_server(ips: HashSet<IpAddr>, http_ports: HashSet<u16>, app_service_provider: ServiceProviderScope, middleware_pipeline: MiddlewareNext) -> std::io::Result<()> {
    let listeners = {
        // Every ip is served on every port. The order is fixed (IPv4 first), so the result never depends on HashSet order.
        let mut ips: Vec<IpAddr> = ips.into_iter().collect();
        ips.sort_by_key(|ip| (ip.is_ipv6(), *ip));
        let mut ports: Vec<u16> = http_ports.into_iter().collect();
        ports.sort_unstable();
        let ipv4_any_is_served = ips.contains(&IpAddr::V4(Ipv4Addr::UNSPECIFIED));

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
        listeners
    };

    let routing_service = app_service_provider.get_service::<RoutingService>().await;
    let hypercfg = {
        let configuration_service = app_service_provider.get_service::<ConfigurationService>().await;
        configuration_service.get::<crate::configuration::HyperConfig>().capture().unwrap_or_default().0
    };
    let app_service_provider = Arc::new(app_service_provider);
    #[cfg(panic = "unwind")]
    super::handle_error::HandleError::install_request_panic_hook();
    futures::future::try_join_all(listeners.into_iter().map(|listener| {
        let app_service_provider = app_service_provider.clone();
        let middleware_pipeline = middleware_pipeline.clone();
        let routing_service = routing_service.clone();
        let hypercfg = hypercfg.clone();
        async move {
            loop {
                match listener.accept().await {
                    Ok((stream, _)) => {
                        let app_service_provider = app_service_provider.clone();
                        let middleware_pipeline = middleware_pipeline.clone();
                        let routing_service = routing_service.clone();
                        let hypercfg = hypercfg.clone();
                        tokio::spawn(async move {
                            if let Err(e) = crate::http_listener::hyper_service::hyper_service(stream, app_service_provider, middleware_pipeline, routing_service, hypercfg).await {
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
                        continue;
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
