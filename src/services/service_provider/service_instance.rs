use std::{any::Any, sync::Arc};
use tokio::sync::OnceCell;

use crate::dependency_injection::Serv;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ServiceType {
    Singleton,
    Scope,
    Transient,
}
pub(crate) type ServiceInstanceType = OnceCell<Serv<dyn Any + Send + Sync>>;
pub(crate) struct ServiceInstance {
    pub service_type: ServiceType,
    pub instance: Arc<ServiceInstanceType>,
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
            service_type: self.service_type,
            instance: Arc::new(OnceCell::new()),
        }
    }
}
