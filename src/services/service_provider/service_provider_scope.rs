use std::{
    any::{Any, TypeId},
    collections::HashMap,
    sync::{Arc, OnceLock},
};

use crate::{dependcy_injection::DependcyInjectableService, services::service_provider::{ServiceInstance, ServiceType}};
pub struct ServiceProviderScope {
    _inner_map: HashMap<TypeId, ServiceInstance>,
}
impl ServiceProviderScope {
    pub(crate) fn new() -> Self {
        Self { _inner_map: HashMap::new() }
    }
    pub fn get_service<T>(&self) -> Arc<T>
    where
        T: DependcyInjectableService + Sized + Send + Sync + 'static,
    {
        let type_id = TypeId::of::<T>();
        match self._inner_map.get(&type_id) {
            Some(instance) => match instance.service_type {
                ServiceType::Singleton | ServiceType::Scope => {
                    return instance
                        .instance
                        .get_or_init(|| Arc::new(T::inject_service(self)))
                        .clone()
                        .downcast::<T>()
                        .expect("Type mismatch when downcasting service");
                }
                ServiceType::Transient => Arc::new(T::inject_service(self)),
            },
            None => {
                panic!("Service {} not found in scope", std::any::type_name::<T>());
            }
        }
    }
    pub fn add_instance<T>(&mut self, service: Arc<T>, service_type: ServiceType)
    where
        T: DependcyInjectableService + Send + Sync + 'static,
    {
        let type_id = TypeId::of::<T>();
        let instance: Arc<OnceLock<Arc<dyn Any + Send + Sync>>> = Arc::new(OnceLock::new());
        _ = instance.set(service);
        self._inner_map.insert(type_id, ServiceInstance { service_type, instance: instance });
    }
    pub fn add_singleton<T>(&mut self)
    where
        T: DependcyInjectableService + Send + Sync + 'static,
    {
        let type_id = TypeId::of::<T>();
        self._inner_map.insert(
            type_id,
            ServiceInstance {
                service_type: ServiceType::Singleton,
                instance: Arc::new(OnceLock::new()),
            },
        );
    }
    pub fn add_scope<T>(&mut self)
    where
        T: DependcyInjectableService + Send + Sync + 'static,
    {
        let type_id = TypeId::of::<T>();
        self._inner_map.insert(
            type_id,
            ServiceInstance {
                service_type: ServiceType::Scope,
                instance: Arc::new(OnceLock::new()),
            },
        );
    }
    pub fn add_transient<T>(&mut self)
    where
        T: DependcyInjectableService + Send + Sync + 'static,
    {
        let type_id = TypeId::of::<T>();
        self._inner_map.insert(
            type_id,
            ServiceInstance {
                service_type: ServiceType::Transient,
                instance: Arc::new(OnceLock::new()),
            },
        );
    }
    pub fn create_scope(&self) -> ServiceProviderScope {
        ServiceProviderScope { _inner_map: self._inner_map.clone() }
    }
    pub fn contains_service<T>(&self) -> bool
    where
        T: 'static,
    {
        let type_id = TypeId::of::<T>();
        self._inner_map.contains_key(&type_id)
    }
    pub fn contains_type_id(&self, type_id: &TypeId) -> bool {
        self._inner_map.contains_key(type_id)
    }
}
