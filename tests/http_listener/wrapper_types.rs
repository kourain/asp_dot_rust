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
