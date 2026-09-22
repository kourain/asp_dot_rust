use asp_dot_rust::ApplicationBuilder;

use crate::toml_configuration_tests::settings::AppSettings;

#[tokio::test]
async fn loads_the_whole_file_into_the_target_type() {
    let mut builder = ApplicationBuilder::new("TestTomlWholeFile");

    builder.configuration.add_toml_cfg("tests/fixtures/app_settings.toml");
    builder.configuration.configure::<AppSettings>("app_setting");

    let settings = builder
        .configuration
        .get::<AppSettings>()
        .expect("AppSettings must be registered in ConfigurationService after add_toml_configuration");

    assert_eq!(settings.app_name, "asp_dot_rust demo");
    assert_eq!(settings.max_connections, 128);
}

#[tokio::test]
#[should_panic(expected = "Failed to read configuration file")]
async fn panics_when_the_file_does_not_exist() {
    let mut builder = ApplicationBuilder::new("TestTomlMissingFile");
    builder.configuration.add_toml_cfg("tests/fixtures/does_not_exist.toml");
}

#[tokio::test]
#[should_panic(expected = "Failed to parse")]
async fn panics_when_the_file_is_not_valid_toml() {
    let mut builder = ApplicationBuilder::new("TestTomlInvalidSyntax");
    builder.configuration.add_toml_cfg("tests/fixtures/invalid.toml");
}

#[tokio::test]
async fn optional_still_loads_the_file_when_it_exists() {
    let mut builder = ApplicationBuilder::new("TestTomlOptionalPresent");

    builder.configuration.add_toml_cfg("tests/fixtures/app_settings.toml");
    builder.configuration.configure::<AppSettings>("app_setting");

    let settings = builder.configuration.get::<AppSettings>().unwrap();
    assert_eq!(settings.app_name, "asp_dot_rust demo");
}

#[tokio::test]
#[should_panic(expected = "Failed to parse")]
async fn optional_still_panics_when_a_present_file_is_invalid() {
    let mut builder = ApplicationBuilder::new("TestTomlOptionalInvalid");
    // The file exists but is malformed: this must NOT be treated as "missing".
    builder.configuration.add_toml_cfg("tests/fixtures/invalid.toml");
}
