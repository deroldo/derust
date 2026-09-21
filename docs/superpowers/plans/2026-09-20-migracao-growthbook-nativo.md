# Migração GrowthBook para API Nativa da SDK — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Eliminar a casca própria do derust (`GrowthBookConfig`, `initialize()`,
`growth_book_attributes()`) em torno da crate `growthbook-rust-sdk` (versão 1.1.0,
inalterada) e expor diretamente, através do módulo `growthbookx`, os tipos e funções
nativos da SDK — sem camada de tradução própria.

**Architecture:** `crates/derust/src/growthbookx/mod.rs` deixa de conter lógica própria
e passa a ser um módulo de puros re-exports (`pub use`) dos itens públicos da
`growthbook-rust-sdk` necessários para inicializar o cliente, avaliar features e
tratar atributos/erros. O re-export hoje duplicado em `crates/derust/src/httpx/mod.rs`
(`pub use growthbook_rust_sdk::client::*;`) é removido dali e centralizado em
`growthbookx`, que passa a ser o único ponto de entrada público para tudo relacionado
a GrowthBook — coerente com a tabela de features do projeto (`growthbook` →
`growthbookx`). `AppContext` já recebe e devolve o tipo nativo `GrowthBookClient`
(confirmado na exploração de código, ver "Achados da exploração" abaixo) — **não há
necessidade de alterar a assinatura de `AppContext::new()` nem de `AppContext::growth_book()`**,
contrariando a hipótese de risco levantada no plano de negócio original.

**Tech Stack:** Rust 1.96 (edition 2021), `growthbook-rust-sdk = "1.1.0"` (dependência
opcional via feature `growthbook`), Cargo workspace com crate `derust` +
`examples/growthbook`.

**Spec:** Plano de negócio aprovado no vault Obsidian "Deroldo":
`Projects/derust/plans/migracao_growthbook_nativo.md`
(cópia local consultada em:
`/Users/deroldo/Library/Mobile Documents/iCloud~md~obsidian/Documents/Deroldo/Projects/derust/plans/migracao_growthbook_nativo.md`)

## Achados da exploração de código (contexto para todas as tarefas)

- `crates/derust/src/growthbookx/mod.rs` (39 linhas) hoje contém:
  - `struct GrowthBookConfig { growth_book_url, sdk_key, update_interval, http_timeout }`
  - `async fn initialize(config: &GrowthBookConfig) -> Result<GrowthBookClient, Box<dyn std::error::Error>>` — só chama `GrowthBookClient::new(url, sdk_key, update_interval, http_timeout)` e faz `map_err` para `Box<dyn Error>`.
  - `fn growth_book_attributes(value: Value, tags: &HttpTags) -> Result<Vec<GrowthBookAttribute>, HttpError>` — só chama `GrowthBookAttribute::from(value)` e converte o erro para `HttpError`.
- A SDK nativa (`growthbook-rust-sdk` 1.1.0, código-fonte lido em
  `~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/growthbook-rust-sdk-1.1.0/src/`)
  já expõe diretamente, sem necessidade de wrapper:
  - `growthbook_rust_sdk::client::GrowthBookClient::new(api_url: &str, sdk_key: &str, update_interval: Option<Duration>, http_timeout: Option<Duration>) -> Result<Self, GrowthbookError>` (em `src/client.rs`).
  - `growthbook_rust_sdk::client::GrowthBookClientTrait` (métodos `is_on`, `is_off`, `feature_result`, `total_features`).
  - `growthbook_rust_sdk::model_public::GrowthBookAttribute::from(value: serde_json::Value) -> Result<Vec<GrowthBookAttribute>, GrowthbookError>` (em `src/model_public.rs`) — mesmo comportamento de `growth_book_attributes`, mas sem tradução de erro.
  - `growthbook_rust_sdk::model_public::{GrowthBookAttribute, GrowthBookAttributeValue, FeatureResult, Experiment, ExperimentResult}`.
  - `growthbook_rust_sdk::error::{GrowthbookError, GrowthbookErrorCode}` — `GrowthbookError` implementa `std::error::Error` e `Display` (em `src/error.rs`), então pode ser usado diretamente em `Result<_, Box<dyn std::error::Error>>` como já ocorre em `main.rs` (`fn main() -> Result<(), Box<dyn std::error::Error>>`).
- `crates/derust/src/httpx/context.rs` já declara o campo `growth_book: GrowthBookClient`
  e o parâmetro `growth_book: GrowthBookClient` em `AppContext::new()` importando
  **diretamente** de `growthbook_rust_sdk::client::GrowthBookClient` (linha 10) — não
  passa pela casca de `growthbookx`. Ou seja, **nenhuma mudança é necessária em
  `context.rs`** para este plano: a API pública de `AppContext` já é nativa.
- `crates/derust/src/httpx/mod.rs` já contém, condicionado à feature `growthbook`:
  `pub use growthbook_rust_sdk::client::*;` (linha 32-33) — isso duplica, em parte, o
  que `growthbookx` deveria re-exportar. Ver decisão de arquitetura abaixo.
- Não há testes automatizados dedicados ao módulo `growthbookx` hoje (nenhum
  `#[cfg(test)]` no arquivo, nenhum diretório `tests/` no crate `derust` cobrindo
  growthbook). Os testes existentes de exemplo (`#[cfg(test)]`) seguem o padrão em
  `crates/derust/src/envx/environment.rs`, `crates/derust/src/tracex/initialize.rs` e
  `crates/derust/src/httpx/auth_extractor.rs` (módulo de teste inline no mesmo
  arquivo).
- Versão atual da crate: `crates/derust/Cargo.toml` → `version = "0.4.6"`. O workspace
  usa `[workspace.package] version = "0.1.0"`, mas não é herdada pelo crate `derust`
  (que declara sua própria versão explícita) — só o `Cargo.toml` do crate importa.
- Não existe arquivo `CHANGELOG.md` no repositório hoje.

## Decisão de arquitetura tomada nesta fase (documentada para revisão)

O plano de negócio pede para expor a API nativa "na superfície pública do derust" mas
não especifica **onde** centralizar os re-exports, dado que hoje já existe uma
duplicidade (`httpx::mod.rs` re-exporta `growthbook_rust_sdk::client::*`, enquanto
`growthbookx::mod.rs` contém a casca própria só com `GrowthBookConfig`/`initialize`/
`growth_book_attributes`, sem re-exportar `model_public`/`error`). Como esta é uma
decisão técnica de organização de módulo (não uma regra de negócio) e o `AskUserQuestion`
não estava disponível nesta sessão de refinamento, a decisão tomada — a ser validada
em code review — foi:

- **`growthbookx` passa a ser o único ponto de entrada público** para tudo relacionado
  a GrowthBook (client, trait, atributos, resultado de feature, erros), coerente com a
  tabela de features do `CLAUDE.md` (`growthbook` → módulo `growthbookx`).
- O re-export duplicado em `crates/derust/src/httpx/mod.rs`
  (`pub use growthbook_rust_sdk::client::*;`) é **removido**, pois passa a ser redundante
  com o que `growthbookx` expõe. Isso é uma mudança adicional de superfície pública (quem
  hoje importa `derust::httpx::GrowthBookClientTrait` — como faz
  `examples/growthbook/src/main.rs` — passará a importar `derust::growthbookx::GrowthBookClientTrait`).
  Como o plano já aceita breaking change de API pública e bump major, este ajuste está
  dentro do espírito do plano (eliminar duplicidade/casca) mas **é uma decisão técnica
  desta fase de refinamento, não do plano de negócio original** — deixe isso destacado
  no PR para quem revisar.
- Se, em code review, esta centralização for rejeitada (ex: preferir manter o re-export
  em `httpx` por retrocompatibilidade de import path), a Tarefa 2 é a única que precisa
  ser refeita — as demais tarefas não dependem de qual módulo re-exporta, apenas de que
  `growthbookx::GrowthBookClient` (ou o caminho escolhido) compile e resolva para o
  mesmo tipo usado por `AppContext`.

## Global Constraints

- Não alterar a versão da dependência `growthbook-rust-sdk` — permanece `1.1.0` (fora de
  escopo, incluindo não mexer no comentário `# wating for https://github.com/will-bank/growthbook-rust-sdk/pull/5` em `Cargo.toml:42`).
- Não alterar nenhuma regra de negócio de avaliação de feature flag (cache, intervalo de
  atualização, condições) além do que a SDK já oferece nativamente.
- Não tocar em nenhuma outra feature do derust não relacionada a GrowthBook.
- `crates/derust/src/growthbookx/mod.rs` não pode mais conter `GrowthBookConfig`,
  `initialize()` nem `growth_book_attributes()` ao final (critério de sucesso do plano).
- `task lint` (`cargo fmt --all -- --check && cargo clippy -- -D warnings`) e
  `task test` (`cargo nextest run`) devem passar a cada tarefa, com a feature
  `growthbook` habilitada explicitamente onde relevante (`cargo build --features growthbook`,
  `cargo clippy --features growthbook -- -D warnings`).
- Bump de versão: como a crate está em série `0.x` (SemVer pre-1.0, onde o dígito
  `minor` faz o papel de "major" para compatibilidade, por convenção do Cargo/Rust),
  o bump desta mudança breaking é `0.4.6` → `0.5.0` (não `1.0.0`, que seria uma
  mudança maior de percepção de estabilidade não pedida pelo plano). Isso satisfaz o
  critério de sucesso "a versão da crate recebe um bump major" no sentido do
  versionamento pre-1.0 usado pelo próprio projeto (ver commits `bump derust version to
  0.4.x` no histórico, que só bumpavam patch).

---

### Task 1: Reescrever `growthbookx` como módulo de re-export nativo, sem wrappers

**Dificuldade:** Simples

**Depende de:** nada (pode começar imediatamente)

**Bloqueia:** Task 2, Task 3, Task 4

**Files:**
- Modify: `crates/derust/src/growthbookx/mod.rs`

**Interfaces:**
- Consumes: tipos públicos da crate `growthbook_rust_sdk` 1.1.0 já disponível como
  dependência opcional (feature `growthbook`): `growthbook_rust_sdk::client::{GrowthBookClient, GrowthBookClientTrait}`, `growthbook_rust_sdk::model_public::{GrowthBookAttribute, GrowthBookAttributeValue, FeatureResult, Experiment, ExperimentResult}`, `growthbook_rust_sdk::error::{GrowthbookError, GrowthbookErrorCode}`.
- Produces: módulo público `derust::growthbookx` re-exportando todos os itens acima
  (para uso por `AppContext`, exemplos e consumidores externos). Nenhuma função ou
  struct própria do derust permanece neste módulo.

**Contexto:** O arquivo atual (39 linhas) contém uma casca própria que duplica
conceitos da SDK. O objetivo desta tarefa é eliminar essa casca por completo e deixar
o módulo apenas re-exportando os tipos nativos necessários, sem nenhuma lógica ou
tradução de erro proprietária do derust.

- [ ] **Step 1: Ler o arquivo atual para confirmar o estado antes de editar**

Rode: `cat crates/derust/src/growthbookx/mod.rs`

Confirme que ele contém exatamente `GrowthBookConfig`, `initialize()` e
`growth_book_attributes()` como descrito acima (se o conteúdo divergir, pare e avalie
antes de prosseguir — pode indicar que outra tarefa já alterou o arquivo).

- [ ] **Step 2: Substituir o conteúdo do arquivo por re-exports puros**

Escreva `crates/derust/src/growthbookx/mod.rs` com exatamente este conteúdo:

```rust
//! Re-export direto da API pública da crate `growthbook-rust-sdk` (versão fixada em
//! `crates/derust/Cargo.toml`). O derust não adiciona nenhuma camada própria de
//! configuração, cliente ou tratamento de erro sobre o GrowthBook — use os tipos e
//! funções nativos da SDK diretamente através deste módulo.

pub use growthbook_rust_sdk::client::{GrowthBookClient, GrowthBookClientTrait};
pub use growthbook_rust_sdk::error::{GrowthbookError, GrowthbookErrorCode};
pub use growthbook_rust_sdk::model_public::{
    Experiment, ExperimentResult, FeatureResult, GrowthBookAttribute, GrowthBookAttributeValue,
};
```

- [ ] **Step 3: Compilar com a feature `growthbook` para validar que não há erro de import**

Rode: `cargo build -p derust --features growthbook`
Esperado: build sem erros. Se houver erro de item não encontrado (ex: nome de tipo
diferente), ajuste os `pub use` para os nomes reais expostos pela versão `1.1.0` da
SDK (você pode inspecionar o código-fonte baixado em
`~/.cargo/registry/src/*/growthbook-rust-sdk-1.1.0/src/{client,model_public,error}.rs`
para confirmar os nomes exatos).

- [ ] **Step 4: Rodar lint**

Rode: `cargo fmt --all -- --check && cargo clippy --features growthbook -- -D warnings`
Esperado: sem erros. Se `cargo fmt` reclamar, rode `cargo fmt --all` e reveja o diff.

- [ ] **Step 5: Commit**

```bash
git add crates/derust/src/growthbookx/mod.rs
git commit -m "refactor(growthbookx): expose native growthbook-rust-sdk API, remove wrapper layer"
```

---

### Task 2: Remover re-export duplicado de `httpx::mod.rs` (centralizar em `growthbookx`)

**Dificuldade:** Simples

**Depende de:** Task 1 (para que `growthbookx` já exponha `GrowthBookClientTrait` antes de removê-lo de `httpx`)

**Bloqueia:** Task 3, Task 4

**Files:**
- Modify: `crates/derust/src/httpx/mod.rs`

**Interfaces:**
- Consumes: `derust::growthbookx::{GrowthBookClient, GrowthBookClientTrait, ...}` (produzido na Task 1).
- Produces: `derust::httpx` deixa de re-exportar itens do GrowthBook; consumidores que
  hoje importam `derust::httpx::GrowthBookClientTrait` (ex: `examples/growthbook/src/main.rs`,
  ver Task 3) devem importar de `derust::growthbookx::GrowthBookClientTrait` a partir
  desta tarefa.

**Contexto:** Esta é a decisão de arquitetura documentada na seção "Decisão de
arquitetura tomada nesta fase" acima — leia-a antes de implementar esta tarefa. Em
resumo: `growthbookx` passa a ser o único ponto de entrada público para GrowthBook,
removendo a duplicidade hoje existente em `httpx::mod.rs`.

- [ ] **Step 1: Localizar e remover o re-export**

Em `crates/derust/src/httpx/mod.rs`, remova estas duas linhas (atualmente por volta da
linha 32-33):

```rust
#[cfg(feature = "growthbook")]
pub use growthbook_rust_sdk::client::*;
```

Não remova nada mais do arquivo — `context.rs` (usado internamente por `httpx`)
continua importando `growthbook_rust_sdk::client::GrowthBookClient` diretamente para
seu próprio uso interno (campo privado de `AppContext`); isso não depende do re-export
público removido aqui e **não deve ser alterado nesta tarefa nem em nenhuma outra**
deste plano — `AppContext::new()` e `AppContext::growth_book()` já usam o tipo nativo e
não precisam de nenhuma mudança de assinatura.

- [ ] **Step 2: Compilar o crate inteiro com a feature `growthbook`**

Rode: `cargo build -p derust --features growthbook`
Esperado: build sem erros (o campo interno de `context.rs` continua resolvendo pois
importa diretamente da crate `growthbook_rust_sdk`, não do re-export removido).

- [ ] **Step 3: Rodar lint**

Rode: `cargo fmt --all -- --check && cargo clippy --features growthbook -- -D warnings`
Esperado: sem erros.

- [ ] **Step 4: Commit**

```bash
git add crates/derust/src/httpx/mod.rs
git commit -m "refactor(httpx): remove duplicated growthbook re-export, centralize in growthbookx"
```

---

### Task 3: Atualizar `examples/growthbook/src/main.rs` para a API nativa

**Dificuldade:** Médio

**Depende de:** Task 1, Task 2 (precisa que `growthbookx` já exponha os tipos nativos e que `httpx` não os exponha mais, para escrever os imports finais corretamente)

**Bloqueia:** nada (pode rodar em paralelo com Task 4)

**Files:**
- Modify: `examples/growthbook/src/main.rs`

**Interfaces:**
- Consumes: `derust::growthbookx::{GrowthBookClient, GrowthBookClientTrait, GrowthBookAttribute}` (produzidos na Task 1); `derust::httpx::{start, AppContext, HttpError, HttpTags}` (inalterados); `AppContext::new(app_name, env, growth_book, state)` (assinatura inalterada, confirmada na exploração de código — `growth_book` continua sendo do tipo `GrowthBookClient`).
- Produces: exemplo funcional demonstrando `GrowthBookClient::new(...)` chamado
  diretamente (sem `GrowthBookConfig`/`initialize()`) e `GrowthBookAttribute::from(...)`
  chamado diretamente (sem `growth_book_attributes()`/`HttpTags`/`HttpError` na
  conversão de atributos).

**Contexto:** O arquivo atual usa `derust::growthbookx::{growth_book_attributes,
GrowthBookConfig}` e `derust::growthbookx::initialize`, além de importar
`GrowthBookClientTrait` de `derust::httpx`. Esta tarefa reescreve o `main.rs` para usar
diretamente `GrowthBookClient::new(...)` (que retorna `Result<GrowthBookClient,
GrowthbookError>`, e `GrowthbookError` implementa `std::error::Error`, então funciona
com o `?` dentro de `fn main() -> Result<(), Box<dyn std::error::Error>>` sem nenhuma
conversão manual) e `GrowthBookAttribute::from(value)` diretamente (que retorna
`Result<Vec<GrowthBookAttribute>, GrowthbookError>` — note que isso muda o tipo de erro
do handler HTTP: antes o erro virava `HttpError` automaticamente via `?` porque
`growth_book_attributes` já fazia essa tradução; agora o handler precisa converter
`GrowthbookError` para `HttpError` manualmente, como mostrado no Step 2 abaixo).

- [ ] **Step 1: Ler o arquivo atual completo**

Rode: `cat examples/growthbook/src/main.rs`
(referência: conteúdo já lido nesta fase de refinamento, reproduzido abaixo para
comparação linha a linha durante a edição)

- [ ] **Step 2: Reescrever o arquivo com a API nativa**

Substitua todo o conteúdo de `examples/growthbook/src/main.rs` por:

```rust
use axum::extract::State;
use axum::http::StatusCode;
use axum::routing::get;
use axum::Router;
use rand::Rng;
use serde_json::json;
use derust::envx::Environment;
use derust::growthbookx::{GrowthBookAttribute, GrowthBookClient, GrowthBookClientTrait};
use derust::httpx::json::JsonResponse;
use derust::httpx::{start, AppContext, HttpError, HttpTags};

#[derive(Clone)]
pub struct AppState {
    pub bar_v1: String,
    pub bar_v2: String,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let env = Environment::detect().ok().unwrap_or(Environment::Local);

    // any cloneable struct
    let app_state = AppState {
        bar_v1: "bar".to_string(),
        bar_v2: "foobar".to_string(),
    };

    // required to access growthbook admin dashboard to create the sdk-key: http://localhost:3000
    // API 100% nativa da growthbook-rust-sdk: sem casca própria do derust.
    let growthbook = GrowthBookClient::new(
        "http://localhost:3100",
        // change it with your created sdk-key
        "sdk-key",
        None, // update_interval
        None, // http_timeout
    )
    .await
    .map_err(|error| Box::new(error) as Box<dyn std::error::Error>)?;

    let application_name = "sample";

    // easy way to get application context things, like your application state struct
    let context = AppContext::new(application_name, env, growthbook, app_state)?;

    let port = 3001;
    let router = Router::new().nest("/foo", Router::new().route("/", get(handler)));

    // automatic health-check route
    // automatic route response status code log
    start(port, context, router).await
}

#[derive(serde::Serialize)]
pub struct FooResponse {
    pub foo: String,
}

async fn handler(
    State(context): State<AppContext<AppState>>,
) -> Result<JsonResponse<FooResponse>, HttpError> {
    let tags = HttpTags::default();

    let pair = rand::thread_rng().gen_range(0..100) % 2 == 0;

    // creating growthbook attributes to match conditions, direto da SDK nativa
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

    Ok(JsonResponse::new(
        StatusCode::OK,
        FooResponse { foo: bar },
        tags,
    ))
}
```

- [ ] **Step 3: Compilar o exemplo**

Rode: `cargo build -p growthbook --manifest-path examples/growthbook/Cargo.toml`
(confira o nome do pacote em `examples/growthbook/Cargo.toml` com
`grep '^name' examples/growthbook/Cargo.toml` antes de rodar, caso o nome do pacote
não seja `growthbook`)
Esperado: build sem erros.

- [ ] **Step 4: Rodar lint no exemplo**

Rode: `cargo fmt --all -- --check` (a partir da raiz do workspace principal do exemplo —
`examples/growthbook` é um crate standalone fora do workspace `derust`, então rode
`cd examples/growthbook && cargo fmt --all -- --check && cargo clippy -- -D warnings`)
Esperado: sem erros.

- [ ] **Step 5: Commit**

```bash
git add examples/growthbook/src/main.rs
git commit -m "docs(examples): update growthbook example to native SDK API"
```

---

### Task 4: Atualizar `crates/derust/src/growthbookx/README.md`

**Dificuldade:** Simples

**Depende de:** Task 1, Task 2, Task 3 (documenta o resultado final das três — deve ser a última tarefa de conteúdo antes do bump de versão)

**Bloqueia:** Task 5

**Files:**
- Modify: `crates/derust/src/growthbookx/README.md`

**Interfaces:**
- Consumes: código final de `examples/growthbook/src/main.rs` produzido na Task 3 (a
  documentação deve refletir literalmente esse exemplo, não uma variação).
- Produces: documentação README atualizada, sem nenhuma referência a
  `GrowthBookConfig`, `growthbookx::initialize` ou `growth_book_attributes`.

**Contexto:** O README atual (85 linhas) documenta a casca antiga com blocos de código
`GrowthBookConfig { ... }` e `growthbookx::initialize(&gb_config)`. Substitua pelos
blocos de código nativos, mantendo a mesma estrutura de seções (Cargo.toml, main.rs,
handler).

- [ ] **Step 1: Ler o README atual**

Rode: `cat crates/derust/src/growthbookx/README.md`

- [ ] **Step 2: Reescrever o README com os blocos de código nativos**

Substitua todo o conteúdo de `crates/derust/src/growthbookx/README.md` por:

````markdown
# derust - growthbook

Este módulo re-exporta diretamente a API pública da crate
[`growthbook-rust-sdk`](https://crates.io/crates/growthbook-rust-sdk) (versão fixada em
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
    // API 100% nativa da growthbook-rust-sdk: sem casca própria do derust.
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
  — note que o tipo de erro retornado agora é `growthbook_rust_sdk::error::GrowthbookError`
  (também re-exportado por `derust::growthbookx::GrowthbookError`), não mais
  `derust::httpx::HttpError`. Se você usava esse erro diretamente em um handler HTTP,
  converta-o manualmente para `HttpError` (veja o exemplo acima).
- `derust::httpx::GrowthBookClientTrait` também deixou de existir — importe de
  `derust::growthbookx::GrowthBookClientTrait`.
````

- [ ] **Step 3: Revisar visualmente o diff**

Rode: `git diff crates/derust/src/growthbookx/README.md`
Confirme que não sobrou nenhuma referência a `GrowthBookConfig`, `growthbookx::initialize`
ou `growth_book_attributes` (`grep -n "GrowthBookConfig\|growthbookx::initialize\|growth_book_attributes" crates/derust/src/growthbookx/README.md` deve retornar vazio, exceto na seção "Breaking change" que os menciona apenas como referência histórica).

- [ ] **Step 4: Commit**

```bash
git add crates/derust/src/growthbookx/README.md
git commit -m "docs(growthbookx): document native SDK usage and breaking change"
```

---

### Task 5: Bump de versão major (0.x) e validação final do crate

**Dificuldade:** Simples

**Depende de:** Task 1, Task 2, Task 3, Task 4 (precisa que toda a migração de código e documentação já esteja completa e validada antes de fechar com o bump de versão)

**Bloqueia:** nada (última tarefa do plano)

**Files:**
- Modify: `crates/derust/Cargo.toml`

**Interfaces:**
- Consumes: nada de código — apenas fecha o plano com o bump de versão e a validação
  full-suite exigida pelos critérios de sucesso do plano de negócio.
- Produces: `derust` publicável em `0.5.0`, documentando a quebra de compatibilidade.

**Contexto:** Ver "Global Constraints" acima para a justificativa do bump para
`0.5.0` (não `1.0.0`) dado o versionamento pre-1.0 já em uso pelo projeto.

- [ ] **Step 1: Atualizar a versão no Cargo.toml do crate**

Em `crates/derust/Cargo.toml`, altere:

```toml
version = "0.4.6"
```

para:

```toml
version = "0.5.0"
```

- [ ] **Step 2: Rodar a suíte completa de testes do crate com a feature `growthbook`**

Rode: `cargo nextest run --features growthbook` (a partir de `crates/derust`, ou
`cargo nextest run -p derust --features growthbook` a partir da raiz do workspace)
Esperado: todos os testes passam, 0 falhas.

- [ ] **Step 3: Rodar a suíte completa de testes com o conjunto default de features (sem `growthbook`) para garantir que nada quebrou fora da feature**

Rode: `task test` (equivalente a `cargo nextest run`)
Esperado: todos os testes passam, 0 falhas.

- [ ] **Step 4: Rodar lint completo do workspace**

Rode: `task lint` (equivalente a `cargo fmt --all -- --check && cargo clippy -- -D warnings`)
Esperado: sem erros. Rode também `cargo clippy --features growthbook -- -D warnings`
para cobrir a feature opcional.

- [ ] **Step 5: Build do exemplo `growthbook` (checagem final de ponta a ponta)**

Rode: `cd examples/growthbook && cargo build`
Esperado: build sem erros.

- [ ] **Step 6: Confirmar critérios de sucesso do plano de negócio, um a um**

Rode cada verificação e confirme o resultado esperado antes de prosseguir:

```bash
grep -n "GrowthBookConfig\|fn initialize\|fn growth_book_attributes" crates/derust/src/growthbookx/mod.rs
```
Esperado: vazio (nenhuma ocorrência).

```bash
grep -n "version" crates/derust/Cargo.toml | head -1
```
Esperado: `version = "0.5.0"`.

- [ ] **Step 7: Commit**

```bash
git add crates/derust/Cargo.toml
git commit -m "chore: bump derust version to 0.5.0 (breaking change: native growthbook API)"
```

---

## Plano de testes e revisão por tarefa (resumo)

| Tarefa | Plano de testes | Plano de revisão |
|---|---|---|
| 1 — `growthbookx` puro re-export | `cargo build --features growthbook` + `cargo clippy --features growthbook -- -D warnings` | Confirmar que nenhum tipo/função próprio do derust permanece no arquivo; nomes re-exportados batem com os públicos da SDK 1.1.0 |
| 2 — remover re-export duplicado de `httpx` | `cargo build --features growthbook` (crate inteiro) | Confirmar que `context.rs` não foi tocado e que `AppContext::new`/`growth_book()` seguem com a mesma assinatura |
| 3 — atualizar exemplo | `cargo build` do crate `examples/growthbook`; `cargo clippy -- -D warnings` no exemplo | Rodar mentalmente o handler: erro de `GrowthBookAttribute::from` deve virar `HttpError` explicitamente (não mais automático) |
| 4 — atualizar README | Revisão textual (grep por termos removidos) | Confirmar que os blocos de código do README são idênticos ao `main.rs` final da Task 3 |
| 5 — bump de versão e validação final | `cargo nextest run --features growthbook`, `task test`, `task lint`, build do exemplo | Confirmar os 5 critérios de sucesso do plano de negócio um a um (ver Step 6) |

## Paralelização

- Tasks 1 → 2 são sequenciais (2 depende do resultado de 1).
- Task 3 depende de 1 e 2 (precisa saber de onde importar os tipos finais).
- Task 4 depende de 1, 2 e 3 (documenta o resultado final).
- Task 3 e Task 4 **não podem** ser paralelas entre si nesta ordem porque Task 4 cita
  literalmente o código de Task 3 — mas nada impede inverter a ordem se preferir
  (documentar antes do exemplo) desde que o conteúdo final seja idêntico entre os dois.
- Task 5 é sempre a última, depende de todas as anteriores.
- Não há oportunidade real de paralelismo entre subagentes neste plano — é uma cadeia
  linear curta (5 tarefas simples/médias) sobre um único módulo pequeno.
