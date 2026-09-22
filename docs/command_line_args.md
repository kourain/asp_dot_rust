# Command-Line Arguments (`StartupAppConfiguration`)

ASP.Rust automatically parses `std::env::args()` on every application start,
similar in spirit to ASP.NET Core's command-line configuration provider. The
raw process arguments are turned into a flat `key -> value` map
(`StartupAppConfiguration`) that is:

1. Used to populate a few framework-recognized settings directly on the
   `ApplicationBuilder` (`ip`, `http-port`, `https-port`).
2. Registered in `ConfigurationService` so any application-defined flag can
   be read back later, including from an injectable service.

This happens automatically — `ApplicationBuilder::new()` calls the internal
`with_args()` step for you. There is no public method to opt out of it.

## Recognized token forms

Parsing is a single left-to-right pass over the token list:

| Form | Example | Result |
| ---- | ------- | ------ |
| `--key=value` | `--http-port=8080` | `"http-port" -> "8080"` |
| `--key value` | `--http-port 8080` | `"http-port" -> "8080"` (consumes the next token, unless it also starts with `--`) |
| `--flag` (no value) | `--verbose` | `"verbose" -> "true"` (boolean flag) |
| anything without a `--` prefix | `serve` | ignored (treated as a positional argument) |

Additional parsing rules, verified against `tests/command_line_configuration.rs`:

- **Last occurrence wins.** `--http-port 8080 --http-port 9090` resolves to
  `"9090"`.
- **A boolean flag does not consume the next flag.**
  `--verbose --http-port 8080` yields `verbose = "true"` *and*
  `http-port = "8080"`, not `verbose = "--http-port"`.
- **Positional tokens (no `--` prefix) are dropped entirely** and do not
  count toward `len()`.
- A bare `--` with nothing after it (empty key) is ignored.

## Framework-recognized flags

These flags are consumed by `ApplicationBuilder::with_args()` and applied
directly to the builder, in addition to being stored in
`StartupAppConfiguration` like any other flag:

| Flag | Applied via | Notes | Version require |
| ---- | ----------- | ----- | --------------- |
| `--ip <addr>` | `ApplicationBuilder::with_ip` | Must parse as `std::net::IpAddr`; an invalid value panics (`Invalid IP address format`). | 0.2.0 |
| `--http-port <port>` | `ApplicationBuilder::with_http_port` | Parsed as `u16` via `get_parsed`; silently ignored if missing or not a valid `u16`. | 0.2.0 |
| `--https-port <port>` | `ApplicationBuilder::with_https_port` | Same parsing as `--http-port`. HTTPS support itself is not implemented yet (see [Known limitations](#known-limitations-what-not-work)). | 0.2.0 |

Any other flag (e.g. `--verbose`, `--env=dev`, `--worker-threads 4`) is
parsed and stored, but **not** interpreted by the framework — reading it is
entirely up to your own code.

## Reading custom flags in application code

Once `ApplicationBuilder::new()` has run, `StartupAppConfiguration` is
already registered in `ConfigurationService`, so it can be read the same way
as any other configuration object — either directly off the builder, or
later through dependency injection.

```rust
use asp_dot_rust::{ApplicationBuilder, configuration::StartupAppConfiguration};

let mut builder = ApplicationBuilder::new("MyApp");

let cli = builder.configuration.get::<StartupAppConfiguration>().unwrap();

if cli.contains("verbose") {
    // --verbose was passed
}

let worker_threads: u32 = cli.get_parsed("worker-threads").unwrap_or(4);
```

From an injectable service, using the same `Option<Arc<T>>` pattern
documented in `docs/dependency_injection.md`:

```rust
use std::sync::Arc;
use asp_dot_rust::macros::inject_require;
use asp_dot_rust::configuration::StartupAppConfiguration;

pub struct WorkerService {
    thread_count: u32,
}

#[inject_require]
impl WorkerService {
    pub fn new(cli: Option<Arc<StartupAppConfiguration>>) -> Self {
        let thread_count = cli
            .and_then(|c| c.get_parsed::<u32>("worker-threads"))
            .unwrap_or(4);
        Self { thread_count }
    }
}
```

## Parsing an arbitrary argument list directly

`StartupAppConfiguration::parse` is public so tests (and other custom entry
points) can parse a source other than `std::env::args()`:

```rust
use asp_dot_rust::configuration::StartupAppConfiguration;

let cli = StartupAppConfiguration::default()
    .parse(["--http-port", "8080", "--verbose"].iter().map(|s| s.to_string()));

assert_eq!(cli.get("http-port"), Some("8080"));
assert_eq!(cli.get("verbose"), Some("true"));
```

In normal application code you should not need this — it exists mainly for
testing and for callers who want to parse a custom source instead of the
real process arguments.

## Known limitations (what not work)

- **No short/combined flags.** Only the long `--flag` form is recognized;
  there is no `-v`, and boolean flags cannot be combined (`-abc`).
- **No arrays / repeated values.** A repeated `--key` does not accumulate
  into a list — only the last occurrence is kept.
- **No validation of unknown flags.** Any `--something` not recognized by
  the framework is stored but never flagged as invalid or unused — typos in
  flag names fail silently.
- **Invalid `--ip` panics immediately** (`expect("Invalid IP address format")`)
  rather than falling back to a default or returning an error.

## API summary

| Method | On | Description |
| ------ | -- | ----------- |
| `parse(args)` | `StartupAppConfiguration` | Parses an arbitrary iterator of string-like tokens into a fresh `StartupAppConfiguration`. Mainly for tests/custom entry points — production code gets this for free via `ApplicationBuilder::new()`. |
| `get(key)` | `StartupAppConfiguration` | Returns the raw `&str` value for `key`, or `None` if not present. |
| `get_parsed::<T>(key)` | `StartupAppConfiguration` | Returns the value for `key` parsed as `T: FromStr`, or `None` if missing or unparsable. |
| `contains(key)` | `StartupAppConfiguration` | Returns whether `key` was present on the command line. |
| `len()` | `StartupAppConfiguration` | Number of distinct keys parsed. |
| `is_empty()` | `StartupAppConfiguration` | Whether no flags were parsed at all. |
