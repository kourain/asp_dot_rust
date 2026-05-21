mod application_hosted_service;
mod memory_cache_backgroundservice;

pub use memory_cache_backgroundservice::MemoryCacheBackgroundService;
pub use application_hosted_service::ApplicationHostedService;

#[async_trait::async_trait]
pub trait BackGroundService: Send + Sync {
    fn with_service_provider() -> Self
    where
        Self: Sized;
    async fn invoke_async(&mut self);
}
