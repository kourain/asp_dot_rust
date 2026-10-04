mod action_result;
pub mod macro_rules;
pub(crate) mod routing;

pub use crate::macros::{controller_inject, controller_route, delete, get, head, options, patch, post, put, route};
pub use action_result::ActionResult;
pub use routing::*;

pub struct FromQuery<T>(T)
where
    T: Default;

pub struct FromBody<T>(T)
where
    T: Default;

pub struct FromForm<T>(T)
where
    T: Default;
