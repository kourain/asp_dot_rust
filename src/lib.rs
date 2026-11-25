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
        ActionResult, BindError, FromBody, FromForm, FromOptionQuery, FromQuery, FromRoute, controller_inject, controller_route, delete, get, head, options, patch, post, put, query, results,
        results::RouteResult,
        Validate, ValidationError,
    };
    #[allow(deprecated)]
    pub use crate::dependency_injection::Cfg;
    pub use crate::dependency_injection::{CfgOption, CfgReload, CfgRequire, Serv};
    pub use crate::http_context::{HttpContext, HttpContextRef};
    pub use crate::middleware::Middleware;
    pub use crate::services::app_queue::AppQueueService;
}
pub mod utils;
pub mod third_party {
    pub use async_trait;
    pub use chrono;
    pub use http;
    pub use http_body;
    pub use http_body_util;
    pub use inventory;
    pub use jsonwebtoken;
    pub use matchit;
    pub use quick_xml;
    pub use regex;
    pub use serde;
    pub use serde_json;
    pub use serde_urlencoded;
    pub use tokio;
    pub use toml;
    pub use urlencoding;
    pub use uuid;
}

pub use application::{Application, ApplicationBuilder};

pub type MutexAsync<T> = tokio::sync::Mutex<T>;
pub type ArcAny = std::sync::Arc<dyn std::any::Any + Send + Sync + 'static>;
