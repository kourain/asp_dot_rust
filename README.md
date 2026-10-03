# ASP.Rust

Hard code a minimal framework, style base on ASP.NET (C#)

## Quick Start

### 1. Create a application

## MVC

### 1. Controller

Minimal Controller with routing, matched by a `matchit`-based radix tree
router with dynamic path parameters (`/{id}`) and per-method dispatch.

See [`docs/routing.md`](docs/routing.md) for the full guide, including path
parameters, query string parsing, and `404`/`405` resolution.

### 2. Model

Not available yet

### 3. View

Not available yet

## Dependency Injection

ASP.Rust includes a lightweight, `TypeId`-based dependency injection container inspired by ASP.NET Core's `IServiceCollection`.

Three lifetimes are supported — `add_singleton`,`add_scope`, and `add_transient` — and circular dependencies between services are detected automatically when the application is built, in every build profile (will be optional in future).

See [`docs/dependency_injection.md`](docs/dependency_injection.md) for the full guide, including configuration injection, controller injection, and known limitations.
