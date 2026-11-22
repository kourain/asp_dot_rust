use crate::{ApplicationBuilder, logging::LOGGER, services::routing::RoutingServiceBuilder};

pub struct ControllerBootstrapRegistration {
    pub bootstrap: fn(&mut RoutingServiceBuilder),
}

inventory::collect!(ControllerBootstrapRegistration);

impl RoutingServiceBuilder {
    pub(crate) fn bootstrap_registered_controllers(&mut self) {
        LOGGER::info("Bootstrapping registered controllers...");
        for registration in inventory::iter::<ControllerBootstrapRegistration> {
            (registration.bootstrap)(self);
        }
    }
}

impl ApplicationBuilder {
    pub fn add_controllers(&mut self) -> &mut Self {
        self.routing_builder.bootstrap_registered_controllers();
        self
    }
}
