use std::{
    any::{TypeId, type_name},
    collections::HashMap,
    sync::Arc,
};

use tokio::sync::OnceCell;

use crate::{
    dependency_injection::DependencyInjectableService,
    services::service_provider::{ServiceInstance, ServiceInstanceType, ServiceType},
};
pub struct ServiceProviderScope {
    _inner_map: HashMap<TypeId, ServiceInstance>,
}
impl ServiceProviderScope {
    pub(crate) fn new() -> Self {
        Self { _inner_map: HashMap::new() }
    }
    pub async fn get_service<T>(&self) -> Arc<T>
    where
        T: DependencyInjectableService + Sized + Send + Sync + 'static,
    {
        let type_id = TypeId::of::<T>();
        match self._inner_map.get(&type_id) {
            Some(instance) => match instance.service_type {
                ServiceType::Singleton | ServiceType::Scope => {
                    let arc_any = instance
                        .instance
                        .get_or_init(|| async {
                            let mut sv = T::inject(self).await;
                            sv.on_service_created().await;
                            Arc::new(sv) as Arc<dyn std::any::Any + Send + Sync>
                        })
                        .await;
                    arc_any.clone().downcast::<T>().expect("Type mismatch when downcasting service")
                }
                ServiceType::Transient => {
                    let mut sv = T::inject(self).await;
                    sv.on_service_created().await;
                    Arc::new(sv)
                }
            },
            None => {
                panic!("Service {} not found in scope", std::any::type_name::<T>());
            }
        }
    }
    pub fn add_instance<T>(&mut self, service: Arc<T>, service_type: ServiceType)
    where
        T: DependencyInjectableService + Send + Sync + 'static,
    {
        if service_type == ServiceType::Transient {
            let type_name = type_name::<T>();
            panic!("Cann't add {type_name} to ServiceProviderScope as Transient")
        }
        let type_id = TypeId::of::<T>();
        if self._inner_map.contains_key(&type_id) {
            let type_name = type_name::<T>();
            panic!("{type_name} already exist in ServiceProviderScope")
        }
        let instance: Arc<ServiceInstanceType> = Arc::new(OnceCell::new());
        let _ = instance.set(service as Arc<dyn std::any::Any + Send + Sync>);
        self._inner_map.insert(type_id, ServiceInstance { service_type, instance });
    }
    pub fn add_singleton<T>(&mut self)
    where
        T: DependencyInjectableService + Send + Sync + 'static,
    {
        let type_id = TypeId::of::<T>();
        self._inner_map.insert(
            type_id,
            ServiceInstance {
                service_type: ServiceType::Singleton,
                instance: Arc::new(OnceCell::new()),
            },
        );
    }
    pub fn add_scope<T>(&mut self)
    where
        T: DependencyInjectableService + Send + Sync + 'static,
    {
        let type_id = TypeId::of::<T>();
        self._inner_map.insert(
            type_id,
            ServiceInstance {
                service_type: ServiceType::Scope,
                instance: Arc::new(OnceCell::new()),
            },
        );
    }
    pub fn add_transient<T>(&mut self)
    where
        T: DependencyInjectableService + Send + Sync + 'static,
    {
        let type_id = TypeId::of::<T>();
        self._inner_map.insert(
            type_id,
            ServiceInstance {
                service_type: ServiceType::Transient,
                instance: Arc::new(OnceCell::new()),
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
