pub mod cycle_check;
pub mod edge;
pub mod inject_types;

pub use edge::DependencyEdge;
pub use inject_types::{Cfg, CfgReload, CfgRequire, Serv};
pub use inventory;

#[async_trait::async_trait]
pub trait DependencyInjectableService: Send + Sync + 'static {
    async fn inject(
        service_scope: &crate::services::service_provider::ServiceProviderScope,
    ) -> Self
    where
        Self: Sized;
    async fn on_service_created(&mut self) {}
}
pub trait DependencyInjectableController: Send + Sync + 'static {
    fn inject(
        http_context: &mut crate::http_context::HttpContext,
        is_valid: std::sync::Arc<std::sync::atomic::AtomicBool>,
    ) -> Self
    where
        Self: Sized;
}
