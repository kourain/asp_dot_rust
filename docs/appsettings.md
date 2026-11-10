# App Settings (TOML Configuration)

ASP.RS loads typed configuration sections from TOML files through `ConfigurationService`, inspired by ASP.NET Core's `appsettings.json` + `IOptions<T>` pattern. Every configuration is an entry of `ConfigurationService`, the entry is bound to a top-level TOML section and is updated again when the file changes.

## Core concepts

| Concept | API | Purpose |
| ------- | --- | ------- |
| File | `add_toml_file(path, watch_file_change)` | Adds a TOML file. Nothing is read yet. `watch_file_change` tells if a change of the file reloads the configuration. |
| Entry | `configure::<T>(key)` | Binds the top-level table `key` to `T`. The entry has no value until a file loads it. |
| Entry with a policy | `configure_entry::<T>(key, allow_file)` | Same as `configure`, `allow_file` tells which files may update the entry. |
| Static entry | `insert::<T>(\|cfg\| ...)` | Builds `T` from `T::default()` and the closure, no file changes it. |
| Load | `reload_all_async()` | Reads every file and updates every entry. `Application::run` calls it before it builds the middleware pipeline and opens a port. |
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
builder.configuration.reload_all_async().await;

let rate_limit = builder.configuration.get::<RateLimitConfiguration>().capture().unwrap();
assert_eq!(rate_limit.max_requests, 100);
```

`add_toml_file`, `configure`, `configure_entry` and `insert` return `&mut Self`, so calls can be chained. The order of `add_toml_file` and `configure` does not matter because nothing is read until `reload_all_async`. `./appsettings.toml` is added by `ApplicationBuilder::new`.

Any type that implements `Deserialize + Send + Sync + 'static` can be bound, it does not need to be one of the framework's built-in configuration types.

## Get a configuration

`ConfigurationService::get::<T>()` returns a `CfgReload<T>`, a handle that follows every reload of the configuration. The other wrappers are made from it:

| Type | Made by | No value |
| ---- | ------- | -------- |
| `CfgReload<T>` | `get::<T>()` | the handle stays, its value is empty |
| `CfgOption<T>` (`Cfg<T>`, deprecated) | `get::<T>().capture()` | `has_value()` is `false` |
| `CfgRequire<T>` | `get::<T>().capture().unwrap()` | `unwrap()` panics |

- `CfgOption<T>` is the value at the moment of `capture()`, it does not change when the file is reloaded. It has `has_value()`, `unwrap()` (panics when there is no value) and `unwrap_or_default()`, and it derefs to `Option<Arc<T>>`.
- `CfgRequire<T>` derefs to `Arc<T>`, so the fields of `T` are used directly.
- `Cfg<T>` is the old name of `CfgOption<T>`, it is deprecated and the struct will be renamed to `CfgOption<T>`.
- A configuration has no value when it is not registered, its section is missing in every file, or it is not loaded yet.

The injection macros (`#[inject]`, `#[derive(DependencyInjectableService)]` and `#[middleware]`) generate this code for parameters and fields of the three types, see [`dependency_injection.md`](./dependency_injection.md).

```rust
use asp_dot_rust::dependency_injection::{CfgOption, CfgReload};

#[inject]
impl RateLimiterService {
    pub fn new(snapshot: CfgOption<RateLimitConfiguration>, live: CfgReload<RateLimitConfiguration>) -> Self {
        Self { snapshot, live }
    }
}
// later, on every request: the value of the file right now
// let max = self.live.capture().unwrap().max_requests;
```

Services are built after `Application::run` loaded the configuration, so a `CfgOption<T>` has its value there. A service built before the load (for example resolved from the builder) holds a `CfgOption<T>` without a value, its `CfgReload<T>` gets the value once the configuration is loaded.

## Hot reload

Every `add_toml_file(path, true)` file is watched by `ConfigurationWatcherBackgroundService`, which `ApplicationBuilder::build()` adds to the application. When a file changes it is read again and every entry that is allowed to follow it is updated.

- The folder of each file is watched, so a file replaced by an editor (write a temp file, then rename) or created later is still seen. Events are collected for 2 seconds before the reload.
- A file added with `watch_file_change = false` is read by `reload_all_async()` only.
- `configure_entry::<T>(key, allow_file)` limits the files that update an entry: `NotAllowed` (never from a file), `Allowed(vec!["appsettings.toml"])` (the path ends with the name), `AllowedRegexPattern(regex)` (matched on the full path) or `AllowedAll` (what `configure` uses).
- A section that is not in the file keeps the current value. A file that cannot be read or parsed, or a section that does not match `T`, is logged and the current values are kept.
- A file is applied entry by entry: when several files have the same section, the one read last wins, `reload_all_async()` reads the files in the order they were added.
- An entry made by `insert::<T>()` is never changed by a file.

## Missing files and sections

- A file that does not exist is skipped (it is read when it is created later), nothing panics. A file that is not valid TOML is logged and skipped.
- A section that is not in any file leaves the configuration without a value, use `has_value()` or `unwrap_or_default()`.
- A section that does not deserialize into `T` is logged as `Failed to parse [<key>] as <type name>` and the configuration keeps its current value (no value at the first load).

## Known limitations

- There is no environment-variable override layer and no support for nested section paths (e.g. `"Section:SubSection"`), only top-level TOML tables can be bound.

## API summary

| Method | On | Description |
| ------ | -- | ----------- |
| `add_toml_file(path, watch_file_change)` | `ConfigurationService` | Add a TOML file, nothing is read. |
| `configure::<T>(key)` | `ConfigurationService` | Bind `key` to `T`, updated from any file. |
| `configure_entry::<T>(key, allow_file)` | `ConfigurationService` | Same as `configure`, with a `CfgEntryAllowReloadFile` policy. |
| `insert::<T>(custom)` | `ConfigurationService` | Static entry made from `T::default()` and `custom`, replaces an existing one. |
| `contains::<T>()` | `ConfigurationService` | Check whether `T` is registered, it may still have no value. |
| `get::<T>()` | `ConfigurationService` | The `CfgReload<T>` handle, empty if `T` is not registered. |
| `reload_all_async()` | `ConfigurationService` | Read every file and update every entry. |
| `capture()` | `CfgReload<T>` | The `CfgOption<T>` value now. |
| `has_value()` / `unwrap()` / `unwrap_or_default()` | `CfgOption<T>` | Check / get the `CfgRequire<T>`, panics without a value / get it or `T::default()`. |
