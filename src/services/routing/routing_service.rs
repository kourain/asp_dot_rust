use crate::services::{
    openapi::RegisteredOperation,
    routing::{Route, RoutingResult, routing_result::ResolvedRoute},
};
use asp_dot_rust_macros::DependencyInjectableService;
use matchit::Router;
use std::{
    collections::{HashMap, HashSet},
    fmt::Debug,
    sync::Arc,
};

#[derive(Clone, Default, Debug, DependencyInjectableService)]
pub struct RoutingService {
    /// key: "route", value: HashMap<http_method, resolved controller action info>
    #[di(default)]
    pub(crate) _router: Router<HashMap<http::Method, Arc<Route>>>,
    /// Flat list of registered operations for OpenAPI generation.
    #[di(default)]
    pub(crate) _operations: Vec<RegisteredOperation>,
}

impl RoutingService {
    pub fn resolve(&self, uri: &http::Uri, method: &http::Method) -> ResolvedRoute {
        let matched = self._router.at(uri.path());
        let query_params = if let Some(query_string) = uri.query() {
            HashMap::from_iter(query_string.split('&').filter_map(|pair| {
                let mut parts = pair.splitn(2, '=');
                let key = urlencoding::decode(parts.next().unwrap_or("")).ok()?.into();
                let value = urlencoding::decode(parts.next().unwrap_or("")).ok()?.into();
                Some((key, value))
            }))
        } else {
            HashMap::new()
        };
        match matched {
            Err(_) => ResolvedRoute {
                path_params: HashMap::new(),
                router_info: RoutingResult::NotFound,
                query_params,
            },
            Ok(matched) => {
                if let Some(route_info) = matched.value.get(method) {
                    let params = HashMap::from_iter(matched.params.iter().map(|(k, v)| (k.into(), v.into())));
                    ResolvedRoute {
                        path_params: params,
                        router_info: RoutingResult::Found(route_info.clone()),
                        query_params,
                    }
                } else {
                    ResolvedRoute {
                        path_params: HashMap::new(),
                        router_info: RoutingResult::MethodNotAllowed(matched.value.keys().cloned().collect()),
                        query_params,
                    }
                }
            }
        }
    }
    pub fn get_allowed_methods(&self, path: &str) -> HashSet<http::Method> {
        let matched = self._router.at(path);
        if let Ok(matched) = matched {
            matched.value.keys().cloned().collect()
        } else {
            HashSet::new()
        }
    }

    /// Returns all registered operations for OpenAPI document generation.
    pub fn registered_operations(&self) -> Vec<RegisteredOperation> {
        self._operations.clone()
    }
}
