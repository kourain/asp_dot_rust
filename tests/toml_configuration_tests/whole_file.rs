use asp_dot_rust::ApplicationBuilder;

use crate::toml_configuration_tests::settings::AppSettings;

#[tokio::test]
async fn loads_the_whole_file_into_the_target_type() {
    let mut builder = ApplicationBuilder::new("TestTomlWholeFile");

    builder.configuration.add_toml_file("tests/fixtures/app_settings.toml", false);
    builder.configuration.configure::<AppSettings>("app_setting");
    builder.configuration.reload_all_async().await;

    let settings = builder.configuration.get::<AppSettings>().capture().unwrap();

    assert_eq!(settings.app_name, "asp_dot_rust demo");
    assert_eq!(settings.max_connections, 128);
}

#[tokio::test]
async fn a_file_that_does_not_exist_is_skipped() {
    let mut builder = ApplicationBuilder::new("TestTomlMissingFile");
    builder.configuration.add_toml_file("tests/fixtures/does_not_exist.toml", false);
    builder.configuration.add_toml_file("tests/fixtures/app_settings.toml", false);
    builder.configuration.configure::<AppSettings>("app_setting");
    builder.configuration.reload_all_async().await;

    // the other file is still loaded
    assert_eq!(builder.configuration.get::<AppSettings>().capture().unwrap().max_connections, 128);
}

#[tokio::test]
async fn a_file_that_is_not_valid_toml_is_skipped() {
    let mut builder = ApplicationBuilder::new("TestTomlInvalidSyntax");
    builder.configuration.add_toml_file("tests/fixtures/invalid.toml", false);
    builder.configuration.configure::<AppSettings>("app_setting");
    builder.configuration.reload_all_async().await;

    assert!(!builder.configuration.get::<AppSettings>().capture().has_value());

    // a bad file does not stop the next one
    builder.configuration.add_toml_file("tests/fixtures/app_settings.toml", false);
    builder.configuration.reload_all_async().await;
    assert_eq!(builder.configuration.get::<AppSettings>().capture().unwrap().app_name, "asp_dot_rust demo");
}
