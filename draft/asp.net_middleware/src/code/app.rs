use std::sync::{Arc, atomic::AtomicU64};

use crate::code::mw::{ApplicationMiddwareBuilder, Middleware};

pub struct Application {
    pub mw: ApplicationMiddwareBuilder,
    pub sp: ApplicationServiceProvider,
}

impl Application {
    pub fn new() -> Self {
        let sp = Arc::new(ServiceProviderScope::default());
        let mw = ApplicationMiddwareBuilder::new();
        Self { mw, sp }
    }
    pub fn add_middleware<T: Middleware + 'static>(&mut self) {
        self.mw.add_middleware::<T>();
    }
    pub async fn run(mut self, ctx: &mut HttpContext) {
        let mw = self.mw.build(&self.sp);
        mw.call_async(ctx).await;
    }
}

#[derive(Default)]
pub struct HttpContext {
    pub http_request: String,
    pub serv_provider: ServiceProviderScope,
}
#[derive(Default)]
pub struct ServiceProviderScope {
    demo: Arc<AtomicU64>,
}
impl ServiceProviderScope {
    pub fn get_service_demo(&self) -> Arc<AtomicU64> {
        self.demo.clone()
    }
}
pub type ApplicationServiceProvider = Arc<ServiceProviderScope>;
