use crate::services::routing::Route;
use std::{collections::HashMap, sync::Arc};

#[derive(Debug, Default)]
pub struct ResolvedRoute {
    /// key: http_method, value: ControllerInfo
    pub router_info: RoutingResult,
    pub path_params: HashMap<String, String>,
    pub query_params: HashMap<String, String>,
}

#[derive(Debug, Default)]
pub enum RoutingResult {
    Found(Arc<Route>),
    #[default]
    NotFound,
    MethodNotAllowed(Vec<http::Method>),
}
