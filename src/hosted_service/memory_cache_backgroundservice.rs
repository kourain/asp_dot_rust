use async_trait::async_trait;

use crate::{hosted_service::BackGroundService, logging::LOGGER, services::memory_cache::MemoryCacheService};

pub struct MemoryCacheBackgroundService {
    memcache: MemoryCacheService,
}
#[async_trait]
impl BackGroundService for MemoryCacheBackgroundService {
    fn with_service_provider() -> Self
    where
        Self: Sized,
    {
        MemoryCacheBackgroundService { memcache: MemoryCacheService::default() }
    }
    async fn invoke_async(&mut self) {
        LOGGER::verbose("run memory cache release");
    }
}
