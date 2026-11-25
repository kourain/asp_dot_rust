# Model binding

Action methods may take binder parameters. The `#[controller_route]` macro resolves them before the action runs. The parameter name is the key used for path and query lookup.

## Binders

| Binder | Source | Missing / parse fail |
| --- | --- | --- |
| `FromRoute<T>` | `path_params` | HTTP 404 |
| `FromQuery<T>` | `query_params` | HTTP 422 |
| `FromOptionQuery<T>` | `query_params` | `None` (no error) |
| `FromBody<T>` | request body | HTTP 422 for invalid data, 400 for read failures. See the `QUERY` exceptions below. |
| `FromForm<T>` | form body | Same as `FromBody<T>`. |

Supported scalar `T` for route/query: `String`, integers, floats, `bool`, `char` (`FromParamValue`).

A route may have at most one of `FromBody` or `FromForm`.

## FromBody / FromForm

| `T` | Behaviour |
| --- | --- |
| `String` written as a single identifier | Reads raw UTF-8 text after checking the media type. |
| Other type spelling or a deserializable model | `FromBody` uses JSON or XML. `FromForm` uses form-urlencoded deserialization. |

For `FromBody<T>`, the serde path accepts `application/json`, `text/json`, `application/xml` and `text/xml`. A missing header defaults to JSON. The raw `FromBody<String>` path additionally accepts `text/plain` and assumes plain text when the header is absent. The macro selects raw text by the spelling `String`. A type alias or `std::string::String` selects serde for either binder.

`FromForm<T>` accepts `application/x-www-form-urlencoded` and also attempts binding when the header is missing or the media type is empty. Both binders ignore media type parameters and compare media types case-insensitively.

## DTO validation

Implement `Validate` on a DTO to run its rules in every controller action that accepts it through `FromBody<T>` or `FromForm<T>`. The router calls `validate()` after deserialization and before invoking the action. A DTO with only `Deserialize` proceeds directly to the action. Calling `bind_serde` directly performs deserialization alone.

## Example

```rust
use asp_dot_rust::prelude::*;
use asp_dot_rust::third_party::async_trait;
use serde::Deserialize;

#[derive(Deserialize)]
struct CreateUser {
    name: String,
    age: u32,
}

impl Validate for CreateUser {
    fn validate(&self) -> Result<(), ValidationError> {
        if self.name.chars().count() > 80 {
            return Err(ValidationError::new("name", "maximum length is 80"));
        }
        Ok(())
    }
}

asp_dot_rust::api_controller!(UsersController {});

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
        results::ok(format!("user {}, verbose={:?}", *id, *verbose))
    }

    #[get("/search")]
    pub async fn search(&mut self, q: FromQuery<String>) -> impl ActionResult {
        results::ok(format!("q={}", *q))
    }

    #[post("/")]
    pub async fn create(&mut self, body: FromBody<CreateUser>) -> impl ActionResult {
        results::ok(format!("created {}", body.name))
    }

    #[post("/raw")]
    pub async fn raw(&mut self, body: FromBody<String>) -> impl ActionResult {
        results::ok(body.into_inner())
    }
}
```

## Errors

The router writes the `BindError` message as `text/plain`. Route parameter errors return 404, body read failures return 400, and invalid query, body or form values normally return 422. For a matched HTTP `QUERY` action, the router returns 400 before binding if `Content-Type` is missing or blank. An unsupported media type reported by a binder on `QUERY` returns 415.

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
