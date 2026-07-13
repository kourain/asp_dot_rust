use crate::{http_context, utils::ShareMutPtr};
pub type HttpContextRef = ShareMutPtr<http_context::HttpContext>;

pub trait WithHttpContext {
    fn str_name() -> &'static str {
        "UnknownController"
    }
    fn new(http_context: HttpContextRef) -> Self;
    // fn routing(&mut self);
}
