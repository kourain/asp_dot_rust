pub mod action_params;
mod action_result;
pub mod macro_rules;
pub(crate) mod routing;
pub mod validation;

pub use crate::macros::{attr, controller_inject, controller_route, delete, get, head, max_body_size, options, patch, post, put, query, route};
pub use action_params::{
    BindError, Body, FieldError, Form, FromBody, FromForm, FromOptionBody, FromOptionForm, FromOptionQuery, FromParamField, FromParamValue, FromQuery, FromRoute, OptionalField,
    RequiredField,
};
pub use action_result::{ActionResult, results};
pub use routing::*;
pub use validation::{Validate, ValidationError};

#[async_trait::async_trait]
pub trait Controller: Send + 'static {
    async fn routing(&mut self, method_name: &'static str) -> Result<Box<dyn ActionResult + '_>, RoutingError>;
}
