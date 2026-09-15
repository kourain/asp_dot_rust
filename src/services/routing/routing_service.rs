use crate::{dependency_injection::DependencyInjectableService, http_context::HttpContext};
use matchit::Router;
use std::{
    collections::{HashMap, HashSet},
    fmt::Debug,
    future::Future,
    pin::Pin,
    sync::Arc,
};

type ControllerInvoke = for<'a> fn(&'a mut HttpContext, &'static str) -> Pin<Box<dyn Future<Output = ()> + Send + 'a>>;
#[derive(Clone, Debug)]
pub struct ControllerInfo {
    pub controller_name: &'static str,
    pub controller_type_name: &'static str,
    pub action_name: &'static str,
    pub(crate) invoke_async: ControllerInvoke,
}
#[derive(Debug)]
pub struct ResolvedRoute {
    /// key: http_method, value: ControllerInfo
    pub router_info: Arc<HashMap<http::Method, Arc<ControllerInfo>>>,
    pub path: String,
    pub query_string: String,
    pub path_params: HashMap<String, String>,
    pub query_params: HashMap<String, String>,
}
#[derive(Clone, Default, Debug)]
pub struct RoutingService {
    /// key: "route", value: HashMap<http_method, resolved controller action info>
    pub(crate) _router: Router<Arc<HashMap<http::Method, Arc<ControllerInfo>>>>,
}

impl DependencyInjectableService for RoutingService {
    fn inject(_service_scope: &crate::services::service_provider::service_provider_scope::ServiceProviderScope) -> Self
    where
        Self: Sized,
    {
        RoutingService::default()
    }
}
impl RoutingService {
    pub fn resolve(&self, full_path: &str) -> Option<ResolvedRoute> {
        let query_pos = full_path.find('?').unwrap_or(full_path.len());
        let path = &full_path[..query_pos];
        let query_string = if query_pos < full_path.len() { &full_path[query_pos + 1..] } else { "" };
        let matched = self._router.at(path);
        match matched {
            Err(_) => None,
            Ok(matched) => {
                let params = HashMap::from_iter(matched.params.iter().map(|(k, v)| (k.into(), v.into())));
                return Some(ResolvedRoute {
                    path: path.into(),
                    path_params: params,
                    router_info: matched.value.clone(),
                    query_string: query_string.into(),
                    query_params: HashMap::from_iter(query_string.split('&').filter_map(|pair| {
                        let mut parts = pair.split('=');
                        let key = parts.next()?.into();
                        let value = urlencoding::decode(parts.next()?).ok()?.into();
                        Some((key, value))
                    })),
                });
            }
        }
    }
    pub fn get_allowed_methods(&self, path: &str) -> HashSet<http::Method> {
        let matched = self._router.at(path);
        if let Ok(matched) = matched { matched.value.keys().cloned().collect() } else { HashSet::new() }
    }
}
