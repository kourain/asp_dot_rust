use crate::services::routing::{Route, RoutingResult, routing_result::ResolvedRoute};
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
        let (query_params, invalid_query_params) = uri.query().map(parse_query).unwrap_or_default();
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

/// Percent-decode one query component. `+` is a space (form encoding), `%2B` stays a plus.
/// `None` when the decoded bytes are not valid UTF-8.
fn decode_query_component(raw: &str) -> Option<String> {
    if raw.contains('+') {
        urlencoding::decode(&raw.replace('+', " ")).ok().map(std::borrow::Cow::into_owned)
    } else {
        urlencoding::decode(raw).ok().map(std::borrow::Cow::into_owned)
    }
}

/// Split a query string into decoded `(valid, invalid)` parameter maps.
///
/// - Empty pairs (`a=1&&b=2`) and pairs with an empty key (`=v`) are skipped.
/// - A key without `=` (`?debug`) has an empty value, so it can be used as a flag.
/// - A repeated key keeps its last occurrence, whether that occurrence is valid or not.
/// - A value that is not valid UTF-8 after decoding goes to `invalid` with its raw text,
///   so the binder can report it instead of seeing a missing or altered value.
/// - A key that is not valid UTF-8 cannot equal an action parameter name and is dropped.
fn parse_query(query: &str) -> (HashMap<String, String>, HashMap<String, String>) {
    let mut valid = HashMap::new();
    let mut invalid = HashMap::new();
    for pair in query.split('&').filter(|pair| !pair.is_empty()) {
        let (raw_key, raw_value) = pair.split_once('=').unwrap_or((pair, ""));
        let Some(key) = decode_query_component(raw_key).filter(|key| !key.is_empty()) else {
            continue;
        };
        match decode_query_component(raw_value) {
            Some(value) => {
                invalid.remove(&key);
                valid.insert(key, value);
            }
            None => {
                valid.remove(&key);
                invalid.insert(key, raw_value.to_string());
            }
        }
    }
    (valid, invalid)
}
