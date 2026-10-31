use asp_dot_rust::{ApplicationBuilder, configuration::StartupAppConfiguration};

fn args(raw: &[&str]) -> StartupAppConfiguration {
    StartupAppConfiguration::default().parse(raw.iter().map(|s| s.to_string()))
}

#[tokio::test]
async fn parses_key_value_pair_as_two_tokens() {
    let cli = args(&["--http-port", "8080"]);
    assert_eq!(cli.get("http-port"), Some("8080"));
}

#[tokio::test]
async fn parses_key_equals_value_form() {
    let cli = args(&["--http-port=8080"]);
    assert_eq!(cli.get("http-port"), Some("8080"));
}

#[tokio::test]
async fn boolean_flag_defaults_to_true() {
    let cli = args(&["--verbose"]);
    assert_eq!(cli.get("verbose"), Some("true"));
}

#[tokio::test]
async fn boolean_flag_followed_by_another_flag_does_not_consume_it() {
    let cli = args(&["--verbose", "--http-port", "8080"]);
    assert_eq!(cli.get("verbose"), Some("true"));
    assert_eq!(cli.get("http-port"), Some("8080"));
}

#[tokio::test]
async fn ignores_positional_arguments_without_double_dash_prefix() {
    let cli = args(&["serve", "--http-port", "8080"]);
    assert_eq!(cli.len(), 1);
    assert_eq!(cli.get("http-port"), Some("8080"));
}

#[tokio::test]
async fn last_occurrence_of_a_repeated_key_wins() {
    let cli = args(&["--http-port", "8080", "--http-port", "9090"]);
    assert_eq!(cli.get("http-port"), Some("9090"));
}

#[tokio::test]
async fn get_parsed_converts_to_the_requested_type() {
    let cli = args(&["--http-port", "8080"]);
    assert_eq!(cli.get_parsed::<u16>("http-port"), Some(8080));
}

#[tokio::test]
async fn get_parsed_returns_none_for_an_invalid_value() {
    let cli = args(&["--http-port", "not-a-number"]);
    assert_eq!(cli.get_parsed::<u16>("http-port"), None);
}

#[tokio::test]
async fn get_parsed_returns_none_for_a_missing_key() {
    let cli = args(&["--http-port", "8080"]);
    assert_eq!(cli.get_parsed::<u16>("https-port"), None);
}

fn create_args_str(list: &[&str]) -> Vec<String> {
    list.iter().map(|s| s.to_string()).collect()
}

#[tokio::test]
async fn https_port_flag_does_not_panic_and_http_port_still_applies() {
    let mut builder = ApplicationBuilder::new("Args");
    builder.with_custom_args(create_args_str(&["--http-port", "18911", "--https-port", "18912"]));
    assert!(builder.http_port.contains(&18911));
    assert!(builder.https_port.contains(&18912));
}

#[tokio::test]
#[should_panic(expected = "already in use by HTTP")]
async fn https_port_conflicting_with_http_port_is_still_rejected() {
    let mut builder = ApplicationBuilder::new("Args");
    builder.with_custom_args(create_args_str(&["--http-port", "18913", "--https-port", "18913"]));
}
