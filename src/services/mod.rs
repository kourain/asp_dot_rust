use crate::services::{configuration::ApplicationConfiguration, service_provider::service_provider_scope::ServiceProviderScope};

pub mod configuration;
pub mod memory_cache;
pub mod routing;
pub mod service_provider;
pub mod app_queue;

pub trait Service {
    fn name(&self) -> &'static str;
    // fn inject_service(config: ApplicationConfiguration, service_sopce: ServiceProviderScope){}
}
