use std::{collections::HashSet, net::IpAddr, path::Path, sync::Arc};

use crate::{
    Application,
    hosted_service::ApplicationHostedService,
    http_listener::build_tls_server_config,
    logging::LOGGER,
    middleware::app_middlewares::ApplicationMiddlewares,
    services::{
        configuration::ConfigurationService,
        service_provider::{ServiceType, service_provider_scope::ServiceProviderScope},
    },
    utils::build_info,
};

pub struct ApplicationBuilder {
    pub name: String,
    pub ip: HashSet<IpAddr>,
    pub http_port: HashSet<u16>,
    pub https_port: HashSet<u16>,
    pub configuration: ConfigurationService,
    pub service: ServiceProviderScope,
    pub(crate) hosted_services: ApplicationHostedService,
    pub(crate) tls_config: Option<Arc<rustls::ServerConfig>>,
}

impl ApplicationBuilder {
    pub fn new(name: &str) -> Self {
        let mut app = Self {
            name: name.to_string(),
            ip: HashSet::new(),
            http_port: HashSet::new(),
            https_port: HashSet::new(),
            configuration: ConfigurationService::new(),
            service: ServiceProviderScope::new(),
            hosted_services: Vec::new(),
            tls_config: None,
        };
        app.configuration.load_default_toml();
        app.with_args();
        LOGGER::verbose(format!("build at: {}", build_info::get_build_time_utc()));
        LOGGER::info(format!("Initializing application builder: {}", name));
        app
    }

    pub fn with_ip(&mut self, ip: impl Into<String>) -> &mut Self {
        if self.ip.contains(&"0.0.0.0".parse().unwrap()) || self.ip.contains(&"::".parse().unwrap()) {
            self.ip.clear();
        }
        self.ip.insert(ip.into().parse::<std::net::IpAddr>().expect("Invalid IP address format"));
        self
    }

    pub fn with_loopback_ip(&mut self) -> &mut Self {
        if self.ip.contains(&"0.0.0.0".parse().unwrap()) || self.ip.contains(&"::".parse().unwrap()) {
            self.ip.clear();
        }
        self.ip.insert("127.0.0.1".parse().unwrap());
        self
    }
    /// Binds the application to all available network interfaces
    pub fn with_any_ip(&mut self) -> &mut Self {
        self.ip.clear();
        self.ip.insert("0.0.0.0".parse().unwrap());
        self.ip.insert("::".parse().unwrap());
        self
    }
    pub fn with_http_port(&mut self, port: u16) -> &mut Self {
        if self.https_port.contains(&port) {
            panic!("Port {} is already in use by HTTPS. Please choose a different port for HTTP.", port);
        }
        self.http_port.insert(port);
        self
    }

    /// Registers an HTTPS port and loads the TLS certificate/private key
    /// used to serve it. Both files must be PEM-encoded.
    ///
    /// `cert_path` should contain the full certificate chain (leaf certificate
    /// first, followed by any intermediates). `key_path` should contain a
    /// single PKCS#8, PKCS#1 (RSA), or SEC1 (EC) private key.
    ///
    /// Note: only one certificate/key pair is currently supported per
    /// application. Calling this multiple times with different cert/key
    /// paths will overwrite the previously loaded TLS configuration — all
    /// registered HTTPS ports share the same certificate.
    ///
    /// Panics at build time (fail-fast) if the certificate or key file
    /// cannot be read or is malformed, so misconfiguration is caught at
    /// startup instead of on the first incoming HTTPS request.
    pub fn with_https_port(&mut self, port: u16, cert_path: impl AsRef<Path>, key_path: impl AsRef<Path>) -> &mut Self {
        if self.http_port.contains(&port) {
            panic!("Port {} is already in use by HTTP. Please choose a different port for HTTPS.", port);
        }
        self.https_port.insert(port);
        match build_tls_server_config(cert_path.as_ref(), key_path.as_ref()) {
            Ok(config) => {
                self.tls_config = Some(config);
            }
            Err(e) => panic!("Failed to load TLS certificate/key for HTTPS: {}", e),
        }
        self
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
            tls_config: self.tls_config,
            service_provider: service,
            _middlewares: ApplicationMiddlewares::new(),
            _hosted_services: Vec::new(),
            runner_id: uuid::Uuid::now_v7(),
        }
    }
}
