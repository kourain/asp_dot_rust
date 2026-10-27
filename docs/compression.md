# Compression

`CompressionMiddleware` compresses the response body with gzip, deflate, brotli or zstd.
The algorithm is chosen from the `Accept-Encoding` request header (highest q-value first,
then zstd > br > gzip > deflate). It follows the behavior of `CompressionLayer` in `tower-http`.

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
builder.configuration.configure::<CompressionConfiguration>("compression");
```

Missing fields use the values above. Without any configuration the defaults are used.

| Field | Default | Effect |
| ----- | ------- | ------ |
| `enable_gzip`, `enable_deflate`, `enable_brotli`, `enable_zstd` | `true` | Turn one algorithm on or off. |
| `min_size` | `32` | Bodies smaller than this many bytes are not compressed. |
| `level` | `default` | `fastest`, `default` or `best`, applied to every algorithm. |

## Behavior

- Skipped: empty body, `204`, `304`, `206`, existing `Content-Encoding` or `Content-Range`,
  `Cache-Control: no-transform`, and content types `image/*` (except `image/svg+xml`),
  `audio/*`, `video/*`, `text/event-stream`, `application/grpc`.
- On a compressed response: `Content-Encoding` is set, `Content-Length` is replaced,
  `Accept-Ranges` is removed.
- `Vary: Accept-Encoding` is added to every response that could be compressed.
- If the compressed body is not smaller, the original body is sent.
- `deflate` uses the zlib format, as required by HTTP.

## Known limitations

- The whole body is compressed in memory, there is no streaming compression.
- Compression runs on the request task. For large dynamic bodies use `level = "fastest"`
  or disable brotli, because the default brotli level is slow.
- Unlike `tower-http` 0.7, a request that rejects every encoding gets the uncompressed
  body instead of `406 Not Acceptable`.
