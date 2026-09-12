# App Settings (TOML Configuration)

ASP.Rust supports loading typed configuration sections from TOML files
through `ConfigurationService`, inspired by ASP.NET Core's
`appsettings.json` + `IOptions<T>` pattern. A TOML file is parsed into raw
tables first, then individual top-level sections are bound into strongly
typed Rust structs on demand via `serde`.

## Core concepts

| Concept | Type | Purpose |
| ------- | ---- | ------- |
| Raw file store | `ConfigurationService::add_toml_cfg` | Reads and parses a TOML file into a `toml::Table`, keyed internally by its path. Can be called multiple times to load several files. |
| Section binding | `ConfigurationService::configure::<T>(section)` | Looks up a top-level table named `section` across every loaded file, deserializes it into `T`, and registers `T` in the same `ConfigurationService` used for regular configuration objects. |
| Typed config object | any `T: DeserializeOwned + Default + Debug + Clone + Send + Sync + 'static` | The Rust struct a section is bound to, e.g. `RateLimitConfiguration`, `JwtAuthConfiguration`, or an app-defined type. |
| Consumption | `ConfigurationService::get::<T>()` or `Cfg<T>` constructor parameter | Same retrieval path already used for configuration registered with `insert::<T>()` — see `docs/dependency_injection.md`. |

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
    .add_toml_cfg("appsettings.toml")
    .configure::<RateLimitConfiguration>("rate_limit")
    .configure::<JwtAuthConfiguration>("jwt");

let rate_limit = builder.configuration.get::<RateLimitConfiguration>().unwrap();
assert_eq!(rate_limit.max_requests, 100);
```

`add_toml_cfg` and `configure` both return `&mut Self`, so calls can be
chained the same way `service.add_singleton::<T>()` calls are chained.

## Binding an app-defined section

Any type that implements `Deserialize`, `Default`, `Debug`, `Clone`, `Send`
and `Sync` can be used as a section target — it does not need to be one of
the framework's built-in configuration types:

```rust
use serde::Deserialize;

#[derive(Debug, Default, Clone, Deserialize)]
pub struct AppSettings {
    pub app_name: String,
    #[serde(default)]
    pub max_connections: u32,
}
```

```rust
builder.configuration.add_toml_cfg("app_settings.toml");
builder.configuration.configure::<AppSettings>("app_setting");

let settings = builder.configuration.get::<AppSettings>().unwrap();
```

## Loading the default `appsettings`

`load_main_toml()` is a shortcut that loads a fixed, hard-coded path:
It auto call when ApplicationBuilder creating

- default: appsetting.toml
- env: custom by env `ASP_RS_ENVIRONMENT=dev` or startup args `--env=dev`
=> load appsetting.dev.toml

## Consuming configuration in an injectable service

Once a section has been bound with `configure::<T>()`, it is retrievable
through the same `Cfg<T>` constructor-parameter mechanism documented
in `docs/dependency_injection.md`:

```rust
use asp_dot_rust::dependcy_injection::Cfg;
use asp_dot_rust::macros::inject;

pub struct RateLimiterService {
    config: RateLimitConfiguration,
}

#[inject]
impl RateLimiterService {
    pub fn new(config: Cfg<RateLimitConfiguration>) -> Self {
        Self { config: config.0.map(|c| (*c).clone()).unwrap_or_default() }
    }
}
```

## Missing sections and missing files

- If a section key is not present in any loaded file, `configure::<T>()`
  registers `T::default()` instead of panicking.
- If the file passed to `add_toml_cfg` cannot be read (e.g. it does not
  exist), the failure is only logged as a warning; the call does not panic
  and no table is added for that path. A subsequent `configure::<T>()` call
  then falls back to `T::default()` for any section that was only expected
  to come from that file.
- If a file *is* found but is not valid TOML, `add_toml_cfg` panics
  immediately with `Failed to parse <path> file <error>`.
- If a section *is* found but does not deserialize into `T`, `configure::<T>()`
  panics with `Failed to parse [<section>] of toml <path> as <type name>`.

## Known limitations (What not work)

- When the same section name exists in more than one file loaded via
  `add_toml_cfg`, `configure::<T>()` iterates the internal file map in
  unspecified order, so which file "wins" is not guaranteed and is not
  documented as deterministic.
- `configure::<T>()` requires `T: Default`; a section type with no sensible
  default value must still provide one (e.g. via `#[derive(Default)]` or a
  manual `impl`).
- There is no environment-variable override layer and no support for
  nested/segmented section paths (e.g. ASP.NET Core's `"Section:SubSection"`
  syntax) — only flat, top-level TOML tables can be bound.
- There is currently no hot-reload: files are parsed once when
  `add_toml_cfg` is called, and `configure::<T>()` must be called again
  manually to react to any change.

## API summary

| Method | On | Description |
| ------ | -- | ----------- |
| `add_toml_cfg(path)` | `ConfigurationService` | Read and parse a TOML file, storing it under `path`. Panics if the file exists but fails to parse; logs a warning (does not panic) if the file cannot be read. |
| `load_main_toml()` | `ConfigurationService` | Shortcut for `add_toml_cfg("./appsettings.toml")`. |
| `configure::<T>(section)` | `ConfigurationService` | Deserialize the `section` table from every loaded file into `T` and register it (falls back to `T::default()` if the section is absent anywhere). Panics if the section exists but does not match `T`. |
| `insert::<T>(config)` | `ConfigurationService` | Register a configuration value directly, bypassing TOML. Logs a warning if `T` was already registered. |
| `get::<T>()` | `ConfigurationService` | Retrieve a previously registered configuration value, whether it came from `insert` or `configure`. Returns `None` if `T` was never registered. |
| `contains::<T>()` | `ConfigurationService` | Check whether a configuration value of type `T` is registered. |
