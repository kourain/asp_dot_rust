# App Settings (TOML Configuration)

`ConfigurationService` stores typed configuration entries. Bind a top-level TOML key to a Rust type to load its value from a file, or use `insert()` for a value supplied entirely by application code.

## Core concepts

| Concept | API | Purpose |
| ------- | --- | ------- |
| File | `add_toml_file(path, watch_file_change)` | Adds a TOML file. Nothing is read yet. `watch_file_change` tells if a change of the file reloads the configuration. |
| Entry | `configure::<T>(key)` | Binds the top-level value at `key` to `T`. The entry has no value until a file loads it. |
| Entry with a policy | `configure_entry::<T>(key, allow_file)` | Same as `configure`, `allow_file` tells which files may update the entry. |
| Static entry | `insert::<T>(\|cfg\| ...)` | Builds `T` from `T::default()` and the closure, no file changes it. |
| Load | `load_all_async()` | Reads registered files and applies eligible entries. `Application::run` calls it before it builds the middleware pipeline and opens a port. |
| Get | `get::<T>() -> CfgReload<T>` | The only way to get a configuration, see below. |

## Loading a TOML file and binding a section

```toml
# appsettings.toml
[rate_limit]
max_requests = 100
limit_seconds = 60
block_duration_seconds = 300

[jwt]
secret_key = "test-secret"
token_expiration_seconds = 7200
token_issuer = "asp_dot_rust_tests"
token_audience = "asp_dot_rust_tests_clients"
```

```rust
use asp_dot_rust::{ApplicationBuilder, configuration::{RateLimitConfiguration, JwtAuthConfiguration}};

let mut builder = ApplicationBuilder::new("MyApp");

builder
    .configuration
    .add_toml_file("appsettings.toml", true)
    .configure::<RateLimitConfiguration>("rate_limit")
    .configure::<JwtAuthConfiguration>("jwt");

// `Application::run` does this before it opens a port, call it yourself to read a value earlier
builder.configuration.load_all_async().await;

let rate_limit = builder.configuration.get::<RateLimitConfiguration>().capture().unwrap();
assert_eq!(rate_limit.max_requests, 100);
```

`ApplicationBuilder::new()` registers the literal path `.\appsettings.toml` for watching. On Windows this resolves relative to the working directory. On other platforms, pass a native path such as `"appsettings.toml"` explicitly. Adding the same resolved path again updates its watch flag. File and entry registration can happen in either order before `load_all_async()`.

Any type that implements `serde::de::DeserializeOwned + Send + Sync + 'static` can be bound.

## Get a configuration

`ConfigurationService::get::<T>()` returns a `CfgReload<T>` handle that follows every reload. All wrappers require a registered configuration type. Use `insert::<T>()` for a static value or `configure::<T>()` to bind a TOML section.

| Type | Made by | Registered entry without a value |
| ---- | ------- | -------- |
| `CfgReload<T>` | `get::<T>()` | the handle stays, its value is empty |
| `CfgOption<T>` (`Cfg<T>`, deprecated) | `get::<T>().capture()` | `has_value()` is `false` |
| `CfgRequire<T>` | `get::<T>().capture().unwrap()` | `unwrap()` panics |

- `CfgOption<T>` is the value at the moment of `capture()`, it does not change when the file is reloaded. It has `has_value()`, `unwrap()` (panics when there is no value) and `unwrap_or_default()`, and it derefs to `Option<Arc<T>>`.
- `CfgRequire<T>` derefs to `Arc<T>`, so the fields of `T` are used directly.
- `Cfg<T>` is a deprecated type alias for `CfgOption<T>`.
- A configured entry can be empty while waiting for its section to load.

The injection macros (`#[inject]`, `#[derive(DependencyInjectableService)]` and `#[middleware]`) generate this code for parameters and fields of the three types, see [`dependency_injection.md`](./dependency_injection.md).

```rust
use asp_dot_rust::{
    configuration::RateLimitConfiguration,
    dependency_injection::{CfgOption, CfgReload},
    macros::inject,
};

struct RateLimiterService {
    snapshot: CfgOption<RateLimitConfiguration>,
    live: CfgReload<RateLimitConfiguration>,
}

#[inject]
impl RateLimiterService {
    pub fn new(snapshot: CfgOption<RateLimitConfiguration>, live: CfgReload<RateLimitConfiguration>) -> Self {
        Self { snapshot, live }
    }
}
// later, on every request: the value of the file right now
// let max = self.live.capture().unwrap().max_requests;
```

`Application::run` loads the configuration files before building the middleware pipeline. `CfgOption<T>` captures the entry's current value when injected. If the section loads later, `CfgReload<T>` picks up the value on the next capture.

## Hot reload

Every `add_toml_file(path, true)` file is watched by `ConfigurationWatcherBackgroundService`, which `ApplicationBuilder::build()` adds to the application. When a file changes it is read again and every entry that is allowed to follow it is updated.

- The folder of each file is watched, so a file replaced by an editor (write a temp file, then rename) or created later is still seen. Events are collected for 2 seconds before the reload.
- A file added with `watch_file_change = false` is read by `load_all_async()` only.
- `configure_entry::<T>(key, policy)` controls which files can supply the value. `AllowedAll` accepts every registered file, `AllowedFileNames(vec!["appsettings.toml"])` matches a path suffix, `AllowedFullPaths(Vec<PathBuf>)` matches exact paths, and `AllowedRegexPattern(regex)` tests the full path.
- `NotAllowed` permits matching keys from files during the **first** `load_all_async()` call. It skips subsequent manual loads and watcher updates for that entry.
- A section that is not in the file keeps the current value. A file that cannot be read or parsed, or a section that does not match `T`, is logged and the current values are kept.
- `load_all_async()` applies files in registration order. The last successfully applied value wins. The watcher processes only changed files. If several change in the same batch, their processing order is unspecified.
- An entry made by `insert::<T>()` is never changed by a file.

## Missing files and sections

- A file that does not exist is skipped (it is read when it is created later), nothing panics. A file that is not valid TOML is logged and skipped.
- A missing or invalid section preserves the current value. An entry created with `configure()` starts empty. One created with `configure_default_entry()` starts with `T::default()`.
- A deserialization failure logs `Failed to parse [<key>] as <type name>`. Use `capture().has_value()` to check an entry that may be empty.

## Known limitations

- Lookup uses one literal top-level key. Its value can be a scalar, array or table that deserializes into `T`. A key such as `"Section:SubSection"` is looked up literally. Environment variables do not participate in loading.

## API summary

| Method | On | Description |
| ------ | -- | ----------- |
| `add_toml_file(path, watch_file_change)` | `ConfigurationService` | Add a TOML file, nothing is read. |
| `configure::<T>(key)` | `ConfigurationService` | Bind `key` to `T`, updated from any file. |
| `configure_entry::<T>(key, allow_file)` | `ConfigurationService` | Same as `configure`, with a `CfgEntryAllowReloadFile` policy. |
| `configure_default_entry::<T>(key, policy)` | `ConfigurationService` | Bind a key with an initial `T::default()` and a file policy. |
| `insert::<T>(custom)` | `ConfigurationService` | Replace the entry with a static value made from `T::default()` and `custom`. |
| `update::<T>(custom)` | `ConfigurationService` | Modify a clone of the current value (or its default) and publish it to existing handles. If the type is absent, insert a static entry. |
| `contains::<T>()` | `ConfigurationService` | Check whether `T` is registered, it may still have no value. |
| `get::<T>()` | `ConfigurationService` | Returns the `CfgReload<T>` handle. Panics if `T` is unregistered. |
| `load_all_async()` | `ConfigurationService` | Load eligible values from registered files in order. |
| `capture()` | `CfgReload<T>` | The `CfgOption<T>` value now. |
| `has_value()` / `unwrap()` / `unwrap_or_default()` | `CfgOption<T>` | Check / get the `CfgRequire<T>`, panics without a value / get it or `T::default()`. |
