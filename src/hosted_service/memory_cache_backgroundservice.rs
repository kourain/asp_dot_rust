use std::sync::atomic::AtomicU64;

use async_trait::async_trait;

use crate::{hosted_service::BackGroundService, logging::LOGGER, services::memory_cache::MemoryCacheService};

pub struct MemoryCacheBackgroundService {
    memcache: MemoryCacheService,
    release_after_seconds: AtomicU64,
}
#[async_trait]
impl BackGroundService for MemoryCacheBackgroundService {
    fn with_service_provider() -> Self
    where
        Self: Sized,
    {
        MemoryCacheBackgroundService {
            memcache: MemoryCacheService::default(),
            release_after_seconds: AtomicU64::new(300), // default 5 minutes
        }
    }
    async fn invoke_async(&mut self) {
        LOGGER::verbose("run memory cache release");
        loop {
            self.memcache.release_expired_cache();
            tokio::time::sleep(std::time::Duration::from_secs(self.release_after_seconds.load(std::sync::atomic::Ordering::Relaxed))).await;
        }
    }
}
