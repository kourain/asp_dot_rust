use asp_dot_rust::{
    ApplicationBuilder,
    dependency_injection::{CfgOption, CfgReload, CfgRequire},
    services::configuration::ConfigurationService,
};
use asp_dot_rust_macros::{DependencyInjectableService, inject};
use serde::Deserialize;

#[derive(Debug, Default, Clone, PartialEq, Deserialize)]
pub struct DiSettings {
    pub app_name: String,
    pub max_connections: u32,
}

/// every kind of configuration wrapper injected by `#[derive(DependencyInjectableService)]`
#[derive(DependencyInjectableService)]
pub struct DerivedConfigService {
    pub option: CfgOption<DiSettings>,
    pub required: CfgRequire<DiSettings>,
    pub reload: CfgReload<DiSettings>,
}

/// every kind of configuration wrapper injected by `#[inject]` on `new`
pub struct InjectedConfigService {
    pub option: CfgOption<DiSettings>,
    pub required: CfgRequire<DiSettings>,
    pub reload: CfgReload<DiSettings>,
}

#[inject]
impl InjectedConfigService {
    pub fn new(option: CfgOption<DiSettings>, required: CfgRequire<DiSettings>, reload: CfgReload<DiSettings>) -> Self {
        Self { option, required, reload }
    }
}

/// a service that only takes the optional configuration
#[derive(DependencyInjectableService)]
pub struct OptionalConfigService {
    pub option: CfgOption<DiSettings>,
}

#[derive(DependencyInjectableService)]
pub struct ReloadOnlyService {
    pub reload: CfgReload<DiSettings>,
}

#[tokio::test]
async fn derive_injects_every_configuration_wrapper() {
    let mut builder = ApplicationBuilder::new("TestDiConfigDerive");
    builder.configuration.insert::<DiSettings>(|cfg| {
        cfg.app_name = "inserted".to_string();
        cfg.max_connections = 3;
    });
    builder.service.add_singleton::<DerivedConfigService>();
    let app = builder.build();

    let service = app.service_provider.get_service::<DerivedConfigService>().await;
    assert!(service.option.has_value());
    assert_eq!(service.option.clone().unwrap().app_name, "inserted");
    assert_eq!(service.required.max_connections, 3);
    assert_eq!(service.reload.capture().unwrap().app_name, "inserted");
}

#[tokio::test]
async fn inject_attribute_injects_every_configuration_wrapper() {
    let mut builder = ApplicationBuilder::new("TestDiConfigInject");
    builder.configuration.insert::<DiSettings>(|cfg| {
        cfg.app_name = "inserted".to_string();
        cfg.max_connections = 4;
    });
    builder.service.add_singleton::<InjectedConfigService>();
    let app = builder.build();

    let service = app.service_provider.get_service::<InjectedConfigService>().await;
    assert_eq!(service.option.clone().unwrap().max_connections, 4);
    assert_eq!(service.required.app_name, "inserted");
    assert!(service.reload.capture().has_value());
}

#[tokio::test]
async fn a_configuration_loaded_after_the_service_was_built_reaches_only_the_reload_handle() {
    let mut builder = ApplicationBuilder::new("TestDiConfigLoadedLate");
    builder.configuration.add_toml_file("tests/fixtures/app_settings.toml", false).configure::<DiSettings>("app_setting");
    builder.service.add_scope::<OptionalConfigService>();
    builder.service.add_singleton::<ReloadOnlyService>();
    let app = builder.build();

    // resolved before `reload_all_async`: the snapshot has no value, the handle follows the load
    let before = app.service_provider.get_service::<OptionalConfigService>().await;
    let handle_service = app.service_provider.get_service::<ReloadOnlyService>().await;
    assert!(!before.option.has_value());
    assert!(!handle_service.reload.capture().has_value());

    app.service_provider.get_service::<ConfigurationService>().await.reload_all_async().await;

    assert!(!before.option.has_value(), "a snapshot never changes");
    assert_eq!(handle_service.reload.capture().unwrap().max_connections, 128);

    // a service built after the load sees the value
    let after = app.service_provider.create_scope().get_service::<OptionalConfigService>().await;
    assert_eq!(after.option.clone().unwrap().app_name, "asp_dot_rust demo");
}

#[tokio::test]
#[should_panic(expected = "has no value")]
async fn a_required_configuration_without_a_value_panics_when_the_service_is_built() {
    let mut builder = ApplicationBuilder::new("TestDiConfigRequiredPanics");
    builder.configuration.configure::<DiSettings>("app_setting");
    builder.service.add_singleton::<DerivedConfigService>();
    let app = builder.build();

    // nothing loaded the configuration
    app.service_provider.get_service::<DerivedConfigService>().await;
}
