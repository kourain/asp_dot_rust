use std::{
    any::Any,
    sync::{Arc, OnceLock},
};

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ServiceType {
    Singleton,
    Scope,
    Transient,
}
pub(crate) struct ServiceInstance {
    pub service_type: ServiceType,
    pub instance: Arc<OnceLock<Arc<dyn Any + Send + Sync>>>,
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
