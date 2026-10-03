use crate::http_context::HttpContext;
use std::{collections::HashMap, pin::Pin, sync::Arc};

pub(crate) type ControllerInvoke = for<'a> fn(&'a mut HttpContext, &'static str) -> Pin<Box<dyn Future<Output = ()> + Send + 'a>>;

#[derive(Debug)]
pub struct RoutingInfo {
    pub controller_name: &'static str,
    pub controller_type_name: &'static str,
    pub action_name: &'static str,
    pub(crate) invoke_async: Arc<ControllerInvoke>,
}

#[derive(Debug, Default)]
pub struct ResolvedRoute {
    /// key: http_method, value: ControllerInfo
    pub router_info: RoutingResult,
    pub path_params: HashMap<String, String>,
    pub query_params: HashMap<String, String>,
}

#[derive(Debug, Default)]
pub enum RoutingResult {
    Found(Arc<RoutingInfo>),
    #[default]
    NotFound,
    MethodNotAllowed(Vec<http::Method>),
}
