use crate::services::routing::{Route, RoutingResult, query_string::parse_query_string, routing_result::ResolvedRoute};
use asp_dot_rust_macros::DependencyInjectableService;
use matchit::Router;
use std::{
    collections::{HashMap, HashSet},
    fmt::Debug,
    sync::Arc,
};

#[derive(Clone, Default, Debug, DependencyInjectableService)]
pub struct RoutingService {
    #[di(default)]
    pub(crate) _router: Router<HashMap<http::Method, Arc<Route>>>,
}

impl RoutingService {
    pub fn resolve(&self, uri: &http::Uri, method: &http::Method) -> ResolvedRoute {
        let query_params = uri.query().map(parse_query_string).unwrap_or_default();
        let mut resolved = ResolvedRoute { query_params, ..Default::default() };
        let Ok(matched) = self._router.at(Self::normalize_path(uri.path())) else {
            return resolved;
        };
        let route = match *method {
            http::Method::HEAD => matched.value.get(method).or_else(|| matched.value.get(&http::Method::GET)),
            _ => matched.value.get(method),
        };
        let Some(route) = route else {
            resolved.router_info = RoutingResult::MethodNotAllowed(matched.value.keys().cloned().collect());
            return resolved;
        };

        resolved.router_info = RoutingResult::Found(route.clone());
        // Decode after matching so an encoded slash stays inside its path segment.
        for (name, raw) in matched.params.iter() {
            let value = urlencoding::decode(raw).map(|value| value.into_owned()).map_err(|_| raw.to_owned());
            resolved.path_params.insert(name.to_owned(), value);
        }
        resolved
    }
    fn normalize_path(path: &str) -> &str {
        let trimmed = path.trim_end_matches('/');
        if trimmed.is_empty() && path.starts_with('/') { "/" } else { trimmed }
    }
    pub fn get_allowed_methods(&self, path: &str) -> HashSet<http::Method> {
        let matched = self._router.at(Self::normalize_path(path));
        if let Ok(matched) = matched { matched.value.keys().cloned().collect() } else { HashSet::new() }
    }
}
