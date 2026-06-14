use crate::{http_context, utils::ShareMutPtr};
pub type HttpContextRef = ShareMutPtr<http_context::HttpContext>;

pub trait WithHttpContext {
    fn str_name() -> &'static str;
    fn new_with_http_context(http_context: &mut http_context::HttpContext) -> Self;
    // fn routing(&mut self);
}
