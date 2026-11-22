use crate::{
    controller::{ActionRoute, Controller},
    dependency_injection::DependencyInjectableController,
    services::{
        openapi::{OpenApiService, RegisteredOperation},
        routing::{ControllerActivator, Route, RouteAttribute, RouteAttributeMap, RoutingService},
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
        T: DependencyInjectableController + Controller,
    {
        if !self._registered_controllers.insert(TypeId::of::<T>()) {
            panic!("Registered Controller {} twice", std::any::type_name::<T>());
        }
        for action in action_routes {
            let route = Self::join_route(root_route, action.route);
            self.add_controller_route::<T>(route, action.method, action.action_name, action.route_attribute_map);
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
    pub fn add_controller_route<T>(&mut self, route: String, methods: Vec<&'static str>, action_name: &'static str, route_attr_map: RouteAttributeMap)
    where
        T: DependencyInjectableController + Controller,
    {
        let controller_name: &'static str = std::any::type_name::<T>().rsplit("::").next().unwrap_or(std::any::type_name::<T>());
        let activator_async: ControllerActivator = |http_context| Box::pin(async move { Box::new(T::inject(http_context).await) as Box<dyn Controller> });
        let route_info = Arc::new(Route {
            controller_name,
            controller_type_name: std::any::type_name::<T>(),
            action_name,
            route_attr: RouteAttribute::new(route_attr_map),
            activator_async,
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
    pub fn build(self) -> (RoutingService, OpenApiService) {
        (RoutingService { _router: self._router }, OpenApiService { _operations: self._operations })
    }
}
