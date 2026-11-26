use crate::services::routing::Route;
use std::{collections::HashMap, sync::Arc};

#[derive(Debug, Default)]
pub struct ResolvedRoute {
    pub router_info: RoutingResult,
    /// Path parameters decoded after matching.
    pub path_params: HashMap<String, Result<String, String>>,
    /// Form-decoded query parameters. Repeated keys keep their last value.
    pub query_params: HashMap<String, Result<String, String>>,
}

#[derive(Debug, Default)]
pub enum RoutingResult {
    Found(Arc<Route>),
    #[default]
    NotFound,
    MethodNotAllowed(Vec<http::Method>),
}
