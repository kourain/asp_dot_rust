mod action_result;
pub mod macro_rules;
pub(crate) mod routing;

pub use crate::macros::{controller_inject_require, controller_route, delete, get, head, options, patch, post, put, route};
pub use action_result::ActionResult;
pub use routing::*;
