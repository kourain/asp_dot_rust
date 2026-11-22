# Command-line arguments

`ApplicationBuilder::new()` registers an empty `StartupArgsConfiguration`. To read the process arguments, call `with_args()` on that configuration before `build()`. It skips the executable name and parses the remaining arguments. You can read custom flags immediately. The builder applies the IP and port flags during `build()`.

```rust
use asp_dot_rust::{ApplicationBuilder, configuration::StartupArgsConfiguration};

let mut builder = ApplicationBuilder::new("MyApp");
builder.configuration.update::<StartupArgsConfiguration>(|cli| cli.with_args());

let cli = builder.configuration.get::<StartupArgsConfiguration>().capture().unwrap();
let worker_threads = cli.get_parsed::<u32>("worker-threads").unwrap_or(4);
let app = builder.build();
```

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

During `build()` the builder reads these keys from the registered configuration.

| Flag | Applied via | Behavior |
| ---- | ----------- | -------- |
| `--ip <addr>` | `with_ip` | Parses a `std::net::IpAddr`. An invalid address panics during `build()`. |
| `--http-port <port>` | `with_http_port` | Adds the port if the value parses as `u16`. |
| `--https-port <port>` | `with_https_port` | Records the port and logs a warning. The server currently opens HTTP listeners only. Using the same port for HTTP and HTTPS panics. |

Any other flag (e.g. `--verbose`, `--env=dev`, `--worker-threads 4`) is
parsed and stored, but **not** interpreted by the framework — reading it is
entirely up to your own code.

## Reading flags in an injectable service

```rust
use asp_dot_rust::{
    configuration::StartupArgsConfiguration,
    dependency_injection::CfgRequire,
    macros::inject,
};

pub struct WorkerService {
    thread_count: u32,
}

#[inject]
impl WorkerService {
    pub fn new(cli: CfgRequire<StartupArgsConfiguration>) -> Self {
        Self { thread_count: cli.get_parsed("worker-threads").unwrap_or(4) }
    }
}
```

## Parsing an arbitrary argument list directly

`parse()` replaces the values in the existing configuration and returns `&mut Self`. Keep the configuration in a variable to use it after parsing.

```rust
use asp_dot_rust::configuration::StartupArgsConfiguration;

let mut cli = StartupArgsConfiguration::default();
cli.parse(["--http-port", "8080", "--verbose"]);

assert_eq!(cli.get("http-port"), Some("8080"));
assert_eq!(cli.get("verbose"), Some("true"));
```

## Known limitations (what not work)

- **No short/combined flags.** Only the long `--flag` form is recognized;
  there is no `-v`, and boolean flags cannot be combined (`-abc`).
- **No arrays / repeated values.** A repeated `--key` does not accumulate
  into a list — only the last occurrence is kept.
- **No validation of unknown flags.** Any `--something` not recognized by
  the framework is stored but never flagged as invalid or unused — typos in
  flag names fail silently.
- An invalid `ip` value panics when `build()` applies the configuration.

## API summary

| Method | On | Description |
| ------ | -- | ----------- |
| `with_args()` | `StartupArgsConfiguration` | Replace the current values with parsed process arguments. |
| `parse(args)` | `StartupArgsConfiguration` | Replace the current values with those from an iterator and return a mutable reference to the configuration. |
| `get(key)` | `StartupArgsConfiguration` | Returns the raw `&str` value for `key`, or `None` if not present. |
| `get_parsed::<T>(key)` | `StartupArgsConfiguration` | Returns the value for `key` parsed as `T: FromStr`, or `None` if missing or unparsable. |
| `contains(key)` | `StartupArgsConfiguration` | Returns whether `key` was present on the command line. |
| `len()` | `StartupArgsConfiguration` | Number of distinct keys parsed. |
| `is_empty()` | `StartupArgsConfiguration` | Whether no flags were parsed at all. |
