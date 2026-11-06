use asp_dot_rust::{ApplicationBuilder, configuration::StartupArgsConfiguration};

fn args(raw: &[&str]) -> StartupArgsConfiguration {
    let mut rs = StartupArgsConfiguration::default();
    rs.parse(raw.iter().map(|s| s.to_string()));
    rs
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

/// `--https-port` without `--tls-cert` and `--tls-key` only logs a warning: the HTTPS port is skipped and HTTP keeps working.
#[tokio::test]
async fn https_port_flag_without_tls_files_is_skipped_and_http_port_still_applies() {
    let mut builder = ApplicationBuilder::new("Args");
    builder.configuration.update::<StartupArgsConfiguration>(|cfg| {
        cfg.parse(create_args_str(&["--http-port", "18911", "--https-port", "18912"]));
    });
    let server = tokio::spawn(builder.build().run());
    tokio::time::timeout(std::time::Duration::from_secs(5), async {
        while tokio::net::TcpStream::connect(("127.0.0.1", 18911)).await.is_err() {
            tokio::time::sleep(std::time::Duration::from_millis(20)).await;
        }
    }).await.expect("HTTP listener did not start");
    assert!(tokio::net::TcpStream::connect(("127.0.0.1", 18912)).await.is_err());
    server.abort();
}

#[tokio::test]
#[should_panic(expected = "already in use by HTTP")]
async fn https_port_conflicting_with_http_port_is_still_rejected() {
    let mut builder = ApplicationBuilder::new("Args");
    builder.configuration.update::<StartupArgsConfiguration>(|cfg| {
        cfg.parse(create_args_str(&["--http-port", "18913", "--https-port", "18913", "--tls-cert", "missing_cert.pem", "--tls-key", "missing_key.pem"]));
    });
    builder.build();
}
