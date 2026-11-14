mod http_context;
mod http_header;
mod http_request;
mod http_response;

use crate::utils::ShareMutPtr;
pub use http_context::{HttpContext, USER_CONTEXT_KEY};
pub use http_header::HttpHeaderExt;
pub use http_request::{HttpRequest, RequestBody};
pub use http_response::HttpResponse;

/// public HttpContext to Controller::http_context
pub type HttpContextRef = ShareMutPtr<http_context::HttpContext>;
