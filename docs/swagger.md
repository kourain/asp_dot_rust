# Swagger / OpenAPI

ASP.Rust builds an OpenAPI 3.2 document from controller actions registered by `add_controllers()` and serves Swagger UI without extra annotations.

## Enable

```rust
use asp_dot_rust::ApplicationBuilder;

let mut builder = ApplicationBuilder::new("MyApp");
builder.add_controllers();
let mut app = builder.build();
app.use_swagger();
```

Call `use_swagger()` after `add_controllers()` so the route list is available.

## Endpoints

| Path | Response |
| ---- | -------- |
| `/swagger`, `/swagger/`, `/swagger/index.html` | Swagger UI HTML with assets from the CDN |
| `/swagger/v1/swagger.json`, `/swagger/v1/swagger.json/` | OpenAPI 3.2 JSON document |

Use GET to retrieve the UI or document. The middleware dispatches on the path alone, so other HTTP methods also reach these handlers.

## Document content

For each registered route the generator emits:

- path (matchit style, e.g. `/home/{id}`)
- HTTP method
- `tags`: controller name
- `operationId` uses `{Controller}_{action}` and adds the method and path when needed for uniqueness
- path parameters for ordinary `{name}` segments (type string, required). The generator scans for brace pairs. Escaped literal braces such as `{{id}}` can therefore produce a spurious parameter, and `{*rest}` yields the parameter name `*rest`.
- response `200` with description `OK`

Request body, query parameters and response schemas are not inferred yet; extend the document or replace it later if needed.

## Dependencies

- `serde_json` for JSON serialization
- Swagger UI assets are loaded from `unpkg.com` (no local static files required)
