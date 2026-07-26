pub use inventory;
use std::any::TypeId;

pub trait DependcyInjectableService: Send + Sync + 'static {
    fn inject_service(service_scope: &crate::services::service_provider::service_provider_scope::ServiceProviderScope) -> Self
    where
        Self: Sized;
}

/// One cạnh trong đồ thị phụ thuộc: owner phụ thuộc vào danh sách dependencies
pub struct DependencyEdge {
    pub owner: fn() -> TypeId,
    pub owner_name: &'static str,
    pub dependencies: fn() -> Vec<(TypeId, &'static str)>,
}

inventory::collect!(DependencyEdge);
