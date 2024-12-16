# derust - postgres

## [Example](https://github.com/deroldo/derust/tree/main/examples/database/postgres)

```toml
# Cargo.toml

[package]
# ...

[dependencies]
derust = { version = "0.1.1", features = ["postgres"] }

# ...
```

```rust
// main.rs

// ...
use derust::databasex::{DatabaseConfig, PostgresDatabase, Repository};
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

    // get connection
    let mut _conn = context.database().get_connection(false, &tags).await?;

    // begin transaction
    let mut trx = context.database().begin_transaction(&context, &tags).await?;

    // available queries for PoolConnection or PostgresTransaction:
    // - fetch_one
    // - fetch_optional
    // - fetch_all
    // - count
    // - exists
    // fetch functions can be used to insert or update commands
    
    let query = query_as("select * from foo");

    let foos: Vec<Foo> = trx.fetch_all(&context, "foo_count", query, &tags).await?;

    // commit transaction
    trx.commit_transaction(&tags).await?;
    
    // ...
}
```