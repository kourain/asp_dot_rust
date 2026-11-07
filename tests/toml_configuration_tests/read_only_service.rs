use std::sync::Arc;

use asp_dot_rust::{ApplicationBuilder, configuration::RateLimitConfiguration, services::configuration::ConfigurationService};

use crate::toml_configuration_tests::settings::AppSettings;

#[tokio::test]
async fn built_application_exposes_the_values_added_to_the_builder() {
    let mut builder = ApplicationBuilder::new("TestReadOnlyServiceValues");
    builder
        .configuration
        .add_toml_cfg("tests/fixtures/appsettings.toml")
        .configure::<RateLimitConfiguration>("rate_limit");
    let app = builder.build();

    assert!(app.configuration.contains::<RateLimitConfiguration>());
    let rate_limit = app.configuration.get::<RateLimitConfiguration>();
    let rate_limit = rate_limit.as_ref().expect("rate limit must be registered");
    assert_eq!(rate_limit.max_requests, 100);
    assert_eq!(rate_limit.limit_seconds, 60);
}

#[tokio::test]
async fn get_returns_none_wrapper_for_unregistered_type() {
    let app = ApplicationBuilder::new("TestReadOnlyServiceMissing").build();

    assert!(!app.configuration.contains::<AppSettings>());
    assert!(app.configuration.get::<AppSettings>().is_none());
    assert!(app.configuration.get_reload::<AppSettings>().is_none());
}

#[tokio::test]
#[should_panic(expected = "is required but was never registered")]
async fn require_panics_for_unregistered_type() {
    let app = ApplicationBuilder::new("TestReadOnlyServiceRequire").build();
    let _ = app.configuration.require::<AppSettings>();
}

#[tokio::test]
async fn provider_resolves_the_same_instance_as_the_application() {
    let app = ApplicationBuilder::new("TestReadOnlyServiceSameInstance").build();

    let from_provider = app.service_provider.get_service::<ConfigurationService>().await;
    assert!(Arc::ptr_eq(&*from_provider, &app.configuration));
}

#[tokio::test]
async fn reload_all_refreshes_only_reload_sections() {
    let path = std::env::temp_dir().join(format!("asp_dot_rust_reload_{}.toml", uuid_like()));
    std::fs::write(&path, "[app_setting]\napp_name = \"first\"\nmax_connections = 1\n").unwrap();
    let path_str = path.to_string_lossy().to_string();

    let mut builder = ApplicationBuilder::new("TestReadOnlyServiceReload");
    builder
        .configuration
        .add_toml_cfg(&path_str)
        .configure::<AppSettings>("app_setting")
        .configure_reload::<AppSettings>("app_setting");
    let app = builder.build();

    let reloadable = app.configuration.get_reload::<AppSettings>().expect("reload section must be registered");
    assert_eq!(reloadable.load().max_connections, 1);

    std::fs::write(&path, "[app_setting]\napp_name = \"second\"\nmax_connections = 2\n").unwrap();
    app.configuration.reload_all();
    std::fs::remove_file(&path).ok();

    assert_eq!(reloadable.load().app_name, "second");
    assert_eq!(reloadable.load().max_connections, 2);
    // a plain `configure` section keeps the value read at build time
    assert_eq!(app.configuration.get::<AppSettings>().as_ref().unwrap().max_connections, 1);
}

fn uuid_like() -> String {
    let nanos = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos();
    format!("{}_{}", std::process::id(), nanos)
}
