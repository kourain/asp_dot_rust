use std::sync::{LazyLock, Mutex};

use crate::{ApplicationBuilder, controller::ActionRoute, logging::LOGGER, services::routing::RoutingService};

pub static CONTROLLER_REGISTRY: LazyLock<Mutex<RoutingService>> = LazyLock::new(|| Mutex::new(RoutingService::default()));
#[derive(PartialEq, Eq, Hash, Clone, Debug)]
pub struct ControllerCollect {
    pub(crate) type_id: std::any::TypeId,
    pub(crate) type_name: &'static str,
    pub(crate) controller_name: &'static str,
}
pub struct ControllerBootstrapRegistration {
    pub bootstrap: fn() -> ControllerCollect,
}

inventory::collect!(ControllerBootstrapRegistration);

pub(crate) fn bootstrap_registered_controllers() {
    LOGGER::info("Bootstrapping registered controllers...");
    for registration in inventory::iter::<ControllerBootstrapRegistration> {
        let controller_collect = (registration.bootstrap)();
        LOGGER::info(format!("Bootstrapped controller: {}", controller_collect.type_name));
    }
}

pub fn register_controller<T: 'static>(root_route: &str, action_routes: Vec<ActionRoute>) -> ControllerCollect
where
    T: crate::controller::WithHttpContext + crate::controller::Routing + Send + 'static,
{
    let mut registry = CONTROLLER_REGISTRY.lock().expect("Failed to lock controller registry");
    registry.register_controller::<T>(root_route, action_routes)
}

impl ApplicationBuilder {
    pub fn add_controllers(&mut self) -> &mut Self {
        LOGGER::info("Registering controllers...");
        bootstrap_registered_controllers();
        let routing_snapshot = CONTROLLER_REGISTRY.lock().expect("Failed to lock controller registry").clone();
        LOGGER::debug(format!("{:#?}", routing_snapshot));
        self.service_provider.add_singleton::<RoutingService>(routing_snapshot);
        self
    }
}
