mod application;
pub mod configuration;
pub mod controller;
pub mod dependcy_injection;
pub mod extensions;
pub mod hosted_service;
pub mod http_context;
pub mod http_listener;
pub mod logging;
pub mod middleware;
pub mod services;
pub mod threading;
pub mod macros {
    pub use asp_dot_rust_macros::*;
}
pub mod prelude {
    pub use crate::controller::{ActionResult, controller_inject_require, controller_route, delete, get, head, options, patch, post, put, route};
    pub use crate::dependcy_injection::{Cfg, CfgReload, CfgRequire, Serv};
    pub use crate::http_context::{HttpContext, HttpContextRef};
    pub use crate::middleware::Middleware;
    pub use crate::services::app_queue::AppQueueService;
    pub use crate::utils::ShareMutPtr;
}
pub mod utils;

pub use application::{Application, ApplicationBuilder};
pub use inventory;

pub type MutexAsync<T> = tokio::sync::Mutex<T>;
