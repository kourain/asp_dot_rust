use crate::{
    controller::{ActionRoute, Routing},
    dependency_injection::DependencyInjectableController,
    services::{
        openapi::RegisteredOperation,
        routing::{ControllerInvoke, MAX_BODY_SIZE_ATTRIBUTE, Route, RouteAttribute, RouteAttributeMap, RoutingService},
    },
};
use matchit::Router;
use std::{
    any::TypeId,
    collections::{HashMap, HashSet},
    str::FromStr,
    sync::Arc,
};

#[derive(Default)]
pub struct RoutingServiceBuilder {
    _router: Router<HashMap<http::Method, Arc<Route>>>,
    pub(crate) _registered_controllers: HashSet<TypeId>,
    /// Flat list of operations for OpenAPI generation.
    _operations: Vec<RegisteredOperation>,
}

impl RoutingServiceBuilder {
    #[track_caller]
    pub fn register_controller<T>(&mut self, root_route: &'static str, action_routes: Vec<ActionRoute>)
    where
        T: DependencyInjectableController + Routing + Send + 'static,
    {
        let controller_invoker: ControllerInvoke = |http_context, action_name| {
            Box::pin(async move {
                let mut controller = T::inject(http_context).await;
                controller.routing(action_name).await;
            })
        };
        if !self._registered_controllers.insert(TypeId::of::<T>()) {
            panic!("Registered Controller {} twice", std::any::type_name::<T>());
        }
        for action in action_routes {
            let route = Self::join_route(root_route, action.route);
            self.add_controller_route::<T>(route, action.method, action.action_name, action.route_attribute_map, controller_invoker.clone());
        }
    }

    pub fn join_route(root_route: &str, action_route: &str) -> String {
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

    #[track_caller]
    pub fn add_controller_route<T>(&mut self, route: String, methods: Vec<&'static str>, action_name: &'static str, route_attr_map: RouteAttributeMap, invoker: ControllerInvoke)
    where
        T: DependencyInjectableController + Routing + Send + 'static,
    {
        // a limit that is not a usize would be ignored when a request comes in, refuse it at startup
        if route_attr_map.get(MAX_BODY_SIZE_ATTRIBUTE).is_some_and(|value| !value.is::<usize>()) {
            panic!(
                "Attribute `{}` of {}::{} must be a usize (the limit in bytes)",
                MAX_BODY_SIZE_ATTRIBUTE,
                std::any::type_name::<T>(),
                action_name
            );
        }
        let controller_name = std::any::type_name::<T>().rsplit("::").next().unwrap_or(std::any::type_name::<T>());
        let route_info = Arc::new(Route {
            controller_name,
            controller_type_name: std::any::type_name::<T>(),
            action_name,
            route_attr: RouteAttribute::new(route_attr_map),
            invoke_async: invoker,
        });

        // Remove by the exact pattern: matching a URL here could confuse static
        // routes with parameters or silently merge different parameter names.
        let mut method_map = self._router.remove(&route).unwrap_or_default();
        for method in methods {
            let http_method = match http::Method::from_str(&method.to_uppercase()) {
                Ok(method) => method,
                Err(error) => panic!("Invalid HTTP method {method} for route {route} at {}::{action_name}: {error}", std::any::type_name::<T>()),
            };
            if let Some(old) = method_map.get(&http_method) {
                panic!(
                    "Duplicate route \"{}\", method {} at \"{}::{}\" and \"{}::{}\"",
                    route, method, old.controller_type_name, old.action_name, route_info.controller_type_name, route_info.action_name
                );
            }
            method_map.insert(http_method.clone(), route_info.clone());
            self._operations.push(RegisteredOperation {
                path: route.clone(),
                method: http_method,
                controller_name,
                action_name,
            });
        }
        if let Err(error) = self._router.insert(route.clone(), method_map) {
            panic!("Failed to insert route {route} at {}::{action_name}: {error}", std::any::type_name::<T>());
        }
    }
    pub fn build(self) -> RoutingService {
        RoutingService {
            _router: self._router,
            _operations: self._operations,
        }
    }
}
