use std::{
    any::{Any, TypeId},
    collections::{HashMap, HashSet},
    sync::Arc,
};

use crate::{Application, ApplicationBuilder, logging::LOGGER};

pub type ApplicationServiceProvider = Arc<ServiceProvider>;
pub struct ServiceProvider {
    scoped_factories: HashSet<TypeId>,
    singleton_services: HashMap<TypeId, Arc<dyn Any + Send + Sync>>,
}

impl ServiceProvider {
    pub fn new() -> Self {
        Self {
            scoped_factories: HashSet::new(),
            singleton_services: HashMap::new(),
        }
    }

    pub fn add_scope<T>(&mut self)
    where
        T: Default + Send + Sync + 'static,
    {
        self.scoped_factories.insert(TypeId::of::<T>());
        LOGGER::info(format!("Registered Scoped Service: {}", std::any::type_name::<T>()));
    }

    pub fn add_singleton<T>(&mut self, service: T)
    where
        T: Default + Send + Sync + 'static,
    {
        self.singleton_services.insert(TypeId::of::<T>(), Arc::new(service));
        LOGGER::info(format!("Registered Singleton Service: {}", std::any::type_name::<T>()));
    }

    pub fn get_service<T>(&self) -> Arc<T>
    where
        T: Default + Send + Sync + 'static,
    {
        let type_id = TypeId::of::<T>();

        if let Some(singleton) = self.singleton_services.get(&type_id) {
            return singleton.clone().downcast::<T>().unwrap();
        }
        if self.scoped_factories.contains(&TypeId::of::<T>()) {
            #[allow(unreachable_code)]
            return Arc::new(T::default());
        }
        panic!("Service {} not found", std::any::type_name::<T>());
    }

    pub fn try_get_service<T>(&self) -> Option<Arc<T>>
    where
        T: Default + Send + Sync + 'static,
    {
        let type_id = TypeId::of::<T>();

        if let Some(singleton) = self.singleton_services.get(&type_id) {
            return Arc::downcast::<T>(Arc::clone(singleton)).ok();
        }

        if self.scoped_factories.contains(&TypeId::of::<T>()) {
            #[allow(unreachable_code)]
            return Some(Arc::new(T::default()));
        }
        None
    }
}

impl Default for ServiceProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl Application {
    pub fn get_service<T>(&self) -> Arc<T>
    where
        T: Default + Send + Sync + 'static,
    {
        self.service.get_service::<T>()
    }

    pub fn try_get_service<T>(&self) -> Option<Arc<T>>
    where
        T: Default + Send + Sync + 'static,
    {
        self.service.try_get_service::<T>()
    }
}
impl ApplicationBuilder {
    pub fn add_singleton<T>(&mut self)
    where
        T: Default + Send + Sync + 'static,
    {
        LOGGER::info(format!("Register Singleton Service: {}", std::any::type_name::<T>()));
        self.service_provider.add_singleton::<T>(T::default());
    }

    pub fn add_scope<T>(&mut self)
    where
        T: Default + Send + Sync + 'static,
    {
        self.service_provider.add_scope::<T>();
    }
}
