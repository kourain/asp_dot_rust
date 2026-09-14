mod application_hosted_service;
mod memory_cache_backgroundservice;

use crate::dependency_injection::DependencyInjectableService;
pub use application_hosted_service::ApplicationHostedService;
pub use memory_cache_backgroundservice::MemoryCacheBackgroundService;

#[async_trait::async_trait]
pub trait BackGroundService: DependencyInjectableService + Send + Sync {
    async fn invoke_async(&mut self);
}
