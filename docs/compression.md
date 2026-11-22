# Compression

`CompressionMiddleware` compresses the response body with gzip, deflate, brotli or zstd.
The algorithm is chosen from the `Accept-Encoding` request header (highest q-value first,
then zstd > br > gzip > deflate).

## Usage

```rust
let mut app = builder.build();
app.use_compression(); // register before middlewares that produce the body
```

The first registered middleware is the outermost one, so register `use_compression()`
before `use_static_files()` and other middlewares whose body you want to compress.

## Configuration

```toml
[compression]
enable_gzip = true
enable_deflate = true
enable_brotli = true
enable_zstd = true
min_size = 32
level = "default" # fastest | default | best
```

```rust
use asp_dot_rust::configuration::CompressionConfiguration;

builder.configuration.configure::<CompressionConfiguration>("compression");
```

The middleware captures its settings when the pipeline is built. Missing TOML fields use the defaults below. An unregistered configuration or a registered entry with no value also uses those defaults.

| Field | Default | Effect |
| ----- | ------- | ------ |
| `enable_gzip`, `enable_deflate`, `enable_brotli`, `enable_zstd` | `true` | Turn one algorithm on or off. |
| `min_size` | `32` | Bodies smaller than this many bytes are not compressed. |
| `level` | `default` | `fastest`, `default` or `best`, applied to every algorithm. |

## Behavior

- Compression skips a response whose headers have already been sent, an empty body, a body below `min_size`, and responses with status 1xx, 204, 304 or 206.
- It also skips responses with `Content-Encoding` or `Content-Range`, or whose first `Cache-Control` value contains `no-transform` (case-insensitive substring match).
- Excluded content types use case-insensitive prefix matching for `image/` (except `image/svg+xml`), `audio/`, `video/`, `text/event-stream` and `application/grpc`.
- On a compressed response: `Content-Encoding` is set, `Content-Length` is replaced,
  `Accept-Ranges` is removed.
- Eligible responses get `Vary: Accept-Encoding` before an encoding is chosen, unless an existing `Vary` value already contains `accept-encoding` (case-insensitive substring match).
- If the compressed body is not smaller, the original body is sent.
- `deflate` uses the zlib format, as required by HTTP.

## Known limitations

- The whole body is compressed in memory, there is no streaming compression.
- Compression runs on the request task. For large dynamic bodies use `level = "fastest"`
  or disable brotli, because the default brotli level is slow.
- Unlike `tower-http` 0.7, a request that rejects every encoding gets the uncompressed
  body instead of `406 Not Acceptable`.
