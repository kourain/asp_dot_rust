pub use inventory;
use std::any::TypeId;

/// One edge in the dependency graph is dependent on the list of dependencies: the owner depends on the list of dependencies.
pub struct DependencyEdge {
    pub owner: fn() -> TypeId,
    pub owner_name: &'static str,
    pub dependencies: fn() -> Vec<(TypeId, &'static str)>,
}

inventory::collect!(DependencyEdge);
