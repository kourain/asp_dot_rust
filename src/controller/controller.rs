use crate::{http_context, utils::{ShareMutPtr, StructName}};
pub type HttpContextRef = ShareMutPtr<http_context::HttpContext>;

pub trait WithHttpContext: StructName {
    fn new(http_context: HttpContextRef) -> Self;
    // fn routing(&mut self);
}
