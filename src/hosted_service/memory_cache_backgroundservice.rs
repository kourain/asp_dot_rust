use async_trait::async_trait;
use std::sync::atomic::AtomicU64;

use crate::{
    dependency_injection::{DependencyInjectableService, Serv}, hosted_service::BackGroundService, logging::LOGGER, services::{memory_cache::MemoryCacheService, service_provider::service_provider_scope::ServiceProviderScope},
};

pub struct MemoryCacheBackgroundService {
    memcache: Serv<MemoryCacheService>,
    release_after_seconds: AtomicU64,
}
#[async_trait]
impl BackGroundService for MemoryCacheBackgroundService {
    async fn invoke_async(&mut self) {
        loop {
            LOGGER::verbose("run memory cache release");
            self.memcache.release_expired_cache();
            tokio::time::sleep(std::time::Duration::from_secs(self.release_after_seconds.load(std::sync::atomic::Ordering::Relaxed))).await;
        }
    }
}
#[async_trait::async_trait]
impl DependencyInjectableService for MemoryCacheBackgroundService {
    async fn inject(service_scope: &ServiceProviderScope) -> Self {
        let memcache = service_scope.get_service::<MemoryCacheService>().await;
        MemoryCacheBackgroundService {
            memcache,
            release_after_seconds: AtomicU64::new(60),
        }
    }
}
