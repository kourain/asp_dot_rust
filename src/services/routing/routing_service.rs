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
        let (query_params, invalid_query_params) = uri.query().map(parse_query_string).unwrap_or_default();
        let mut resolved = ResolvedRoute { query_params, invalid_query_params, ..Default::default() };
        let Ok(matched) = self._router.at(uri.path()) else {
            return resolved;
        };
        let Some(route) = matched.value.get(method) else {
            resolved.router_info = RoutingResult::MethodNotAllowed(matched.value.keys().cloned().collect());
            return resolved;
        };

        resolved.router_info = RoutingResult::Found(route.clone());
        // Decode after matching so an encoded slash stays inside its path segment.
        for (name, raw) in matched.params.iter() {
            match urlencoding::decode(raw) {
                Ok(value) => resolved.path_params.insert(name.to_owned(), value.into_owned()),
                Err(_) => resolved.invalid_path_params.insert(name.to_owned(), raw.to_owned()),
            };
        }
        resolved
    }
    pub fn get_allowed_methods(&self, path: &str) -> HashSet<http::Method> {
        let matched = self._router.at(path);
        if let Ok(matched) = matched { matched.value.keys().cloned().collect() } else { HashSet::new() }
    }
}
