pub mod configuration;
pub mod routing;

pub use configuration::*;
pub use routing::*;

pub trait Service {
    fn name(&self) -> &'static str;
}
