use std::fmt::Debug;

use crate::{application::ApplicationBuilder, logging::LOGGER};

impl ApplicationBuilder {
    pub fn add_default_configuration<T>(&mut self) -> &mut Self
    where
        T: Default + Debug + Clone + Send + Sync + 'static,
    {
        if self.configuration.contains::<T>() {
            panic!("Configuration of type {} already exists", std::any::type_name::<T>());
        }
        LOGGER::info(format!("Add Default Config for: {}", std::any::type_name::<T>()));
        LOGGER::verbose(format!("{:#?}", T::default()));
        self.configuration.insert::<T>(|_| {});
        self
    }

    pub fn add_custom_configuration<T>(&mut self, custom_config: impl Fn(&mut T)) -> &mut Self
    where
        T: Default + Debug + Clone + Send + Sync + 'static,
    {
        if self.configuration.contains::<T>() {
            panic!("Configuration of type {} already exists", std::any::type_name::<T>());
        }
        self.configuration.insert::<T>(custom_config);
        LOGGER::info(format!("Add Custom Config for: {}", std::any::type_name::<T>()));
        self
    }
}
