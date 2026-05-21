pub mod configuration;
pub mod memory_cache;
pub mod routing;

pub trait Service {
    fn name(&self) -> &'static str;
}
