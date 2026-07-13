use std::fmt::Debug;

use crate::{application::ApplicationBuilder, logging::LOGGER};

impl ApplicationBuilder {
    pub fn add_default_configuration<T>(&mut self) -> &mut Self
    where
        T: Default + Debug + Clone + Send + Sync + 'static,
    {
        let config = T::default();

        if self.configuration.constains::<T>() {
            panic!("Configuration of type {} already exists", std::any::type_name::<T>());
        }
        LOGGER::info(format!("Add Default Config for: {}", std::any::type_name::<T>()));
        LOGGER::verbose(format!("{:#?}", config.clone()));
        self.configuration.insert::<T>(config);
        self
    }

    pub fn add_custom_configuration<T>(&mut self, cors_config: impl FnOnce(&mut T)) -> &mut Self
    where
        T: Default + Debug + Clone + Send + Sync + 'static,
    {
        let mut config = T::default();

        cors_config(&mut config);

        if self.configuration.constains::<T>() {
            panic!("Configuration of type {} already exists", std::any::type_name::<T>());
        }
        self.configuration.insert::<T>(config);
        LOGGER::info(format!("Add Custom Config for: {}", std::any::type_name::<T>()));
        self
    }
}
