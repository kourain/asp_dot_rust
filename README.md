# ASP.Rust

Hard code a minimal framework, style base on ASP.NET (C#)

## Quick Start

### 1. Create a application

## MVC

### 1. Controller

Minimal Controller with routing

### 2. Model

Not available yet

### 3. View

Not available yet

## Dependency Injection

ASP.Rust includes a lightweight, `TypeId`-based dependency injection container inspired by ASP.NET Core's `IServiceCollection`.

Three lifetimes are supported — `add_singleton`,`add_scope`, and `add_transient` — and circular dependencies between services are detected automatically when the application is built, in every build profile (will be optional in future).

See [`docs/dependency_injection.md`](docs/dependency_injection.md) for the full guide, including configuration injection, controller injection, and known limitations.
