use std::{
    any::{Any, TypeId},
    collections::HashMap,
    sync::Arc,
};

use crate::dependcy_injection::DependcyInjectableService;
pub struct ConfigurationService {
    _inner: HashMap<TypeId, Arc<dyn Any + Send + Sync>>,
}
impl DependcyInjectableService for ConfigurationService {
    fn inject(_service_provider: &crate::services::service_provider::ServiceProviderScope) -> Self {
        Self::new()
    }
}
impl ConfigurationService {
    pub fn new() -> Self {
        Self { _inner: HashMap::new() }
    }
    pub fn constains<T: 'static + Send + Sync>(&self) -> bool {
        self._inner.contains_key(&TypeId::of::<T>())
    }
    pub fn insert<T: 'static + Send + Sync>(&mut self, config: T)
    where
        T: Clone + Send + Sync + 'static,
    {
        self._inner.insert(TypeId::of::<T>(), Arc::new(config));
    }
    pub fn get<T: 'static + Send + Sync>(&self) -> Option<Arc<T>>
    where
        T: Clone + Send + Sync + 'static,
    {
        match self._inner.get(&TypeId::of::<T>()) {
            Some(config) => config.clone().downcast::<T>().ok(),
            None => None,
        }
    }
}
