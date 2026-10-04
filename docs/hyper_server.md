# Http/Https Server : Hyper

## HyperConfig (HTTP Server Tuning)

`HyperConfig` is a regular configuration type (see `docs/appsettings.md` / `docs/dependency_injection.md`) that tunes the underlying `hyper` server used by every HTTP/HTTPS listener: how long to wait for a slow client to finish sending headers, and how TTP/2 keep-alive pings behave.

### Fields

| Field | Type | Default | Applies to | Effect |
| ----- | ---- | ------- | ---------- | ------ |
| `header_read_timeout` | `u64` (seconds) | `10` | HTTP/1.1 | On a kept-alive connection, how long the server waits for the client to finish sending request headers before closing the connection. Prevents a slow or idle client from holding a connection/task open forever. |
| `http2_keep_alive_interval` | `u64` (seconds) | `20` | HTTP/2 | How often the server sends an HTTP/2 PING frame on an otherwise idle connection. |
| `http2_keep_alive_timeout` | `u64` (seconds) | `10` | HTTP/2 | How long the server waits for a PING to be acknowledged before treating the connection as dead and closing it. |
| `max_request_body_size` | `usize` (bytes) | `100 * 1024 * 1024` (100 MB) | — | **Not currently enforced** (see Known limitations). |
| `max_request_headers_size` | `usize` (bytes) | `8 * 1024` (8 KB) | — | **Not currently enforced** (see Known limitations). |
| `max_response_headers_size` | `usize` (bytes) | `8 * 1024` (8 KB) | — | **Not currently enforced** (see Known limitations). |

### Using the defaults

`HyperConfig` does not need to be registered manually. If no value has
been registered via `add_default_configuration` `add_custom_configuration`/ `insert`, the HTTP listener falls back to `HyperConfig::default()` automatically.

### Customizing it

```rust
use asp_dot_rust::{ApplicationBuilder, configuration::HyperConfig};

let mut builder = ApplicationBuilder::new("MyApp");

builder.add_custom_configuration::<HyperConfig>(|cfg| {
    cfg.header_read_timeout = 5;
    cfg.http2_keep_alive_interval = 15;
    cfg.http2_keep_alive_timeout = 5;
});
```

`add_default_configuration::<HyperConfig>()` registers `HyperConfig::default()` as-is.
Both `add_default_configuration` and `add_custom_configuration` panic if a `HyperConfig` has already been registered — call it only once, before `build()`/`run()`.

`HyperConfig` can also be loaded from a TOML section the same way as any other configuration type, via `ConfigurationService::configure::<HyperConfig>(section)` (see `docs/appsettings.md`).

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

### Command line / `appsettings.toml`

The HTTPS port and certificate paths can also be supplied on the command line (see `docs/command_line_args.md`):

```bash
my_app --https-port 8443 --tls-cert certs/fullchain.pem --tls-key certs/privkey.pem
```

`--https-port` is ignored (with a warning logged) if either `--tls-cert` or `--tls-key` is missing — HTTPS is only enabled when all three are present.

## Known limitations (What not work)

- `max_request_body_size`, `max_request_headers_size`, and `max_response_headers_size` are currently **unused** — setting them has no effect on the server. The server runs with `hyper`'s own built-in defaults for these limits regardless of what is configured here.
- `header_read_timeout` only bounds the time to read request *headers*; it does not bound how long sending the request *body* may take.
- One `HyperConfig` applies to every listener (every IP/port registered via `with_any_ip`/`with_http_port`/etc.) — there is no per-listener override.
- No hot-reload: the value is read once per connection when the listener accepts it; changing the registered `HyperConfig` at runtime does not affect connections already being served.
