mod application;
pub mod configuration;
pub mod controller;
pub mod dependency_injection;
pub mod extensions;
pub mod hosted_service;
pub mod http_context;
pub mod http_listener;
pub mod logging;
pub mod middleware;
pub mod services;
pub mod macros {
    pub use asp_dot_rust_macros::*;
}
pub mod prelude {
    pub use crate::controller::{
        ActionResult, BindError, FromBody, FromForm, FromOptionQuery, FromQuery, FromRoute, controller_inject, controller_route, delete, get, head, options, patch, post, put, route,
    };
    #[allow(deprecated)]
    pub use crate::dependency_injection::Cfg;
    pub use crate::dependency_injection::{CfgOption, CfgReload, CfgRequire, Serv};
    pub use crate::http_context::{HttpContext, HttpContextRef};
    pub use crate::middleware::Middleware;
    pub use crate::services::app_queue::AppQueueService;
    pub use crate::utils::ShareMutPtr;
}
pub mod utils;
pub mod third_party {
    pub use async_trait;
    pub use inventory;
    pub use regex;
    pub use serde;
    pub use serde_json;
    pub use tokio;
    pub use toml;
    pub use urlencoding;
    pub use uuid;
}

pub use application::{Application, ApplicationBuilder};

pub type MutexAsync<T> = tokio::sync::Mutex<T>;
