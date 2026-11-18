use asp_dot_rust::{
    controller::{ActionRoute, Routing},
    dependency_injection::DependencyInjectableController,
    http_context::HttpContextRef,
    services::routing::RoutingServiceBuilder,
};

struct ControllerA;
#[async_trait::async_trait]
impl DependencyInjectableController for ControllerA {
    async fn inject(_http_context: HttpContextRef) -> Self {
        ControllerA
    }
}
#[async_trait::async_trait]
impl Routing for ControllerA {
    async fn routing(&mut self, _method_name: &'static str) {}
}

struct ControllerB;
#[async_trait::async_trait]
impl DependencyInjectableController for ControllerB {
    async fn inject(_http_context: HttpContextRef) -> Self {
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
#[should_panic(expected = "Duplicate route")]
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
fn registration_panics_when_two_routes_conflict_on_param_names() {
    let mut builder = RoutingServiceBuilder::default();
    builder.register_controller::<ControllerA>("user", vec![ActionRoute::new("by_id", vec!["get"], "{id}")]);
    builder.register_controller::<ControllerB>("user", vec![ActionRoute::new("by_name", vec!["post"], "{name}")]);
}

#[test]
fn static_and_parameter_routes_keep_their_own_methods_in_either_order() {
    use asp_dot_rust::services::routing::RoutingResult;

    for reverse in [false, true] {
        let mut routes = vec![ActionRoute::new("by_id", vec!["get"], "{id}"), ActionRoute::new("create", vec!["post"], "new")];
        if reverse {
            routes.reverse();
        }
        let mut builder = RoutingServiceBuilder::default();
        builder.register_controller::<ControllerA>("user", routes);
        let service = builder.build();
        assert!(matches!(
            service.resolve(&"/user/new".parse().unwrap(), &http::Method::GET).router_info,
            RoutingResult::MethodNotAllowed(_)
        ));
        let by_id = service.resolve(&"/user/42".parse().unwrap(), &http::Method::GET);
        assert_eq!(by_id.path_params.get("id").map(String::as_str), Some("42"));
        assert!(matches!(by_id.router_info, RoutingResult::Found(_)));
        assert!(matches!(service.resolve(&"/user/new".parse().unwrap(), &http::Method::POST).router_info, RoutingResult::Found(_)));
    }
}

#[test]
fn identical_parameter_patterns_can_register_different_methods() {
    let mut builder = RoutingServiceBuilder::default();
    builder.register_controller::<ControllerA>("user", vec![ActionRoute::new("get", vec!["get"], "{id}")]);
    builder.register_controller::<ControllerB>("user", vec![ActionRoute::new("update", vec!["put"], "{id}")]);
    let service = builder.build();
    assert_eq!(service.get_allowed_methods("/user/42"), std::collections::HashSet::from([http::Method::GET, http::Method::PUT]));
}

#[test]
fn escaped_braces_can_register_different_methods() {
    let mut builder = RoutingServiceBuilder::default();
    builder.register_controller::<ControllerA>("user", vec![ActionRoute::new("get", vec!["get"], "{{id}}")]);
    builder.register_controller::<ControllerB>("user", vec![ActionRoute::new("create", vec!["post"], "{{id}}")]);
    assert_eq!(
        builder.build().get_allowed_methods("/user/{id}"),
        std::collections::HashSet::from([http::Method::GET, http::Method::POST])
    );
}

#[test]
#[should_panic(expected = "Duplicate route")]
fn duplicate_methods_in_one_action_are_rejected() {
    let mut builder = RoutingServiceBuilder::default();
    builder.register_controller::<ControllerA>("home", vec![ActionRoute::new("index", vec!["get", "GET"], "")]);
}

#[test]
#[should_panic(expected = "Failed to insert route")]
fn invalid_patterns_are_rejected_during_registration() {
    let mut builder = RoutingServiceBuilder::default();
    builder.register_controller::<ControllerA>("user", vec![ActionRoute::new("index", vec!["get"], "{id")]);
}

#[test]
fn build_succeeds_for_non_conflicting_routes() {
    let mut builder = RoutingServiceBuilder::default();
    builder.register_controller::<ControllerA>("home", vec![ActionRoute::new("index", vec!["get"], "")]);
    builder.register_controller::<ControllerB>("user", vec![ActionRoute::new("by_id", vec!["get"], "{id}")]);
    let _ = builder.build();
    assert!(true);
}

#[test]
fn with_attribute_replaces_the_value_of_the_same_key() {
    let route = ActionRoute::new("index", vec!["get"], "").with_attribute("level", 1u8).with_attribute("level", 2u8);
    assert_eq!(route.route_attribute_map.len(), 1);
    assert_eq!(route.route_attribute_map["level"].downcast_ref::<u8>(), Some(&2));
}

#[test]
fn resolved_route_has_its_attributes_stored_as_their_own_type() {
    use asp_dot_rust::services::routing::RoutingResult;
    let mut builder = RoutingServiceBuilder::default();
    builder.register_controller::<ControllerA>(
        "home",
        vec![
            ActionRoute::new("index", vec!["get"], "")
                .with_attribute("level", 5u8)
                .with_attribute("zone", "main")
                .with_attribute("ratio", 0.5f32),
        ],
    );
    let service = builder.build();
    let uri: http::Uri = "/home".parse().unwrap();
    let RoutingResult::Found(route) = service.resolve(&uri, &http::Method::GET).router_info else {
        panic!("the route must be found");
    };
    assert_eq!(route.attributes().get::<u8>("level"), Some(5));
    // another type is not a match, a number is not converted
    assert_eq!(route.attributes().get::<i32>("level"), None);
    assert_eq!(route.attributes().get::<&'static str>("zone"), Some("main"));
    assert_eq!(route.attributes().get::<f32>("ratio"), Some(0.5));
    assert_eq!(route.attributes().get::<u8>("missing"), None);
    assert!(route.attributes().contains("zone") && !route.attributes().contains("missing"));
}

#[test]
fn max_body_size_as_usize_is_accepted() {
    let mut builder = RoutingServiceBuilder::default();
    builder.register_controller::<ControllerA>("home", vec![ActionRoute::new("index", vec!["get"], "").with_attribute("max_body_size", 1024usize)]);
    let _ = builder.build();
}

#[test]
#[should_panic(expected = "Attribute `max_body_size` of")]
fn max_body_size_that_is_not_a_usize_panics_at_startup() {
    let mut builder = RoutingServiceBuilder::default();
    // 1024 is an i32 here: it would be ignored when a request comes in, so it is refused when the route is registered
    builder.register_controller::<ControllerA>("home", vec![ActionRoute::new("index", vec!["get"], "").with_attribute("max_body_size", 1024)]);
}
