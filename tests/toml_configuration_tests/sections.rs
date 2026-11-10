use asp_dot_rust::{ApplicationBuilder, configuration::JwtAuthConfiguration, configuration::RateLimitConfiguration};

#[tokio::test]
async fn loads_multiple_typed_sections_from_one_file() {
    let mut builder = ApplicationBuilder::new("TestTomlSections");

    builder
        .configuration
        .add_toml_file("tests/fixtures/appsettings.toml", false)
        .configure::<RateLimitConfiguration>("rate_limit")
        .configure::<JwtAuthConfiguration>("jwt");
    builder.configuration.reload_all_async().await;

    let rate_limit = builder.configuration.get::<RateLimitConfiguration>().capture().unwrap();
    assert_eq!(rate_limit.max_requests, 100);
    assert_eq!(rate_limit.limit_seconds, 60);
    assert_eq!(rate_limit.block_duration_seconds, 300);

    let jwt = builder.configuration.get::<JwtAuthConfiguration>().capture().unwrap();
    assert_eq!(jwt.secret_key, "test-secret");
    assert_eq!(jwt.token_expiration_seconds, 7200);
    assert_eq!(jwt.token_issuer, "asp_dot_rust_tests");
}

#[tokio::test]
async fn a_missing_section_has_no_value() {
    let mut builder = ApplicationBuilder::new("TestTomlMissingSection");
    builder
        .configuration
        .add_toml_file("tests/fixtures/appsettings.toml", false)
        .configure::<RateLimitConfiguration>("does_not_exist");
    builder.configuration.reload_all_async().await;

    assert!(builder.configuration.contains::<RateLimitConfiguration>());
    assert!(!builder.configuration.get::<RateLimitConfiguration>().capture().has_value());
}
