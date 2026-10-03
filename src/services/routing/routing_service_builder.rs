use crate::{
    controller::{ActionRoute, Routing},
    dependency_injection::DependencyInjectableController,
    services::routing::{
        ControllerCollect, RoutingService,
        routing_result::{ControllerInvoke, RoutingInfo},
    },
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
    #[di(default)]
    _router: HashMap<String, HashMap<http::Method, Arc<RoutingInfo>>>,
    #[di(default)]
    _registered_controllers: HashSet<TypeId>,
}

impl RoutingServiceBuilder {
    pub fn register_controller<T>(&mut self, root_route: &'static str, action_routes: Vec<ActionRoute>) -> ControllerCollect
    where
        T: DependencyInjectableController + Routing + Send + 'static,
    {
        let controller_invoker: Arc<ControllerInvoke> = Arc::new(|http_context: &mut crate::http_context::HttpContext, action_name: &'static str| {
            Box::pin(async move {
                let is_valid = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(true));
                let mut controller = T::inject(http_context, is_valid.clone());
                controller.routing(action_name).await;
                is_valid.store(false, std::sync::atomic::Ordering::Release);
            })
        });
        if !self._registered_controllers.insert(TypeId::of::<T>()) {
            panic!("Registered Controller {} twice", std::any::type_name::<T>());
        }
        for action in action_routes {
            let route = Self::join_route(root_route, action.route);
            self.add_controller_route::<T>(route, action.method, action.action_name, controller_invoker.clone());
        }
        ControllerCollect {
            type_id: TypeId::of::<T>(),
            type_name: std::any::type_name::<T>(),
            controller_name: std::any::type_name::<T>().rsplit("::").next().unwrap_or(std::any::type_name::<T>()),
        }
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

    pub fn add_controller_route<T>(&mut self, route: String, methods: Vec<&'static str>, action_name: &'static str, invoker: Arc<ControllerInvoke>)
    where
        T: DependencyInjectableController + Routing + Send + 'static,
    {
        let route_info = RoutingInfo {
            controller_name: std::any::type_name::<T>().rsplit("::").next().unwrap_or(std::any::type_name::<T>()),
            controller_type_name: std::any::type_name::<T>(),
            action_name,
            invoke_async: invoker,
        };

        match self._router.get_mut(&route) {
            Some(exist_route) => {
                for method in methods {
                    // Route already exists, update it
                    let http_method = http::Method::from_str(&method.to_uppercase()).unwrap();
                    if let Some(old) = exist_route.insert(http_method, Arc::new(route_info.clone())) {
                        panic!("Doublicate route {}, method {} at {}::{}", route, method, old.controller_type_name, old.action_name)
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
            match result._router.insert(&route.0, route.1) {
                Ok(_) => {}
                Err(e) => {
                    panic!("Failed to insert route {}: {:?}", route.0, e);
                }
            }
        }
        result
    }
}
