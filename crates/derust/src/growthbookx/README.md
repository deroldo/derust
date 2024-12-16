# derust - growthbook

## [Example](https://github.com/deroldo/derust/tree/main/examples/growthbook)

```toml
# Cargo.toml

[package]
# ...

[dependencies]
derust = { version = "0.1.1", feature = ["growthbook"] }

# ...
```

```rust
// main.rs

// ...
use derust::growthbookx;
use derust::growthbookx::{growth_book_attributes, GrowthBookConfig};
// ...

#[derive(Clone)]
pub struct AppState {
    pub bar_v1: String,
    pub bar_v2: String,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // ...

    // any cloneable struct
    let app_state = AppState {
        bar_v1: "bar".to_string(),
        bar_v2: "foobar".to_string(),
    };

    // required to access growthbook admin dashboard to create the sdk-key: http://localhost:3000
    let gb_config = GrowthBookConfig {
        growth_book_url: "http://localhost:3100".to_string(),
        sdk_key: "sdk-key".to_string(),
        update_interval: None,
        http_timeout: None,
    };
    let growthbook = growthbookx::initialize(&gb_config).await?;

    let application_name = "sample";

    // easy way to get application context things, like your application state struct
    let context = AppContext::new(application_name, env, growthbook, app_state)?;

    // start as the basic 
    // ... 
}
```

```rust
// any async function

async fn handler(
    State(context): State<AppContext<AppState>>,
) -> Result<JsonResponse<FooResponse>, HttpError> {
    let tags = HttpTags::default();

    let pair = rand::thread_rng().gen_range(0..100) % 2 == 0;

    // creating growhtbook attributes to match conditions
    let attrs = growth_book_attributes(json!({
        "pair": pair,
    }), &tags)?;

    // boolean condition
    // can you also get `feature_result` and parse to String or your struct type
    let bar = if context.growth_book().is_on("test", Some(attrs)) {
        context.state().bar_v1.clone()
    } else {
        context.state().bar_v2.clone()
    };

    // ...
}
```