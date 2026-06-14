use std::sync::Arc;

use crate::http_context::http_request::HttpRequest;
use crate::http_context::http_response::HttpResponse;
use crate::logging::LOGGER;
use crate::services::service_provider::application_scope::ApplicationServiceProvider;
use crate::services::service_provider::http_request_scope::HttpContextServiceProvider;
use crate::services::configuration::ApplicationConfiguration;
pub struct HttpContext {
    pub request: HttpRequest,
    pub response: HttpResponse,
    pub routing_info: Option<crate::services::routing::ResolvedRoute>,
    pub(crate) services: HttpContextServiceProvider,
    pub(crate) application_config: ApplicationConfiguration,
    pub(crate) application_service_provider: ApplicationServiceProvider,
}

impl HttpContext {
    pub(crate) fn new(
        request: HttpRequest,
        response: HttpResponse,
        application_config: ApplicationConfiguration,
        application_service_provider: ApplicationServiceProvider,
    ) -> Self {
        Self {
            request,
            response,
            routing_info: None,
            services: HttpContextServiceProvider::new(),
            application_config,
            application_service_provider,
        }
    }
    pub fn get_service<T>(&mut self) -> Arc<T>
    where
        T: Default + Send + Sync + 'static,
    {
        if let Some(service) = self.services.get_service::<T>() {
            return service;
        }
        let service = self.application_service_provider.get_service::<T>();
        self.services.add_singleton(service.clone());
        service
    }

    pub fn try_get_service<T>(&self) -> Option<Arc<T>>
    where
        T: Default + Send + Sync + 'static,
    {
        if let Some(service) = self.services.get_service::<T>() {
            return Some(service);
        }

        self.application_service_provider.try_get_service::<T>()
    }

    pub fn get_app_config<T>(&self) -> T
    where
        T: Default + Clone + 'static,
    {
        let config = self.application_config.get(std::any::type_name::<T>());
        if config.is_none() {
            LOGGER::error(format!("Configuration of type {} not found", std::any::type_name::<T>()));
            panic!("Configuration of type {} not found", std::any::type_name::<T>());
        }
        config.and_then(|v| v.downcast_ref::<T>()).cloned().unwrap_or_else(|| {
            LOGGER::error(format!("Configuration of type {} not found", std::any::type_name::<T>()));
            panic!("Configuration of type {} not found", std::any::type_name::<T>());
        })
    }
}
