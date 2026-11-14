# Swagger / OpenAPI

ASP.Rust builds an OpenAPI 3.0 document from every controller action registered by `add_controllers()` and serves Swagger UI without extra annotations.

## Enable

```rust
use asp_dot_rust::ApplicationBuilder;

let mut app = ApplicationBuilder::new("MyApp");
app.add_controllers()
    .use_swagger();
// then build and run as usual
```

Call `use_swagger()` after `add_controllers()` so the route list is available.

## Endpoints

| Path | Response |
| ---- | -------- |
| `GET /swagger` | Swagger UI (HTML, loads UI from CDN) |
| `GET /swagger/v1/swagger.json` | OpenAPI 3.0 JSON document |

## Document content

For each registered route the generator emits:

- path (matchit style, e.g. `/home/{id}`)
- HTTP method
- `tags`: controller name
- `operationId`: `{Controller}_{action}`
- path parameters from `{name}` segments (type string, required)
- response `200` with description `OK`

Request body, query parameters and response schemas are not inferred yet; extend the document or replace it later if needed.

## Dependencies

- `serde_json` for JSON serialization
- Swagger UI assets are loaded from `unpkg.com` (no local static files required)
