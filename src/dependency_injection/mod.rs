mod cycle_check;
mod edge;
mod inject_types;

#[doc(hidden)]
pub use edge::{Dependency, ServiceDependencies};
#[allow(deprecated)]
pub use inject_types::Cfg;
pub use inject_types::{CfgOption, CfgReload, CfgRequire, Serv};

#[async_trait::async_trait]
pub trait DependencyInjectableService: Send + Sync + 'static {
    async fn inject(service_scope: &crate::services::service_provider::ServiceProviderScope) -> Self
    where
        Self: Sized;
    fn forgot_register_message() -> String
    where
        Self: Sized,
    {
        format!("Service {} not registered yet, did you forget?", std::any::type_name::<Self>())
    }
}

#[async_trait::async_trait]
pub trait DependencyInjectableController: Send + Sync + 'static {
    async fn inject(http_context: crate::http_context::HttpContextRef) -> Self
    where
        Self: Sized;
}

/// Creates a middleware from the services and the next middleware.
#[async_trait::async_trait]
pub trait DependencyInjectableMiddleware {
    async fn inject(service_scope: &crate::services::service_provider::ServiceProviderScope, next: crate::middleware::MiddlewareNext) -> Self
    where
        Self: Sized;
}
