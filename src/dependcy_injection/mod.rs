pub mod cycle_check;
pub mod edge;

pub use edge::DependencyEdge;
pub use inventory;
pub trait DependcyInjectableService: Send + Sync + 'static {
    fn inject_service(service_scope: &crate::services::service_provider::service_provider_scope::ServiceProviderScope) -> Self
    where
        Self: Sized;
}
