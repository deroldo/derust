# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Commands

This project uses [Task](https://taskfile.dev/) for common operations:

```bash
task build          # cargo build
task build:release  # cargo build --release
task test           # cargo nextest run
task test:watch     # cargo watch -x "nextest run"
task lint           # cargo fmt --all -- --check && cargo clippy -- -D warnings
task lint:fix       # cargo fmt --all && cargo clippy
task setup          # install cargo-nextest and cargo-watch
```

Run a single test:
```bash
cargo nextest run <test_name>
cargo nextest run --test <integration_test_file>
```

Rust toolchain: `1.96` (pinned via `rust-toolchain`).

## Architecture

`derust` is a Rust library crate (published to crates.io) that provides an opinionated bootstrap layer on top of Tokio + Axum. The workspace has one library crate (`crates/derust`) and several standalone examples (`examples/`).

### Feature flags drive everything

The crate is entirely feature-gated. Enabling a feature pulls in the relevant dependencies and exposes the corresponding module. The default feature is `http_server`, which also enables `env` and `tracex`.

| Feature | Module | Purpose |
|---|---|---|
| `env` (default via `http_server`) | `envx` | Config loading from env vars / `.env` files |
| `http_server` (default) | `httpx`, `tracex` | Axum server bootstrap, middleware stack, tracing |
| `http_client` | `http_clientx` | Reqwest client with OpenTelemetry tracing |
| `postgres` | `databasex` | SQLx Postgres pool + transaction helpers |
| `outbox` | `outboxx` | Outbox pattern processor (requires `postgres`) |
| `aws` | `awsx` | SQS, SNS, Secrets Manager clients |
| `env_from_secrets_manager` | `awsx` + `envx` | Load config from AWS Secrets Manager |
| `statsd` | `metricx` | StatsD metrics with auto HTTP/DB instrumentation |
| `prometheus` | `metricx` | Prometheus metrics with auto HTTP/DB instrumentation |
| `growthbook` | `growthbookx` | GrowthBook feature flag client |
| `start_test` | — | Exposes `start_test()` for integration tests |

JWT authentication support is always available as part of `http_server` (default). `protect-endpoints-core` is re-exported as `derust::httpx::protect_endpoints_core`. Applications also need `protect-axum` and `jsonwebtoken` as direct dependencies — see README for usage.

### Core abstractions

**`AppContext<S>`** (`httpx/context.rs`) — the central state object passed as Axum `State`. Holds: app name, `Environment`, optionally `PostgresDatabase`, metric config, `GrowthBookClient`, and the user-defined state struct `S`. Constructed with `AppContext::new(...)` then passed to `start()`.

**`start()` / `start_only_api()`** (`httpx/server.rs`) — entry point that applies all middleware and starts the Axum server. With the `outbox` feature, `start()` also spawns the outbox processor.

**`AppContext::new()` signature changes with features** — the constructor parameters vary at compile time based on enabled features (e.g., `database`, `statsd_config`, `prometheus_config`, `growth_book` are each conditionally present).

### Config loading (`envx`)

`load_app_config::<T>(env, prefix)` deserialises env vars into any `serde::Deserialize` struct. Nested structs use `__` as separator (e.g., `APP__FOO__BAR` maps to `app_config.foo.bar`). Loads `.env.local` / `.env.test` automatically in local/test environments.

With `env_from_secrets_manager`, secrets are fetched from AWS Secrets Manager and merged into the config map before deserialisation.

### HTTP layer (`httpx`)

- **`HttpTags`** — key-value pairs threaded through handlers and logged alongside every response.
- **`JsonResponse`** / **`TextResponse`** — typed response wrappers returned from handlers.
- **`HttpError`** — error type that implements `IntoResponse`; carries status code, message, optional body, headers, and tags.
- Built-in middleware: request timeout (env `SERVER_TIMEOUT_IN_MILLIS`, default 10 000 ms), tracing/OpenTelemetry (B3 traceparent), compression (gzip), sensitive header redaction, panic catching, and automatic response-status logging.
- Automatic routes: `GET /health` (always), `GET /metrics` (prometheus feature only).

### Database (`databasex`)

`PostgresDatabase` wraps a SQLx `PgPool`. Repository traits (`Repository`, `PgConnectionRepository`, `PgTransactionRepository`, `PgOptionTransactionRepository`) provide typed `fetch_one`, `fetch_optional`, `fetch_all`, `count`, `exists` methods and auto-record duration metrics when a metric feature is enabled.

### Metrics (`metricx`)

Both StatsD and Prometheus share the same meter API: `increment`, `increment_one`, `current_gauge`, `record_money`, `record_duration`, `start_stopwatch`. Auto-instruments HTTP requests (`http_server_seconds`), HTTP client calls (`http_client_seconds`), and DB queries (`repository_transaction_seconds`, `repository_query_seconds`). High-cardinality tags are filtered via `denied_metric_tags` / `denied_metric_tags_by_regex`.

### Testing

Enable `start_test` feature and use `start_test(context, router, listener)` with a `TcpListener::bind("127.0.0.1:0")` to spin up a real server on a random port for integration tests.
