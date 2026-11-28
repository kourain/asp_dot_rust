use super::{body_of, raw_request, raw_request_with_body, start, status_of};
use asp_dot_rust::{api_controller, prelude::*};
use std::str::FromStr;

type Code = std::option::Option<String>;
type Text = String;

// A domain type can have the same name as the standard optional type.
struct Option<T>(T);

impl<T: FromStr> FromStr for Option<T> {
    type Err = T::Err;

    fn from_str(raw: &str) -> Result<Self, Self::Err> {
        raw.parse().map(Self)
    }
}

api_controller!(pub WrapperTypesController {});

#[controller_route]
#[controller_inject]
impl WrapperTypesController {
    fn new(http_context: HttpContextRef) -> Self {
        Self { http_context }
    }

    #[get("/wrapper-types/{id}")]
    fn values(&self, id: FromRoute<Option<u32>>, #[validate(length(2))] code: FromQuery<Code>) -> impl ActionResult {
        results::ok(format!("{} {:?}", id.0.0, *code))
    }

    #[post("/wrapper-raw-body")]
    fn raw_body(&self, body: Body<Text>) -> impl ActionResult {
        results::ok(body.into_inner())
    }

    #[post("/wrapper-raw-form")]
    fn raw_form(&self, form: Form<std::string::String>) -> impl ActionResult {
        results::ok(form.into_inner())
    }

    #[get("/bind-options")]
    #[get("/bind-options/{id}")]
    fn bind_options(
        &self,
        #[bind(name = "id", default = 7)] _id: FromRoute<u32>,
        #[asp_dot_rust::controller::bind(name = "limit", default = 0)]
        #[validate(min(1))] count: FromQuery<u32>,
        #[bind(name = "abcd", default = "aaaa", validate_default = false)]
        #[validate(length(2))] code: FromQuery<Code>,
    ) -> impl ActionResult {
        results::ok(format!("{} {} {}", *_id, *count, code.as_deref().unwrap()))
    }

    #[post("/bind-body")]
    fn bind_body(&self, #[bind(name = "value", default = "fallback")] field: FromBody<Code>) -> impl ActionResult {
        results::ok(format!("{:?}", *field))
    }

    #[post("/bind-form")]
    fn bind_form(&self, #[bind(name = "value", default = 7)] field: FromForm<u32>) -> impl ActionResult {
        results::ok(field.to_string())
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn bind_uses_defaults_only_for_absent_fields() {
    let port = 18725;
    start(port, |_| {}, |_| {}).await;
    for (path, status, expected) in [
        ("/bind-options", 422, "limit"),
        ("/bind-options?limit=1", 200, "7 1 aaaa"),
        ("/bind-options/9?limit=2&abcd=ok", 200, "9 2 ok"),
        ("/bind-options?limit=1&abcd=aaaa", 422, "abcd"),
        ("/bind-options?limit=1&abcd=", 422, "abcd"),
        ("/bind-options?limit=bad", 422, "limit"),
        ("/bind-options/bad?limit=1", 404, "id"),
    ] {
        let response = raw_request(port, "GET", path, "").await;
        assert_eq!(status_of(&response), status, "{response}");
        assert!(body_of(&response).contains(expected), "{response}");
    }
    for (path, media, body, status, expected) in [
        ("/bind-body", "application/json", "{}", 200, r#"Some("fallback")"#),
        ("/bind-body", "application/json", r#"{"value":null}"#, 200, "None"),
        ("/bind-body", "application/json", r#"{"value":17}"#, 422, "value"),
        ("/bind-form", "application/x-www-form-urlencoded", "", 200, "7"),
        ("/bind-form", "application/x-www-form-urlencoded", "value=12", 200, "12"),
        ("/bind-form", "application/x-www-form-urlencoded", "value=", 422, "value"),
    ] {
        let headers = format!("Content-Type: {media}\r\nContent-Length: {}\r\n", body.len());
        let response = raw_request_with_body(port, "POST", path, &headers, body).await;
        assert_eq!(status_of(&response), status, "{response}");
        assert!(body_of(&response).contains(expected), "{response}");
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn binding_and_validation_use_the_resolved_field_type() {
    let port = 18722;
    start(port, |_| {}, |_| {}).await;
    for (path, status, expected) in [
        ("/wrapper-types/7", 200, "7 None"),
        ("/wrapper-types/7?code=ok", 200, "7 Some(\"ok\")"),
        ("/wrapper-types/7?code=x", 422, "code"),
        ("/wrapper-types/bad", 404, "id"),
    ] {
        let response = raw_request(port, "GET", path, "").await;
        assert_eq!(status_of(&response), status, "{path}: {response}");
        assert!(body_of(&response).contains(expected), "{path}: {response}");
    }
    for (path, media, raw) in [
        ("/wrapper-raw-body", "text/plain", "plain text"),
        ("/wrapper-raw-form", "application/x-www-form-urlencoded", "code=a+b"),
    ] {
        let headers = format!("Content-Type: {media}\r\nContent-Length: {}\r\n", raw.len());
        let response = raw_request_with_body(port, "POST", path, &headers, raw).await;
        assert_eq!(status_of(&response), 200, "{response}");
        assert_eq!(body_of(&response), raw);
    }
}
