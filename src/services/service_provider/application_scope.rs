use crate::{ApplicationBuilder, dependcy_injection::InjectableService, logging::LOGGER};

impl ApplicationBuilder {
    pub fn add_singleton<T>(&mut self)
    where
        T: InjectableService + Send + Sync + 'static,
    {
        LOGGER::info(format!("Register Singleton Service: {}", std::any::type_name::<T>()));
        self.service_provider.add_singleton::<T>();
    }

    pub fn add_scope<T>(&mut self)
    where
        T: InjectableService + Send + Sync + 'static,
    {
        LOGGER::info(format!("Register Scope Service: {}", std::any::type_name::<T>()));
        self.service_provider.add_scope::<T>();
    }
    pub fn add_transient<T>(&mut self)
    where
        T: InjectableService + Send + Sync + 'static,
    {
        LOGGER::info(format!("Register Transient Service: {}", std::any::type_name::<T>()));
        self.service_provider.add_transient::<T>();
    }
}
