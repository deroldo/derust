# derust - outbox

## [Example](https://github.com/deroldo/derust/tree/main/examples/outbox)

```toml
# Cargo.toml

[package]
# ...

[dependencies]
derust = { version = "0.1.0", features = ["outbox"] }

# ...
```

```rust
// main.rs

// ...
use derust::databasex::{DatabaseConfig, PostgresDatabase, Repository};
use derust::outboxx;
use derust::outboxx::OutboxProcessorResources;
// ...

#[derive(Clone)]
pub struct AppState {
    pub app_config: AppConfig,
}

// required serde::Deserialize
#[derive(Clone, Deserialize)]
pub struct AppConfig {
    pub port: u16,
    pub database_config: DatabaseConfig,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // ...
    
    // app config loader
    let app_config: AppConfig = load_app_config(env, None).await?;
    // any cloneable struct
    let app_state = AppState {
        app_config,
    };

    // create postgres database pool
    let database = PostgresDatabase::create_from_config(&app_state.app_config.database_config).await?;

    // easy way to get application context things, like your application state struct and database
    let context = AppContext::new(application_name, env, database, app_state)?;

    // configuring outbox resources:
    // postgres pool is required
    // sns and sqs clients are optional
    let outbox_resources = OutboxProcessorResources::new(database.read_write.clone(), None, None);

    // automatic health-check route
    // automatic route response status code log
    // automatic start outbox-pattern-processor
    start(port, context, router, outbox_resources).await
}
```

```rust
// any async function 

async fn handler(
    State(context): State<AppContext<AppState>>,
) -> Result<JsonResponse<FooResponse>, HttpError> {
    // ...

    let tags = HttpTags::default();

    let mut trx = context.database().begin_transaction(&context, &tags).await?;

    // sending outbox event
    // you can also use `send_to_sqs` or `send_to_sns` functions
    outboxx::send_to_http(
        &context,
        &mut trx.transaction,
        Uuid::now_v7(),
        "https://any-base-path.com/foo",
        None,
        &json!({
            "outbox": true,
        }),
        &tags,
    )
        .await?;

    trx.commit_transaction(&tags).await?;
    
    // ...
}
```