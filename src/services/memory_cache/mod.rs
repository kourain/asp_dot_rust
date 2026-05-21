use crate::{ApplicationBuilder, hosted_service::MemoryCacheBackgroundService};

#[derive(Default)]
pub struct MemoryCacheService{

}
impl ApplicationBuilder{
    pub fn add_memory_cache(&mut self) -> &mut Self{
        self.add_singleton::<MemoryCacheService>();
        self.add_hosted_service::<MemoryCacheBackgroundService>();
        self
    }
}