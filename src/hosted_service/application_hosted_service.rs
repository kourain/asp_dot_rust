use crate::{ApplicationBuilder, hosted_service::BackGroundService, logging::LOGGER};

pub type ApplicationHostedService = Vec<(&'static str,Box<dyn BackGroundService>)>;

impl ApplicationBuilder {
    pub fn add_hosted_service<T>(&mut self) -> &mut Self
    where
        T: BackGroundService + 'static,
    {
        LOGGER::info(format!("Registering background service: {}", std::any::type_name::<T>()));
        self.hosted_services.push((std::any::type_name::<T>(),Box::new(T::with_service_provider())));
        self
    }
}
