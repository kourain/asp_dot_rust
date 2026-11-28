# HyperConfig (HTTP server tuning)

`HyperConfig` controls request limits and connection timers for every HTTP listener. The listener captures the current configuration when it accepts a TCP connection. A successful configuration reload applies to future connections. Requests on an existing connection keep that connection's snapshot.

| Field | Type and default | Effect |
| ----- | ---------------- | ------ |
| `header_read_timeout` | `u64`, 10 seconds | Time allowed to read HTTP/1 request headers. |
| `http2_keep_alive_interval` | `u64`, 20 seconds | Interval between HTTP/2 keep-alive pings. |
| `http2_keep_alive_timeout` | `u64`, 10 seconds | Time to wait for acknowledgement of a keep-alive ping. |
| `max_request_body_size` | `usize`, 100 MiB | Default request body limit. A matched route can override it with `#[max_body_size(...)]`. |
| `max_request_headers_size` | `usize`, 32 KiB | Caps parsed request headers. HTTP/2 also receives this value as its maximum header list size, capped at `u32::MAX`. |

Before the middleware pipeline runs, ASP.RS checks the request headers, counting the name length, value length and four bytes of overhead per field. An excess returns **431**. A declared `Content-Length` above the effective body limit returns **413**. The incoming body stream also enforces the limit while it is read. [Per-route body limits](./routing.md#request-body-limit-per-route)

## Configure the listener

The builder registers `HyperConfig::default()` under the TOML key `hyper_server`. The built-in file policy matches the full path constructed from the executable directory and `\appsettings.toml`. The default watched file is relative to the working directory, so those paths can differ. Use `configuration.update()` for an in-code change that preserves the file binding.

```rust
use asp_dot_rust::{ApplicationBuilder, configuration::HyperConfig};

let mut builder = ApplicationBuilder::new("MyApp");
builder.configuration.update::<HyperConfig>(|cfg| {
    cfg.header_read_timeout = 5;
    cfg.max_request_body_size = 8 * 1024 * 1024;
});
```

To bind a file of your choice, call `builder.configuration.add_toml_file("server.toml", true).configure::<HyperConfig>("hyper_server")`. Supply all five fields in the TOML section. If the registered entry has no value, a new connection uses `HyperConfig::default()`. [TOML configuration](./appsettings.md)
