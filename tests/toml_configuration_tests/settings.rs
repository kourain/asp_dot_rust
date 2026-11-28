use serde::Deserialize;

/// A configuration type used only by these tests, kept separate from the
/// framework's built-in configuration types so tests here don't collide with
/// `dependency_injection_tests` or other integration test files registering
/// the same `ConfigurationService` type in the same process.
#[derive(Debug, Default, Clone, PartialEq, Deserialize)]
pub struct AppSettings {
    pub app_name: String,
    #[serde(default)]
    pub max_connections: u32,
}
