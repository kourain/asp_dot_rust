# Model binding

Action methods may take binder parameters. The `#[controller_route]` macro resolves them before the action runs. The parameter name is the key used for path and query lookup.

## Binders

| Binder | Source | Missing / parse fail |
| --- | --- | --- |
| `FromRoute<T>` | `path_params` | HTTP 404 |
| `FromQuery<T>` | `query_params` | HTTP 422 |
| `FromOptionQuery<T>` | `query_params` | `None` (no error) |
| `FromBody<T>` | request body | HTTP 422 (or 400 if body cannot be read) |
| `FromForm<T>` | form body | HTTP 422 (or 400 if body cannot be read) |

Supported scalar `T` for route/query: `String`, integers, floats, `bool`, `char` (`FromParamValue`).

A route may have at most one of `FromBody` or `FromForm`.

## FromBody / FromForm

| `T` | Behaviour |
| --- | --- |
| `String` | Body is read as UTF-8 text (no serde). |
| other (`Deserialize`) | JSON or XML by `Content-Type` (`FromBody`); form-urlencoded (`FromForm`). |

`FromBody` media types: `application/json`, `text/json`, `application/xml`, `text/xml` (missing header defaults to JSON). Mismatch → 422.

`FromForm` media type: `application/x-www-form-urlencoded`. Mismatch → 422.

## Example

```rust
use asp_dot_rust::prelude::*;
use serde::Deserialize;

#[derive(Deserialize)]
struct CreateUser {
    name: String,
    age: u32,
}

#[controller_route("users")]
#[controller_inject]
impl UsersController {
    fn new(http_context: HttpContextRef) -> Self {
        Self { http_context }
    }

    #[get("/{id}")]
    pub async fn show(
        &mut self,
        id: FromRoute<u32>,
        verbose: FromOptionQuery<bool>,
    ) -> impl ActionResult {
        format!("user {id}, verbose={:?}", *verbose)
    }

    #[get("/search")]
    pub async fn search(&mut self, q: FromQuery<String>) -> impl ActionResult {
        format!("q={}", *q)
    }

    #[post("/")]
    pub async fn create(&mut self, body: FromBody<CreateUser>) -> impl ActionResult {
        format!("created {}", body.name)
    }

    #[post("/raw")]
    pub async fn raw(&mut self, body: FromBody<String>) -> impl ActionResult {
        body.into_inner()
    }
}
```

## Errors

`BindError` carries the message written as `text/plain` body. Status codes: route → 404, query/body/form → 422, body read failure → 400.

## Body buffering (multiple reads)

`hyper` delivers the body as a one-shot stream. ASP.RS can buffer it in memory, like ASP.NET `HttpRequest.EnableBuffering`:

```rust
self.http_context.request.enable_buffering().await.unwrap();
// first read (FromBody / read_body_bytes)
self.http_context.request.seek_body(0).unwrap();
// second read of the same bytes
```

- `enable_buffering` loads the stream into memory (no-op if already buffered).
- `seek_body(0)` resets the read position.
- `FromBody` / `FromForm` call `read_body_bytes`, which buffers on first use; seek to `0` before binding again if the same request must be parsed twice.
- `request.body` is `RequestBody` (`Stream` or `Buffered`), not raw `Limited<Incoming>`.
