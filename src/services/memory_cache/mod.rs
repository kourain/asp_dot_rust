mod memory_cache_service;

use crate::{ApplicationBuilder, hosted_service::MemoryCacheBackgroundService};
pub use memory_cache_service::MemoryCacheService;
impl ApplicationBuilder {
    pub fn add_memory_cache(&mut self) -> &mut Self {
        self.add_singleton::<MemoryCacheService>();
        self.add_hosted_service::<MemoryCacheBackgroundService>();
        self
    }
}
