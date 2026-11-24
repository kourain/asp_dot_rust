use asp_dot_rust::{
    api_controller,
    controller::{
        ActionResult, FromRoute, get,
        results::{Json, Raw, Xml, not_found, ok},
    },
    http_context::HttpContextRef,
    macros::{controller_inject, controller_route},
};
use serde::Serialize;

use super::{Conn, start_server};

#[derive(Serialize)]
#[serde(transparent)]
struct Label<'a>(&'a str);

api_controller!(BodyController {
    text: String,
    label: Label<'static>,
    numbers: [u8; 64],
});

#[controller_route("/body")]
#[controller_inject]
impl BodyController {
    fn new(http_context: HttpContextRef) -> Self {
        Self {
            http_context,
            text: "A & B".into(),
            label: Label("A & B"),
            numbers: [42; 64],
        }
    }

    #[get("/{kind}")]
    fn body(&self, kind: FromRoute<String>) -> Box<dyn ActionResult + '_> {
        let text = self.text.as_str();
        match kind.as_str() {
            "label-ref" => ok(&self.label).into(),
            "label" => ok(Label(text)).into(),
            "raw" => ok(Raw(text.as_bytes())).into(),
            "plain" => ok(text).into(),
            "owned" => ok(text.to_owned()).into(),
            "json" => ok(Json(text)).into(),
            "xml" => ok(Xml(text)).into(),
            "words" => ok(vec![text]).into(),
            "vec" => ok(Vec::from(self.numbers)).into(),
            "slice" => ok(&self.numbers[..]).into(),
            _ => not_found("unknown body").into(),
        }
    }
}

#[tokio::test]
async fn a_borrowed_model_that_serializes_as_a_string_still_negotiates() {
    let mut conn = Conn::connect(start_server(|_| {}).await).await;
    let (head, body) = conn.get("/body/label-ref", &[("Accept", "application/json")]).await;
    assert_eq!(head.header("content-type"), Some("application/json"));
    assert_eq!(body, br#""A & B""#);
    let (head, body) = conn.get("/body/label", &[("Accept", "application/xml")]).await;
    assert_eq!(head.header("content-type"), Some("application/xml"));
    assert_eq!(body, b"<response><value>A &amp; B</value></response>");
}

#[tokio::test]
async fn one_status_helper_supports_negotiation_and_explicit_formats() {
    let mut conn = Conn::connect(start_server(|_| {}).await).await;
    let (head, body) = conn.get("/body/raw", &[("Accept", "application/json")]).await;
    assert_eq!(head.header("content-type"), Some("application/octet-stream"));
    assert_eq!(body, b"A & B");

    for (path, accept) in [("/body/plain", "application/json"), ("/body/owned", "application/xml")] {
        let (head, body) = conn.get(path, &[("Accept", accept)]).await;
        assert_eq!(head.header("content-type"), Some("text/plain"));
        assert_eq!(body, b"A & B");
    }
    for (path, accept, content_type, expected) in [
        ("/body/json", "application/xml", "application/json", br#""A & B""#.as_slice()),
        ("/body/xml", "application/json", "application/xml", b"<response><value>A &amp; B</value></response>".as_slice()),
    ] {
        let (head, body) = conn.get(path, &[("Accept", accept)]).await;
        assert_eq!(head.header("content-type"), Some(content_type));
        assert!(head.header("vary").is_none());
        assert_eq!(body, expected);
    }

    let (head, body) = conn.get("/body/words", &[("Accept", "application/json")]).await;
    assert_eq!(head.header("vary"), Some("Accept"));
    assert_eq!(body, br#"["A & B"]"#);
    let expected = serde_json::to_vec(&[42u8; 64][..]).unwrap();
    for path in ["/body/vec", "/body/slice"] {
        let (_, body) = conn.get(path, &[("Accept", "application/json")]).await;
        assert_eq!(body, expected);
    }
    let (head, _) = conn.get("/body/slice", &[("Accept", "text/html")]).await;
    assert_eq!(head.status, 406);
}
