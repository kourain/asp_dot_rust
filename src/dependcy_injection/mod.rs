pub mod cycle_check;
pub mod edge;

pub use edge::DependencyEdge;
pub use inventory;

    pub trait DependcyInjectableService: Send + Sync + 'static {
    fn inject(service_scope: &crate::services::service_provider::service_provider_scope::ServiceProviderScope) -> Self
    where
        Self: Sized;
}
pub trait DependcyInjectableController: Send + Sync + 'static {
    fn inject(http_context: &mut crate::http_context::HttpContext, is_valid: std::sync::Arc<std::sync::atomic::AtomicBool>) -> Self
    where
        Self: Sized;
}
