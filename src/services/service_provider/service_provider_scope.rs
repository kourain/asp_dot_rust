use std::{
    any::{Any, TypeId},
    sync::{Arc, OnceLock},
};

use crate::dependcy_injection::InjectableService;
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ServiceType {
    Singleton,
    Scope,
    Transient,
}
struct ServiceInstance {
    service_type: ServiceType,
    instance: Arc<OnceLock<Arc<dyn Any + Send + Sync>>>,
}
impl Clone for ServiceInstance {
    fn clone(&self) -> Self {
        if self.service_type == ServiceType::Singleton {
            return ServiceInstance {
                service_type: self.service_type,
                instance: self.instance.clone(),
            };
        }
        ServiceInstance {
            service_type: self.service_type.clone(),
            instance: Arc::new(OnceLock::new()),
        }
    }
}
pub struct ServiceProviderScope {
    _inner_map: dashmap::DashMap<TypeId, ServiceInstance>,
}
impl ServiceProviderScope {
    pub(crate) fn new() -> Self {
        Self { _inner_map: dashmap::DashMap::new() }
    }
    pub fn get_service<T>(&self) -> Arc<T>
    where
        T: InjectableService + Sized + Send + Sync + 'static,
    {
        let type_id = TypeId::of::<T>();
        match self._inner_map.get(&type_id) {
            Some(instance) => match instance.service_type {
                ServiceType::Singleton => {
                    return instance
                        .instance
                        .get_or_init(|| Arc::new(T::inject_service(self)))
                        .clone()
                        .downcast::<T>()
                        .expect("Type mismatch when downcasting service");
                }
                ServiceType::Scope => {
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
    pub fn add_instance_type<T>(&mut self, service: Arc<T>, service_type: ServiceType)
    where
        T: InjectableService + Send + Sync + 'static,
    {
        let type_id = TypeId::of::<T>();
        let instance: Arc<OnceLock<Arc<dyn Any + Send + Sync>>> = Arc::new(OnceLock::new());
        _ = instance.set(service);
        self._inner_map.insert(type_id, ServiceInstance { service_type, instance: instance });
    }
    pub fn add_singleton<T>(&mut self)
    where
        T: InjectableService + Send + Sync + 'static,
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
        T: InjectableService + Send + Sync + 'static,
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
        T: InjectableService + Send + Sync + 'static,
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
}
