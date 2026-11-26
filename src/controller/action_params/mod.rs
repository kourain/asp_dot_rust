mod body;
mod body_reader;
mod error;
mod form;
mod from_body;
mod from_form;
mod from_param_value;
mod from_query;
mod from_route;
mod marker;

pub use body::Body;
pub use error::BindError;
pub use form::Form;
pub use from_body::{FromBody, FromOptionBody};
pub use from_form::{FromForm, FromOptionForm};
pub use from_param_value::{FieldError, FromParamField, FromParamValue};
pub use from_query::{FromOptionQuery, FromQuery};
pub use from_route::FromRoute;
pub use marker::{OptionalField, RequiredField};

/// Fallback used by controller actions for whole-body DTOs. The inherent `bind` on
/// `Body<String>` and `Form<String>` takes precedence for raw text.
#[doc(hidden)]
pub trait BindSerdeBody: Sized {
    fn bind(ctx: &mut crate::http_context::HttpContext) -> impl std::future::Future<Output = Result<Self, BindError>> + Send;
}
