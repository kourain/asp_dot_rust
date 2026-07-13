use std::sync::Arc;

use crate::{ApplicationBuilder, dependcy_injection::DependcyInjectableService, logging::LOGGER};

impl ApplicationBuilder {
    pub fn add_singleton<T>(&mut self)
    where
        T: DependcyInjectableService + Send + Sync + 'static,
    {
        LOGGER::info(format!("Register Singleton Service: {}", std::any::type_name::<T>()));
        self.service_provider.add_singleton::<T>();
    }

    pub fn add_scope<T>(&mut self)
    where
        T: DependcyInjectableService + Send + Sync + 'static,
    {
        LOGGER::info(format!("Register Scope Service: {}", std::any::type_name::<T>()));
        self.service_provider.add_scope::<T>();
    }
    pub fn add_transient<T>(&mut self)
    where
        T: DependcyInjectableService + Send + Sync + 'static,
    {
        LOGGER::info(format!("Register Transient Service: {}", std::any::type_name::<T>()));
        self.service_provider.add_transient::<T>();
    }
    pub fn add_instance<T>(&mut self, instance: T, service_type: crate::services::service_provider::service_provider_scope::ServiceType)
    where
        T: DependcyInjectableService + Send + Sync + 'static,
    {
        LOGGER::info(format!("Register Instance Service: {}", std::any::type_name::<T>()));
        self.service_provider.add_instance::<T>(Arc::new(instance), service_type);
    }
}
