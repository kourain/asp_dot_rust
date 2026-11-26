use asp_dot_rust::controller::{ActionResult, ActionRoute, Controller, RoutingError};
use asp_dot_rust::dependency_injection::DependencyInjectableController;
use asp_dot_rust::http_context::HttpContextRef;
use asp_dot_rust::services::routing::{RoutingResult, RoutingService, RoutingServiceBuilder};
use std::collections::HashSet;

struct TestController;
#[async_trait::async_trait]
impl DependencyInjectableController for TestController {
    async fn inject(_http_context: HttpContextRef) -> Self {
        TestController
    }
}
#[async_trait::async_trait]
impl Controller for TestController {
    async fn routing(&mut self, _method_name: &'static str) -> Result<Box<dyn ActionResult + '_>, RoutingError> {
        Err(RoutingError::ActionNotFound)
    }
}

/// `/home` accepts GET and POST, `/home/{id}` and `/home/files/{*path}` accept GET only.
fn build_test_service() -> RoutingService {
    let mut builder = RoutingServiceBuilder::default();
    builder.register_controller::<TestController>(
        "home",
        vec![
            ActionRoute::new("index", vec!["get"], ""),
            ActionRoute::new("create", vec!["post"], ""),
            ActionRoute::new("by_id", vec!["get"], "{id}"),
            ActionRoute::new("file", vec!["get"], "files/{*path}"),
        ],
    );
    let (service, _) = builder.build();
    service
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

fn resolve_get(path_and_query: &str) -> asp_dot_rust::services::routing::ResolvedRoute {
    build_test_service().resolve(&path_and_query.parse::<http::Uri>().unwrap(), &http::Method::GET)
}

#[test]
fn path_params_are_percent_decoded_after_matching() {
    assert_eq!(resolve_get("/home/a%20b").path_params.get("id"), Some(&"a b".to_string()));
    assert_eq!(resolve_get("/home/%E1%BA%BF").path_params.get("id"), Some(&"ế".to_string()));
    // `+` is a plain character in a path.
    assert_eq!(resolve_get("/home/a+b").path_params.get("id"), Some(&"a+b".to_string()));
    // An encoded slash stays inside its segment, the route still matched on the raw path.
    let resolved = resolve_get("/home/a%2Fb");
    assert!(matches!(resolved.router_info, RoutingResult::Found(_)));
    assert_eq!(resolved.path_params.get("id"), Some(&"a/b".to_string()));
    assert_eq!(resolve_get("/home/files/x%20y/z").path_params.get("path"), Some(&"x y/z".to_string()));
}

#[test]
fn path_params_with_invalid_utf8_are_reported_not_altered() {
    let resolved = resolve_get("/home/%FF");
    assert!(matches!(resolved.router_info, RoutingResult::Found(_)));
    assert!(resolved.path_params.get("id").is_none());
    assert_eq!(resolved.invalid_path_params.get("id"), Some(&"%FF".to_string()));
}

#[test]
fn query_plus_is_a_space_and_encoded_plus_is_a_plus() {
    let resolved = resolve_get("/home?a+b=c+d&sum=1%2B1");
    assert_eq!(resolved.query_params.get("a b"), Some(&"c d".to_string()));
    assert_eq!(resolved.query_params.get("sum"), Some(&"1+1".to_string()));
}

#[test]
fn query_value_keeps_everything_after_the_first_equals_sign() {
    assert_eq!(resolve_get("/home?a=b=c").query_params.get("a"), Some(&"b=c".to_string()));
}

#[test]
fn query_flag_is_present_with_an_empty_value_and_absent_keys_are_missing() {
    let resolved = resolve_get("/home?debug&other=");
    assert_eq!(resolved.query_params.get("debug"), Some(&"".to_string()));
    assert_eq!(resolved.query_params.get("other"), Some(&"".to_string()));
    assert!(resolved.query_params.get("missing").is_none());
}

#[test]
fn query_repeated_key_keeps_the_last_occurrence() {
    let resolved = resolve_get("/home?id=1&id=2&id=3");
    assert_eq!(resolved.query_params.get("id"), Some(&"3".to_string()));
    assert_eq!(resolved.query_params.len(), 1);
}

#[test]
fn query_empty_pairs_and_empty_keys_are_skipped() {
    let resolved = resolve_get("/home?&&x=1&&=v&");
    assert_eq!(resolved.query_params.len(), 1);
    assert_eq!(resolved.query_params.get("x"), Some(&"1".to_string()));
}

#[test]
fn query_value_with_invalid_utf8_is_reported_with_its_raw_text() {
    let resolved = resolve_get("/home?term=%FF&ok=1");
    assert!(resolved.query_params.get("term").is_none());
    assert_eq!(resolved.invalid_query_params.get("term"), Some(&"%FF".to_string()));
    assert_eq!(resolved.query_params.get("ok"), Some(&"1".to_string()));
}

#[test]
fn query_last_occurrence_decides_between_a_valid_and_an_invalid_value() {
    let invalid_last = resolve_get("/home?term=ok&term=%FF");
    assert!(invalid_last.query_params.get("term").is_none());
    assert!(invalid_last.invalid_query_params.contains_key("term"));
    let valid_last = resolve_get("/home?term=%FF&term=ok");
    assert_eq!(valid_last.query_params.get("term"), Some(&"ok".to_string()));
    assert!(valid_last.invalid_query_params.is_empty());
}

#[test]
fn query_key_with_invalid_utf8_is_dropped() {
    let resolved = resolve_get("/home?%FF=v&ok=1");
    assert_eq!(resolved.query_params.len(), 1);
    assert!(resolved.invalid_query_params.is_empty());
}

#[test]
fn query_is_parsed_even_when_the_route_is_not_found_or_the_method_is_not_allowed() {
    let service = build_test_service();
    let not_found = service.resolve(&"/nope?a=1".parse().unwrap(), &http::Method::GET);
    assert_eq!(not_found.query_params.get("a"), Some(&"1".to_string()));
    let not_allowed = service.resolve(&"/home?a=1".parse().unwrap(), &http::Method::DELETE);
    assert_eq!(not_allowed.query_params.get("a"), Some(&"1".to_string()));
    assert!(not_allowed.path_params.is_empty());
}
