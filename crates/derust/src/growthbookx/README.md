# derust - growthbook

Este módulo re-exporta diretamente a API pública da crate oficial
[`growthbook-rust`](https://crates.io/crates/growthbook-rust) (versão fixada em
`crates/derust/Cargo.toml`). O derust não adiciona nenhuma camada própria de
configuração, cliente ou tratamento de erro sobre o GrowthBook — use os tipos e
funções nativos da SDK diretamente através de `derust::growthbookx`.

## [Example](https://github.com/deroldo/derust/tree/main/examples/growthbook)

```toml
# Cargo.toml

[package]
# ...

[dependencies]
derust = { version = "<last-version>", features = ["growthbook"] }

# ...
```

```rust
// main.rs

// ...
use derust::growthbookx::{GrowthBookAttribute, GrowthBookClient, GrowthBookClientTrait};
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
    // API 100% nativa da growthbook-rust (SDK oficial): sem casca própria do derust.
    let growthbook = GrowthBookClient::new(
        "http://localhost:3100",
        "sdk-key",
        None, // update_interval
        None, // http_timeout
    )
    .await
    .map_err(|error| Box::new(error) as Box<dyn std::error::Error>)?;

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

    // creating growthbook attributes directly from the SDK's native API.
    // note: `GrowthBookAttribute::from` returns the SDK's own `GrowthbookError`, not
    // derust's `HttpError` — convert it explicitly where needed, as shown below.
    let attrs = GrowthBookAttribute::from(json!({
        "pair": pair,
    }))
    .map_err(|error| {
        HttpError::without_body(
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to parse growth book attributes: {error}"),
            tags.clone(),
        )
    })?;

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

## Breaking change (v0.5.0)

A partir da versão `0.5.0`, `derust::growthbookx` não expõe mais `GrowthBookConfig`,
`growthbookx::initialize()` nem `growth_book_attributes()`. Use diretamente:

- `GrowthBookClient::new(api_url, sdk_key, update_interval, http_timeout)` no lugar de
  `growthbookx::initialize(&GrowthBookConfig { .. })`.
- `GrowthBookAttribute::from(value)` no lugar de `growth_book_attributes(value, &tags)`
  — note que o tipo de erro retornado agora é `growthbook_rust::error::GrowthbookError`
  (também re-exportado por `derust::growthbookx::GrowthbookError`), não mais
  `derust::httpx::HttpError`. Se você usava esse erro diretamente em um handler HTTP,
  converta-o manualmente para `HttpError` (veja o exemplo acima).
- `derust::httpx::GrowthBookClientTrait` também deixou de existir — importe de
  `derust::growthbookx::GrowthBookClientTrait`.

Ainda dentro da `0.5.0`, a dependência interna troca da crate não-oficial
[`growthbook-rust-sdk`](https://crates.io/crates/growthbook-rust-sdk)
(`will-bank/growthbook-rust-sdk`) para a crate **oficial**
[`growthbook-rust`](https://crates.io/crates/growthbook-rust)
(`growthbook/growthbook-rust`). A API pública exposta por `derust::growthbookx`
permanece com os mesmos nomes e assinaturas (`GrowthBookClient::new(...)`,
`GrowthBookClientTrait`, `GrowthBookAttribute::from(...)`, `GrowthbookError`, etc.),
mas os tipos concretos agora vêm da crate oficial — se você importava algo diretamente
de `growthbook_rust_sdk::*` (fora do re-export de `derust::growthbookx`), troque para
`growthbook_rust::*` ou, preferencialmente, use sempre `derust::growthbookx::*`.
