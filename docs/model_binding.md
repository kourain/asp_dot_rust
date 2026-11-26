# Model binding

Action methods may take binder parameters. The `#[controller_route]` macro resolves them before the action runs. The parameter name is the key used for path and query lookup.

## Binders

| Binder | Source | Missing / parse fail |
| --- | --- | --- |
| `FromRoute<T>` | `path_params` | HTTP 404 |
| `FromQuery<T>` | `query_params` | HTTP 422 |
| `FromOptionQuery<T>` | `query_params` | Alias of `FromQuery<Option<T>>`. Missing gives `None`. A supplied value that does not parse is HTTP 422, with or without `#[validate(...)]`. |
| `FromBody<T>` | one field of the JSON / XML body | HTTP 422 for a missing or invalid field, 400 for read failures. See the `QUERY` exceptions below. |
| `FromOptionBody<T>` | one field of the JSON / XML body | Alias of `FromBody<Option<T>>`. Missing or `null` gives `None`. |
| `FromForm<T>` | one field of the form body | Same as `FromBody<T>`. |
| `FromOptionForm<T>` | one field of the form body | Alias of `FromForm<Option<T>>`. Missing gives `None`. A supplied value that does not parse is HTTP 422. |
| `Body<T>` | the whole body | HTTP 422 for invalid data, 400 for read failures. |
| `Form<T>` | the whole form body | Same as `Body<T>`. |

`FromRoute`, `FromQuery`, `FromForm`, `FromBody` and their `FromOption...` aliases bind **one value** and the argument name is its key. `Body` and `Form` bind the **whole** body into a DTO (or raw text).

### Optional values

`FromOptionQuery<T>`, `FromOptionForm<T>` and `FromOptionBody<T>` are type aliases of `FromQuery<Option<T>>`, `FromForm<Option<T>>` and `FromBody<Option<T>>`, so the two spellings are the same type and the macro treats them the same. Because they are aliases, they cannot be used as a constructor or a pattern (write `FromQuery(Some(1))`).

A route parameter is always present, so **`FromRoute<Option<T>>` is not allowed** and the macro reports it at compile time.

An optional value that is there must still parse: `?limit=abc` is HTTP 422 for `FromOptionQuery<u32>`, it never becomes `None`. For `FromOptionBody<T>` a `null` is `None`.

Supported `T` for route/query: every type that implements `std::str::FromStr` (`FromParamValue` is implemented for all of them): `String`, integers, floats, `bool`, `char`, `uuid::Uuid`, `ulid::Ulid`, `std::net::IpAddr` and your own types. A type that cannot implement `FromStr` can implement `FromParamValue` directly (but not both).

### Value syntax

- **Path values** are percent-decoded after the route matched, so `/users/a%20b` binds `a b` and an encoded `%2F` stays inside its segment. A value that is not valid UTF-8 after decoding is HTTP 404 for `FromRoute`.
- **Query keys and values** are decoded as form data: `+` is a space, `%2B` is a plus. A value that is not valid UTF-8 after decoding is HTTP 422 for both query binders. It is never replaced by a placeholder character or turned into `None`. A key that is not valid UTF-8 cannot match a parameter name and is ignored.
- **A key without `=` has an empty value** (`?debug` and `?debug=` are the same). It can be used as a flag: `bool` reads an empty value as `true`, so `FromOptionQuery<bool>` is `Some(true)` for `?debug` and `None` when the key is absent, and `.flag()` gives `true` / `false` (`FromOptionQuery<bool>` and `FromOptionForm<bool>` have it). `String` accepts `""`, every other type rejects it with HTTP 422.

> **Warning: a repeated query key keeps only its last value.** `?id=1&id=2` binds `2` and drops `1` without an error, for `FromQuery<T>`, `FromForm<T>` and their `FromOption...` aliases alike. To collect all the values or to reject a repeated key, read `http_context.request.uri.query()` in the action.

An action may have any number of body binders, but they must read the same kind of body: form binders (`FromForm`, `Form`) cannot be mixed with body binders (`FromBody`, `Body`), the macro reports that at compile time.

## Query validation

Place a rule on a query parameter in the action signature. Both `FromQuery<T>` and `FromOptionQuery<T>` (that is `FromQuery<Option<T>>`) support the same rules. A rule is checked on the `T` inside the `Option`, and a missing optional parameter skips it.

```rust
#[get("/search")]
fn search(
    &self,
    #[validate(length(1, 80))] term: FromQuery<String>,
    #[validate(range(min = 1, max = 100))] limit: FromQuery<u8>,
    #[validate(range(min = -10, max = 10))] offset: FromOptionQuery<i8>,
    #[validate(range(min = 0.0, max = 1.0))] score: FromOptionQuery<f64>,
) -> impl ActionResult {
    results::ok(format!("{} {} {:?} {:?}", *term, *limit, *offset, *score))
}
```

`length(6)` requires exactly six Unicode scalar values in the decoded string. `length(1, 80)` accepts lengths from 1 through 80. Named bounds such as `length(max = 80)` and `length(min = 1, max = 80)` also work. `range` compares values in the parameter's numeric type and requires a finite floating-point value.

The controller macro calls `FromQuery::<T>::rule(value, field, arguments)` for each annotated parameter. To add a rule, define an associated function on the binder or implement a local extension trait for `FromQuery<T>` and bring it into the controller's scope. The function borrows the parsed value, receives the field name, and returns `Result<(), ValidationError>`. A single positional argument is passed directly, multiple positional arguments as a tuple, and named bounds as `controller::validation::QueryBounds`.

An absent optional field skips validation. A supplied field, including an empty value, must parse and satisfy its rule. Failures return HTTP 422 with the parameter name before the action runs.

## Body and form

### One field: `FromBody<T>` / `FromForm<T>`

```rust
#[post("/users")]
fn create(&mut self, name: FromBody<String>, age: FromBody<u32>, nick: FromBody<Option<String>>) -> impl ActionResult { /* ... */ }

#[post("/login")]
fn login(&mut self, user: FromForm<String>, remember: FromForm<bool>) -> impl ActionResult { /* ... */ }
```

| Binder | Reads | `T` |
| --- | --- | --- |
| `FromBody<T>` | member `name` of the top-level JSON object, or child element `name` of the XML root element | any serde `Deserialize` type |
| `FromForm<T>` | field `name` of `application/x-www-form-urlencoded` | any `FromParamValue` (every `FromStr` type) |

- `FromBody` accepts `application/json`, `text/json`, `application/xml` and `text/xml`, JSON is assumed when the header is missing. `FromForm` accepts `application/x-www-form-urlencoded` and also a missing or empty media type. Both ignore media type parameters and compare media types case-insensitively.
- A missing required field is HTTP 422. Both `FromForm<Option<U>>` and `FromBody<Option<U>>` bind a missing field as `None`. A JSON `null` also binds as `None` for an optional body field. A field of the wrong type, a body that is not an object, and a Content-Type mismatch are HTTP 422.
- Form values are decoded like query values: `+` is a space, `%2B` is a plus, a key without `=` is an empty value (`bool` reads it as `true`) and a value that is not valid UTF-8 is HTTP 422.
- `#[validate(...)]` is for query parameters only. To validate body fields use `Body<Dto>` / `Form<Dto>` with `Validate`.

> `FromForm` parses the last occurrence of a key. For a repeated scalar field, `FromBody` parses each occurrence and returns the last value. For example, `{"age":"x","age":2}` returns HTTP 422 for `FromBody<u32>`.

Each field binder searches the buffered body for its argument. For many fields prefer one DTO binder.

### The whole body: `Body<T>` / `Form<T>`

| `T` | Behaviour |
| --- | --- |
| `String` written as a single identifier | Reads raw UTF-8 text after checking the media type. |
| Other type spelling or a deserializable model | `Body` uses JSON or XML. `Form` uses form-urlencoded deserialization. |

For `Body<T>`, the serde path accepts `application/json`, `text/json`, `application/xml` and `text/xml`. A missing header defaults to JSON. The raw `Body<String>` path additionally accepts `text/plain` and assumes plain text when the header is absent. The macro selects raw text by the spelling `String`. A type alias or `std::string::String` selects serde for either binder.

`Form<T>` accepts `application/x-www-form-urlencoded` and also attempts binding when the header is missing or the media type is empty.

## DTO validation

Implement `Validate` on a DTO to run its rules in every controller action that accepts it through `Body<T>` or `Form<T>`. The router calls `validate()` after deserialization and before invoking the action. A DTO with only `Deserialize` proceeds directly to the action. Calling `bind_serde` directly performs deserialization alone.

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
    pub async fn create(&mut self, body: Body<CreateUser>) -> impl ActionResult {
        results::ok(format!("created {}", body.name))
    }

    #[post("/raw")]
    pub async fn raw(&mut self, body: Body<String>) -> impl ActionResult {
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
let first = self.http_context.request.body.read_to_end().await.unwrap();
self.http_context.request.seek_body(0).unwrap();
let second = self.http_context.request.body.read_to_end().await.unwrap();
assert_eq!(first, second);
```

- `enable_buffering` loads the stream into memory (no-op if already buffered).
- `seek_body(0)` resets the read position.
- The body binders (`FromBody`, `FromForm`, `Body`, `Form`) buffer the body on first use, read it from the start and rewind it to `0` afterwards. Several binders can read the same body and the action can still read it itself, no manual `seek_body(0)` is needed.
- `request.body` is `RequestBody` (`Stream` or `Buffered`), not raw `Limited<Incoming>`.
