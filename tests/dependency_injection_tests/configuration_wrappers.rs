use asp_dot_rust::{
    ApplicationBuilder,
    dependency_injection::{Cfg, CfgReload, CfgRequire},
};
use asp_dot_rust_macros::{DependencyInjectableService, inject};

#[derive(Debug, Default, Clone, PartialEq)]
pub struct WrapperTestSettings {
    pub name: String,
}

#[derive(Debug, Default, Clone, PartialEq)]
pub struct WrapperTestMissing;

#[derive(DependencyInjectableService)]
pub struct DeriveConfigService {
    pub required: CfgRequire<WrapperTestSettings>,
    pub optional: Cfg<WrapperTestMissing>,
}

pub struct InjectConfigService {
    pub required: CfgRequire<WrapperTestSettings>,
    pub optional: Cfg<WrapperTestMissing>,
}

#[inject]
impl InjectConfigService {
    pub fn new(required: CfgRequire<WrapperTestSettings>, optional: Cfg<WrapperTestMissing>) -> Self {
        Self { required, optional }
    }
}

#[derive(DependencyInjectableService)]
pub struct ReloadConfigService {
    pub reloadable: CfgReload<WrapperTestSettings>,
}

#[tokio::test]
async fn configuration_wrappers_are_resolved_from_the_built_configuration() {
    let mut builder = ApplicationBuilder::new("TestConfigurationWrappers");
    builder.add_custom_configuration(|c: &mut WrapperTestSettings| c.name = "wrapped".into());
    builder.service.add_singleton::<DeriveConfigService>();
    builder.service.add_singleton::<InjectConfigService>();
    let app = builder.build();

    let derived = app.service_provider.get_service::<DeriveConfigService>().await;
    assert_eq!(derived.required.name, "wrapped");
    assert!(derived.optional.is_none());

    let injected = app.service_provider.get_service::<InjectConfigService>().await;
    assert_eq!(injected.required.name, "wrapped");
    assert!(injected.optional.is_none());
}

#[tokio::test]
#[should_panic(expected = "is required but was never registered")]
async fn cfg_require_panics_when_the_configuration_is_missing() {
    let mut builder = ApplicationBuilder::new("TestConfigurationWrappersMissing");
    builder.service.add_singleton::<DeriveConfigService>();
    let app = builder.build();

    let _ = app.service_provider.get_service::<DeriveConfigService>().await;
}

#[tokio::test]
#[should_panic(expected = "was never registered via configure_reload")]
async fn cfg_reload_panics_when_the_section_was_not_bound_for_reload() {
    let mut builder = ApplicationBuilder::new("TestConfigurationWrappersReloadMissing");
    builder.service.add_singleton::<ReloadConfigService>();
    let app = builder.build();

    let _ = app.service_provider.get_service::<ReloadConfigService>().await;
}
