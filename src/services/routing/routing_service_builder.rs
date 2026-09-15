use crate::{
    controller::{ActionRoute, Routing},
    dependency_injection::DependencyInjectableController,
    services::routing::{ControllerCollect, ControllerInfo, RoutingService},
};
use asp_dot_rust_macros::DependencyInjectableService;
use std::{
    any::TypeId,
    collections::{HashMap, HashSet},
    fmt::Debug,
    str::FromStr,
    sync::Arc,
};

#[derive(Clone, Default, Debug, DependencyInjectableService)]
pub struct RoutingServiceBuilder {
    /// key: "route", value: HashMap<http_method, resolved controller action info>
    _router: HashMap<String, HashMap<http::Method, Arc<ControllerInfo>>>,
    _registered_controllers: HashSet<ControllerCollect>,
}

impl RoutingServiceBuilder {
    pub fn register_controller<T: 'static>(&mut self, root_route: &'static str, action_routes: Vec<ActionRoute>) -> ControllerCollect
    where
        T: DependencyInjectableController + Routing + Send + 'static,
    {
        for action in action_routes {
            let route = Self::join_route(root_route, action.route);
            self.add_route::<T>(route, action.method, action.action_name);
        }
        let controller_collect = ControllerCollect {
            type_id: TypeId::of::<T>(),
            type_name: std::any::type_name::<T>(),
            controller_name: std::any::type_name::<T>().rsplit("::").next().unwrap_or(std::any::type_name::<T>()),
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
        T: DependencyInjectableController + Routing + Send + 'static,
    {
        let route_info = ControllerInfo {
            controller_name: std::any::type_name::<T>().rsplit("::").next().unwrap_or(std::any::type_name::<T>()),
            controller_type_name: std::any::type_name::<T>(),
            action_name: action_name,
            invoke_async: |http_context, action_name| {
                Box::pin(async move {
                    let is_valid = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(true));
                    let mut controller = T::inject(http_context, is_valid.clone());
                    controller.routing(action_name).await;
                    is_valid.store(false, std::sync::atomic::Ordering::Release);
                })
            },
        };

        match self._router.get_mut(&route) {
            Some(exist_route) => {
                for method in methods {
                    // Route already exists, update it
                    let http_method = http::Method::from_str(&method.to_uppercase()).unwrap();
                    match exist_route.insert(http_method, Arc::new(route_info.clone())) {
                        Some(old) => {
                            panic!("Doublicate route {}, method {} at {}::{}", route, &method, old.controller_type_name, old.action_name)
                        }
                        None => {}
                    }
                }
            }
            None => {
                // Route doesn't exist, insert it
                let mut method_map = HashMap::new();
                for method in methods {
                    method_map.insert(http::Method::from_str(&method.to_uppercase()).unwrap(), Arc::new(route_info.clone()));
                }
                self._router.insert(route, method_map);
            }
        }
    }
    pub fn build(self) -> RoutingService {
        let mut result = RoutingService::default();
        for route in self._router {
            _ = result._router.insert(route.0, Arc::new(route.1));
        }
        result
    }
}
