use crate::services::routing::Route;
use std::{collections::HashMap, sync::Arc};

#[derive(Debug, Default)]
pub struct ResolvedRoute {
    pub router_info: RoutingResult,
    /// Percent-decoded path parameters. A parameter whose value is not valid UTF-8 after decoding is not here, see `invalid_path_params`.
    pub path_params: HashMap<String, String>,
    /// Decoded query parameters (`+` is a space, `%XX` is decoded). A key without `=` has an empty value, a repeated key keeps its last value.
    pub query_params: HashMap<String, String>,
    /// Path parameters whose percent-encoded value decodes to invalid UTF-8: parameter name -> the raw (still encoded) value.
    pub invalid_path_params: HashMap<String, String>,
    /// Query parameters whose percent-encoded value decodes to invalid UTF-8: decoded key -> the raw (still encoded) value.
    /// A pair whose key is not valid UTF-8 cannot match any parameter name and is dropped.
    pub invalid_query_params: HashMap<String, String>,
}

#[derive(Debug, Default)]
pub enum RoutingResult {
    Found(Arc<Route>),
    #[default]
    NotFound,
    MethodNotAllowed(Vec<http::Method>),
}
