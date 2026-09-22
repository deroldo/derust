# Adicionar push OTLP para métricas e logs — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Fazer `tracex::init()` também montar pipelines de push OTLP para métricas
(`MeterProvider` + `PeriodicReader`) e logs (`LoggerProvider` +
`opentelemetry-appender-tracing`), coexistindo com o pipeline de traces OTLP já
existente e com o `/metrics` Prometheus (`metricx`) atual, sem quebrar nenhum
consumidor que hoje não configura OTLP para esses dois sinais.

**Architecture:** Toda a mudança fica dentro do módulo `tracex` (decisão desta fase,
ver "Decisões técnicas tomadas nesta fase" abaixo): dois arquivos novos,
`otlp_metrics.rs` e `otlp_logs.rs`, cada um expondo uma função `build_*` que lê as
mesmas env vars padrão OTLP (`OTEL_EXPORTER_OTLP_ENDPOINT`,
`OTEL_EXPORTER_OTLP_PROTOCOL`, `OTEL_EXPORTER_OTLP_HEADERS`, e os overrides
específicos de sinal `OTEL_EXPORTER_OTLP_METRICS_*`/`OTEL_EXPORTER_OTLP_LOGS_*`, lidos
automaticamente pelos builders do `opentelemetry-otlp`) e retorna `Option<...Provider>`
(`None` + `tracing::warn!` quando a env não permite montar um exporter — mesmo padrão
hoje usado por `build_otel_layer()`/`init_tracerprovider()` da crate
`init-tracing-opentelemetry` para traces). `tracex::initialize.rs` passa a chamar as
duas novas funções dentro de `init()`, adiciona a camada
`opentelemetry_appender_tracing::layer::OpenTelemetryTracingBridge` ao
`tracing_subscriber::Registry` (ao lado do `fmt::layer()` já existente) quando o
`LoggerProvider` foi criado, registra o `MeterProvider` globalmente via
`opentelemetry::global::set_meter_provider`, e devolve um novo tipo
`tracex::Guard` (substituindo o `TracingGuard` reexportado hoje implicitamente) que
encapsula os três providers (trace/metrics/logs) e faz flush+shutdown de todos no
`Drop`, preservando o padrão de uso `let _guard = tracex::init()?;` já documentado no
README e usado nos exemplos.

**Tech Stack:** Rust 1.96 (edition 2021). Dependências novas: `opentelemetry_sdk`
(0.30, já resolvida transitivamente na mesma versão — sem risco de conflito) como
dependência direta, e `opentelemetry-appender-tracing` (nova, versão a fixar em
`0.30.x` para casar com a família `opentelemetry` 0.30 já usada pelo workspace).
Reaproveita `opentelemetry-otlp` 0.30 (já dependência do crate, com as features
`"metrics"` e `"logs"` já habilitadas em `Cargo.toml` — confirmado por leitura direta,
nenhuma mudança de feature necessária nesse ponto do escopo original) e
`init_tracing_opentelemetry::resource::DetectResource` (já público, reaproveitado para
gerar o mesmo `Resource` — mesmo `service.name`/atributos — usado pelos três
pipelines).

**Spec:** Plano de negócio aprovado no vault Obsidian "Deroldo":
`Projects/derust/plans/adicionar_push_otlp_metricas_e_logs.md`
(cópia local consultada em:
`/Users/deroldo/Library/Mobile Documents/iCloud~md~obsidian/Documents/Deroldo/Projects/derust/plans/adicionar_push_otlp_metricas_e_logs.md`)

## Achados da exploração de código (contexto para todas as tarefas)

- `crates/derust/src/tracex/initialize.rs` (154 linhas) hoje monta só o pipeline de
  traces: `build_otel_layer()` (de `init_tracing_opentelemetry::tracing_subscriber_ext`)
  cria um `OpenTelemetryLayer` + `TracingGuard` (struct da própria dependência
  `init-tracing-opentelemetry`, contém só `tracer_provider: SdkTracerProvider`, e no
  `Drop` chama `force_flush()` + `shutdown()`). `init()` monta dois
  `tracing_subscriber::registry()` (um temporário para logar o próprio setup, um final
  com `layer` OTLP + `build_loglevel_filter_layer()` + `fmt::layer()`) e retorna
  `Result<TracingGuard, Box<dyn std::error::Error>>`.
- `build_loglevel_filter_layer()` já lê `RUST_LOG`/`OTEL_LOG_LEVEL`, monta a
  `EnvFilter` e tem um teste de regressão específico
  (`builds_otlp_http_protobuf_exporter_without_client_conflict`) documentando um bug
  histórico de unificação de features do Cargo entre `opentelemetry-otlp` direto e via
  `init-tracing-opentelemetry`. Esse teste roda `init()` de ponta a ponta com
  `OTEL_EXPORTER_OTLP_PROTOCOL=http/protobuf` + `OTEL_EXPORTER_OTLP_ENDPOINT` setados e
  espera `Ok(_)`. É o modelo pedido pelo escopo do plano de negócio para os novos
  testes de métricas/logs.
- `crates/derust/src/tracex/log.rs` (23 linhas) já usa só macros padrão do crate
  `tracing` (`trace!`, `debug!`, `info!`, `warn!`, `error!`). **Nenhuma mudança
  necessária neste arquivo**: uma vez que a camada `OpenTelemetryTracingBridge` for
  adicionada ao `Registry` em `initialize.rs`, todo log emitido via
  `tracex::log::*` passa a ser capturado automaticamente também pelo `LoggerProvider`
  OTLP, sem duplicar instrumentação.
- `crates/derust/Cargo.toml`: a dependência `opentelemetry-otlp` já está declarada com
  `features = ["http", "logs", "tracing", "serde", "integration-testing", "reqwest",
  "metrics", "reqwest-blocking-client"]` — as features `"metrics"` e `"logs"` **já
  estão habilitadas**, então o item de escopo "adicionar a feature `metrics` em
  `opentelemetry-otlp`" já está satisfeito no estado atual do repositório; não há
  Cargo.toml diff nesse ponto específico (mantido documentado aqui só para não ser
  redescoberto/reintroduzido por engano). No `[workspace.dependencies]` (raiz
  `Cargo.toml`), a versão efetivamente usada tem features adicionais:
  `["...", "reqwest-rustls", "http-proto", "tls"]`.
- `opentelemetry-otlp` 0.30 expõe `MetricExporter::builder().with_http()` /
  `.with_tonic()` (gated por `http-proto`/`http-json` e `grpc-tonic` respectivamente —
  `http-proto` já habilitado no workspace; `grpc-tonic` **não** está na lista explícita
  do `derust/Cargo.toml`, mas está habilitado transitivamente pela unificação de
  features do Cargo, porque `init-tracing-opentelemetry` (dependência de traces) já
  declara `opentelemetry-otlp` com `features = ["grpc-tonic", "trace"]` — confirmado
  lendo `~/.cargo/registry/.../init-tracing-opentelemetry-0.29.0/Cargo.toml:56-61` — e
  Cargo une todas as features pedidas por qualquer dependente da mesma versão de uma
  crate no grafo). Isso significa que `.with_tonic()` para `MetricExporter`/
  `LogExporter` **já compila hoje sem nenhuma mudança de feature**, pelo mesmo motivo
  que `grpc` já funciona para traces. Confirme isso na Task 1 antes de prosseguir (é o
  tipo de suposição que, se errada, muda o Cargo.toml da Task 1).
- `MetricExporter::builder().with_http().build()` e
  `LogExporter::builder().with_http().build()` **já leem sozinhos**
  `OTEL_EXPORTER_OTLP_ENDPOINT`/`OTEL_EXPORTER_OTLP_HEADERS` (com fallback para os
  overrides `OTEL_EXPORTER_OTLP_METRICS_*`/`OTEL_EXPORTER_OTLP_LOGS_*`) — confirmado
  lendo `opentelemetry-otlp-0.30.0/src/exporter/http/mod.rs:380-384`. Não é necessário
  nenhum parsing manual de endpoint/headers nas novas funções; só é preciso decidir
  `.with_http()` vs `.with_tonic()` a partir de `OTEL_EXPORTER_OTLP_PROTOCOL` (mesma
  lógica de `infer_protocol`/`read_protocol_and_endpoint_from_env`, hoje privadas em
  `init_tracing_opentelemetry::otlp`, reimplementadas localmente e simplificadas — ver
  Task 2).
- API confirmada por leitura direta do código-fonte cacheado
  (`opentelemetry_sdk-0.30.0`, `opentelemetry-otlp-0.30.0`):
  - `opentelemetry_sdk::metrics::SdkMeterProvider::builder().with_periodic_exporter(exporter).with_resource(resource).build()`
    — não é necessário montar `PeriodicReader` manualmente, `with_periodic_exporter`
    já cria um `PeriodicReader` interno com o intervalo padrão do SDK.
  - `opentelemetry_sdk::metrics::SdkMeterProvider` expõe `force_flush(&self) ->
    OTelSdkResult` e `shutdown(&self) -> OTelSdkResult` (`meter_provider.rs:97,122`).
  - `opentelemetry_sdk::logs::SdkLoggerProvider::builder().with_batch_exporter(exporter).with_resource(resource).build()`
    (`logger_provider.rs:226,253,77`).
  - `opentelemetry_sdk::logs::SdkLoggerProvider` expõe `force_flush(&self) ->
    OTelSdkResult` e `shutdown(&self) -> OTelSdkResult` (`logger_provider.rs:86,129`).
  - `opentelemetry_otlp::MetricExporter`/`opentelemetry_otlp::LogExporter` (não
    `MetricsExporter`/`LogsExporter` — nomes confirmados em `metric.rs:114`/`logs.rs:114`
    do crate `opentelemetry-otlp` local).
- `init_tracing_opentelemetry::resource::DetectResource` é público (`pub struct
  DetectResource`, `pub fn build(mut self) -> Resource`) e é o que `otlp.rs` usa hoje
  para o `Resource` de traces (`DetectResource::default().build()`). Reaproveitar essa
  mesma chamada para métricas/logs garante `service.name` idêntico nos três sinais,
  atendendo ao risco "logs podem divergir entre stdout/OTLP" apontado no plano de
  negócio (na parte de atributos de resource, que é o que os backends usam para
  correlacionar métricas/logs/traces de uma mesma aplicação).
- `opentelemetry-appender-tracing` **não é dependência hoje** (confirmado — ausente do
  `Cargo.lock`). É a lib oficial do projeto `open-telemetry/opentelemetry-rust` para
  plugar o crate `tracing` num `LoggerProvider` OTLP (`OpenTelemetryTracingBridge::new(&logger_provider)`
  implementa `tracing_subscriber::Layer`); não há biblioteca alternativa madura para
  isso, e ela é irmã de versão das demais crates `opentelemetry*` já usadas — fixar em
  `0.30`.
- Não existe hoje um bridge oficial/maduro entre o crate `metrics` (facade usado por
  `metricx`, macros `counter!`/`histogram!`) e `opentelemetry::metrics`. Conclusão
  técnica desta fase (não é regra de negócio — é constatação sobre o ecossistema de
  bibliotecas): **não é viável** fazer a instrumentação já feita via `metrics` também
  alimentar o `MeterProvider` OTLP sem reescrevê-la. Isso confirma o cenário de risco já
  aceito no próprio plano de negócio ("push OTLP de métricas customizadas usa a API
  `opentelemetry::metrics` diretamente") — a Task 4 documenta isso explicitamente no
  README, não é uma surpresa a ser descoberta em code review.
- Não existe `CHANGELOG.md` no repositório hoje (confirmado por busca). A Task 6 cria o
  arquivo pela primeira vez, com uma única entrada para esta mudança (não faz backfill
  histórico de versões anteriores — fora de escopo).
- Não há automação de CI para `cargo publish` (nenhum workflow em `.github/` menciona
  `publish`/`crates.io`, nenhum alvo `publish` no `Taskfile.yml`). O passo de
  publicação no crates.io do escopo do plano de negócio é, portanto, uma ação manual
  fora do que um agente de implementação consegue executar sozinho (requer
  credenciais/token de `cargo login` da conta do mantenedor) — a Task 6 deixa isso
  explícito como passo manual de responsabilidade humana, não bloqueante para as
  tarefas de código.
- Versão atual do crate (`crates/derust/Cargo.toml`): `0.5.0`. Como é feature aditiva
  (não-breaking), o bump é de minor: `0.5.0` → `0.6.0` (mesma convenção pre-1.0 já usada
  no histórico do projeto, documentada no refinamento anterior
  `docs/superpowers/plans/2026-09-20-migracao-growthbook-nativo.md`).

## Decisões técnicas tomadas nesta fase (documentadas para revisão)

Estas decisões foram confirmadas com o usuário via a orquestração (não são regra de
negócio nova — o "como" implementar o que o plano de negócio já decidiu no "o quê"):

1. **Organização de módulo:** estender `tracex` (dois arquivos novos dentro do módulo,
   `otlp_metrics.rs` e `otlp_logs.rs`, chamados por `initialize.rs`), em vez de criar um
   módulo novo `otlpx`. Motivo: reaproveita o parsing de env e o padrão
   warn-sem-quebrar já implementado em `tracex`, e mantém `tracex::init()` como único
   ponto de entrada para os 3 sinais de observabilidade, coerente com a tabela de
   features do `CLAUDE.md` (`http_server` → `tracex`).
2. **Sem novas Cargo features de compilação:** os 3 pipelines (traces, métricas, logs)
   continuam sempre compilados dentro da feature `http_server`/`tracex` já existente;
   cada um decide em runtime, olhando as envs `OTEL_EXPORTER_OTLP_*`, se cria o
   exporter — mesmo padrão de opt-in que já existe hoje para traces. Não há
   `otlp_metrics`/`otlp_logs` como Cargo feature.
3. **Tipo de retorno de `tracex::init()` muda de `TracingGuard` (reexportado
   implicitamente da dependência `init-tracing-opentelemetry`) para um novo tipo
   próprio `tracex::Guard`.** Decisão técnica tomada nesta fase sem novo
   `AskUserQuestion` (é uma consequência direta e sem alternativa razoável do
   requisito já aceito no plano de negócio de fazer flush/shutdown de métricas e logs
   ao encerrar a aplicação — não dá para fazer isso mantendo o tipo externo
   `TracingGuard`, que só conhece o `tracer_provider`). Risco de compatibilidade é
   **baixo na prática**: o padrão de uso documentado no README e usado em todos os
   `examples/*` é `let _guard = tracex::init();` (sem anotação de tipo explícita, sem
   `?` até — ver `examples/trace/src/main.rs:18`), então a troca do tipo concreto não
   quebra nenhum consumidor que segue o padrão documentado. Só quebraria um consumidor
   que anota explicitamente `let guard: init_tracing_opentelemetry::tracing_subscriber_ext::TracingGuard
   = tracex::init()?;` — padrão não documentado e não usado em nenhum exemplo do
   próprio derust. Destacar isso no PR para quem revisar.

## Global Constraints

- Não alterar o pipeline de traces já existente (`build_otel_layer()` continua sendo
  chamado exatamente como hoje; `otlp_metrics.rs`/`otlp_logs.rs` são aditivos).
- Não remover nem alterar o `/metrics` Prometheus (`metricx`) — ele continua existindo
  e funcionando exatamente como hoje, como opção de pull independente do push OTLP.
- Não alterar a forma como aplicações hoje instrumentam métricas via macros do crate
  `metrics` (`counter!`/`histogram!` etc. em `metricx`) — ver constatação técnica acima
  sobre não haver bridge viável; documentar, não reescrever.
- Ambos os pipelines novos (métricas e logs) devem ser opt-in por detecção de env var
  em runtime, nunca por feature de compilação — replicar o padrão de warn-sem-quebrar
  já usado por `build_otel_layer()`/`init_tracerprovider()` para traces: ausência de
  `OTEL_EXPORTER_OTLP_ENDPOINT`/`OTEL_EXPORTER_OTLP_PROTOCOL` → nenhum exporter criado,
  apenas `tracing::warn!`, `init()` continua retornando `Ok(_)`.
- `task lint` (`cargo fmt --all -- --check && cargo clippy -- -D warnings`) e
  `task test` (`cargo nextest run`) devem passar a cada tarefa de código. Rodar também
  com a feature `http_server` explícita onde relevante (é a feature default, mas
  confirme com `cargo build --features http_server` / `cargo clippy --features
  http_server -- -D warnings`).
- Bump de versão: `0.5.0` → `0.6.0` (minor, feature aditiva — ver "Achados da
  exploração" acima para a justificativa do padrão pre-1.0 do projeto).
- Testes de regressão obrigatórios em `tracex/initialize.rs` (extensão do arquivo já
  existente, seguindo o modelo do teste
  `builds_otlp_http_protobuf_exporter_without_client_conflict`): pipeline de métricas
  builda sem erro com env HTTP/protobuf setada; pipeline de logs builda sem erro com
  env HTTP/protobuf setada; ausência de qualquer env OTLP não quebra `init()` (todos os
  três pipelines devolvem `None` internamente e `init()` continua `Ok(_)`).

---

### Task 1: Adicionar dependências novas e confirmar que `grpc-tonic` já compila para `MetricExporter`/`LogExporter`

**Dificuldade:** Simples

**Depende de:** nada (pode começar imediatamente)

**Bloqueia:** Task 2, Task 3

**Files:**
- Modify: `Cargo.toml` (raiz do workspace, seção `[workspace.dependencies]`)
- Modify: `crates/derust/Cargo.toml` (seção `[dependencies]` e feature `http_server`)

**Interfaces:**
- Consumes: nada de código do derust.
- Produces: `opentelemetry_sdk` (0.30, com as features necessárias para
  `metrics::SdkMeterProvider`/`logs::SdkLoggerProvider`) e
  `opentelemetry-appender-tracing` (0.30.x) disponíveis como dependências diretas
  opcionais da feature `http_server`, prontas para uso nas Tasks 2 e 3.

**Contexto:** Ver "Achados da exploração de código" acima: `opentelemetry_sdk 0.30.0`
já está resolvido transitivamente (mesma versão da família `opentelemetry` 0.30 já
usada), então declará-lo direto não deve mudar nenhuma versão resolvida no
`Cargo.lock` além de adicioná-lo à lista de dependências diretas.
`opentelemetry-appender-tracing` é dependência nova.

- [x] **Step 1: Adicionar `opentelemetry_sdk` e `opentelemetry-appender-tracing` ao `[workspace.dependencies]` da raiz**

No `Cargo.toml` da raiz do workspace, logo abaixo da linha `opentelemetry-http =
{ version = "0.30.0", features = ["reqwest"] }`, adicione:

```toml
opentelemetry_sdk = { version = "0.30.0", features = ["metrics", "logs"] }
opentelemetry-appender-tracing = { version = "0.30.0" }
```

- [x] **Step 2: Declarar as duas dependências em `crates/derust/Cargo.toml`**

Na seção `# Observability` de `[dependencies]`, logo abaixo da linha `opentelemetry-http = { workspace = true, features = ["reqwest"], optional = true }`, adicione:

```toml
opentelemetry_sdk = { workspace = true, optional = true }
opentelemetry-appender-tracing = { workspace = true, optional = true }
```

- [x] **Step 3: Adicionar as duas novas deps à feature `http_server`**

Em `crates/derust/Cargo.toml`, na lista `http_server = [...]`, adicione as duas linhas
(mantendo o restante da lista intacto):

```toml
    "dep:opentelemetry_sdk",
    "dep:opentelemetry-appender-tracing",
```

- [x] **Step 4: Confirmar que o crate ainda compila com as novas deps (sem uso ainda)**

Rode: `cargo build -p derust --features http_server`
Esperado: build sem erros (as deps ficam disponíveis mas não usadas ainda — pode gerar
warning de "unused extern crate" seria estranho para deps de biblioteca, mas cargo não
emite esse warning para dependências declaradas e não usadas em Rust; se aparecer
algum erro de resolução de versão, pare e investigue antes de prosseguir — pode indicar
que a suposição de unificação de features documentada em "Achados da exploração" está
errada).

- [x] **Step 5: Confirmar que `grpc-tonic` já está disponível para `MetricExporter`/`LogExporter` sem mudança extra de feature**

Rode:
```bash
cat > /tmp/otlp_grpc_check.rs <<'EOF'
#[allow(dead_code)]
fn check() {
    let _ = opentelemetry_otlp::MetricExporter::builder().with_tonic();
    let _ = opentelemetry_otlp::LogExporter::builder().with_tonic();
}
EOF
```
Não é necessário rodar esse arquivo como binário — em vez disso, adicione
temporariamente essas duas linhas dentro de um `#[cfg(test)] mod test { ... }` novo e
vazio em `crates/derust/src/tracex/initialize.rs` (ex.: dentro de uma função de teste
descartável `#[test] fn otlp_tonic_builders_compile() {}` com o corpo das duas linhas
acima) e rode `cargo build -p derust --features http_server --tests`. Esperado:
compila sem erro "no method named `with_tonic`". Depois de confirmar, **remova** esse
teste descartável — ele não deve permanecer no arquivo final (era só para validar a
suposição desta task; a Task 2 já cobre `with_tonic` de verdade via os testes de
regressão pedidos pelo plano de negócio).

Se `with_tonic()` **não** compilar: adicione `"opentelemetry-otlp/grpc-tonic"` à lista
de features de `opentelemetry-otlp` em `Cargo.toml` (raiz) antes de prosseguir, e
anote esse ajuste nesta task antes de fazer o commit.

- [x] **Step 6: Rodar lint**

Rode: `cargo fmt --all -- --check && cargo clippy --features http_server -- -D warnings`
Esperado: sem erros.

- [x] **Step 7: Commit**

```bash
git add Cargo.toml crates/derust/Cargo.toml Cargo.lock
git commit -m "chore: add opentelemetry_sdk and opentelemetry-appender-tracing dependencies"
```

---

### Task 2: Implementar pipeline OTLP de métricas (`otlp_metrics.rs`) e testes de regressão

**Dificuldade:** Médio

**Depende de:** Task 1

**Bloqueia:** Task 4 (que integra este pipeline em `init()`)

**Files:**
- Create: `crates/derust/src/tracex/otlp_metrics.rs`
- Modify: `crates/derust/src/tracex/mod.rs` (adicionar `mod otlp_metrics;`)

**Interfaces:**
- Consumes: `opentelemetry_sdk::{metrics::SdkMeterProvider, Resource}`,
  `opentelemetry_otlp::MetricExporter` (Task 1), `init_tracing_opentelemetry::resource::DetectResource`
  (já disponível via a dependência `init-tracing-opentelemetry` existente).
- Produces: `pub(crate) fn build_otlp_meter_provider(resource: Resource) ->
  Option<SdkMeterProvider>` — usada pela Task 4 dentro de `initialize.rs`. Retorna
  `None` (e loga `tracing::warn!`) quando não há env suficiente para montar um
  exporter; nunca retorna `Err`/propaga erro (consistente com o requisito de não
  quebrar o boot).

**Contexto:** Mesma lógica de inferência de protocolo usada hoje para traces
(`init_tracing_opentelemetry::otlp::infer_protocol`/`read_protocol_and_endpoint_from_env`,
ambas privadas na dependência, por isso reimplementadas aqui de forma simplificada — só
precisamos decidir `http/protobuf` vs `grpc`, o endpoint/headers já são lidos
automaticamente pelos builders do `opentelemetry-otlp`, conforme "Achados da
exploração"). Usa `OTEL_EXPORTER_OTLP_PROTOCOL` (com fallback futuro natural para
`OTEL_EXPORTER_OTLP_METRICS_PROTOCOL`, que o SDK já lê sozinho dentro do exporter — não
precisamos replicar essa parte, só a escolha de transporte).

- [x] **Step 1: Escrever o arquivo `otlp_metrics.rs`**

```rust
use opentelemetry_otlp::MetricExporter;
use opentelemetry_sdk::metrics::SdkMeterProvider;
use opentelemetry_sdk::Resource;

/// Builds an OTLP push pipeline for metrics (`SdkMeterProvider` backed by a
/// `PeriodicReader`, created internally by `with_periodic_exporter`), reading the same
/// `OTEL_EXPORTER_OTLP_*` env vars already used for traces. Returns `None` (after
/// logging a warning) when no exporter can be built — this must never fail
/// `tracex::init()`'s boot, mirroring the existing behaviour for traces.
pub(crate) fn build_otlp_meter_provider(resource: Resource) -> Option<SdkMeterProvider> {
    let protocol = infer_metrics_protocol();

    let exporter = match protocol.as_deref() {
        Some("http/protobuf") => match MetricExporter::builder().with_http().build() {
            Ok(exporter) => Some(exporter),
            Err(error) => {
                tracing::warn!(
                    "failed to build OTLP metric exporter (http/protobuf): {error}; no OTLP metrics will be pushed"
                );
                None
            }
        },
        Some("grpc") => match MetricExporter::builder().with_tonic().build() {
            Ok(exporter) => Some(exporter),
            Err(error) => {
                tracing::warn!(
                    "failed to build OTLP metric exporter (grpc): {error}; no OTLP metrics will be pushed"
                );
                None
            }
        },
        Some(other) => {
            tracing::warn!(
                "unknown OTEL_EXPORTER_OTLP_PROTOCOL '{other}'; no OTLP metric exporter will be created"
            );
            None
        }
        None => {
            tracing::warn!(
                "no OTEL_EXPORTER_OTLP_ENDPOINT/OTEL_EXPORTER_OTLP_PROTOCOL set; no OTLP metric exporter will be created"
            );
            None
        }
    }?;

    Some(
        SdkMeterProvider::builder()
            .with_periodic_exporter(exporter)
            .with_resource(resource)
            .build(),
    )
}

/// Mirrors `init_tracing_opentelemetry::otlp::infer_protocol`'s decision (private in
/// that crate): explicit `OTEL_EXPORTER_OTLP_PROTOCOL` wins; otherwise, infer from the
/// endpoint's default port (`:4317` => grpc, anything else with an endpoint set =>
/// http/protobuf); no endpoint and no protocol => `None`.
fn infer_metrics_protocol() -> Option<String> {
    if let Ok(protocol) = std::env::var("OTEL_EXPORTER_OTLP_PROTOCOL") {
        return Some(protocol);
    }
    let endpoint = std::env::var("OTEL_EXPORTER_OTLP_ENDPOINT").ok()?;
    if endpoint.contains(":4317") {
        Some("grpc".to_string())
    } else {
        Some("http/protobuf".to_string())
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use std::env;
    use std::sync::Mutex;

    // Mirrors the ENV_LOCK pattern in `initialize.rs` — these tests mutate
    // process-wide env vars.
    static ENV_LOCK: Mutex<()> = Mutex::new(());

    fn reset_env() {
        env::remove_var("OTEL_EXPORTER_OTLP_PROTOCOL");
        env::remove_var("OTEL_EXPORTER_OTLP_ENDPOINT");
    }

    #[test]
    fn builds_meter_provider_with_http_protobuf_env() {
        let _guard = ENV_LOCK.lock().unwrap();
        reset_env();
        env::set_var("OTEL_EXPORTER_OTLP_PROTOCOL", "http/protobuf");
        env::set_var("OTEL_EXPORTER_OTLP_ENDPOINT", "http://localhost:4318");

        let resource = Resource::builder_empty().build();
        let result = build_otlp_meter_provider(resource);

        assert!(
            result.is_some(),
            "expected build_otlp_meter_provider to return Some(_) with OTEL_EXPORTER_OTLP_PROTOCOL=http/protobuf"
        );

        reset_env();
    }

    #[test]
    fn returns_none_without_any_otlp_env() {
        let _guard = ENV_LOCK.lock().unwrap();
        reset_env();

        let resource = Resource::builder_empty().build();
        let result = build_otlp_meter_provider(resource);

        assert!(
            result.is_none(),
            "expected build_otlp_meter_provider to return None when no OTLP env is set"
        );

        reset_env();
    }
}
```

- [x] **Step 2: Registrar o módulo em `tracex/mod.rs`**

Em `crates/derust/src/tracex/mod.rs`, adicione `mod otlp_metrics;` junto às outras
declarações de módulo (o módulo é privado — só `initialize.rs`, no mesmo crate, precisa
chamar `otlp_metrics::build_otlp_meter_provider`; não é `pub`, não faz parte da API
pública do derust):

```rust
mod initialize;
mod otlp_metrics;
pub mod log;

pub use initialize::*;
```

- [x] **Step 3: Rodar os novos testes isoladamente**

Rode: `cargo nextest run -p derust --features http_server -- otlp_metrics`
Esperado: `builds_meter_provider_with_http_protobuf_env` e
`returns_none_without_any_otlp_env` passam.

- [x] **Step 4: Rodar lint**

Rode: `cargo fmt --all -- --check && cargo clippy --features http_server -- -D warnings`
Esperado: sem erros. Se `MetricExporter`/`SdkMeterProvider` gerarem warning de import
não usado em outro lugar, confirme que não há duplicidade de import com o que a Task 4
vai adicionar em `initialize.rs` (ainda não deveria haver, pois `initialize.rs` só é
tocado na Task 4).

- [x] **Step 5: Commit**

```bash
git add crates/derust/src/tracex/otlp_metrics.rs crates/derust/src/tracex/mod.rs
git commit -m "feat(tracex): add OTLP metrics pipeline builder (not yet wired into init())"
```

---

### Task 3: Implementar pipeline OTLP de logs (`otlp_logs.rs`) e testes de regressão

**Dificuldade:** Médio

**Depende de:** Task 1 (pode rodar em paralelo com a Task 2 — nenhuma das duas toca
`initialize.rs` nem depende da outra; ambas só dependem das deps adicionadas na Task 1)

**Bloqueia:** Task 4

**Files:**
- Create: `crates/derust/src/tracex/otlp_logs.rs`
- Modify: `crates/derust/src/tracex/mod.rs` (adicionar `mod otlp_logs;`)

**Interfaces:**
- Consumes: `opentelemetry_sdk::{logs::SdkLoggerProvider, Resource}`,
  `opentelemetry_otlp::LogExporter` (Task 1).
- Produces: `pub(crate) fn build_otlp_logger_provider(resource: Resource) ->
  Option<SdkLoggerProvider>` — usada pela Task 4. Mesma semântica de
  `build_otlp_meter_provider` (Task 2): `None` + `tracing::warn!` em vez de erro.

**Contexto:** Estrutura idêntica à Task 2, trocando `MetricExporter`/`SdkMeterProvider`
por `LogExporter`/`SdkLoggerProvider`, e `with_periodic_exporter` por
`with_batch_exporter` (logs no `opentelemetry_sdk` usam processador em lote, não
`PeriodicReader` — API diferente confirmada em
`opentelemetry_sdk-0.30.0/src/logs/logger_provider.rs:226`).

- [x] **Step 1: Escrever o arquivo `otlp_logs.rs`**

```rust
use opentelemetry_otlp::LogExporter;
use opentelemetry_sdk::logs::SdkLoggerProvider;
use opentelemetry_sdk::Resource;

/// Builds an OTLP push pipeline for logs (`SdkLoggerProvider` backed by a batch log
/// processor), reading the same `OTEL_EXPORTER_OTLP_*` env vars already used for
/// traces. Returns `None` (after logging a warning) when no exporter can be built —
/// this must never fail `tracex::init()`'s boot, mirroring the existing behaviour for
/// traces.
pub(crate) fn build_otlp_logger_provider(resource: Resource) -> Option<SdkLoggerProvider> {
    let protocol = infer_logs_protocol();

    let exporter = match protocol.as_deref() {
        Some("http/protobuf") => match LogExporter::builder().with_http().build() {
            Ok(exporter) => Some(exporter),
            Err(error) => {
                tracing::warn!(
                    "failed to build OTLP log exporter (http/protobuf): {error}; no OTLP logs will be pushed"
                );
                None
            }
        },
        Some("grpc") => match LogExporter::builder().with_tonic().build() {
            Ok(exporter) => Some(exporter),
            Err(error) => {
                tracing::warn!(
                    "failed to build OTLP log exporter (grpc): {error}; no OTLP logs will be pushed"
                );
                None
            }
        },
        Some(other) => {
            tracing::warn!(
                "unknown OTEL_EXPORTER_OTLP_PROTOCOL '{other}'; no OTLP log exporter will be created"
            );
            None
        }
        None => {
            tracing::warn!(
                "no OTEL_EXPORTER_OTLP_ENDPOINT/OTEL_EXPORTER_OTLP_PROTOCOL set; no OTLP log exporter will be created"
            );
            None
        }
    }?;

    Some(
        SdkLoggerProvider::builder()
            .with_batch_exporter(exporter)
            .with_resource(resource)
            .build(),
    )
}

/// Mirrors `otlp_metrics::infer_metrics_protocol` — duplicated intentionally (small,
/// signal-specific, and each function only needs to know its own signal's env
/// fallback semantics; the alternative of sharing one generic helper was rejected to
/// keep each file readable on its own, consistent with the existing traces code in
/// `init_tracing_opentelemetry::otlp`, which does the same per-signal inference).
fn infer_logs_protocol() -> Option<String> {
    if let Ok(protocol) = std::env::var("OTEL_EXPORTER_OTLP_PROTOCOL") {
        return Some(protocol);
    }
    let endpoint = std::env::var("OTEL_EXPORTER_OTLP_ENDPOINT").ok()?;
    if endpoint.contains(":4317") {
        Some("grpc".to_string())
    } else {
        Some("http/protobuf".to_string())
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use std::env;
    use std::sync::Mutex;

    static ENV_LOCK: Mutex<()> = Mutex::new(());

    fn reset_env() {
        env::remove_var("OTEL_EXPORTER_OTLP_PROTOCOL");
        env::remove_var("OTEL_EXPORTER_OTLP_ENDPOINT");
    }

    #[test]
    fn builds_logger_provider_with_http_protobuf_env() {
        let _guard = ENV_LOCK.lock().unwrap();
        reset_env();
        env::set_var("OTEL_EXPORTER_OTLP_PROTOCOL", "http/protobuf");
        env::set_var("OTEL_EXPORTER_OTLP_ENDPOINT", "http://localhost:4318");

        let resource = Resource::builder_empty().build();
        let result = build_otlp_logger_provider(resource);

        assert!(
            result.is_some(),
            "expected build_otlp_logger_provider to return Some(_) with OTEL_EXPORTER_OTLP_PROTOCOL=http/protobuf"
        );

        reset_env();
    }

    #[test]
    fn returns_none_without_any_otlp_env() {
        let _guard = ENV_LOCK.lock().unwrap();
        reset_env();

        let resource = Resource::builder_empty().build();
        let result = build_otlp_logger_provider(resource);

        assert!(
            result.is_none(),
            "expected build_otlp_logger_provider to return None when no OTLP env is set"
        );

        reset_env();
    }
}
```

- [x] **Step 2: Registrar o módulo em `tracex/mod.rs`**

```rust
mod initialize;
mod otlp_logs;
mod otlp_metrics;
pub mod log;

pub use initialize::*;
```

(Se a Task 2 já rodou antes desta, o arquivo já vai ter `mod otlp_metrics;` — apenas
adicione a linha `mod otlp_logs;` junto, sem duplicar nada.)

- [x] **Step 3: Rodar os novos testes isoladamente**

Rode: `cargo nextest run -p derust --features http_server -- otlp_logs`
Esperado: `builds_logger_provider_with_http_protobuf_env` e
`returns_none_without_any_otlp_env` passam.

- [x] **Step 4: Rodar lint**

Rode: `cargo fmt --all -- --check && cargo clippy --features http_server -- -D warnings`
Esperado: sem erros.

- [x] **Step 5: Commit**

```bash
git add crates/derust/src/tracex/otlp_logs.rs crates/derust/src/tracex/mod.rs
git commit -m "feat(tracex): add OTLP logs pipeline builder (not yet wired into init())"
```

---

### Task 4: Integrar os dois pipelines em `tracex::init()` com um `Guard` combinado

**Dificuldade:** Complexo

**Depende de:** Task 2, Task 3 (usa as duas funções `build_otlp_*`)

**Bloqueia:** Task 5, Task 6

**Files:**
- Modify: `crates/derust/src/tracex/initialize.rs`

**Interfaces:**
- Consumes: `otlp_metrics::build_otlp_meter_provider(resource: Resource) ->
  Option<SdkMeterProvider>` (Task 2), `otlp_logs::build_otlp_logger_provider(resource:
  Resource) -> Option<SdkLoggerProvider>` (Task 3),
  `opentelemetry_appender_tracing::layer::OpenTelemetryTracingBridge::new(&logger_provider)`
  (dependência da Task 1), `init_tracing_opentelemetry::resource::DetectResource`
  (já disponível).
- Produces: `pub struct Guard { .. }` novo, público, substituindo `TracingGuard` no
  tipo de retorno de `pub fn init() -> Result<Guard, Box<dyn std::error::Error>>`. Faz
  flush+shutdown de trace/metrics/logs no `Drop`. Este é o tipo que passa a ser
  reexportado implicitamente por `tracex::mod.rs` via `pub use initialize::*;` (sem
  mudança necessária em `mod.rs` para isso, já é automático).

**Contexto:** Ver "Decisões técnicas tomadas nesta fase", item 3, para a justificativa
de introduzir `Guard` no lugar de `TracingGuard`. O `Resource` é construído **uma vez**
(`DetectResource::default().build()`) e reaproveitado nos três pipelines (trace já fazia
isso dentro de `build_otel_layer()`, que constrói seu próprio `Resource` internamente e
não o expõe — por isso métricas/logs constroem o *seu próprio* `Resource` idêntico via
uma segunda chamada a `DetectResource::default().build()`, já que `build_otel_layer()`
não devolve o `Resource` que usou para reutilização externa. As duas chamadas produzem
o mesmo resultado porque `DetectResource` lê as mesmas envs/detecção de ambiente em
ambos os casos — não há divergência de `service.name` entre os três sinais apesar de
serem duas chamadas separadas).

- [x] **Step 1: Ler o arquivo atual completo para confirmar as linhas exatas antes de editar**

Rode: `cat crates/derust/src/tracex/initialize.rs`
Confirme que bate com o conteúdo já lido nesta fase de refinamento (reproduzido em
"Achados da exploração de código" acima). Se divergir, pare e reavalie antes de editar.

- [x] **Step 2: Reescrever o topo do arquivo (imports, `init()`, novo `Guard`)**

Substitua as linhas 1 a 22 do arquivo atual (do primeiro `use` até o fechamento de
`init()`, ou seja, tudo antes de `const DERUST_OTEL_DEBUG_ENV_NAME`) por:

```rust
use init_tracing_opentelemetry::resource::DetectResource;
use init_tracing_opentelemetry::tracing_subscriber_ext::{build_otel_layer, TracingGuard};
use opentelemetry_appender_tracing::layer::OpenTelemetryTracingBridge;
use opentelemetry_sdk::logs::SdkLoggerProvider;
use opentelemetry_sdk::metrics::SdkMeterProvider;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::{fmt, EnvFilter};

use crate::tracex::otlp_logs::build_otlp_logger_provider;
use crate::tracex::otlp_metrics::build_otlp_meter_provider;

pub fn init() -> Result<Guard, Box<dyn std::error::Error>> {
    let subscriber = tracing_subscriber::registry()
        .with(build_loglevel_filter_layer())
        .with(fmt::layer());

    let _guard = tracing::subscriber::set_default(subscriber);

    let (trace_layer, trace_guard) = build_otel_layer()?;

    let resource = DetectResource::default().build();
    let meter_provider = build_otlp_meter_provider(resource.clone());
    if let Some(meter_provider) = &meter_provider {
        opentelemetry::global::set_meter_provider(meter_provider.clone());
    }
    let logger_provider = build_otlp_logger_provider(resource);
    let otlp_log_layer = logger_provider
        .as_ref()
        .map(|provider| OpenTelemetryTracingBridge::new(provider));

    let subscriber = tracing_subscriber::registry()
        .with(trace_layer)
        .with(otlp_log_layer)
        .with(build_loglevel_filter_layer())
        .with(fmt::layer());

    tracing::subscriber::set_global_default(subscriber)?;

    Ok(Guard {
        trace_guard,
        meter_provider,
        logger_provider,
    })
}

/// On Drop, flushes and shuts down every OTLP pipeline that was actually created
/// (trace is always created — see `build_otel_layer()` — metrics/logs are `Option`
/// because they are opt-in by env, per this crate's `http_server`/`tracex` design).
/// Errors from `force_flush`/`shutdown` are intentionally ignored here, mirroring
/// `TracingGuard`'s existing `Drop` behaviour for traces (`init-tracing-opentelemetry`
/// v0.29.0, `tracing_subscriber_ext.rs:129-132`): shutdown must never panic during
/// application termination.
#[must_use = "Recommend holding with 'let _guard = ' pattern to ensure final traces/metrics/logs are sent to the server"]
pub struct Guard {
    trace_guard: TracingGuard,
    meter_provider: Option<SdkMeterProvider>,
    logger_provider: Option<SdkLoggerProvider>,
}

impl Guard {
    /// The wrapped trace guard (kept for backward-compatible access to the tracer
    /// provider, same accessor pattern as the previous `TracingGuard::tracer_provider()`).
    #[must_use]
    pub fn trace_guard(&self) -> &TracingGuard {
        &self.trace_guard
    }
}

impl Drop for Guard {
    fn drop(&mut self) {
        if let Some(meter_provider) = &self.meter_provider {
            let _ = meter_provider.force_flush();
            let _ = meter_provider.shutdown();
        }
        if let Some(logger_provider) = &self.logger_provider {
            let _ = logger_provider.force_flush();
            let _ = logger_provider.shutdown();
        }
        // `self.trace_guard` is dropped automatically right after this block, which
        // triggers its own `Drop` impl (force_flush + shutdown of the trace pipeline) —
        // no explicit call needed here.
    }
}
```

- [x] **Step 3: Confirmar que o resto do arquivo (a partir de `const DERUST_OTEL_DEBUG_ENV_NAME`) não precisa de nenhuma mudança**

`build_loglevel_filter_layer()` e seus testes existentes continuam idênticos — não
tocar nessa parte do arquivo nesta task.

- [x] **Step 4: Adicionar os 3 testes de regressão pedidos pelo plano de negócio, ao final do `mod test` existente**

Adicione estes três testes dentro do `mod test` já existente (mesmo bloco dos testes
atuais, reaproveitando `ENV_LOCK`/`reset_env` já definidos ali):

```rust
    #[test]
    fn builds_otlp_metrics_pipeline_without_error_with_http_protobuf_env() {
        let _guard = ENV_LOCK.lock().unwrap();
        reset_env();
        env::set_var("OTEL_EXPORTER_OTLP_PROTOCOL", "http/protobuf");
        env::set_var("OTEL_EXPORTER_OTLP_ENDPOINT", "http://localhost:4318");

        let result = init();

        assert!(
            result.is_ok(),
            "expected tracex::init() to succeed when OTLP metrics env is set, got: {:?}",
            result.err().map(|error| error.to_string())
        );

        reset_env();
    }

    #[test]
    fn builds_otlp_logs_pipeline_without_error_with_http_protobuf_env() {
        let _guard = ENV_LOCK.lock().unwrap();
        reset_env();
        env::set_var("OTEL_EXPORTER_OTLP_PROTOCOL", "http/protobuf");
        env::set_var("OTEL_EXPORTER_OTLP_ENDPOINT", "http://localhost:4318");

        let result = init();

        assert!(
            result.is_ok(),
            "expected tracex::init() to succeed when OTLP logs env is set, got: {:?}",
            result.err().map(|error| error.to_string())
        );

        reset_env();
    }

    #[test]
    fn does_not_break_boot_when_no_otlp_env_is_set() {
        let _guard = ENV_LOCK.lock().unwrap();
        reset_env();

        let result = init();

        assert!(
            result.is_ok(),
            "expected tracex::init() to succeed (Ok, with all OTLP pipelines as None) when no OTLP env is set, got: {:?}",
            result.err().map(|error| error.to_string())
        );

        reset_env();
    }
```

Nota: os dois primeiros testes ficam praticamente idênticos entre si e ao teste já
existente `builds_otlp_http_protobuf_exporter_without_client_conflict`, porque as
mesmas env vars (`OTEL_EXPORTER_OTLP_ENDPOINT`/`OTEL_EXPORTER_OTLP_PROTOCOL`) disparam
os três pipelines simultaneamente — não há como testar "só métricas" ou "só logs"
isoladamente via `init()` sem também acionar traces, já que os três leem a mesma env
padrão. A cobertura granular por sinal já existe nos testes unitários das Tasks 2/3
(`otlp_metrics.rs`/`otlp_logs.rs`), que testam `build_otlp_meter_provider`/
`build_otlp_logger_provider` isoladamente; estes três testes aqui cobrem especificamente
o requisito do plano de negócio de que `init()` "builda sem erro" ponta a ponta.

- [x] **Step 5: Rodar a suíte de testes do módulo `tracex`**

Rode: `cargo nextest run -p derust --features http_server -- tracex`
Esperado: todos os testes de `tracex` (existentes + novos) passam, 0 falhas.

- [x] **Step 6: Rodar lint**

Rode: `cargo fmt --all -- --check && cargo clippy --features http_server -- -D warnings`
Esperado: sem erros.

- [x] **Step 7: Build e teste dos exemplos que usam `tracex::init()`**

Rode: `cd examples/trace && cargo build` (confirma que `let _guard = tracex::init();`
continua compilando sem anotação de tipo — ver "Decisões técnicas tomadas nesta fase",
item 3)
Esperado: build sem erros.

- [x] **Step 8: Commit**

```bash
git add crates/derust/src/tracex/initialize.rs
git commit -m "feat(tracex): wire OTLP metrics and logs pipelines into init(), replace TracingGuard with combined Guard"
```

---

### Task 5: Documentar em `tracex/README.md` o novo comportamento (opt-in, envs, riscos)

**Dificuldade:** Simples

**Depende de:** Task 4 (documenta o resultado final já integrado)

**Bloqueia:** Task 6

**Files:**
- Modify: `crates/derust/src/tracex/README.md`

**Interfaces:**
- Consumes: comportamento final de `tracex::init()` produzido na Task 4.
- Produces: documentação atualizada, sem código de exemplo novo obrigatório (o
  `main.rs` de `examples/trace` não muda de conteúdo, só se beneficia do novo
  comportamento automaticamente).

**Contexto:** O README atual (48 linhas, reproduzido em "Achados da exploração de
código" da fase de refinamento anterior, mas não citado aqui por já ter sido lido
diretamente nesta fase) documenta só o `_guard = tracex::init()` para traces + logs
locais/B3 traceparent. Adicione, ao final do arquivo, uma nova seção:

- [ ] **Step 1: Ler o README atual**

Rode: `cat crates/derust/src/tracex/README.md`

- [ ] **Step 2: Adicionar a seção "OTLP push for metrics and logs" ao final do arquivo**

Acrescente este bloco ao final de `crates/derust/src/tracex/README.md` (mantendo todo
o conteúdo existente acima intacto):

````markdown

## OTLP push for metrics and logs

Besides traces (already pushed via OTLP when `OTEL_EXPORTER_OTLP_ENDPOINT` /
`OTEL_EXPORTER_OTLP_PROTOCOL` are set), `tracex::init()` also builds OTLP push
pipelines for **metrics** and **logs**, using the exact same env vars — no extra
configuration needed:

- `OTEL_EXPORTER_OTLP_ENDPOINT`
- `OTEL_EXPORTER_OTLP_PROTOCOL` (`http/protobuf` or `grpc`)
- `OTEL_EXPORTER_OTLP_HEADERS` (read automatically by the exporter builders)

If these env vars are not set, `tracex::init()` behaves exactly as before: no OTLP
metrics/logs exporter is created, only a `tracing::warn!` is logged, and the
application boots normally.

### Coexists with the `metricx` Prometheus/StatsD pull path

This does **not** replace `metricx`'s `GET /metrics` (Prometheus) or StatsD push —
those keep working exactly as before, independently. You can enable OTLP push, the
existing pull/StatsD path, both, or neither.

**Important:** metrics instrumented today via the `metrics` crate (the macros used
internally by `metricx`, e.g. `counter!`/`histogram!`) are **not** automatically
forwarded to the OTLP `MeterProvider` — there is no maintained bridge between the
`metrics` crate and `opentelemetry::metrics` today. If you need custom metrics pushed
via OTLP, instrument them directly with the `opentelemetry::metrics` API (via
`opentelemetry::global::meter(...)`, after `tracex::init()` has run) — this means
double instrumentation if you also want the same metric on the Prometheus/StatsD path.
This is a known, documented limitation, not a bug.

### Logs

Every log emitted via `derust::tracex::log::*` (or any `tracing` macro) is
automatically captured by the OTLP logs pipeline once it is enabled — no code change
needed. Logs keep going to stdout as well (unchanged, via `fmt::layer()`); the two
outputs carry the same content and level.

### Cost warning

Enabling OTLP push for metrics and logs increases the volume of data sent to your
observability backend (e.g. Grafana Cloud). Review your backend's ingestion pricing
before enabling this in production.
````

- [ ] **Step 3: Revisar o diff**

Rode: `git diff crates/derust/src/tracex/README.md`
Confirme que o conteúdo anterior do arquivo não foi alterado, só a nova seção foi
adicionada ao final.

- [ ] **Step 4: Commit**

```bash
git add crates/derust/src/tracex/README.md
git commit -m "docs(tracex): document OTLP push for metrics and logs"
```

---

### Task 6: CHANGELOG, bump de versão e validação final do crate

**Dificuldade:** Simples

**Depende de:** Task 1, Task 2, Task 3, Task 4, Task 5 (fecha o plano)

**Bloqueia:** nada (última tarefa do plano)

**Files:**
- Create: `CHANGELOG.md` (raiz do workspace — não existe hoje)
- Modify: `crates/derust/Cargo.toml`

**Interfaces:**
- Consumes: nada de código — fecha o plano com documentação de release e validação
  full-suite.
- Produces: `derust` pronto para publicação em `0.6.0` (o `cargo publish` em si é
  manual, ver Step 5 abaixo).

**Contexto:** Ver "Achados da exploração de código" para a justificativa do bump minor
(`0.5.0` → `0.6.0`, feature aditiva) e da ausência de `CHANGELOG.md` prévio.

- [ ] **Step 1: Criar `CHANGELOG.md`**

Crie `CHANGELOG.md` na raiz do workspace com este conteúdo:

```markdown
# Changelog

All notable changes to `derust` are documented in this file.

## [0.6.0]

### Added

- `tracex::init()` now also builds OTLP push pipelines for **metrics** and **logs**,
  reusing the same `OTEL_EXPORTER_OTLP_ENDPOINT`/`OTEL_EXPORTER_OTLP_PROTOCOL`/
  `OTEL_EXPORTER_OTLP_HEADERS` env vars already used for traces. Opt-in by env
  detection only (no new Cargo feature): when those env vars are not set, behaviour is
  unchanged — no exporter is created, only a warning is logged. Coexists with the
  existing `metricx` Prometheus/StatsD pull path, which is unaffected.
- `tracex::init()`'s return type changes from the external `TracingGuard` (from
  `init-tracing-opentelemetry`) to a new `tracex::Guard`, which additionally flushes
  and shuts down the metrics/logs OTLP pipelines on `Drop`. The documented usage
  pattern (`let _guard = tracex::init()?;`) is unaffected.

### Notes

- Metrics instrumented via the `metrics` crate (used internally by `metricx`) are not
  automatically forwarded to the new OTLP `MeterProvider` — see
  `crates/derust/src/tracex/README.md` for details.
```

- [ ] **Step 2: Atualizar a versão no Cargo.toml do crate**

Em `crates/derust/Cargo.toml`, altere:

```toml
version = "0.5.0"
```

para:

```toml
version = "0.6.0"
```

- [ ] **Step 3: Rodar a suíte completa de testes do crate**

Rode: `task test` (equivalente a `cargo nextest run`)
Esperado: todos os testes passam, 0 falhas (inclui os testes novos das Tasks 2, 3 e 4).

- [ ] **Step 4: Rodar lint completo do workspace**

Rode: `task lint` (equivalente a `cargo fmt --all -- --check && cargo clippy -- -D
warnings`)
Esperado: sem erros. Se houver débito técnico pré-existente e não relacionado a este
plano (precedente documentado em
`docs/superpowers/plans/2026-09-20-migracao-growthbook-nativo.md`, seção "Nota de
execução"), confirme isoladamente que os arquivos tocados por este plano
(`crates/derust/src/tracex/*`, `Cargo.toml`, `crates/derust/Cargo.toml`,
`CHANGELOG.md`) não fazem parte desse débito antes de prosseguir, e registre a mesma
ressalva aqui se aplicável — não é responsabilidade deste plano corrigir débito técnico
de outros módulos.

- [ ] **Step 5: Build de todos os exemplos (checagem final de ponta a ponta)**

Rode, a partir da raiz do workspace: `cargo build --workspace` e, para cada diretório em
`examples/*` (que são crates standalone fora do workspace principal): `cd examples/trace
&& cargo build && cd -` (repita para `examples/basic`, `examples/metrics`, se aplicável
— confirme com `ls examples/` quais realmente dependem de `tracex`/`http_server`, já que
alguns exemplos podem não usar tracing diretamente).
Esperado: build sem erros em todos.

- [ ] **Step 6: Commit**

```bash
git add CHANGELOG.md crates/derust/Cargo.toml
git commit -m "chore: bump derust version to 0.6.0, add CHANGELOG (OTLP push for metrics and logs)"
```

- [ ] **Step 7: Publicação no crates.io (ação manual, fora do escopo de execução automatizada)**

A publicação (`cargo publish -p derust` a partir de `crates/derust`) requer
credenciais de `cargo login` da conta do mantenedor (`diogoderoldo@gmail.com`), que não
estão disponíveis para um agente de implementação. Após esta task ser revisada e
mergeada, a publicação deve ser feita manualmente pelo mantenedor. Não marque este
step como concluído automaticamente — sinalize explicitamente para quem revisar o PR
que este é o único passo restante do critério de sucesso do plano de negócio
("Nova versão minor do derust publicada no crates.io").

---

## Plano de testes e revisão por tarefa (resumo)

| Tarefa | Plano de testes | Plano de revisão |
|---|---|---|
| 1 — dependências novas | `cargo build --features http_server`; checagem manual de `with_tonic()` compilando (Step 5) | Confirmar que `Cargo.lock` não resolveu uma segunda versão de nenhuma crate `opentelemetry*` (`grep -c "^name = \"opentelemetry" Cargo.lock` mudando só se uma versão nova genuinamente nova, como `opentelemetry-appender-tracing`, foi introduzida) |
| 2 — pipeline de métricas | `cargo nextest run -- otlp_metrics` (2 testes: builda com env HTTP, retorna `None` sem env) | Confirmar que a função nunca `panic!`/propaga erro, só retorna `Option` + `warn!` |
| 3 — pipeline de logs | `cargo nextest run -- otlp_logs` (2 testes, mesmo padrão da Task 2) | Mesma checagem da Task 2, mais confirmar `with_batch_exporter` (não `with_periodic_exporter`, que é específico de métricas) |
| 4 — integração em `init()` + `Guard` | `cargo nextest run -- tracex` (inclui os 3 novos testes de regressão pedidos pelo plano de negócio + os 4 testes já existentes) + build de `examples/trace` | Confirmar que `let _guard = tracex::init();` (padrão documentado, sem anotação de tipo) continua compilando; revisar `Drop` de `Guard` chamando shutdown nos 3 providers |
| 5 — documentação | Revisão textual | Confirmar que a seção nova reflete exatamente o comportamento implementado na Task 4 (envs, coexistência com `metricx`, limitação do bridge `metrics`→OTLP) |
| 6 — CHANGELOG, versão, validação final | `task test`, `task lint`, build de todos os exemplos relevantes | Confirmar version bump `0.5.0`→`0.6.0`; confirmar que o Step 7 (publicação manual) está claramente sinalizado como pendência humana, não como falha de execução |

## Paralelização

- Task 1 é sempre a primeira (bloqueia 2 e 3).
- Task 2 e Task 3 podem rodar **em paralelo** entre si (subagentes distintos) — nenhuma
  depende da outra, ambas só dependem da Task 1, e tocam arquivos diferentes
  (`otlp_metrics.rs` vs `otlp_logs.rs`), com uma única linha de conflito potencial em
  `tracex/mod.rs` (cada uma adiciona sua própria linha `mod otlp_*;` — se rodarem em
  paralelo de verdade em duas branches/worktrees, o merge desse arquivo específico
  precisa ser resolvido manualmente, mas o conteúdo não é conflitante).
- Task 4 depende de 2 e 3 (integra as duas funções). Não pode começar antes de ambas
  terminarem.
- Task 5 depende de 4 (documenta o comportamento final).
- Task 6 depende de todas as anteriores (fecha o plano).
- Não há oportunidade de paralelismo além do par (Task 2, Task 3) — o restante é uma
  cadeia linear.
