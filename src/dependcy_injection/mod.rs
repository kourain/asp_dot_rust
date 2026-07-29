pub mod cycle_check;
pub mod edge;

pub use edge::DependencyEdge;
pub use inventory;

use crate::utils::StructName;

pub trait DependcyInjectableService: Send + Sync + 'static {
    fn inject(service_scope: &crate::services::service_provider::service_provider_scope::ServiceProviderScope) -> Self
    where
        Self: Sized;
}
pub trait DependcyInjectableController: StructName + Send + Sync + 'static {
    fn inject(http_context: crate::controller::HttpContextRef) -> Self
    where
        Self: Sized;
}
