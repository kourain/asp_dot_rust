use asp_dot_rust::{configuration::StartupAppConfiguration};

fn args(raw: &[&str]) -> StartupAppConfiguration {
    StartupAppConfiguration::default().parse(raw.iter().map(|s| s.to_string()))
}

#[test]
fn parses_key_value_pair_as_two_tokens() {
    let cli = args(&["--http-port", "8080"]);
    assert_eq!(cli.get("http-port"), Some("8080"));
}

#[test]
fn parses_key_equals_value_form() {
    let cli = args(&["--http-port=8080"]);
    assert_eq!(cli.get("http-port"), Some("8080"));
}

#[test]
fn boolean_flag_defaults_to_true() {
    let cli = args(&["--verbose"]);
    assert_eq!(cli.get("verbose"), Some("true"));
}

#[test]
fn boolean_flag_followed_by_another_flag_does_not_consume_it() {
    let cli = args(&["--verbose", "--http-port", "8080"]);
    assert_eq!(cli.get("verbose"), Some("true"));
    assert_eq!(cli.get("http-port"), Some("8080"));
}

#[test]
fn ignores_positional_arguments_without_double_dash_prefix() {
    let cli = args(&["serve", "--http-port", "8080"]);
    assert_eq!(cli.len(), 1);
    assert_eq!(cli.get("http-port"), Some("8080"));
}

#[test]
fn last_occurrence_of_a_repeated_key_wins() {
    let cli = args(&["--http-port", "8080", "--http-port", "9090"]);
    assert_eq!(cli.get("http-port"), Some("9090"));
}

#[test]
fn get_parsed_converts_to_the_requested_type() {
    let cli = args(&["--http-port", "8080"]);
    assert_eq!(cli.get_parsed::<u16>("http-port"), Some(8080));
}

#[test]
fn get_parsed_returns_none_for_an_invalid_value() {
    let cli = args(&["--http-port", "not-a-number"]);
    assert_eq!(cli.get_parsed::<u16>("http-port"), None);
}

#[test]
fn get_parsed_returns_none_for_a_missing_key() {
    let cli = args(&["--http-port", "8080"]);
    assert_eq!(cli.get_parsed::<u16>("https-port"), None);
}
