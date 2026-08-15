pub(crate) mod http_context;
pub mod http_header;
pub(crate) mod http_request;
pub(crate) mod http_response;

use crate::utils::ShareMutPtr;
pub use http_context::HttpContext;
pub use http_header::AspDotRustHttpHeader;

pub type HttpContextRef = ShareMutPtr<http_context::HttpContext>;
