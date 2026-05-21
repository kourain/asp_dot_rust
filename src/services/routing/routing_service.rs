use crate::{
    controller::{ActionRoute, Routing, WithHttpContext},
    http_context::HttpContext,
    services::routing::ControllerCollect,
};
use matchit::Router;
use std::{
    any::TypeId,
    collections::{HashMap, HashSet},
    fmt::Debug,
    future::Future,
    pin::Pin,
    str::FromStr,
};

type ControllerInvoke = for<'a> fn(&'a mut HttpContext, String) -> Pin<Box<dyn Future<Output = ()> + Send + 'a>>;
#[derive(Clone, Debug)]
pub struct ControllerInfo {
    pub controller_type: TypeId,
    pub controller_name: &'static str,
    pub controller_type_name: &'static str,
    pub action_name: &'static str,
    pub(crate) invoke: ControllerInvoke,
}
#[derive(Debug)]
pub struct ResolvedRoute {
    pub router_info: HashMap<http::Method, ControllerInfo>, // key: http_method, value: ControllerInfo
    pub path: String,
    pub query_string: String,
    pub path_params: HashMap<String, String>,
    pub query_params: HashMap<String, String>,
}
#[derive(Clone)]
pub struct RoutingService {
    // _route: Router<TypeId>,
    _router: Router<HashMap<http::Method, ControllerInfo>>, // key: "route", value: HashMap<http_method, resolved controller action info>
    _registered_controllers: HashSet<ControllerCollect>,
}
impl Debug for RoutingService {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RoutingService")
            .field("registered_routes", &self._router)
            .field("registered_controllers", &self._registered_controllers)
            .finish()
    }
}
impl Default for RoutingService {
    fn default() -> Self {
        Self {
            _router: Router::new(),
            _registered_controllers: HashSet::new(),
        }
    }
}
impl RoutingService {
    pub fn register_controller<T: 'static>(&mut self, root_route: &str, action_routes: Vec<ActionRoute>) -> ControllerCollect
    where
        T: WithHttpContext + Routing + Send + 'static,
    {
        for action in action_routes {
            let route = Self::join_route(root_route, action.route);
            self.add_route::<T>(route, action.method, action.action_name);
        }
        let controller_collect = ControllerCollect {
            type_id: TypeId::of::<T>(),
            type_name: std::any::type_name::<T>(),
            controller_name: T::str_name(),
        };
        self._registered_controllers.insert(controller_collect.clone());
        controller_collect
    }

    fn join_route(root_route: &str, action_route: &str) -> String {
        let root = root_route.trim_matches('/');
        let action = action_route.trim_matches('/');

        if root.is_empty() && action.is_empty() {
            return "/".into();
        }

        if root.is_empty() {
            return format!("/{action}");
        }

        if action.is_empty() {
            return format!("/{root}");
        }

        format!("/{root}/{action}")
    }

    pub fn add_route<T: 'static>(&mut self, route: String, methods: Vec<&'static str>, action_name: &'static str)
    where
        T: WithHttpContext + Routing + Send + 'static,
    {
        let lower_route = route.to_lowercase();
        let controller_type_id = TypeId::of::<T>();
        let route_info = ControllerInfo {
            controller_type: controller_type_id,
            controller_name: T::str_name(),
            controller_type_name: std::any::type_name::<T>(),
            action_name: action_name,
            invoke: |http_context, action_name| {
                Box::pin(async move {
                    let mut controller = T::new_with_http_context(http_context);
                    controller.routing(action_name).await;
                })
            },
        };

        match self._router.at(&route) {
            Ok(_) => {
                for method in methods {
                    // Route already exists, update it
                    self._router.at_mut(&lower_route).unwrap().value.insert(http::Method::from_str(method).unwrap(), route_info.clone());
                }
            }
            Err(_) => {
                // Route doesn't exist, insert it
                let mut method_map = HashMap::new();
                for method in methods {
                    method_map.insert(http::Method::from_str(method).unwrap(), route_info.clone());
                }
                self._router.insert(&lower_route, method_map).unwrap_or_else(|e| {
                    panic!("Controller {} failed to insert route: {}, error: {:?}", std::any::type_name::<T>(), lower_route, e);
                });
            }
        }
    }
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
        let lower_path = path.to_lowercase();
        let matched = self._router.at(&lower_path);
        if let Ok(matched) = matched { matched.value.keys().cloned().collect() } else { HashSet::new() }
    }
}
