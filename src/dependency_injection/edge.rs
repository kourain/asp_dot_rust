use std::any::TypeId;

use crate::services::configuration::ConfigurationService;

#[doc(hidden)]
pub struct ServiceDependencies {
    pub owner: fn() -> TypeId,
    pub owner_name: fn() -> &'static str,
    pub dependencies: fn() -> Vec<Dependency>,
}

inventory::collect!(ServiceDependencies);

#[doc(hidden)]
pub enum Dependency {
    Service(TypeId, &'static str),
    Configuration {
        name: &'static str,
        problem: fn(&ConfigurationService) -> Option<&'static str>,
    },
}

impl Dependency {
    pub fn service<T: 'static>() -> Self {
        Self::Service(TypeId::of::<T>(), std::any::type_name::<T>())
    }

    pub fn configuration<T: Send + Sync + 'static, const REQUIRE_VALUE: bool>() -> Self {
        Self::Configuration {
            name: std::any::type_name::<T>(),
            problem: |configuration| {
                if !configuration.contains::<T>() {
                    Some("is not registered")
                } else if REQUIRE_VALUE && !configuration.get::<T>().capture().has_value() {
                    Some("has no value")
                } else {
                    None
                }
            },
        }
    }
}
