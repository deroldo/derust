# rustboot - env

## [Basic](https://github.com/deroldo/rustboot/tree/main/examples/env/basic) 

```toml
# Cargo.toml

[package]
# ...

[dependencies]
rustboot = { version = "0.1.0" }

# ...
```

```rust
// main.rs

// ...
use rustboot::tracex;
use rustboot::tracex::log::info;
// ...

#[derive(Clone)]
pub struct AppState {
    pub app_config: AppConfig,
}

// required serde::Deserialize
#[derive(Clone, Deserialize)]
pub struct AppConfig {
    pub port: u16,
    // sub-structs must be separated by "__" in your environment property
    pub foo: AppFooConfig,
}

// required serde::Deserialize
#[derive(Clone, Deserialize)]
pub struct AppFooConfig {
    pub bar: String,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // ...

    // optional prefix
    let prefix = "APP";
    // app config loader
    let app_config: AppConfig = load_app_config(env, Some(prefix)).await?;
    // any cloneable struct
    let app_state = AppState {
        app_config,
    };

    // start as the basic 
    // ... 
}
```

## [AWS SecretsManager](https://github.com/deroldo/rustboot/tree/main/examples/env/secrets-manager)

```toml
# Cargo.toml

[package]
# ...

[dependencies]
rustboot = { version = "0.1.0", features = ["env_from_secrets_manager"] }

# ...
```

```rust
// main.rs

// ...
use rustboot::tracex;
use rustboot::tracex::log::info;
// ...

#[derive(Clone, Deserialize)]
pub struct AppConfig {
    pub port: u16,
    pub foo: AppFooConfig,
    pub jwt: AppJwtConfig,
}

// required serde::Deserialize
#[derive(Clone, Deserialize)]
pub struct AppJwtConfig {
    pub private_key: String,
}

// required serde::Deserialize
#[derive(Clone, Deserialize)]
pub struct AppFooConfig {
    pub bar: String,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // ...

    // optional prefix
    let prefix = "APP";
    // required when feature env_from_secrets_manager is enabled
    // should be the AWS secrets-manager id/name
    let secrets_manager_ids = vec!["localstack"];
    // app config loader
    let app_config: AppConfig = load_app_config(env, Some(prefix), secrets_manager_ids).await?;
    // any cloneable struct
    let app_state = AppState {
        app_config,
    };
    
    // start as the basic 
    // ... 
}
```