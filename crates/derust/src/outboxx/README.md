# derust - outbox

## [Example](https://github.com/deroldo/derust/tree/main/examples/outbox)

```toml
# Cargo.toml

[package]
# ...

[dependencies]
derust = { version = "<last-version>", features = ["outbox"] }

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
) -> Result<JsonResponse<FooResponse>, DerustError> {
    // ...

    let tags = DerustTags::default();

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
## Trace id

SQS, SNS and HTTP sends add the current trace to the headers as `x-trace-id` (taken from the
`x-trace-id` in `tags` if present, otherwise from the current span). A header provided by the
caller (any case) is never overwritten, and nothing is added when there is no trace. For SQS/SNS
the header becomes a MessageAttribute; since the limit is 10 and the processor uses one for
`x-idempotent-key`, the trace is silently omitted when the caller already sends 9 or more headers
(8 if `x-idempotent-key` is not among them). For HTTP, destinations that reject unknown headers
may be affected.
