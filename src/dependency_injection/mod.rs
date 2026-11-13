mod cycle_check;
mod edge;
mod inject_types;

pub use edge::DependencyEdge;
#[allow(deprecated)]
pub use inject_types::Cfg;
pub use inject_types::{CfgOption, CfgReload, CfgRequire, Serv};

#[async_trait::async_trait]
pub trait DependencyInjectableService: Send + Sync + 'static {
    async fn inject(service_scope: &crate::services::service_provider::service_provider_scope::ServiceProviderScope) -> Self
    where
        Self: Sized;
}
#[async_trait::async_trait]
pub trait DependencyInjectableController: Send + Sync + 'static {
    async fn inject(http_context: &mut crate::http_context::HttpContext, is_valid: std::sync::Arc<std::sync::atomic::AtomicBool>) -> Self
    where
        Self: Sized;
}
