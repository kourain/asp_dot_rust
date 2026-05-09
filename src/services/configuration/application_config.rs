use std::fmt::Debug;

use crate::{
    application::{Application, ApplicationBuilder},
    logging::LOGGER,
};

impl ApplicationBuilder {
    pub fn add_default_configuration<T>(&mut self) -> &mut Self
    where
        T: Default + Debug + Clone + Send + Sync + 'static,
    {
        let config = T::default();

        if self.config.contains_key(std::any::type_name::<T>()) {
            panic!("Configuration of type {} already exists", std::any::type_name::<T>());
        }
        LOGGER::info(format!("Add Default Config for: {}", std::any::type_name::<T>()));
        LOGGER::verbose(format!("{:#?}", config.clone()));
        self.config.insert(std::any::type_name::<T>().into(), Box::new(config));
        self
    }

    pub fn add_custom_configuration<T, F>(&mut self, cors_config: F) -> &mut Self
    where
        T: Default + Debug + Clone + Send + Sync + 'static,
        F: FnOnce(&mut T),
    {
        let mut config = T::default();

        cors_config(&mut config);

        if self.config.contains_key(std::any::type_name::<T>()) {
            panic!("Configuration of type {} already exists", std::any::type_name::<T>());
        }
        self.config.insert(std::any::type_name::<T>().into(), Box::new(config.clone()));
        LOGGER::info(format!("Add Custom Config for: {}", std::any::type_name::<T>()));
        self
    }
}

impl Application {
    pub fn get_configuration<T>(&self) -> &T
    where
        T: 'static,
    {
        if let Some(config) = self._config.get(std::any::type_name::<T>()) {
            if let Some(config) = config.downcast_ref::<T>() {
                return config;
            } else {
                LOGGER::error(format!(
                    "Configuration found for type {}, but failed to downcast",
                    std::any::type_name::<T>()
                ));
                panic!("Configuration found for type {}, but failed to downcast", std::any::type_name::<T>());
            }
        } else {
            LOGGER::error(format!("Configuration of type {} not found", std::any::type_name::<T>()));
            panic!("Configuration of type {} not found", std::any::type_name::<T>());
        }
    }
}
