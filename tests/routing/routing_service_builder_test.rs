use asp_dot_rust::{
    controller::{ActionRoute, Routing},
    dependency_injection::DependencyInjectableController,
    http_context::HttpContext,
    services::routing::RoutingServiceBuilder,
};
use std::sync::{Arc, atomic::AtomicBool};

struct ControllerA;
impl DependencyInjectableController for ControllerA {
    fn inject(_http_context: &mut HttpContext, _is_valid: Arc<AtomicBool>) -> Self {
        ControllerA
    }
}
#[async_trait::async_trait]
impl Routing for ControllerA {
    async fn routing(&mut self, _method_name: &'static str) {}
}

struct ControllerB;
impl DependencyInjectableController for ControllerB {
    fn inject(_http_context: &mut HttpContext, _is_valid: Arc<AtomicBool>) -> Self {
        ControllerB
    }
}
#[async_trait::async_trait]
impl Routing for ControllerB {
    async fn routing(&mut self, _method_name: &'static str) {}
}

#[test]
fn join_route_normalizes_leading_and_trailing_slashes() {
    assert_eq!(RoutingServiceBuilder::join_route("", ""), "/");
    assert_eq!(RoutingServiceBuilder::join_route("home", ""), "/home");
    assert_eq!(RoutingServiceBuilder::join_route("", "index"), "/index");
    assert_eq!(RoutingServiceBuilder::join_route("/home/", "/index/"), "/home/index");
    assert_eq!(RoutingServiceBuilder::join_route("home", "index"), "/home/index");
}

#[test]
#[should_panic(expected = "Registered Controller")]
fn registering_the_same_controller_twice_panics() {
    let mut builder = RoutingServiceBuilder::default();
    builder.register_controller::<ControllerA>("home", vec![ActionRoute::new("index", vec!["get"], "")]);
    builder.register_controller::<ControllerA>("other", vec![ActionRoute::new("index", vec!["get"], "")]);
}

#[test]
#[should_panic(expected = "Doublicate route")]
fn two_controllers_on_the_same_route_and_method_panics() {
    let mut builder = RoutingServiceBuilder::default();
    builder.register_controller::<ControllerA>("home", vec![ActionRoute::new("index", vec!["get"], "")]);
    builder.register_controller::<ControllerB>("home", vec![ActionRoute::new("index", vec!["get"], "")]);
}

#[test]
fn same_route_different_methods_from_different_controllers_is_allowed() {
    let mut builder = RoutingServiceBuilder::default();
    builder.register_controller::<ControllerA>("home", vec![ActionRoute::new("index", vec!["get"], "")]);
    builder.register_controller::<ControllerB>("home", vec![ActionRoute::new("index", vec!["post"], "")]);
    let _ = builder.build();
    assert!(true);
}

#[test]
#[should_panic(expected = "Failed to insert route")]
fn build_panics_when_two_routes_conflict_on_param_names() {
    let mut builder = RoutingServiceBuilder::default();
    builder.register_controller::<ControllerA>("user", vec![ActionRoute::new("by_id", vec!["get"], "{id}")]);
    builder.register_controller::<ControllerB>("user", vec![ActionRoute::new("by_name", vec!["post"], "{name}")]);
    builder.build();
}

#[test]
fn build_succeeds_for_non_conflicting_routes() {
    let mut builder = RoutingServiceBuilder::default();
    builder.register_controller::<ControllerA>("home", vec![ActionRoute::new("index", vec!["get"], "")]);
    builder.register_controller::<ControllerB>("user", vec![ActionRoute::new("by_id", vec!["get"], "{id}")]);
    let _ = builder.build();
    assert!(true);
}
