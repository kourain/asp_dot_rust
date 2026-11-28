use asp_dot_rust::controller::{
    BindError, FromOptionQuery, FromParamValue, FromQuery, FromRoute,
};

#[test]
fn from_param_value_string_and_numbers() {
    assert_eq!(String::from_param_value("hello").unwrap(), "hello");
    assert_eq!(u32::from_param_value("42").unwrap(), 42);
    assert!(i8::from_param_value("x").is_err());
    assert_eq!(bool::from_param_value("true").unwrap(), true);
}

#[test]
fn bind_error_status_codes() {
    assert_eq!(
        BindError::MissingRouteParam("id".into()).status_code(),
        http::StatusCode::NOT_FOUND
    );
    assert_eq!(
        BindError::ParseRouteParam {
            name: "id".into(),
            value: "x".into()
        }
        .status_code(),
        http::StatusCode::NOT_FOUND
    );
    assert_eq!(
        BindError::MissingQueryParam("q".into()).status_code(),
        http::StatusCode::UNPROCESSABLE_ENTITY
    );
    assert_eq!(
        BindError::UnsupportedMediaType("text/plain".into()).status_code(),
        http::StatusCode::UNPROCESSABLE_ENTITY
    );
    assert_eq!(
        BindError::BodyRead("limit".into()).status_code(),
        http::StatusCode::BAD_REQUEST
    );
}

#[test]
fn form_and_json_serde_paths() {
    #[derive(Debug, serde::Deserialize, PartialEq)]
    struct User {
        name: String,
        age: u32,
    }
    let json = br#"{"name":"Ada","age":36}"#;
    let u: User = serde_json::from_slice(json).unwrap();
    assert_eq!(u.name, "Ada");
    let form = "name=Ada&age=36";
    let u2: User = serde_urlencoded::from_str(form).unwrap();
    assert_eq!(u2, u);
}

#[test]
fn xml_serde_path() {
    #[derive(Debug, serde::Deserialize, PartialEq, serde::Serialize)]
    #[serde(rename = "User")]
    struct User {
        name: String,
        age: u32,
    }
    let user = User {
        name: "Ada".into(),
        age: 36,
    };
    let xml = quick_xml::se::to_string(&user).unwrap();
    let parsed: User = quick_xml::de::from_str(&xml).unwrap();
    assert_eq!(parsed, user);
}
