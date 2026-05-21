use std::{any::{Any, TypeId}, collections::HashMap, sync::Arc};

pub(crate) struct HttpContextServiceProvider {
    services: HashMap<TypeId, Arc<dyn Any + Send + Sync>>,
}

impl HttpContextServiceProvider {
    pub(crate) fn new() -> Self {
        Self {
            services: HashMap::new(),
        }
    }

    pub(crate) fn add_singleton<T>(&mut self, service: T)
    where
        T: Send + Sync + 'static,
    {
        self.services
            .insert(TypeId::of::<T>(), Arc::new(service));
    }

    pub(crate) fn get_service<T>(&self) -> Option<Arc<T>>
    where
        T: Send + Sync + 'static,
    {
        let type_id = TypeId::of::<T>();

        if let Some(singleton) = self.services.get(&type_id) {
            return Arc::downcast::<T>(Arc::clone(singleton)).ok();
        }

        None
    }
}

impl Default for HttpContextServiceProvider {
    fn default() -> Self {
        Self::new()
    }
}