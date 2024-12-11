# rustboot - trace

[Example](../../../../examples/trace)

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

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // automatic log subscribe and add b3 traceparent
    let _guard = tracex::init();
    
    // start as the basic 
    // ... 
}

async fn handler(
    State(context): State<AppContext<AppState>>,
    // automatic add tags into log
) -> Result<JsonResponse, HttpError> {
    // ...
    
    // tags to be added into log
    let tags = HttpTags::from([("customer_id", customer_id)]);
    
    // automatic add tags into log
    info("Request handler", &tags);
    
    // ...
}
```