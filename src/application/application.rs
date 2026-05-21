use std::{collections::HashSet, net::IpAddr, sync::Arc};

use tokio::task::JoinSet;

use crate::{
    ApplicationBuilder,
    hosted_service::ApplicationHostedService,
    http_listener::run_http_server_async,
    logging::LOGGER,
    middleware::{app_middlewares::ApplicationMiddlewares, auto_route::AutoRouteMiddleware, request_timeout::RequestTimeoutMiddleware},
    services::configuration::ApplicationConfiguration,
    services::service_provider::application_scope::ApplicationServiceProvider,
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
        self.add_middleware::<RequestTimeoutMiddleware>();
        self.add_middleware::<AutoRouteMiddleware>();

        if self.ip.is_empty() {
            self.ip.insert("127.0.0.1".parse::<std::net::IpAddr>().unwrap());
        }
        if self.http_port.is_empty() {
            self.http_port.insert(8080);
        }
        self._middlewares.build_pipeline();
        let mut hosted_services_app = self._hosted_services;
        self._hosted_services = Vec::new(); // clear hosted services from app since we're moving them to the async block
        let app = Arc::new(self);
        tokio::try_join!(
            futures::future::try_join_all(hosted_services_app.iter().map(|bg_service| async move {
                LOGGER::info(format!("Starting background service: {}", bg_service.0));
                bg_service.1.invoke_async().await;
                Ok(())
            })),
            run_http_server_async(app)
        )?;

        Ok(())
    }
}
