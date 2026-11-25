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

## HTTPS / TLS

Register an HTTPS listener with `ApplicationBuilder::with_https_port`. Both the HTTP and the HTTPS listeners can be registered on the same `ApplicationBuilder` and will run side by side; `HyperConfig` (above) applies to both.

```rust
use asp_dot_rust::ApplicationBuilder;

let mut builder = ApplicationBuilder::new("MyApp");

builder
    .with_http_port(8080)
    .with_https_port(8443, "certs/fullchain.pem", "certs/privkey.pem");
```

- `cert_path` must be a PEM-encoded certificate chain: the leaf certificate first, followed by any intermediate certificates.
- `key_path` must be a PEM-encoded private key: PKCS#8, PKCS#1 (RSA), or SEC1 (EC).
- Only **one** certificate/key pair is supported per application. Calling `with_https_port` more than once with different paths overwrites the previously loaded TLS configuration — every registered HTTPS port shares the same certificate (SNI/multi-cert is not supported).
- Client certificate authentication (mTLS) is not supported; the server never requests a client certificate.
- Failing to read or parse the certificate/key **panics at build time** (fail-fast), so a misconfigured deployment is caught at startup instead of on the first incoming HTTPS request.
- The HTTPS listener advertises both `h2` and `http/1.1` via ALPN, so clients that support HTTP/2 over TLS will use it automatically; plain HTTP (no TLS) is always served as HTTP/1.1 or HTTP/2 via [h2c](https://httpwg.org/specs/rfc7540.html#rfc.section.3.2), negotiated the same way as the existing HTTP listener.

### Command line

When the application loads process arguments with `StartupArgsConfiguration::with_args()`, the builder reads the HTTPS port and certificate paths during `build()`. See [Command-line arguments](./command_line_args.md).

```bash
my_app --https-port 8443 --tls-cert certs/fullchain.pem --tls-key certs/privkey.pem
```

`--https-port` is ignored (with a warning logged) if either `--tls-cert` or `--tls-key` is missing — HTTPS is only enabled when all three are present.
