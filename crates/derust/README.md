# derust

Easy way to start your Rust asynchronous application server using [Tokio](https://tokio.rs/)
and [Axum](https://github.com/tokio-rs/axum) frameworks.

[![MIT licensed][mit-badge]][mit-url]

[mit-badge]: https://img.shields.io/badge/license-MIT-blue.svg

[mit-url]: https://github.com/deroldo/derust/blob/main/LICENSE

## [Basic usage example](../../examples/basic)

```toml
# Cargo.toml

[package]
name = "sample"
version = "0.1.0"
edition = "2021"

[dependencies]
derust = { version = "0.1.1" }

tokio = { version = "1.42.0", features = ["full"] }
axum = { version = "0.7.9", default-features = true, features = ["macros", "tokio"] }
serde_json = { version = "1.0.133" }
```

```rust
// main.rs

use axum::extract::State;
use axum::http::StatusCode;
use axum::routing::get;
use axum::Router;
use derust::envx::Environment;
use derust::httpx::json::JsonResponse;
use derust::httpx::{start, AppContext, HttpError, HttpTags};
use derust::tracex;
use serde_json::json;

#[derive(Clone)]
pub struct AppState {
    pub bar: String,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // automatic log subscribe and add b3 traceparent
    let _guard = tracex::init();
    
    let env = Environment::detect().ok().unwrap_or(Environment::Local);
    
    // any cloneable struct
    let app_state = AppState {
      bar: "bar".to_string(),
    };
    
    let application_name = "sample";
    
    // easy way to get application context things, like your application state struct
    let context = AppContext::new(application_name, env, app_state)?;
    
    let port = 3000;
    let router = Router::new().nest("/foo", Router::new().route("/", get(handler)));
    
    // automatic health-check route
    // automatic route response status code log
    start(port, context, router).await
}

async fn handler(
    State(context): State<AppContext<AppState>>,
    // automatic add tags into log
) -> Result<JsonResponse, HttpError> {
    let customer_id = "1";
    
    // tags to be added into log
    let tags = HttpTags::from([("customer_id", customer_id)]);
    
    // getting your application state from context
    let bar = context.state().bar.clone();
    
    Ok(JsonResponse::new(
      StatusCode::OK,
      json!({ "foo": bar }),
      tags,
    ))
}
```

## Features

- [aws](https://github.com/deroldo/derust/tree/main/crates/derust/src/awsx)
- database
  - [postgres](src/databasex/postgresx/README.md)
- [env](https://github.com/deroldo/derust/tree/main/crates/derust/src/envx) (default)
- [growthbook](https://github.com/deroldo/derust/tree/main/crates/derust/src/growthbookx)
- [http_client](https://github.com/deroldo/derust/tree/main/crates/derust/src/http_clientx)
- metrics
  - [prometheus](https://github.com/deroldo/derust/tree/main/crates/derust/src/metricx/registries/prometheus)
  - [statsd](https://github.com/deroldo/derust/tree/main/crates/derust/src/metricx/registries/statsd)
- [outbox](https://github.com/deroldo/derust/tree/main/crates/derust/src/outboxx)
- [trace](https://github.com/deroldo/derust/tree/main/crates/derust/src/tracex) (default)
- SDUI - Server-Driven UI
  - flutter
    - [mirai](https://github.com/deroldo/derust/tree/main/crates/derust/src/sduix/flutter/mirai)

## License
This project is licensed under the MIT license.
