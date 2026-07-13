mod application_hosted_service;
mod memory_cache_backgroundservice;

use crate::dependcy_injection::InjectableService;
pub use application_hosted_service::ApplicationHostedService;
pub use memory_cache_backgroundservice::MemoryCacheBackgroundService;

#[async_trait::async_trait]
pub trait BackGroundService: InjectableService + Send + Sync {
    async fn invoke_async(&mut self);
}
