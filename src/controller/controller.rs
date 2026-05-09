use std::ops::{Deref, DerefMut};

use crate::http_context;

#[derive(Copy, Clone)]
pub struct HttpContextRef(*mut http_context::HttpContext);

unsafe impl Send for HttpContextRef {}
unsafe impl Sync for HttpContextRef {}

impl HttpContextRef {
    pub fn new(http_context: &mut http_context::HttpContext) -> Self {
        Self(http_context as *mut http_context::HttpContext)
    }
}

impl Deref for HttpContextRef {
    type Target = http_context::HttpContext;

    fn deref(&self) -> &Self::Target {
        unsafe { &*self.0 }
    }
}

impl DerefMut for HttpContextRef {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { &mut *self.0 }
    }
}

pub trait WithHttpContext {
    fn str_name() -> &'static str;
    fn new_with_http_context(http_context: &mut http_context::HttpContext) -> Self;
    // fn routing(&mut self);
}