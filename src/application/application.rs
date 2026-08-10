use std::{collections::HashSet, net::IpAddr, sync::Arc};

use crate::{
    ApplicationBuilder,
    hosted_service::ApplicationHostedService,
    http_listener::{hyper_server, hyper_server_tls},
    logging::LOGGER,
    middleware::{app_middlewares::ApplicationMiddlewares},
    services::service_provider::service_provider_scope::ServiceProviderScope,
};

pub struct Application {
    pub name: String,
    pub runner_id: uuid::Uuid,
    pub(crate) ip: HashSet<IpAddr>,
    pub(crate) http_port: HashSet<u16>,
    pub(crate) https_port: HashSet<u16>,
    pub(crate) tls_config: Option<Arc<rustls::ServerConfig>>,
    pub service_provider: ServiceProviderScope,
    pub(crate) _middlewares: ApplicationMiddlewares,
    pub(crate) _hosted_services: ApplicationHostedService,
}
impl Application {
    pub fn new(name: &str) -> ApplicationBuilder {
        ApplicationBuilder::new(name)
    }

    pub async fn run(mut self) {
        // ensure defaults so the server keeps running even if user didn't set ip/ports
        if self.ip.is_empty() {
            self.ip.insert("127.0.0.1".parse::<std::net::IpAddr>().unwrap());
        }
        if self.http_port.is_empty() {
            self.http_port.insert(8080);
        }
        self._middlewares.build_pipeline();
        let mut hosted_services_app = std::mem::replace(&mut self._hosted_services, Vec::new()); // clear hosted services from app since we're moving them to the async block
        let app = Arc::new(self);
        // let mut hosted_services_app = Vec::new();
        while let Some((name, mut service)) = hosted_services_app.pop() {
            tokio::spawn(async move {
                LOGGER::info(format!("Starting background service: {}", name));
                service.invoke_async().await
            });
        }
        let result = if !app.https_port.is_empty() {
            let tls_config = app.tls_config.clone().expect("https_port was configured without a loaded TLS certificate; this should have been rejected at build()");
            tokio::try_join!(hyper_server(&app), hyper_server_tls(&app, tls_config)).map(|_| ())
        } else {
            hyper_server(&app).await
        };

        if let Err(e) = result {
            LOGGER::error(format!("Server terminated with error: {}", e));
        }
    }
}
