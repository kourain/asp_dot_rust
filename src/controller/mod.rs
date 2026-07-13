mod action_result;
pub(crate) mod app_controller;
pub(crate) mod controller;
pub mod macro_rules;
pub(crate) mod routing;

pub use action_result::ActionResult;
pub use asp_dot_rust_macros::{delete, get, head, options, patch, post, put, route};
pub use controller::{HttpContextRef, StructName, WithHttpContext};
pub use routing::*;
