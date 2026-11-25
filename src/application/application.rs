use std::{collections::HashSet, net::IpAddr, sync::Arc};

use crate::{
    ApplicationBuilder,
    hosted_service::ApplicationHostedService,
    http_listener::{hyper_server, hyper_server_tls},
    logging::LOGGER,
    middleware::app_middlewares::ApplicationMiddlewaresBuilder,
    services::{configuration::ConfigurationService, routing::RoutingServiceBuilder, service_provider::ServiceProviderScope},
};

pub struct Application {
    pub name: String,
    pub runner_id: uuid::Uuid,
    pub(crate) ip: HashSet<IpAddr>,
    pub(crate) http_port: HashSet<u16>,
    pub(crate) https_port: HashSet<u16>,
    pub(crate) tls_config: Option<Arc<rustls::ServerConfig>>,
    pub service_provider: ServiceProviderScope,
    pub(crate) _middlewares: ApplicationMiddlewaresBuilder,
    pub(crate) _hosted_services: ApplicationHostedService,
    pub(crate) routing_builder: RoutingServiceBuilder,
}
impl Application {
    pub fn new(name: &str) -> ApplicationBuilder {
        ApplicationBuilder::new(name)
    }
    async fn run_bacground_service(hosted_services: ApplicationHostedService, service_provider: &ServiceProviderScope) {
        let hosted_services = futures::future::join_all(hosted_services.into_iter().map(|(name, factory)| async move { (name, factory(service_provider).await) })).await;
        for (name, mut service) in hosted_services {
            tokio::spawn(async move {
                LOGGER::info(format!("Starting background service: {}", name));
                service.invoke_async().await;
            });
        }
    }
    pub async fn run(self) {
        let Self {
            name: _,
            runner_id: _,
            mut ip,
            mut http_port,
            https_port,
            tls_config,
            mut service_provider,
            _middlewares: middleware_builder,
            _hosted_services: hosted_services,
            routing_builder,
        } = self;

        if ip.is_empty() {
            ip.insert("127.0.0.1".parse::<std::net::IpAddr>().unwrap());
        }
        if http_port.is_empty() {
            http_port.insert(8080);
        }
        let configuration = service_provider.get_service::<ConfigurationService>().await;
        configuration.load_all_async().await;
        let (routing_sv, open_api_sv) = routing_builder.build();
        service_provider.store_instance(Arc::new(routing_sv));
        service_provider.store_instace_if_exist(Arc::new(open_api_sv));
        service_provider.validate_startup_dependencies(&configuration);
        drop(configuration);

        let middleware_pipeline = middleware_builder.build_pipeline(&service_provider).await;
        Self::run_bacground_service(hosted_services, &service_provider).await;
        let service_provider = Arc::new(service_provider);
        let http = hyper_server(&ip, http_port, service_provider.clone(), middleware_pipeline.clone());
        let result = if https_port.is_empty() {
            http.await
        } else {
            let tls_config = tls_config.expect("HTTPS ports require a loaded TLS certificate");
            tokio::try_join!(http, hyper_server_tls(&ip, https_port, service_provider, middleware_pipeline, tls_config)).map(|_| ())
        };
        if let Err(e) = result {
            panic!("Error occurred while running the application: {}", e);
        }
    }
}
