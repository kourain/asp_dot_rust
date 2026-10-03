use std::{collections::HashMap, pin::Pin, sync::Arc};

use crate::http_context::HttpContext;

pub(crate) type ControllerInvoke = for<'a> fn(&'a mut HttpContext, &'static str) -> Pin<Box<dyn Future<Output = ()> + Send + 'a>>;
#[derive(Clone, Debug)]
pub struct RoutingInfo {
    pub controller_name: &'static str,
    pub controller_type_name: &'static str,
    pub action_name: &'static str,
    pub(crate) invoke_async: Arc<ControllerInvoke>,
}

#[derive(Debug, Default)]
pub struct ResolvedRoute {
    /// key: http_method, value: ControllerInfo
    pub router_info: Option<Arc<RoutingInfo>>,
    pub path_params: HashMap<String, String>,
    pub query_params: HashMap<String, String>,
}
