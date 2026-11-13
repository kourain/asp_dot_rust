use std::{
    path::{Path, PathBuf},
    time::{Duration, Instant},
};

use asp_dot_rust::{
    ApplicationBuilder,
    dependency_injection::{CfgOption, DependencyInjectableService},
    hosted_service::{BackGroundService, ConfigurationWatcherBackgroundService},
    services::configuration::{CfgEntryAllowReloadFile, ConfigurationService},
};
use serde::Deserialize;

use crate::toml_configuration_tests::settings::AppSettings;

#[derive(Debug, Default, Clone, PartialEq, Deserialize)]
struct SectionOne {
    value: u32,
}

#[derive(Debug, Default, Clone, PartialEq, Deserialize)]
struct SectionTwo {
    value: u32,
}

/// a section type without defaults on its fields, so a partial section does not deserialize
#[derive(Debug, Clone, PartialEq, Deserialize)]
struct StrictSection {
    name: String,
}

/// temp folder removed on drop, every test uses its own
struct TempDir(PathBuf);

impl TempDir {
    fn new(name: &str) -> Self {
        let path = std::env::temp_dir().join(format!("asp_dot_rust_reload_{}_{}", name, std::process::id()));
        let _ = std::fs::remove_dir_all(&path);
        std::fs::create_dir_all(&path).unwrap();
        TempDir(path)
    }

    fn file(&self, name: &str) -> PathBuf {
        self.0.join(name)
    }

    fn write(&self, name: &str, content: &str) -> PathBuf {
        let path = self.file(name);
        std::fs::write(&path, content).unwrap();
        path
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn app_settings_toml(name: &str, max_connections: u32) -> String {
    format!("[app_setting]\napp_name = \"{}\"\nmax_connections = {}\n", name, max_connections)
}

async fn wait_until(what: &str, mut condition: impl FnMut() -> bool) {
    let started = Instant::now();
    while !condition() {
        assert!(started.elapsed() < Duration::from_secs(30), "timed out waiting for: {}", what);
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
}

#[tokio::test]
async fn configure_has_no_value_until_the_configuration_is_loaded() {
    let dir = TempDir::new("deferred");
    let file = dir.write("appsettings.toml", &app_settings_toml("loaded", 1));
    let mut builder = ApplicationBuilder::new("TestReloadDeferred");
    builder.configuration.add_toml_file(&file, true).configure::<AppSettings>("app_setting");

    // registered, but nothing is read yet
    assert!(builder.configuration.contains::<AppSettings>());
    let handle = builder.configuration.get::<AppSettings>();
    assert!(!handle.capture().has_value());

    builder.configuration.reload_all_async().await;

    let loaded: CfgOption<AppSettings> = handle.capture();
    assert!(loaded.has_value());
    assert_eq!(loaded.unwrap().app_name, "loaded");
}

#[tokio::test]
async fn configure_does_not_depend_on_the_order_of_add_toml_file() {
    let dir = TempDir::new("order");
    let file = dir.write("appsettings.toml", "[one]\nvalue = 4\n");
    let mut builder = ApplicationBuilder::new("TestReloadOrder");
    builder.configuration.configure::<SectionOne>("one").add_toml_file(&file, true);
    builder.configuration.reload_all_async().await;

    assert_eq!(builder.configuration.get::<SectionOne>().capture().unwrap().value, 4);
}

#[tokio::test]
async fn a_missing_section_keeps_no_value() {
    let dir = TempDir::new("missing_section");
    let file = dir.write("appsettings.toml", "[other]\nvalue = 1\n");
    let mut builder = ApplicationBuilder::new("TestReloadMissingSection");
    builder.configuration.add_toml_file(&file, true).configure::<SectionOne>("one");
    builder.configuration.reload_all_async().await;

    let captured = builder.configuration.get::<SectionOne>().capture();
    assert!(!captured.has_value());
    assert_eq!(captured.unwrap_or_default().value, 0);
}

#[tokio::test]
#[should_panic(expected = "has no value")]
async fn unwrap_panics_while_the_configuration_has_no_value() {
    let dir = TempDir::new("unwrap");
    let file = dir.write("appsettings.toml", "[other]\nvalue = 1\n");
    let mut builder = ApplicationBuilder::new("TestReloadUnwrap");
    builder.configuration.add_toml_file(&file, true).configure::<SectionOne>("one");
    builder.configuration.reload_all_async().await;

    builder.configuration.get::<SectionOne>().capture().unwrap();
}

#[tokio::test]
async fn a_configuration_that_is_not_registered_has_no_value() {
    let builder = ApplicationBuilder::new("TestReloadNotRegistered");

    assert!(!builder.configuration.contains::<SectionTwo>());
    assert!(!builder.configuration.get::<SectionTwo>().capture().has_value());
}

#[test]
#[allow(deprecated)]
fn cfg_option_is_the_same_type_as_cfg() {
    let option: CfgOption<SectionOne> = CfgOption::default();
    let old: asp_dot_rust::dependency_injection::CfgOption<SectionOne> = option;
    assert!(!old.has_value());
}

#[tokio::test]
async fn a_handle_follows_every_reload_and_a_capture_does_not() {
    let dir = TempDir::new("handle");
    let file = dir.write("appsettings.toml", &app_settings_toml("before", 1));
    let mut builder = ApplicationBuilder::new("TestReloadHandle");
    builder.configuration.add_toml_file(&file, true).configure::<AppSettings>("app_setting");
    builder.configuration.reload_all_async().await;

    let handle = builder.configuration.get::<AppSettings>();
    let snapshot = handle.capture().unwrap();
    assert_eq!(snapshot.app_name, "before");

    dir.write("appsettings.toml", &app_settings_toml("after", 2));
    builder.configuration.reload_all_async().await;

    assert_eq!(handle.capture().unwrap().app_name, "after");
    assert_eq!(handle.capture().unwrap().max_connections, 2);
    // a handle taken later and a clone follow too
    assert_eq!(builder.configuration.get::<AppSettings>().capture().unwrap().app_name, "after");
    assert_eq!(handle.clone().capture().unwrap().app_name, "after");
    // what was captured before keeps its value
    assert_eq!(snapshot.app_name, "before");
}

#[tokio::test]
async fn allow_policy_decides_which_files_update_an_entry() {
    let dir = TempDir::new("allow_policy");
    let file_a = dir.write("a.toml", "[one]\nvalue = 1\n[two]\nvalue = 1\n");
    let file_b = dir.write("b.toml", "[one]\nvalue = 2\n[two]\nvalue = 2\n");
    let mut builder = ApplicationBuilder::new("TestReloadAllowPolicy");
    builder
        .configuration
        .add_toml_file(&file_a, true)
        .add_toml_file(&file_b, true)
        .configure_entry::<SectionOne>("one", CfgEntryAllowReloadFile::NotAllowed)
        .configure_entry::<SectionTwo>("two", CfgEntryAllowReloadFile::AllowedFileNames(vec!["a.toml"]));
    builder.configuration.reload_all_async().await;

    assert!(!builder.configuration.get::<SectionOne>().capture().has_value());
    assert_eq!(builder.configuration.get::<SectionTwo>().capture().unwrap().value, 1);
}

#[tokio::test]
async fn the_last_added_file_wins_for_the_same_section() {
    let dir = TempDir::new("last_wins");
    let base = dir.write("appsettings.toml", "[one]\nvalue = 1\n");
    let overlay = dir.write("appsettings.dev.toml", "[one]\nvalue = 2\n");
    let mut builder = ApplicationBuilder::new("TestReloadLastWins");
    builder.configuration.add_toml_file(&base, true).add_toml_file(&overlay, true).configure::<SectionOne>("one");
    builder.configuration.reload_all_async().await;
    assert_eq!(builder.configuration.get::<SectionOne>().capture().unwrap().value, 2);

    // reading the files again keeps the order
    dir.write("appsettings.toml", "[one]\nvalue = 3\n");
    builder.configuration.reload_all_async().await;
    assert_eq!(builder.configuration.get::<SectionOne>().capture().unwrap().value, 2);
}

#[test]
fn allow_policy_matching() {
    let path = "/srv/app/appsettings.dev.toml";
    assert!(CfgEntryAllowReloadFile::AllowedAll.allows(path));
    assert!(!CfgEntryAllowReloadFile::NotAllowed.allows(path));
    assert!(CfgEntryAllowReloadFile::AllowedFileNames(vec!["appsettings.dev.toml"]).allows(path));
    assert!(CfgEntryAllowReloadFile::AllowedFileNames(vec!["app/appsettings.dev.toml"]).allows(path));
    assert!(!CfgEntryAllowReloadFile::AllowedFileNames(vec!["appsettings.toml"]).allows(path));
    let pattern = regex::Regex::new(r"appsettings(\.(dev|prod|stag))?\.toml$").unwrap();
    assert!(CfgEntryAllowReloadFile::AllowedRegexPattern(pattern.clone()).allows(path));
    assert!(CfgEntryAllowReloadFile::AllowedRegexPattern(pattern).allows("/srv/app/appsettings.toml"));
    let strict = regex::Regex::new(r"appsettings\.prod\.toml$").unwrap();
    assert!(!CfgEntryAllowReloadFile::AllowedRegexPattern(strict).allows(path));
    assert!(!Path::new(path).ends_with("dev.toml"));
}

#[tokio::test]
async fn a_bad_reload_keeps_the_current_value() {
    let dir = TempDir::new("bad_reload");
    let file = dir.write("appsettings.toml", "[strict]\nname = \"good\"\n");
    let mut builder = ApplicationBuilder::new("TestReloadBad");
    builder.configuration.add_toml_file(&file, true).configure::<StrictSection>("strict");
    builder.configuration.reload_all_async().await;
    assert_eq!(builder.configuration.get::<StrictSection>().capture().unwrap().name, "good");

    // a section that does not match the type
    dir.write("appsettings.toml", "[strict]\nother = 1\n");
    builder.configuration.reload_all_async().await;
    assert_eq!(builder.configuration.get::<StrictSection>().capture().unwrap().name, "good");

    // a file that is not valid toml
    dir.write("appsettings.toml", "[strict\nname = ");
    builder.configuration.reload_all_async().await;
    assert_eq!(builder.configuration.get::<StrictSection>().capture().unwrap().name, "good");

    // a section that is gone
    dir.write("appsettings.toml", "[unrelated]\nvalue = 1\n");
    builder.configuration.reload_all_async().await;
    assert_eq!(builder.configuration.get::<StrictSection>().capture().unwrap().name, "good");

    // and the next good file is applied
    dir.write("appsettings.toml", "[strict]\nname = \"fixed\"\n");
    builder.configuration.reload_all_async().await;
    assert_eq!(builder.configuration.get::<StrictSection>().capture().unwrap().name, "fixed");
}

#[tokio::test]
async fn an_inserted_value_has_a_handle_and_no_file_changes_it() {
    let dir = TempDir::new("inserted");
    let file = dir.write("appsettings.toml", "[one]\nvalue = 1\n");
    let mut builder = ApplicationBuilder::new("TestReloadInserted");
    builder.configuration.add_toml_file(&file, true);
    builder.configuration.insert::<SectionOne>(|cfg| cfg.value = 10);

    assert!(builder.configuration.contains::<SectionOne>());
    assert_eq!(builder.configuration.get::<SectionOne>().capture().unwrap().value, 10);
    builder.configuration.reload_all_async().await;
    assert_eq!(builder.configuration.get::<SectionOne>().capture().unwrap().value, 10);

    // inserting again replaces the value
    builder.configuration.insert::<SectionOne>(|cfg| cfg.value = 20);
    assert_eq!(builder.configuration.get::<SectionOne>().capture().unwrap().value, 20);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn the_watcher_reloads_when_a_watched_file_changes_or_is_replaced() {
    let dir = TempDir::new("watcher");
    let file = dir.write("appsettings.toml", &app_settings_toml("start", 1));
    let not_watched = dir.write("static.toml", "[one]\nvalue = 1\n");
    let mut builder = ApplicationBuilder::new("TestReloadWatcher");
    builder
        .configuration
        .add_toml_file(&file, true)
        .add_toml_file(&not_watched, false)
        .configure::<AppSettings>("app_setting")
        .configure::<SectionOne>("one");
    let app = builder.build();

    let configuration = app.service_provider.get_service::<ConfigurationService>().await;
    configuration.reload_all_async().await;
    let handle = configuration.get::<AppSettings>();
    let one = configuration.get::<SectionOne>();
    let mut watcher = ConfigurationWatcherBackgroundService::inject(&app.service_provider).await;
    let task = tokio::spawn(async move { watcher.invoke_async().await });
    tokio::time::sleep(Duration::from_millis(1000)).await;

    // written in place, a file that is not watched changes too but is not reloaded
    dir.write("static.toml", "[one]\nvalue = 2\n");
    dir.write("appsettings.toml", &app_settings_toml("in_place", 2));
    wait_until("in place change", || handle.capture().as_ref().is_some_and(|c| c.app_name == "in_place")).await;
    assert_eq!(handle.capture().unwrap().max_connections, 2);
    assert_eq!(one.capture().unwrap().value, 1);

    // an editor writes a temp file and renames it over the config file
    let temp = dir.write("appsettings.toml.tmp", &app_settings_toml("replaced", 3));
    std::fs::rename(temp, &file).unwrap();
    wait_until("replaced file", || handle.capture().as_ref().is_some_and(|c| c.app_name == "replaced")).await;
    assert_eq!(handle.capture().unwrap().max_connections, 3);

    task.abort();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn run_loads_the_configuration_before_it_opens_the_port() {
    let dir = TempDir::new("run");
    let file = dir.write("appsettings.toml", &app_settings_toml("from_run", 5));
    let mut builder = ApplicationBuilder::new("TestReloadRun");
    builder.with_loopback_ip().with_http_port(18790);
    builder.configuration.add_toml_file(&file, true).configure::<AppSettings>("app_setting");
    let app = builder.build();
    let handle = app.service_provider.get_service::<ConfigurationService>().await.get::<AppSettings>();
    assert!(!handle.capture().has_value());

    let task = tokio::spawn(app.run());
    wait_until("the configuration to be loaded by run", || handle.capture().has_value()).await;
    assert_eq!(handle.capture().unwrap().app_name, "from_run");

    task.abort();
}
