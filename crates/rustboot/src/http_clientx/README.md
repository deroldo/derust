# rustboot - http_client

[Example](https://github.com/deroldo/rustboot/tree/main/examples/http_client)

```toml
# Cargo.toml

[package]
# ...

[dependencies]
rustboot = { version = "0.1.0", features = ["http_client"] } 

# ...
```

```rust
// main.rs

// ...
use rustboot::http_clientx::HttpClient;
// ...

#[derive(Clone)]
pub struct AppState {
    // ...
    pub gateway: HttpClient,
    // ...
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // ...

    let gateway = HttpClient::new("rustboot-http-client", "https://any-base-path.com", 1000, 100).await?;

    // any cloneable struct
    let app_state = AppState {
        // ...
        gateway,
        // ...
    };
    
    // start as the basic 
    // ... 
}

#[derive(Deserialize)]
pub struct GatewayResponseDto {
    value: String,
}

async fn handler(
    State(context): State<AppContext<AppState>>,
    // automatic add tags into log
) -> Result<JsonResponse, HttpError> {
    // ...

    let bar: GatewayResponseDto = context.state().gateway.get(&context, "/bar", None, None, &tags).await?;
    // ...
}
```