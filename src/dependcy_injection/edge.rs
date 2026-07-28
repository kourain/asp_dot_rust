pub use inventory;
use std::any::TypeId;

/// One cạnh trong đồ thị phụ thuộc: owner phụ thuộc vào danh sách dependencies
pub struct DependencyEdge {
    pub owner: fn() -> TypeId,
    pub owner_name: &'static str,
    pub dependencies: fn() -> Vec<(TypeId, &'static str)>,
}

inventory::collect!(DependencyEdge);
