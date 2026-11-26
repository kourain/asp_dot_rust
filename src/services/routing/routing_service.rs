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
        let matched = self._router.at(uri.path());
        let (query_params, invalid_query_params) = uri.query().map(parse_query_string).unwrap_or_default();
        match matched {
            Err(_) => ResolvedRoute {
                router_info: RoutingResult::NotFound,
                query_params,
                invalid_query_params,
                ..Default::default()
            },
            Ok(matched) => {
                if let Some(route_info) = matched.value.get(method) {
                    // The router matched the raw path, so an encoded `%2F` never splits a segment. Decode only the matched values.
                    let mut path_params = HashMap::new();
                    let mut invalid_path_params = HashMap::new();
                    for (name, raw) in matched.params.iter() {
                        match urlencoding::decode(raw) {
                            Ok(value) => path_params.insert(name.to_string(), value.into_owned()),
                            Err(_) => invalid_path_params.insert(name.to_string(), raw.to_string()),
                        };
                    }
                    ResolvedRoute {
                        router_info: RoutingResult::Found(route_info.clone()),
                        path_params,
                        query_params,
                        invalid_path_params,
                        invalid_query_params,
                    }
                } else {
                    ResolvedRoute {
                        router_info: RoutingResult::MethodNotAllowed(matched.value.keys().cloned().collect()),
                        query_params,
                        invalid_query_params,
                        ..Default::default()
                    }
                }
            }
        }
    }
    pub fn get_allowed_methods(&self, path: &str) -> HashSet<http::Method> {
        let matched = self._router.at(path);
        if let Ok(matched) = matched { matched.value.keys().cloned().collect() } else { HashSet::new() }
    }
}
