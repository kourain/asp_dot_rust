use crate::{http_context, utils::ShareMutPtr};
pub type HttpContextRef = ShareMutPtr<http_context::HttpContext>;
pub trait StructName {
    fn str_name() -> &'static str;
}
pub trait WithHttpContext: StructName {
    fn new(http_context: HttpContextRef) -> Self;
    // fn routing(&mut self);
}
