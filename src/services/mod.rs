pub mod app_queue;
pub mod configuration;
pub mod memory_cache;
pub mod routing;
pub mod service_provider;

pub trait Service {
    fn name(&self) -> &'static str;
    // fn inject_service(config: ApplicationConfiguration, service_sopce: ServiceProviderScope){}
}
