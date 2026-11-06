use crate::{ApplicationBuilder, hosted_service::MemoryCacheBackgroundService, services::memory_cache::MemoryCacheService};

impl ApplicationBuilder {
    pub fn add_memory_cache(&mut self) -> &mut Self {
        self.service.add_singleton::<MemoryCacheService>();
        self.add_hosted_service::<MemoryCacheBackgroundService>();
        self
    }
}
