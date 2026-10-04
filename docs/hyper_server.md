# HyperConfig (HTTP Server Tuning)

`HyperConfig` is a regular configuration type (see `docs/appsettings.md` /
`docs/dependency_injection.md`) that tunes the underlying `hyper` server
used by every HTTP/HTTPS listener: how long to wait for a slow client to
finish sending headers, and how HTTP/2 keep-alive pings behave.

## Fields

| Field | Type | Default | Applies to | Effect |
| ----- | ---- | ------- | ---------- | ------ |
| `header_read_timeout` | `u64` (seconds) | `10` | HTTP/1.1 | On a kept-alive connection, how long the server waits for the client to finish sending request headers before closing the connection. Prevents a slow or idle client from holding a connection/task open forever. |
| `http2_keep_alive_interval` | `u64` (seconds) | `20` | HTTP/2 | How often the server sends an HTTP/2 PING frame on an otherwise idle connection. |
| `http2_keep_alive_timeout` | `u64` (seconds) | `10` | HTTP/2 | How long the server waits for a PING to be acknowledged before treating the connection as dead and closing it. |
| `max_request_body_size` | `usize` (bytes) | `100 * 1024 * 1024` (100 MB) | — | **Not currently enforced** (see Known limitations). |
| `max_request_headers_size` | `usize` (bytes) | `8 * 1024` (8 KB) | — | **Not currently enforced** (see Known limitations). |
| `max_response_headers_size` | `usize` (bytes) | `8 * 1024` (8 KB) | — | **Not currently enforced** (see Known limitations). |

## Using the defaults

`HyperConfig` does not need to be registered manually. If no value has
been registered via `add_default_configuration`/`add_custom_configuration`/
`insert`, the HTTP listener falls back to `HyperConfig::default()`
automatically.

## Customizing it

```rust
use asp_dot_rust::{ApplicationBuilder, configuration::HyperConfig};

let mut builder = ApplicationBuilder::new("MyApp");

builder.add_custom_configuration::<HyperConfig>(|cfg| {
    cfg.header_read_timeout = 5;
    cfg.http2_keep_alive_interval = 15;
    cfg.http2_keep_alive_timeout = 5;
});
```

`add_default_configuration::<HyperConfig>()` registers `HyperConfig::default()`
as-is. Both `add_default_configuration` and `add_custom_configuration` panic
if a `HyperConfig` has already been registered — call it only once, before
`build()`/`run()`.

`HyperConfig` can also be loaded from a TOML section the same way as any
other configuration type, via `ConfigurationService::configure::<HyperConfig>(section)`
(see `docs/appsettings.md`).

## Known limitations (What not work)

- `max_request_body_size`, `max_request_headers_size`, and
  `max_response_headers_size` are currently **unused** — setting them has
  no effect on the server. The server runs with `hyper`'s own built-in
  defaults for these limits regardless of what is configured here.
- `header_read_timeout` only bounds the time to read request *headers*; it
  does not bound how long sending the request *body* may take.
- One `HyperConfig` applies to every listener (every IP/port registered via
  `with_any_ip`/`with_http_port`/etc.) — there is no per-listener override.
- No hot-reload: the value is read once per connection when the listener
  accepts it; changing the registered `HyperConfig` at runtime does not
  affect connections already being served.
