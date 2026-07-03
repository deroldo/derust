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
derust = { version = "<last-version>" }

tokio = { version = "1.43.0", features = ["full"] }
axum = { version = "0.8.1", default-features = true, features = ["macros", "tokio"] }
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

## Envs

| env                      | default | description                                                                                                                          |
|--------------------------|---------|--------------------------------------------------------------------------------------------------------------------------------------|
| SERVER_TIMEOUT_IN_MILLIS | 10000   | Maximum time in milliseconds that the server will try to respond to a request before returning a timeout error (408 Request Timeout) |

## Tests

Active `start_test` feature
```toml
derust = { version = "<last-version>", features = ["start_test"] }
```

And then:

```rust
let env = Environment::detect().ok().unwrap_or(Environment::Local);

// any cloneable struct
let app_state = AppState {
    bar: "bar".to_string(),
};

let application_name = "sample";

// easy way to get application context things, like your application state struct
let context = AppContext::new(application_name, env, app_state)?;

let router = Router::new().nest("/foo", Router::new().route("/", get(handler)));

let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();

start_test(context, router, listener).await
```

## JWT Authentication

JWT protection for routes is available out of the box via [`protect-endpoints-core`](https://crates.io/crates/protect-endpoints-core) (re-exported as `derust::httpx::protect_endpoints_core`) and [`protect-axum`](https://crates.io/crates/protect-axum).

```toml
# Cargo.toml
[dependencies]
derust = { version = "<last-version>" }
protect-axum = { version = "0.2.0" }
jsonwebtoken = { version = "10.4.0", features = ["rust_crypto"] }
```

```rust
// Define your claims struct and implement AuthoritiesClaims to expose roles
use derust::httpx::protect_endpoints_core::AuthoritiesClaims;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone)]
pub struct AccessClaims {
    pub sub: uuid::Uuid,
    pub exp: i64,
    // ...any other fields
}

impl AuthoritiesClaims for AccessClaims {
    fn roles(&self) -> Vec<String> {
        vec!["USER".to_string()]
    }
}
```

```rust
// Configure protected routes in your router
use derust::httpx::protect_endpoints_core::{Algorithm, AuthoritiesExtractor, DecodingKey, Validation};
use protect_axum::GrantsLayer;

let mut validation = Validation::new(Algorithm::ES256);
validation.set_audience(&["my-service"]);

let protected = Router::new()
    .route("/me", get(me_handler))
    // Layer 2: role-based access control (optional)
    .layer(GrantsLayer::with_extractor(
        AuthoritiesExtractor::<AccessClaims>::grants_extractor,
    ))
    // Layer 1: JWT validation and claims extraction
    .layer(AuthoritiesExtractor::<AccessClaims>::new(decoding_key, validation));
```

### Multiple keys (`kid`-based selection)

To validate tokens signed by different keys, build a `JwtKeystore` mapping each JWT `kid` header to its decoding key. The key is selected by the token's `kid` before signature validation — keys are never tried in sequence:

```rust
use derust::httpx::protect_endpoints_core::{AuthoritiesExtractor, DecodingKey, JwtKeystore};

let keystore = JwtKeystore::new([
    ("key-2024".to_string(), DecodingKey::from_ec_pem(old_pem)?),
    ("key-2025".to_string(), DecodingKey::from_ec_pem(new_pem)?),
]);

let protected = Router::new()
    .route("/me", get(me_handler))
    .layer(AuthoritiesExtractor::<AccessClaims>::with_keystore(keystore, validation));
```

With a multi-key keystore, a token without a `kid` header or with an unknown `kid` is rejected. The single-key constructor `AuthoritiesExtractor::new` is equivalent to `JwtKeystore::single(key)`: the key is the explicit default and validates every token, with or without `kid`.

For migrations where already-issued tokens carry no `kid` header, `JwtKeystore::with_fallback(keys, fallback)` keeps the `kid`-based selection but validates tokens **without** a `kid` against the fallback key. Tokens with an unknown `kid` are still rejected — the fallback only applies to the missing-`kid` case:

```rust
let keystore = JwtKeystore::with_fallback(
    [("key-2025".to_string(), DecodingKey::from_ec_pem(new_pem)?)],
    DecodingKey::from_ec_pem(legacy_pem)?, // validates legacy tokens without kid
);
```

Authentication failures are distinguished for debugging — missing `kid`, unknown `kid`, and invalid signature/expired token are logged as distinct `JwtAuthError` variants (and inserted into the request extensions) — but all of them result in `401 Unauthorized` at the HTTP layer.

Keys can also be declared through configuration (env vars or AWS Secrets Manager, via `load_app_config`). The map key is a free label; the token `kid` is matched against the `kid` field:

```bash
APP__JWT__KEYS__K2024__KID=key-2024
APP__JWT__KEYS__K2024__FORMAT=ec_pem      # secret | base64_secret | rsa_pem | ec_pem | ed_pem
APP__JWT__KEYS__K2024__KEY="-----BEGIN PUBLIC KEY-----..."
APP__JWT__KEYS__K2025__KID=key-2025
APP__JWT__KEYS__K2025__FORMAT=ec_pem
APP__JWT__KEYS__K2025__KEY="-----BEGIN PUBLIC KEY-----..."
APP__JWT__KEYS__K2024__FALLBACK=true      # optional: at most one key; also validates tokens without kid
```

```rust
use derust::httpx::protect_endpoints_core::{JwtKeystore, JwtKeystoreConfig};
use serde::Deserialize;

#[derive(Deserialize)]
struct AppConfig {
    jwt: JwtKeystoreConfig,
}

let config: AppConfig = load_app_config(environment, Some("APP")).await?;
let keystore = JwtKeystore::from_config(&config.jwt)?;
```

Multiple keys enable **zero-downtime key rotation**: start signing new tokens with a new `kid` while keeping the old key in the keystore, wait until all tokens signed with the old key have expired, then remove the old entry.

```rust
// Access claims in your handler via axum Extension
use axum::Extension;

async fn me_handler(
    State(context): State<AppContext<AppState>>,
    Extension(claims): Extension<AccessClaims>,
) -> Result<JsonResponse<ResponseBody>, HttpError> {
    let user_id = claims.sub;
    // ...
}
```

```rust
// Restrict a handler to a specific role using the protect macro
use protect_axum::GrantsLayer;

#[protect_axum::protect(any("ADMIN"))]
async fn admin_handler(
    State(context): State<AppContext<AppState>>,
    Extension(claims): Extension<AccessClaims>,
) -> Result<JsonResponse<ResponseBody>, HttpError> {
    // only reachable when the token carries the "ADMIN" role
    // ...
}
```

The `AuthoritiesExtractor` layer reads the `Authorization: Bearer <token>` header, selects the decoding key (by `kid` when using a keystore), validates the JWT signature and expiration, and inserts the deserialized claims into the request as an Axum `Extension<T>`. If validation fails, it returns `401 Unauthorized`. The `GrantsLayer` attaches the roles returned by `AuthoritiesClaims::roles()` so that `#[protect_axum::protect(any(...))]` can enforce them per handler.

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

## License
This project is licensed under the MIT license.
