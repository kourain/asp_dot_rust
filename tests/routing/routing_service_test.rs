use asp_dot_rust::controller::{ActionRoute, Routing};
use asp_dot_rust::dependency_injection::DependencyInjectableController;
use asp_dot_rust::http_context::HttpContext;
use asp_dot_rust::services::routing::{RoutingResult, RoutingService, RoutingServiceBuilder};
use std::collections::HashSet;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;

struct TestController;
impl DependencyInjectableController for TestController {
    fn inject(_http_context: &mut HttpContext, _is_valid: Arc<AtomicBool>) -> Self {
        TestController
    }
}
#[async_trait::async_trait]
impl Routing for TestController {
    async fn routing(&mut self, _method_name: &'static str) {}
}

/// `/home` accepts GET and POST, `/home/{id}` accepts GET only.
fn build_test_service() -> RoutingService {
    let mut builder = RoutingServiceBuilder::default();
    builder.register_controller::<TestController>(
        "home",
        vec![
            ActionRoute::new("index", vec!["get"], ""),
            ActionRoute::new("create", vec!["post"], ""),
            ActionRoute::new("by_id", vec!["get"], "{id}"),
        ],
    );
    builder.build()
}

#[test]
fn resolve_returns_found_for_a_registered_route_and_method() {
    let service = build_test_service();
    let uri = http::Uri::from_static("/home");
    let resolved = service.resolve(&uri, &http::Method::GET);
    match resolved.router_info {
        RoutingResult::Found(info) => {
            assert_eq!(info.action_name, "index");
        }
        other => panic!("expected Found, got {:?}", other),
    }
}

#[test]
fn resolve_returns_not_found_for_an_unregistered_path() {
    let service = build_test_service();
    let uri = http::Uri::from_static("/does-not-exist");
    let resolved = service.resolve(&uri, &http::Method::GET);
    assert!(matches!(resolved.router_info, RoutingResult::NotFound));
}

#[test]
fn resolve_returns_method_not_allowed_and_lists_registered_methods() {
    let service = build_test_service();
    let uri = http::Uri::from_static("/home");
    let resolved = service.resolve(&uri, &http::Method::DELETE);
    match resolved.router_info {
        RoutingResult::MethodNotAllowed(methods) => {
            let methods: HashSet<_> = methods.into_iter().collect();
            assert_eq!(methods, HashSet::from([http::Method::GET, http::Method::POST]));
        }
        other => panic!("expected MethodNotAllowed, got {:?}", other),
    }
}

#[test]
fn resolve_extracts_path_params() {
    let service = build_test_service();
    let uri = http::Uri::from_static("/home/42");
    let resolved = service.resolve(&uri, &http::Method::GET);
    assert_eq!(resolved.path_params.get("id"), Some(&"42".to_string()));
    assert!(matches!(resolved.router_info, RoutingResult::Found(_)));
}

#[test]
fn query_params_are_url_decoded_for_both_key_and_value() {
    let service = build_test_service();
    let uri = http::Uri::from_static("/home?na%20me=John%20Doe");
    let resolved = service.resolve(&uri, &http::Method::GET);
    assert_eq!(resolved.query_params.get("na me"), Some(&"John Doe".to_string()));
}

#[test]
fn query_params_without_equals_sign_become_empty_string_values() {
    let service = build_test_service();
    let uri = http::Uri::from_static("/home?flag");
    let resolved = service.resolve(&uri, &http::Method::GET);
    assert_eq!(resolved.query_params.get("flag"), Some(&"".to_string()));
}

#[test]
fn get_allowed_methods_returns_registered_methods_for_a_path() {
    let service = build_test_service();
    assert_eq!(service.get_allowed_methods("/home"), HashSet::from([http::Method::GET, http::Method::POST]));
    assert_eq!(service.get_allowed_methods("/home/42"), HashSet::from([http::Method::GET]));
    assert_eq!(service.get_allowed_methods("/does-not-exist"), HashSet::new());
}
