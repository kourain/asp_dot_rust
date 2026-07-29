use std::{collections::HashSet, net::IpAddr, sync::Arc};

#[cfg(debug_assertions)]
use crate::dependcy_injection::cycle_check::check_dependency_cycles;
use crate::{
    Application, hosted_service::ApplicationHostedService, logging::LOGGER, middleware::app_middlewares::ApplicationMiddlewares, services::{configuration::ConfigurationService, service_provider::{ServiceType, service_provider_scope::ServiceProviderScope}}, utils::build_info,
};

pub struct ApplicationBuilder {
    pub name: String,
    pub ip: HashSet<IpAddr>,
    pub http_port: HashSet<u16>,
    pub https_port: HashSet<u16>,
    pub configuration: ConfigurationService,
    pub service: ServiceProviderScope,
    pub(crate) hosted_services: ApplicationHostedService,
}

impl ApplicationBuilder {
    pub fn new(name: &str) -> Self {
        LOGGER::verbose(format!("build at: {}", build_info::get_build_time_utc()));
        LOGGER::info(format!("Initializing application builder: {}", name));
        Self {
            name: name.to_string(),
            ip: HashSet::new(),
            http_port: HashSet::new(),
            https_port: HashSet::new(),
            configuration: ConfigurationService::new(),
            service: ServiceProviderScope::new(),
            hosted_services: Vec::new(),
        }
    }

    pub fn with_ip(&mut self, ip: impl Into<String>) -> &mut Self {
        self.ip.insert(ip.into().parse::<std::net::IpAddr>().expect("Invalid IP address format"));
        self
    }

    pub fn with_loopback_ip(&mut self) -> &mut Self {
        self.ip.insert("127.0.0.1".parse().unwrap());
        self
    }
    /// Binds the application to all available network interfaces
    pub fn with_any_ip(&mut self) -> &mut Self {
        self.ip.insert("0.0.0.0".parse().unwrap());
        self.ip.insert("::".parse().unwrap());
        self
    }
    pub fn with_http_port(&mut self, port: u16) -> &mut Self {
        self.http_port.insert(port);
        self
    }

    pub fn with_https_port(&mut self, port: u16) -> &mut Self {
        self.https_port.insert(port);
        todo!("ADD SSL SUPPORT");
    }

    pub fn build(self) -> Application {
        let mut service = self.service;
        service.add_instance::<ConfigurationService>(Arc::new(self.configuration), ServiceType::Singleton);

        #[cfg(debug_assertions)]
        check_dependency_cycles(&service);

        Application {
            name: self.name,
            ip: self.ip,
            http_port: self.http_port,
            https_port: self.https_port,
            service_provider: service,
            _middlewares: ApplicationMiddlewares::new(),
            _hosted_services: Vec::new(),
            runner_id: uuid::Uuid::now_v7(),
        }
    }
}
