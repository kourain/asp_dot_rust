use asp_dot_rust::controller::{BindError, FromParamValue, FromQuery};

#[test]
fn query_rule_methods_accept_exact_and_inclusive_bounds() {
    assert!(FromQuery::length("ế", "term", 1).is_ok());
    assert_eq!(FromQuery::length("ế", "term", 2).unwrap_err().to_string(), "term: length must be exactly 2");
    assert!(FromQuery::length("ếx", "term", (1, 2)).is_ok());
    assert!(FromQuery::range(&-10i8, "offset", (-10, 10)).is_ok());
    assert!(FromQuery::range(&f64::NAN, "score", (0.0, 1.0)).is_err());
}

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

#[test]
fn from_param_value_accepts_every_from_str_type() {
    let uuid = "0190d9d0-7b1e-7c3a-8f2e-123456789abc";
    assert_eq!(uuid::Uuid::from_param_value(uuid).unwrap().to_string(), uuid);
    assert!(uuid::Uuid::from_param_value("not-a-uuid").is_err());
    let ulid = "01ARZ3NDEKTSV4RRFFQ69G5FAV";
    assert_eq!(ulid::Ulid::from_param_value(ulid).unwrap().to_string(), ulid);
    assert!(ulid::Ulid::from_param_value("01ARZ3NDEKTSV4RRFFQ69G5FAU!").is_err());
    assert_eq!(std::net::IpAddr::from_param_value("127.0.0.1").unwrap(), std::net::Ipv4Addr::LOCALHOST);
    assert_eq!(char::from_param_value("x").unwrap(), 'x');
    assert!(char::from_param_value("xy").is_err());
    assert_eq!(f64::from_param_value("0.5").unwrap(), 0.5);
}

#[test]
fn from_param_value_supports_user_types_with_from_str_or_with_a_direct_impl() {
    #[derive(Debug, PartialEq)]
    struct Code(u16);
    impl std::str::FromStr for Code {
        type Err = ();
        fn from_str(s: &str) -> Result<Self, ()> {
            s.strip_prefix("C-").and_then(|n| n.parse().ok()).map(Code).ok_or(())
        }
    }
    assert_eq!(Code::from_param_value("C-7").unwrap(), Code(7));
    assert!(Code::from_param_value("7").is_err());

    // A type without `FromStr` can still implement the trait itself.
    #[derive(Debug, PartialEq)]
    struct Tag(String);
    impl FromParamValue for Tag {
        fn from_param_value(value: &str) -> Result<Self, ()> {
            if value.is_empty() { Err(()) } else { Ok(Tag(value.to_uppercase())) }
        }
    }
    assert_eq!(Tag::from_param_value("ab").unwrap(), Tag("AB".into()));
    assert!(Tag::from_param_value("").is_err());
}

#[test]
fn from_param_value_empty_value_is_a_flag_only_for_bool() {
    assert_eq!(bool::from_param_value("").unwrap(), true);
    assert_eq!(bool::from_param_value("true").unwrap(), true);
    assert_eq!(bool::from_param_value("false").unwrap(), false);
    assert!(bool::from_param_value("yes").is_err());
    assert_eq!(String::from_param_value("").unwrap(), "");
    assert!(u32::from_param_value("").is_err());
    assert!(f64::from_param_value("").is_err());
    assert!(uuid::Uuid::from_param_value("").is_err());
}
