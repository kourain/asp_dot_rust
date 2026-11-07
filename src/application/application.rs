use std::{collections::HashSet, net::IpAddr, sync::Arc};

use crate::{
    ApplicationBuilder,
    hosted_service::ApplicationHostedService,
    http_listener::hyper_server,
    logging::LOGGER,
    middleware::{app_middlewares::ApplicationMiddlewares},
    services::{configuration::ConfigurationService, service_provider::service_provider_scope::ServiceProviderScope},
};

pub struct Application {
    pub name: String,
    pub runner_id: uuid::Uuid,
    pub(crate) ip: HashSet<IpAddr>,
    pub(crate) http_port: HashSet<u16>,
    pub(crate) https_port: HashSet<u16>,
    pub configuration: Arc<ConfigurationService>,
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
        self._middlewares.build_pipeline(&self.service_provider).await;
        let mut hosted_services_app = std::mem::take(&mut self._hosted_services); // clear hosted services from app since we're moving them to the async block
        let app = Arc::new(self);
        // let mut hosted_services_app = Vec::new();
        while let Some((name, service)) = hosted_services_app.pop() {
            let app_clone = app.clone();
            tokio::spawn(async move {
                LOGGER::info(format!("Starting background service: {}", name));
                service(&app_clone.service_provider).await.invoke_async().await
            });
        }
        if let Err(e) = hyper_server(&app).await {
            panic!("Error occurred while running the application: {}", e);
        }
    }
}
