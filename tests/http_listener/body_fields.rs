use super::{body_of, raw_request_with_body, start, status_of};

async fn post_with(port: u16, path: &str, media: &str, body: impl AsRef<[u8]>) -> String {
    let headers = format!("Content-Type: {media}\r\nContent-Length: {}\r\n", body.as_ref().len());
    raw_request_with_body(port, "POST", path, &headers, body).await
}

const FORM: &str = "application/x-www-form-urlencoded";
const UUID: &str = "0190d9d0-7b1e-7c3a-8f2e-123456789abc";

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn from_form_binds_single_fields_of_the_form() {
    let port = 18717;
    start(port, |_| {}, |_| {}).await;
    for (body, expected) in [
        (format!("name=Ada+Lovelace&age=36&admin&id={UUID}"), format!("Ada Lovelace 36 true {UUID}")),
        (format!("id={UUID}&admin=false&age=1&name=%E1%BA%BF%2B"), format!("ế+ 1 false {UUID}")),
        (format!("name=a&name=b&age=1&admin=&id={UUID}"), format!("b 1 true {UUID}")),
        (format!("name=%FF&%6Eame=ok&age=1&admin&id={UUID}"), format!("ok 1 true {UUID}")),
        (format!("name&age=1&admin&id={UUID}"), format!(" 1 true {UUID}")),
        // unrelated fields and empty pairs are ignored
        (format!("x=1&&name=n&age=2&y&admin=true&id={UUID}"), format!("n 2 true {UUID}")),
    ] {
        let response = post_with(port, "/fields-form", FORM, &body).await;
        assert_eq!(status_of(&response), 200, "{body}: {response}");
        assert_eq!(body_of(&response), expected, "{body}");
    }
    for (body, field) in [
        (format!("name=a&admin&id={UUID}"), "age"),
        (format!("name=a&age=x&admin&id={UUID}"), "age"),
        (format!("name=a&age=1&admin=maybe&id={UUID}"), "admin"),
        ("name=a&age=1&admin&id=nope".to_string(), "id"),
        (format!("name=%FF&age=1&admin&id={UUID}"), "name"),
        (format!("age=1&admin&id={UUID}"), "name"),
    ] {
        let response = post_with(port, "/fields-form", FORM, &body).await;
        assert_eq!(status_of(&response), 422, "{body}: {response}");
        assert!(body_of(&response).contains(field), "{body}: {response}");
    }
    let invalid_utf8 = post_with(port, "/fields-form", FORM, b"name=\xff").await;
    assert_eq!(status_of(&invalid_utf8), 422, "{invalid_utf8}");
    assert!(body_of(&invalid_utf8).contains("not valid UTF-8"), "{invalid_utf8}");
    let wrong_media = post_with(port, "/fields-form", "application/json", "{}").await;
    assert_eq!(status_of(&wrong_media), 422, "{wrong_media}");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn from_body_binds_single_fields_of_json_and_xml() {
    let port = 18718;
    start(port, |_| {}, |_| {}).await;
    let json = "application/json";
    let xml = "application/xml";
    for (media, body, expected) in [
        (json, r#"{"name":"Ada","age":36,"tags":["x","y"]}"#, r#"Ada 36 None ["x", "y"]"#),
        (json, r#"{"other":{"deep":[1]},"tags":[],"age":1,"name":"ế","nick":"A"}"#, r#"ế 1 Some("A") []"#),
        (json, r#"{"name":"a","age":1,"nick":null,"tags":["t"]}"#, r#"a 1 None ["t"]"#),
        (json, r#"{"name":"a","age":1,"age":2,"tags":[]}"#, "a 2 None []"),
        (xml, "<user><name>Ada</name><age>36</age><tags>x</tags></user>", r#"Ada 36 None ["x"]"#),
    ] {
        let response = post_with(port, "/fields-body", media, body).await;
        assert_eq!(status_of(&response), 200, "{body}: {response}");
        assert_eq!(body_of(&response), expected, "{body}");
    }
    for (media, body, field) in [
        (json, r#"{"age":1,"tags":[]}"#, "name"),
        (json, r#"{"name":"a","age":"x","tags":[]}"#, "field `age`"),
        (json, r#"{"name":"a","age":"x","age":2,"tags":[]}"#, "field `age`"),
        (json, r#"{"name":"a","age":1}"#, "tags"),
        (json, r#"["name"]"#, "object"),
        (json, "", "EOF"),
        (json, r#"{"name":"a","age":1,"tags":[]} x"#, "trailing characters"),
        (xml, "<user><age>1</age></user>", "name"),
        ("text/plain", r#"{"name":"a"}"#, "text/plain"),
    ] {
        let response = post_with(port, "/fields-body", media, body).await;
        assert_eq!(status_of(&response), 422, "{body}: {response}");
        assert!(body_of(&response).contains(field), "{body}: {response}");
    }
    let flag = post_with(port, "/body-flag", xml, "<root><flag>true</flag></root>").await;
    assert_eq!(status_of(&flag), 200, "{flag}");
    assert_eq!(body_of(&flag), "true");
    // The QUERY method reads a field too.
    let body = r#"{"term":"hello"}"#;
    let headers = format!("Content-Type: application/json\r\nContent-Length: {}\r\n", body.len());
    let response = raw_request_with_body(port, "QUERY", "/search-term", &headers, body).await;
    assert_eq!(status_of(&response), 200, "{response}");
    assert_eq!(body_of(&response), "hello", "{response}");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn body_binders_rewind_so_whole_and_field_binders_can_share_a_body() {
    let port = 18719;
    start(port, |_| {}, |_| {}).await;
    let response = post_with(port, "/whole-and-field-form", FORM, "term=hello&extra=world").await;
    assert_eq!(status_of(&response), 200, "{response}");
    assert_eq!(body_of(&response), "hello world");
    let json = r#"{"term":"t"}"#;
    let response = post_with(port, "/field-then-raw-body", "application/json", json).await;
    assert_eq!(status_of(&response), 200, "{response}");
    assert_eq!(body_of(&response), format!("t|{json}"));
    let response = post_with(port, "/field-then-action-reads", "application/json", json).await;
    assert_eq!(status_of(&response), 200, "{response}");
    assert_eq!(body_of(&response), format!("t|{json}"));
}
