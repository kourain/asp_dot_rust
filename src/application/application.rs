use std::{collections::HashSet, net::IpAddr, sync::Arc};

use crate::{
    ApplicationBuilder,
    hosted_service::ApplicationHostedService,
    http_listener::run_http_server_async,
    logging::LOGGER,
    middleware::pipeline::ApplicationMiddlewares,
    services::{configuration::ApplicationConfiguration, service_provider::application_scope::ApplicationServiceProvider},
};

pub struct Application {
    pub name: String,
    pub runner_id: uuid::Uuid,
    pub ip: HashSet<IpAddr>,
    pub http_port: HashSet<u16>,
    pub https_port: HashSet<u16>,
    pub service: ApplicationServiceProvider,
    pub(crate) _config: ApplicationConfiguration,
    pub(crate) _middlewares: ApplicationMiddlewares,
    pub(crate) _hosted_services: ApplicationHostedService,
}
impl Application {
    pub fn new(name: &str) -> ApplicationBuilder {
        ApplicationBuilder::new(name)
    }

    pub async fn run(mut self) -> std::io::Result<()> {
        // ensure defaults so the server keeps running even if user didn't set ip/ports
        // self.add_middleware::<RequestTimeoutMiddleware>();
        // self.add_middleware::<AutoRouteMiddleware>();

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
        _ = run_http_server_async(app).await;
        Ok(())
    }
}
