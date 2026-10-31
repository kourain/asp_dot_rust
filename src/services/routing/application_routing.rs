use std::sync::Arc;

use crate::{
    ApplicationBuilder,
    logging::LOGGER,
    services::{routing::RoutingServiceBuilder, service_provider::ServiceType},
};

pub struct ControllerBootstrapRegistration {
    pub bootstrap: fn(&mut RoutingServiceBuilder),
}

inventory::collect!(ControllerBootstrapRegistration);

pub(crate) fn bootstrap_registered_controllers() -> RoutingServiceBuilder {
    LOGGER::info("Bootstrapping registered controllers...");
    let mut routing_sv = RoutingServiceBuilder::default();
    for registration in inventory::iter::<ControllerBootstrapRegistration> {
        (registration.bootstrap)(&mut routing_sv);
    }
    routing_sv
}

impl ApplicationBuilder {
    pub fn add_controllers(&mut self) -> &mut Self {
        let routing_snapshot = bootstrap_registered_controllers();
        self.service.add_instance(Arc::new(routing_snapshot.build()), ServiceType::Singleton);
        self
    }
}
