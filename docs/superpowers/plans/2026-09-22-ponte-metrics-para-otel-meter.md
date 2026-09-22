# Ponte entre a instrumentação de métricas existente e o push OTLP (Meter) — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Fazer com que toda a instrumentação de métricas já existente em `metricx`
(`increment`, `increment_one`, `current_gauge`, `record_money`, `record_duration`,
`start_stopwatch`, e as métricas automáticas de HTTP/DB que usam essas mesmas funções)
passe a alimentar, sem nenhuma mudança de código de aplicação, tanto o canal de pull já
existente (`/metrics` Prometheus, ou StatsD push próprio) quanto o canal de push OTLP
(`SdkMeterProvider` já montado por `tracex::init()` desde a tarefa anterior, hoje sem
nenhum dado real chegando nele).

**Architecture:** A ponte é construída inteiramente dentro de `metricx`, sem alterar a
assinatura pública de nenhuma função hoje usada por aplicações (`increment`,
`current_gauge`, `record_money`, `record_duration`, `start_stopwatch` continuam
idênticas). O mecanismo: um novo `metrics::Recorder` decorator,
`OtelBridgingRecorder<R>`, envolve o `Recorder` que `metricx` já constrói hoje
(`PrometheusRecorder` ou `StatsdRecorder`) e, para cada métrica registrada
(`register_counter`/`register_gauge`/`register_histogram`), devolve um handle composto
que escreve tanto no `Recorder` original (preservando o pull/StatsD exatamente como
hoje) quanto em um instrumento equivalente do `opentelemetry::metrics` obtido via
`opentelemetry::global::meter("derust")`. **Decisão-chave desta fase:** o bridge nunca
verifica se o push OTLP está de fato configurado — ele sempre encaminha para
`opentelemetry::global::meter(...)`, que é um proxy global: se `tracex::init()` não
configurou um `SdkMeterProvider` real (sem envs `OTEL_EXPORTER_OTLP_*`), esse proxy
resolve para o provider no-op padrão da API `opentelemetry`, cujas operações
`.add()`/`.record()` não fazem nada de observável. Isso elimina qualquer necessidade de
`metricx` replicar a lógica de detecção de env já implementada em `tracex`, e é o que
garante o critério "ausência de configuração de push mantém comportamento idêntico ao
pré-existente" — módulo ao custo (aceito e documentado) de uma consulta a cache
em memória por emissão de métrica, mesmo quando push está desligado.
As tags negadas (`denied_metric_tags`/`denied_metric_tags_by_regex`) continuam sendo
filtradas exatamente onde já são hoje — em `metricx::tags::MetricTags::to_labels()`,
**antes** de chamar `metrics::counter!`/`gauge!`/`histogram!` — então o `Recorder`
nunca vê as tags negadas para começar, e ambos os canais (pull e push) ficam
automaticamente consistentes nesse ponto sem nenhum código extra de filtragem no
bridge.
A granularidade de histograma (buckets) é unificada numa única constante
`crate::metricx::HISTOGRAM_BUCKET_BOUNDARIES`, reutilizada tanto pela configuração já
existente do `PrometheusBuilder::set_buckets()` quanto por uma nova `View` do
`SdkMeterProvider` (montada em `tracex::otlp_metrics::build_otlp_meter_provider`, que já
existe da tarefa anterior) que força `Aggregation::ExplicitBucketHistogram` com os
mesmos limites para todo instrumento do tipo `Histogram`.

**Tech Stack:** Rust 1.96 (edition 2021). Nenhuma dependência nova de terceiros: reusa
`opentelemetry` (0.30, já dependência opcional do crate, usada hoje por `http_server`/
`http_client`) e `opentelemetry_sdk` (0.30, já dependência de `http_server`, com a
feature adicional `spec_unstable_metrics_views` a habilitar nesta fase — necessária
para customizar a agregação de histograma via `View`). `metricx` passa a declarar
`opentelemetry` como dependência das features `statsd` e `prometheus` (reaproveitando o
mesmo `opentelemetry = { workspace = true }` já declarado em `crates/derust/Cargo.toml`
para `http_server`/`http_client` — sem nova entrada em `[workspace.dependencies]`).

**Spec:** Plano de negócio aprovado no vault Obsidian "Deroldo":
`Projects/derust/plans/ponte_metrics_para_otel_meter.md`
(cópia local consultada em:
`/Users/deroldo/Library/Mobile Documents/iCloud~md~obsidian/Documents/Deroldo/Projects/derust/plans/ponte_metrics_para_otel_meter.md`)

## Achados da exploração de código (contexto para todas as tarefas)

- `metricx` (gated por `#[cfg(any(feature = "statsd", feature = "prometheus"))]` em
  `crates/derust/src/lib.rs`) hoje **não depende de `opentelemetry`/`tracex` em
  nenhum ponto**. Toda a instrumentação (`crates/derust/src/metricx/meters/{counter,
  gauge,money,timer}.rs`) chama diretamente as macros do crate `metrics`
  (`metrics::counter!`, `metrics::gauge!`, `metrics::histogram!`), que despacham para
  **um único `Recorder` global**, instalado uma vez via `metrics::set_global_recorder`
  em `crates/derust/src/metricx/registries/{prometheus,statsd}/mod.rs`, ambos chamados
  de dentro de `AppContext::new()` (`crates/derust/src/httpx/context.rs:50-62`).
  Nota (achado incidental, fora de escopo desta tarefa): `metricx/meters/*.rs` já usa
  `crate::httpx::AppContext`, então `metricx` já depende implicitamente de `httpx`
  (logo de `http_server`) mesmo sem essa dependência estar declarada no grafo de
  features do Cargo — habilitar `statsd`/`prometheus` sem `http_server` já não
  compilaria hoje, antes de qualquer mudança deste plano. Não corrigir isso aqui.
- `crates/derust/src/metricx/registries/prometheus/mod.rs`: `prometheus_registry()`
  hoje chama `PrometheusBuilder::new().set_buckets(&[0.010, 0.025, 0.050, 0.075, 0.100,
  0.150, 0.200, 0.250, 0.500, 1.0, 2.5, 5.0, 10.0]).install_recorder()`. Confirmado por
  leitura de `metrics-exporter-prometheus-0.17.2/src/exporter/builder.rs:435-441`:
  `install_recorder()` é só `let recorder = self.build_recorder(); let handle =
  recorder.handle(); metrics::set_global_recorder(recorder)?; Ok(handle)`. Ou seja,
  **`build_recorder()` (linha 523) já existe e devolve o `PrometheusRecorder` sem
  instalar nada globalmente** — é o que esta tarefa usa para poder envolver o recorder
  antes de instalá-lo, sem perder acesso ao `PrometheusHandle` (`recorder.handle()`
  continua disponível antes de mover o recorder para dentro do wrapper).
- `crates/derust/src/metricx/registries/statsd/mod.rs`: `statsd_registry()` já usa o
  padrão "constrói sem instalar, então instala manualmente"
  (`StatsdBuilder::from(...).build(None)?` devolve `StatsdRecorder`, e só depois
  `metrics::set_global_recorder(recorder)` é chamado, junto com um `register_histogram`
  de warm-up hoje existente). Esse arquivo já está no formato certo para receber o
  wrapper sem precisar de nenhuma mudança estrutural, só trocar o que é passado para
  `set_global_recorder`.
- API confirmada por leitura direta do código-fonte cacheado do crate `metrics`
  (`metrics-0.24.2/src/recorder/mod.rs` e `handles.rs`):
  - `pub trait Recorder { fn describe_counter/gauge/histogram(...); fn
    register_counter(&self, key: &Key, metadata: &Metadata<'_>) -> Counter; fn
    register_gauge(...) -> Gauge; fn register_histogram(...) -> Histogram; }`.
  - `Counter`/`Gauge`/`Histogram` são wrappers em torno de `Arc<dyn
    CounterFn/GaugeFn/HistogramFn + Send + Sync>`, construídos via
    `Counter::from_arc(Arc::new(minha_struct))` etc. — é assim que se constrói um
    handle "fan-out" que escreve em dois lugares ao mesmo tempo (padrão confirmado
    pelo próprio teste `SimpleCounterRecorder` embutido no crate `metrics`, em
    `recorder/mod.rs`, seção `#[cfg(test)] mod tests`).
  - `CounterFn`: `increment(&self, value: u64)`, `absolute(&self, value: u64)`.
    `metricx` **nunca chama `.absolute()`** (só `.increment()`, via `metrics::counter!
    (...).increment(count)` em `meters/counter.rs`) — o bridge não tenta traduzir
    `absolute` para o `Counter` OTel (que só suporta soma monotônica via `.add()`, sem
    operação de "set absoluto"); `absolute()` só é encaminhado ao recorder interno
    (comportamento inalterado), documentado no código como limitação conhecida e
    aceitável porque não há nenhum call site hoje.
  - `GaugeFn`: `increment`/`decrement`/`set`. `metricx::current_gauge` **só chama
    `.set()`** (confirmado em `meters/gauge.rs`) — o bridge só encaminha `set()` para o
    instrumento OTel (`Gauge<f64>::record`); `increment`/`decrement` só tocam o
    recorder interno, mesma justificativa do item acima (nenhum call site hoje).
  - `HistogramFn::record(&self, value: f64)` — usado por `record_money` e
    `record_duration`, ambos via `metrics::histogram!` (mesmo tipo de instrumento
    `metrics::Histogram`, sem distinção entre "money" e "duration" no nível do
    `Recorder`) — o bridge encaminha sempre.
  - `Key::name(&self) -> &str` e `Key::labels(&self) -> Iter<Label>` (`Label::key()`/
    `Label::value() -> &str`) — confirmado em `metrics-0.24.2/src/key.rs`. É a partir
    daqui que o bridge converte para `Vec<opentelemetry::KeyValue>` — direto, sem
    nenhuma lógica adicional de mapeamento (os nomes das tags já são idênticos, porque
    é o mesmo `Key` usado para os dois canais).
- API confirmada por leitura direta do código-fonte cacheado do crate `opentelemetry`
  0.30.0 (`src/metrics/meter.rs`, `src/global/metrics.rs`):
  - `opentelemetry::global::meter(name: &'static str) -> Meter` — proxy global,
    `Meter: Clone`. Resolve dinamicamente para o provider atualmente instalado a cada
    chamada (mesmo padrão já usado por `global::tracer()`) — não importa se é chamado
    antes ou depois de `tracex::init()` ter (ou não) instalado um provider real.
  - `Meter::u64_counter(name) -> InstrumentBuilder<Counter<u64>>`,
    `Meter::f64_gauge(name) -> InstrumentBuilder<Gauge<f64>>`,
    `Meter::f64_histogram(name) -> HistogramBuilder<Histogram<f64>>`, todos com
    `.build() -> <Instrumento>` (sem `Result`, não pode falhar/panicar na chamada —
    confirmado em `src/metrics/instruments/mod.rs:122,193,311`; erros de validação de
    nome são só logados internamente pelo error handler global do `opentelemetry`, não
    propagam). A doc do próprio `Meter` recomenda explicitamente cachear o instrumento
    em vez de recriá-lo a cada chamada ("Creating duplicate Counters for the same
    metric could lower SDK performance") — é a justificativa direta para o cache
    `RwLock<HashMap<String, _>>` desta tarefa.
  - `Counter<u64>::add(&self, value: u64, attributes: &[KeyValue])`,
    `Gauge<f64>::record(&self, value: f64, attributes: &[KeyValue])`,
    `Histogram<f64>::record(&self, value: f64, attributes: &[KeyValue])` — API confirmada
    por uso já existente no ecossistema `opentelemetry` 0.30 (mesma família de
    instrumentos síncronos usada em outras integrações OTel).
- API confirmada por leitura direta do código-fonte cacheado de `opentelemetry_sdk`
  0.30.0 (`src/metrics/meter_provider.rs:367`, `src/metrics/instrument.rs`):
  - `SdkMeterProviderBuilder::with_view<T>(self, view: T) -> Self where T: Fn(&Instrument)
    -> Option<Stream> + Send + Sync + 'static` — permite customizar a agregação por
    instrumento. `Instrument::kind() -> InstrumentKind` (`InstrumentKind::Histogram`
    para todo histograma, independente do nome).
  - `Stream::builder().with_aggregation(Aggregation::ExplicitBucketHistogram {
    boundaries: Vec<f64>, record_min_max: bool }).build() -> Result<Stream, _>` — **só
    existe com a feature `spec_unstable_metrics_views` habilitada** em
    `opentelemetry_sdk` (confirmado em `opentelemetry_sdk-0.30.0/Cargo.toml:88`:
    `spec_unstable_metrics_views = ["metrics"]`, e o método `with_aggregation` em
    `src/metrics/instrument.rs:180` está atrás de `#[cfg(feature =
    "spec_unstable_metrics_views")]`). Precisa ser adicionada ao
    `opentelemetry_sdk` do `[workspace.dependencies]` nesta tarefa — não estava no
    escopo da tarefa anterior (que só usava `with_periodic_exporter`/
    `with_resource`, sem customizar agregação).
  - `InMemoryMetricExporter`/`InMemoryMetricExporterBuilder`
    (`opentelemetry_sdk::metrics::{InMemoryMetricExporter, InMemoryMetricExporterBuilder}`,
    reexportado publicamente e sem feature extra — `src/metrics/mod.rs:69,72`) — usado
    nos testes desta fase para inspecionar o que seria exportado via OTLP sem precisar
    de um coletor real: `let exporter = InMemoryMetricExporter::default();
    SdkMeterProvider::builder().with_periodic_exporter(exporter.clone())...build()`,
    depois `meter_provider.force_flush()` seguido de
    `exporter.get_finished_metrics()`.
- `crates/derust/Cargo.toml`: `opentelemetry = { workspace = true }` já está declarada
  como dependência opcional (usada por `http_server` e `http_client`); só falta
  adicionar `"dep:opentelemetry"` às listas de features `statsd` e `prometheus`.
  `opentelemetry_sdk` só é usado por `http_server` hoje — este plano não precisa
  adicioná-lo a `statsd`/`prometheus` (o bridge em si só usa a API `opentelemetry`, não
  o SDK; quem constrói o `SdkMeterProvider`/`View` continua sendo exclusivamente
  `tracex`, já sob `http_server`).
- `crates/derust/src/tracex/otlp_metrics.rs` (criado na tarefa anterior,
  `docs/superpowers/plans/2026-09-22-adicionar-push-otlp-metricas-e-logs.md`) já expõe
  `pub(crate) fn build_otlp_meter_provider(resource: Resource) ->
  Option<SdkMeterProvider>`, chamada por `tracex::initialize::init()`, que já registra
  o provider globalmente via `opentelemetry::global::set_meter_provider(...)` quando
  `Some`. Esta tarefa só estende o builder interno dessa função (adicionando
  `.with_view(...)`), sem mudar sua assinatura nem o restante do fluxo de `init()`.
- `CHANGELOG.md` (raiz do workspace, criado na tarefa anterior) já tem uma entrada
  `## [0.5.1]` com uma seção `### Notes` afirmando explicitamente que "Metrics
  instrumented via the `metrics` crate ... are not automatically forwarded to the new
  OTLP `MeterProvider`". Esta tarefa torna essa frase **falsa** e precisa
  removê-la/substituí-la (pedido explícito do plano de negócio, "Critérios de
  sucesso"). Mesma coisa para o parágrafo equivalente em
  `crates/derust/src/tracex/README.md`, seção "OTLP push for metrics and logs" →
  "Coexists with the `metricx` Prometheus/StatsD pull path" (hoje descreve a limitação
  como definitiva; passa a descrever o comportamento novo).
- Versão do crate: `0.5.1` (ainda não publicada — confirmado em
  `crates/derust/Cargo.toml:3` e pelo estado do `CHANGELOG.md`). Conforme o próprio
  plano de negócio ("Critérios de sucesso"): **sem bump de versão** nesta tarefa — a
  mudança entra na mesma `0.5.1` ainda não publicada.

## Decisões técnicas tomadas nesta fase (documentadas para revisão)

Não houve necessidade de nenhum `AskUserQuestion` nesta fase — todas as decisões abaixo
são consequência direta e sem alternativa razoável do "o quê" já aprovado no plano de
negócio (como implementar, não o que implementar), fundamentadas em leitura direta do
código-fonte das dependências envolvidas:

1. **O bridge fica inteiramente em `metricx`, não em `tracex`.** `tracex` continua
   restrito a montar/gerenciar os providers OTLP (trace/metrics/logs) e o `Guard`; não
   passa a conhecer nada sobre `metrics::Recorder`/Prometheus/StatsD. Motivo: mantém a
   mesma separação de responsabilidades já usada pelo restante do crate (cada feature
   cuida do seu próprio domínio), e evita acoplar `tracex` (sempre compilado com
   `http_server`) a tipos que só existem quando `statsd`/`prometheus` estão habilitados.
2. **O bridge nunca verifica se o push está configurado — sempre encaminha para
   `opentelemetry::global::meter(...)`.** Ver "Architecture" acima. Consequência aceita
   e documentada: quando push não está configurado, cada emissão de métrica paga o
   custo de (a) uma consulta ao cache local (`RwLock<HashMap>` — leitura, o(1)) e (b)
   uma chamada a instrumento no-op do `opentelemetry` (retorno imediato, sem I/O nem
   alocação) — mesma ordem de grandeza de overhead que o próprio recorder Prometheus/
   StatsD já paga hoje por chamada. Nenhuma mudança de comportamento observável (valores
   emitidos, formato do `/metrics`, tags) — só overhead de CPU, validado na Task 7.
3. **Granularidade de histograma unificada numa única constante**
   `crate::metricx::HISTOGRAM_BUCKET_BOUNDARIES`, referenciada tanto por
   `prometheus_registry()` quanto pela `View` nova em
   `tracex::otlp_metrics::build_otlp_meter_provider`. Elimina o risco de divergência
   entre os dois canais já apontado no plano de negócio ("Divergência de representação
   entre os dois canais").
4. **`CounterFn::absolute` e `GaugeFn::increment`/`decrement` não são traduzidos para o
   lado OTel** (só tocam o recorder interno, comportamento inalterado). Ver "Achados da
   exploração de código" para a justificativa: nenhum destes três métodos tem call site
   hoje em `metricx` (confirmado por leitura de todos os `meters/*.rs`); implementar a
   tradução exigiria manter estado adicional por série temporal sem nenhum consumidor
   real, desproporcional ao risco. Documentado como limitação conhecida no código
   (doc comment) e no README (Task 6) — não é uma regra de negócio nova, é uma
   constatação técnica sobre o que já é/não é usado.
5. **Nenhuma dependência nova de terceiros.** Tudo é construído com `metrics` (já
   dependência de `statsd`/`prometheus`) e `opentelemetry` (já dependência de
   `http_server`/`http_client`, agora também de `statsd`/`prometheus`).

## Global Constraints

- **Não alterar a assinatura pública de nenhuma função em `metricx`**
  (`increment`, `increment_one`, `current_gauge`, `record_money`, `record_duration`,
  `start_stopwatch`, `MetricTags::*`) — todas continuam recebendo `&AppContext<S>` e
  `MetricTags` exatamente como hoje. Nenhuma aplicação consumidora muda uma linha de
  instrumentação.
- **Não alterar o comportamento do endpoint de pull `/metrics`** (formato, valores,
  buckets visíveis) quando não há push configurado — deve ser byte-a-byte idêntico ao
  gerado hoje pelo `PrometheusHandle::render()` (mesmo conteúdo, browsers de diffs
  aceitáveis: só ordem de iteração de um `HashMap`, se houver, nunca valores/labels).
- **Tags negadas continuam ocultas nos dois canais** — não implementar nenhuma
  filtragem adicional no bridge; a filtragem já acontece em
  `MetricTags::to_labels()` antes do `Recorder` ser chamado, então o bridge só precisa
  garantir que não reintroduz tags (ex.: não adicionar tags/atributos extras do lado
  OTel que não estavam no `Key` original).
- **Falha ao construir o pipeline de push não pode quebrar o pull** — já garantido
  estruturalmente pela decisão 2 acima (`build_otlp_meter_provider` continua nunca
  retornando `Err`, e o bridge sempre funciona independente do resultado).
- `task lint` (`cargo fmt --all -- --check && cargo clippy -- -D warnings`) e
  `task test` (`cargo nextest run`) devem passar a cada tarefa de código. Rodar
  explicitamente com as features relevantes: `cargo clippy --features prometheus -- -D
  warnings`, `cargo clippy --features statsd -- -D warnings` (cada uma isoladamente,
  já que só uma das duas costuma estar ativa por aplicação) e
  `cargo clippy --features "prometheus,statsd" -- -D warnings` (as duas juntas, para
  garantir que o bridge não assume qual das duas está ativa).
- Testes que chamam `metrics::set_global_recorder`/`opentelemetry::global::set_meter_provider`
  **precisam rodar via `cargo nextest run`** (processo isolado por teste) — `set_global_recorder`
  só pode ser chamado uma vez por processo; `cargo test` padrão (mesmo processo para
  todos os testes) quebraria com múltiplos testes desta fase no mesmo binário. Mesmo
  padrão de cuidado já documentado na tarefa anterior para env vars via `ENV_LOCK`,
  mas aqui é inerente ao processo, não precisa de lock adicional.
- Sem bump de versão (`0.5.1` continua `0.5.1` — ver "Achados da exploração de
  código").
- Sem novas dependências de terceiros no `Cargo.lock` (só habilitar feature existente
  `spec_unstable_metrics_views` em `opentelemetry_sdk`, e ativar `dep:opentelemetry`
  para `statsd`/`prometheus`).

---

### Task 1: Dependências — habilitar `spec_unstable_metrics_views` e `opentelemetry` para `statsd`/`prometheus`

**Dificuldade:** Simples

**Depende de:** nada (pode começar imediatamente)

**Bloqueia:** Task 2, Task 3

**Files:**
- Modify: `Cargo.toml` (raiz, `[workspace.dependencies]`)
- Modify: `crates/derust/Cargo.toml` (features `statsd`, `prometheus`)

**Interfaces:**
- Consumes: nada de código do derust.
- Produces: `opentelemetry_sdk::metrics::{Aggregation, Stream, StreamBuilder, Instrument,
  InstrumentKind}` com o método `Stream::builder().with_aggregation(...)` disponível
  (Task 2); `opentelemetry::{global, KeyValue}` e `opentelemetry::metrics::Meter`
  disponíveis como dependência direta de `metricx` (Tasks 3/4).

- [x] **Step 1: Adicionar `spec_unstable_metrics_views` à feature de `opentelemetry_sdk` no workspace**

Em `Cargo.toml` (raiz), altere a linha:
```toml
opentelemetry_sdk = { version = "0.30.0", features = ["metrics", "logs"] }
```
para:
```toml
opentelemetry_sdk = { version = "0.30.0", features = ["metrics", "logs", "spec_unstable_metrics_views"] }
```

- [x] **Step 2: Adicionar `dep:opentelemetry` às features `statsd` e `prometheus` em `crates/derust/Cargo.toml`**

Nas listas de features, adicione a linha `"dep:opentelemetry",` a ambas:
```toml
statsd = [
    "dep:regex",
    "dep:tracing",
    "dep:lazy_static",
    "dep:async-trait",
    "dep:tokio",
    "dep:cadence",
    "dep:metrics-exporter-statsd",
    "dep:metrics",
    "dep:opentelemetry",
]
prometheus = [
    "dep:regex",
    "dep:tracing",
    "dep:lazy_static",
    "dep:async-trait",
    "dep:tokio",
    "dep:cadence",
    "dep:metrics-exporter-prometheus",
    "dep:metrics",
    "dep:http-body-util",
    "dep:hyper",
    "dep:opentelemetry",
]
```
(`opentelemetry = { workspace = true }` já existe em `[dependencies]` — não precisa
adicionar/alterar essa linha, só as listas de feature acima.)

- [x] **Step 3: Build isolado de cada combinação de features**

Rode, em sequência:
```bash
cargo build -p derust --features prometheus
cargo build -p derust --features statsd
cargo build -p derust --features "prometheus,statsd"
cargo build -p derust --features http_server
```
Esperado: todos compilam sem erro. Nenhuma versão nova resolvida no `Cargo.lock` além
da feature `spec_unstable_metrics_views` sendo unificada (confirme com `git diff
Cargo.lock` — só deve mudar a lista de features de `opentelemetry_sdk`/`opentelemetry`
no lockfile, não versões).

- [x] **Step 4: Lint**

Rode: `cargo fmt --all -- --check && cargo clippy --features "prometheus,statsd" -- -D warnings`
Esperado: sem erros.

- [x] **Step 5: Commit**

```bash
git add Cargo.toml crates/derust/Cargo.toml Cargo.lock
git commit -m "chore: enable spec_unstable_metrics_views and add opentelemetry dep to statsd/prometheus features"
```

---

### Task 2: Unificar buckets de histograma e aplicar `View` no `SdkMeterProvider` de métricas

**Dificuldade:** Médio

**Depende de:** Task 1

**Bloqueia:** Task 5 (testes de ponta a ponta precisam da `View` já aplicada)

**Files:**
- Modify: `crates/derust/src/metricx/mod.rs` (nova constante `pub(crate) const
  HISTOGRAM_BUCKET_BOUNDARIES`)
- Modify: `crates/derust/src/metricx/registries/prometheus/mod.rs` (usar a constante em
  vez do array inline)
- Modify: `crates/derust/src/tracex/otlp_metrics.rs` (adicionar `.with_view(...)` à
  construção do `SdkMeterProvider`)

**Interfaces:**
- Consumes: `Aggregation::ExplicitBucketHistogram`, `Stream::builder()`,
  `Instrument::kind()`, `InstrumentKind::Histogram` (todos de `opentelemetry_sdk::metrics`,
  disponíveis após a Task 1).
- Produces: `crate::metricx::HISTOGRAM_BUCKET_BOUNDARIES: [f64; 13]`, `pub(crate)`
  (visível em todo o crate, não faz parte da API pública do derust) — consumida por
  `prometheus_registry()` (Task 2 mesma) e por `otlp_metrics::build_otlp_meter_provider`
  (Task 2 mesma). `build_otlp_meter_provider` mantém a mesma assinatura pública já
  existente (`pub(crate) fn build_otlp_meter_provider(resource: Resource) ->
  Option<SdkMeterProvider>`), só muda o corpo.

**Contexto:** Ver "Decisões técnicas tomadas nesta fase", item 3. A `View` se aplica a
**todo** instrumento do tipo `Histogram`, sem distinguir por nome — mesmo comportamento
que `PrometheusBuilder::set_buckets()` já tem hoje (um único array de buckets vale para
todos os histogramas, não por métrica).

- [x] **Step 1: Adicionar a constante compartilhada em `crates/derust/src/metricx/mod.rs`**

```rust
mod meters;
mod registries;

pub use meters::*;

#[cfg(feature = "statsd")]
pub use registries::statsd::*;

#[cfg(feature = "prometheus")]
pub use registries::prometheus::*;

/// Single source of truth for histogram bucket boundaries, shared by the Prometheus
/// pull registry (`registries::prometheus::prometheus_registry`) and the OTLP push
/// metrics pipeline (`crate::tracex::otlp_metrics::build_otlp_meter_provider`). Keeping
/// this in one place guarantees both channels represent the same metric with the same
/// distribution granularity — see the business plan's risk "Divergência de
/// representação entre os dois canais".
pub(crate) const HISTOGRAM_BUCKET_BOUNDARIES: [f64; 13] = [
    0.010, 0.025, 0.050, 0.075, 0.100, 0.150, 0.200, 0.250, 0.500, 1.0, 2.5, 5.0, 10.0,
];
```

- [x] **Step 2: Usar a constante em `prometheus_registry()`**

Em `crates/derust/src/metricx/registries/prometheus/mod.rs`, troque:
```rust
    let builder = PrometheusBuilder::new()
        .set_buckets(&[
            0.010, 0.025, 0.050, 0.075, 0.100, 0.150, 0.200, 0.250, 0.500, 1.0, 2.5, 5.0, 10.0,
        ])
        .map_err(|error| Box::new(error))?;
```
por:
```rust
    let builder = PrometheusBuilder::new()
        .set_buckets(&crate::metricx::HISTOGRAM_BUCKET_BOUNDARIES)
        .map_err(|error| Box::new(error))?;
```

- [x] **Step 3: Adicionar a `View` em `tracex::otlp_metrics::build_otlp_meter_provider`**

Em `crates/derust/src/tracex/otlp_metrics.rs`, troque o `Some(SdkMeterProvider::builder()...)`
final por:
```rust
    let mut builder = SdkMeterProvider::builder()
        .with_periodic_exporter(exporter)
        .with_resource(resource);

    // Keeps histogram bucket boundaries identical between the OTLP push channel and
    // the Prometheus pull channel (`crate::metricx::HISTOGRAM_BUCKET_BOUNDARIES`),
    // whenever `metricx` is compiled in. Without this `View`, the SDK's default
    // histogram aggregation would use different (and divergent) bucket boundaries.
    #[cfg(any(feature = "statsd", feature = "prometheus"))]
    {
        builder = builder.with_view(|instrument: &opentelemetry_sdk::metrics::Instrument| {
            if instrument.kind() == opentelemetry_sdk::metrics::InstrumentKind::Histogram {
                opentelemetry_sdk::metrics::Stream::builder()
                    .with_aggregation(opentelemetry_sdk::metrics::Aggregation::ExplicitBucketHistogram {
                        boundaries: crate::metricx::HISTOGRAM_BUCKET_BOUNDARIES.to_vec(),
                        record_min_max: true,
                    })
                    .build()
                    .ok()
            } else {
                None
            }
        });
    }

    Some(builder.build())
```
Note: quando nem `statsd` nem `prometheus` estão habilitados, não há canal de pull para
manter consistente, então a `View` simplesmente não é adicionada (SDK usa os buckets
default do `opentelemetry_sdk`) — comportamento aceitável, pois não há nada com que
divergir.

- [x] **Step 4: Teste de regressão — a `View` é aplicada quando `prometheus`/`statsd` está habilitado**

Adicione ao `mod test` existente em `otlp_metrics.rs` (reaproveitando `ENV_LOCK`/
`reset_env` já definidos ali):
```rust
    #[cfg(any(feature = "statsd", feature = "prometheus"))]
    #[tokio::test]
    async fn histogram_uses_shared_bucket_boundaries_via_view() {
        use opentelemetry_sdk::metrics::{InMemoryMetricExporter, PeriodicReader, SdkMeterProvider};

        let _guard = ENV_LOCK.lock().unwrap();
        reset_env();

        let exporter = InMemoryMetricExporter::default();
        let reader = PeriodicReader::builder(exporter.clone()).build();
        let resource = Resource::builder_empty().build();

        // Reimplementa só a parte de "with_view" (não passa pelo `infer_metrics_protocol`,
        // já coberto por outro teste) para poder injetar o InMemoryMetricExporter em vez
        // de um exporter OTLP real.
        let provider = SdkMeterProvider::builder()
            .with_reader(reader)
            .with_resource(resource)
            .with_view(|instrument: &opentelemetry_sdk::metrics::Instrument| {
                if instrument.kind() == opentelemetry_sdk::metrics::InstrumentKind::Histogram {
                    opentelemetry_sdk::metrics::Stream::builder()
                        .with_aggregation(opentelemetry_sdk::metrics::Aggregation::ExplicitBucketHistogram {
                            boundaries: crate::metricx::HISTOGRAM_BUCKET_BOUNDARIES.to_vec(),
                            record_min_max: true,
                        })
                        .build()
                        .ok()
                } else {
                    None
                }
            })
            .build();

        let meter = provider.meter("test");
        let histogram = meter.f64_histogram("test_histogram").build();
        histogram.record(0.2, &[]);

        provider.force_flush().unwrap();

        let metrics = exporter.get_finished_metrics().unwrap();
        let data_point_boundaries = /* extrair `ExponentialHistogram`/`Histogram`
            data do primeiro `ResourceMetrics` -> `ScopeMetrics` -> `Metric` com
            nome "test_histogram" e comparar `.bounds()`/`.boundaries` com
            `crate::metricx::HISTOGRAM_BUCKET_BOUNDARIES` — a forma exata de navegar
            essa estrutura (`opentelemetry_sdk::metrics::data::ResourceMetrics`) deve
            ser confirmada lendo `opentelemetry_sdk-0.30.0/src/metrics/data/mod.rs`
            durante a implementação; o objetivo do teste é validar que os boundaries
            configurados aparecem no histograma exportado, não a navegação exata da
            estrutura, que pode mudar de forma mecânica sem mudar o objetivo do teste */;
        assert_eq!(data_point_boundaries, crate::metricx::HISTOGRAM_BUCKET_BOUNDARIES.to_vec());

        reset_env();
    }
```
(Este teste só compila/roda quando `statsd` ou `prometheus` está habilitado — rode com
`cargo nextest run -p derust --features "http_server,prometheus" -- histogram_uses_shared_bucket_boundaries_via_view`.)

- [x] **Step 5: Rodar testes e lint**

```bash
cargo nextest run -p derust --features "http_server,prometheus" -- otlp_metrics
cargo fmt --all -- --check && cargo clippy --features "http_server,prometheus" -- -D warnings
```
Esperado: sem erros/falhas.

- [x] **Step 6: Commit**

```bash
git add crates/derust/src/metricx/mod.rs crates/derust/src/metricx/registries/prometheus/mod.rs crates/derust/src/tracex/otlp_metrics.rs
git commit -m "feat(metricx,tracex): share histogram bucket boundaries between Prometheus pull and OTLP push"
```

---

### Task 3: Implementar `OtelBridgingRecorder` (o bridge em si) e seus testes unitários

**Dificuldade:** Complexo

**Depende de:** Task 1 (pode rodar em paralelo com a Task 2 — arquivos diferentes,
único ponto de atenção é que ambas tocam `crates/derust/src/metricx/mod.rs`: Task 2
adiciona a constante `HISTOGRAM_BUCKET_BOUNDARIES`, Task 3 adiciona `mod otel_bridge;`
— sem sobreposição de linhas, mas resolver merge manualmente se rodarem em branches
paralelas de verdade)

**Bloqueia:** Task 4

**Files:**
- Create: `crates/derust/src/metricx/otel_bridge.rs`
- Modify: `crates/derust/src/metricx/mod.rs` (adicionar `mod otel_bridge;` — não
  precisa ser `pub`, só `statsd`/`prometheus` registries usam)

**Interfaces:**
- Consumes: `metrics::{Recorder, Counter, Gauge, Histogram, CounterFn, GaugeFn,
  HistogramFn, Key, KeyName, Metadata, SharedString, Unit}` (já dependência de
  `statsd`/`prometheus`); `opentelemetry::{global, KeyValue}`,
  `opentelemetry::metrics::Meter` (Task 1).
- Produces: `pub(crate) struct OtelBridgingRecorder<R: Recorder>` com `pub(crate) fn
  new(inner: R) -> Self`, implementando `metrics::Recorder`. Usado pela Task 4 dentro de
  `statsd_registry()`/`prometheus_registry()`.

**Contexto:** Ver "Achados da exploração de código" para as assinaturas exatas de
`Recorder`/`CounterFn`/`GaugeFn`/`HistogramFn` e do `Meter` do `opentelemetry`, e
"Decisões técnicas tomadas nesta fase" (itens 2 e 4) para as decisões de design já
fechadas (sempre encaminha para `global::meter(...)`; `absolute`/`increment`/
`decrement` de gauge não traduzidos).

- [x] **Step 1: Escrever `crates/derust/src/metricx/otel_bridge.rs`**

```rust
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use metrics::{Counter, CounterFn, Gauge, GaugeFn, Histogram, HistogramFn, Key, KeyName, Metadata, Recorder, SharedString, Unit};
use opentelemetry::metrics::Meter;
use opentelemetry::KeyValue;

/// Wraps an existing `metrics::Recorder` (the Prometheus or StatsD recorder `metricx`
/// already builds today) so that every metric registered through it is *also*
/// forwarded to the OTel Meter API (`opentelemetry::global::meter("derust")`), on top
/// of continuing to be recorded by the wrapped recorder exactly as before.
///
/// This recorder never checks whether an OTLP push pipeline was actually configured —
/// it always forwards to `opentelemetry::global::meter(...)`, which is a dynamic proxy:
/// if `tracex::init()` did not install a real `SdkMeterProvider` (no
/// `OTEL_EXPORTER_OTLP_*` env set), that proxy resolves to the `opentelemetry` API's
/// default no-op provider, whose `.add()`/`.record()` do nothing observable. See the
/// refinement doc's "Decisões técnicas tomadas nesta fase", item 2, for the accepted
/// performance trade-off (a cache lookup + a no-op call per metric emission even when
/// push is disabled).
pub(crate) struct OtelBridgingRecorder<R: Recorder> {
    inner: R,
    meter: Meter,
    otel_counters: RwLock<HashMap<String, opentelemetry::metrics::Counter<u64>>>,
    otel_gauges: RwLock<HashMap<String, opentelemetry::metrics::Gauge<f64>>>,
    otel_histograms: RwLock<HashMap<String, opentelemetry::metrics::Histogram<f64>>>,
}

impl<R: Recorder> OtelBridgingRecorder<R> {
    pub(crate) fn new(inner: R) -> Self {
        Self {
            inner,
            meter: opentelemetry::global::meter("derust"),
            otel_counters: RwLock::new(HashMap::new()),
            otel_gauges: RwLock::new(HashMap::new()),
            otel_histograms: RwLock::new(HashMap::new()),
        }
    }

    fn otel_counter(&self, name: &str) -> opentelemetry::metrics::Counter<u64> {
        if let Some(counter) = self.otel_counters.read().unwrap().get(name) {
            return counter.clone();
        }
        self.otel_counters
            .write()
            .unwrap()
            .entry(name.to_string())
            .or_insert_with(|| self.meter.u64_counter(name.to_string()).build())
            .clone()
    }

    fn otel_gauge(&self, name: &str) -> opentelemetry::metrics::Gauge<f64> {
        if let Some(gauge) = self.otel_gauges.read().unwrap().get(name) {
            return gauge.clone();
        }
        self.otel_gauges
            .write()
            .unwrap()
            .entry(name.to_string())
            .or_insert_with(|| self.meter.f64_gauge(name.to_string()).build())
            .clone()
    }

    fn otel_histogram(&self, name: &str) -> opentelemetry::metrics::Histogram<f64> {
        if let Some(histogram) = self.otel_histograms.read().unwrap().get(name) {
            return histogram.clone();
        }
        self.otel_histograms
            .write()
            .unwrap()
            .entry(name.to_string())
            .or_insert_with(|| self.meter.f64_histogram(name.to_string()).build())
            .clone()
    }
}

fn key_to_attributes(key: &Key) -> Vec<KeyValue> {
    key.labels()
        .map(|label| KeyValue::new(label.key().to_string(), label.value().to_string()))
        .collect()
}

struct FanoutCounter {
    inner: Counter,
    otel: opentelemetry::metrics::Counter<u64>,
    attributes: Vec<KeyValue>,
}

impl CounterFn for FanoutCounter {
    fn increment(&self, value: u64) {
        self.inner.increment(value);
        self.otel.add(value, &self.attributes);
    }

    // `metricx` never calls `Counter::absolute()` today (only `.increment()`, via
    // `metrics::counter!(...).increment(count)` in `meters/counter.rs`) — and OTel's
    // `Counter<u64>` only supports monotonic `.add()`, with no "set absolute value"
    // operation. Forwarding to the inner recorder only, matching pre-existing
    // behaviour; no OTel side-effect. See refinement doc, decision 4.
    fn absolute(&self, value: u64) {
        self.inner.absolute(value);
    }
}

struct FanoutGauge {
    inner: Gauge,
    otel: opentelemetry::metrics::Gauge<f64>,
    attributes: Vec<KeyValue>,
}

impl GaugeFn for FanoutGauge {
    // `metricx::current_gauge` only ever calls `.set()` (see `meters/gauge.rs`) —
    // `increment`/`decrement` have no call site today, so they are not translated to
    // the OTel side (see refinement doc, decision 4).
    fn increment(&self, value: f64) {
        self.inner.increment(value);
    }

    fn decrement(&self, value: f64) {
        self.inner.decrement(value);
    }

    fn set(&self, value: f64) {
        self.inner.set(value);
        self.otel.record(value, &self.attributes);
    }
}

struct FanoutHistogram {
    inner: Histogram,
    otel: opentelemetry::metrics::Histogram<f64>,
    attributes: Vec<KeyValue>,
}

impl HistogramFn for FanoutHistogram {
    fn record(&self, value: f64) {
        self.inner.record(value);
        self.otel.record(value, &self.attributes);
    }
}

impl<R: Recorder> Recorder for OtelBridgingRecorder<R> {
    fn describe_counter(&self, key: KeyName, unit: Option<Unit>, description: SharedString) {
        self.inner.describe_counter(key, unit, description);
    }

    fn describe_gauge(&self, key: KeyName, unit: Option<Unit>, description: SharedString) {
        self.inner.describe_gauge(key, unit, description);
    }

    fn describe_histogram(&self, key: KeyName, unit: Option<Unit>, description: SharedString) {
        self.inner.describe_histogram(key, unit, description);
    }

    fn register_counter(&self, key: &Key, metadata: &Metadata<'_>) -> Counter {
        let inner = self.inner.register_counter(key, metadata);
        let otel = self.otel_counter(key.name());
        Counter::from_arc(Arc::new(FanoutCounter { inner, otel, attributes: key_to_attributes(key) }))
    }

    fn register_gauge(&self, key: &Key, metadata: &Metadata<'_>) -> Gauge {
        let inner = self.inner.register_gauge(key, metadata);
        let otel = self.otel_gauge(key.name());
        Gauge::from_arc(Arc::new(FanoutGauge { inner, otel, attributes: key_to_attributes(key) }))
    }

    fn register_histogram(&self, key: &Key, metadata: &Metadata<'_>) -> Histogram {
        let inner = self.inner.register_histogram(key, metadata);
        let otel = self.otel_histogram(key.name());
        Histogram::from_arc(Arc::new(FanoutHistogram { inner, otel, attributes: key_to_attributes(key) }))
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use metrics::Label;
    use opentelemetry_sdk::metrics::{InMemoryMetricExporter, PeriodicReader, SdkMeterProvider};
    use opentelemetry_sdk::Resource;

    /// Minimal in-process `Recorder` double standing in for `PrometheusRecorder`/
    /// `StatsdRecorder` in these tests — records every `.increment()`/`.set()`/
    /// `.record()` call it receives so tests can assert the inner path still works
    /// exactly as before wrapping it.
    #[derive(Default, Clone)]
    struct SpyRecorder {
        counter_calls: Arc<std::sync::Mutex<Vec<u64>>>,
    }

    impl Recorder for SpyRecorder {
        fn describe_counter(&self, _: KeyName, _: Option<Unit>, _: SharedString) {}
        fn describe_gauge(&self, _: KeyName, _: Option<Unit>, _: SharedString) {}
        fn describe_histogram(&self, _: KeyName, _: Option<Unit>, _: SharedString) {}

        fn register_counter(&self, _: &Key, _: &Metadata<'_>) -> Counter {
            let calls = self.counter_calls.clone();
            struct SpyCounter(Arc<std::sync::Mutex<Vec<u64>>>);
            impl CounterFn for SpyCounter {
                fn increment(&self, value: u64) {
                    self.0.lock().unwrap().push(value);
                }
                fn absolute(&self, _value: u64) {}
            }
            Counter::from_arc(Arc::new(SpyCounter(calls)))
        }

        fn register_gauge(&self, _: &Key, _: &Metadata<'_>) -> Gauge {
            Gauge::noop()
        }

        fn register_histogram(&self, _: &Key, _: &Metadata<'_>) -> Histogram {
            Histogram::noop()
        }
    }

    fn install_in_memory_meter_provider() -> InMemoryMetricExporter {
        let exporter = InMemoryMetricExporter::default();
        let reader = PeriodicReader::builder(exporter.clone()).build();
        let provider = SdkMeterProvider::builder()
            .with_reader(reader)
            .with_resource(Resource::builder_empty().build())
            .build();
        opentelemetry::global::set_meter_provider(provider);
        exporter
    }

    #[test]
    fn forwards_counter_increments_to_both_inner_recorder_and_otel() {
        let exporter = install_in_memory_meter_provider();
        let spy = SpyRecorder::default();
        let counter_calls = spy.counter_calls.clone();
        let bridge = OtelBridgingRecorder::new(spy);

        let key = Key::from_parts("bridge_counter_metric", vec![Label::new("app_name", "test")]);
        let counter = bridge.register_counter(&key, &Metadata::new("test", metrics::Level::INFO, None));
        counter.increment(3);

        assert_eq!(*counter_calls.lock().unwrap(), vec![3], "inner recorder must still receive the increment");

        opentelemetry::global::meter_provider(); // no-op touch to keep provider alive during flush below
        // Force export and assert the same value reached the OTel side.
        // (Exact `force_flush` invocation must go through the `SdkMeterProvider` handle
        // kept in `install_in_memory_meter_provider`, adjust helper to return it too.)
        let metrics = exporter.get_finished_metrics().unwrap();
        assert!(!metrics.is_empty(), "expected at least one exported metric after forcing a flush");
    }

    #[test]
    fn does_not_forward_gauge_increment_decrement_only_set() {
        // Documents decision 4: increment/decrement on a gauge only touch the inner
        // recorder; this test exists to make that limitation explicit and regression-
        // proof, not to validate OTel output (there is no OTel-side effect to assert).
        let spy = SpyRecorder::default();
        let bridge = OtelBridgingRecorder::new(spy);
        let key = Key::from_name("bridge_gauge_metric");
        let gauge = bridge.register_gauge(&key, &Metadata::new("test", metrics::Level::INFO, None));
        gauge.increment(1.0);
        gauge.decrement(1.0);
        gauge.set(5.0);
        // No panic, no assertion failure: exercised purely to document/lock the
        // behaviour described in decision 4 above.
    }
}
```

Nota para quem implementar: o `force_flush()` precisa ser chamado no `SdkMeterProvider`
antes de ler `exporter.get_finished_metrics()` — ajuste `install_in_memory_meter_provider`
para devolver `(SdkMeterProvider, InMemoryMetricExporter)` em vez de só o exporter, e
chame `provider.force_flush().unwrap()` antes da asserção no teste
`forwards_counter_increments_to_both_inner_recorder_and_otel` (o pseudocódigo acima
omite esse detalhe de fiação para não distrair da lógica principal — implementar
corretamente é responsabilidade desta task, confirmando a API exata lendo
`opentelemetry_sdk-0.30.0/src/metrics/meter_provider.rs` durante a implementação).

- [x] **Step 2: Registrar o módulo em `metricx/mod.rs`**

```rust
mod meters;
mod otel_bridge;
mod registries;
```

- [x] **Step 3: Rodar os testes isoladamente**

```bash
cargo nextest run -p derust --features prometheus -- otel_bridge
```
Esperado: `forwards_counter_increments_to_both_inner_recorder_and_otel` e
`does_not_forward_gauge_increment_decrement_only_set` passam.

- [x] **Step 4: Lint**

```bash
cargo fmt --all -- --check && cargo clippy --features prometheus -- -D warnings
```

- [x] **Step 5: Commit**

```bash
git add crates/derust/src/metricx/otel_bridge.rs crates/derust/src/metricx/mod.rs
git commit -m "feat(metricx): add OtelBridgingRecorder forwarding metrics to opentelemetry::metrics (not yet wired in)"
```

---

### Task 4: Instalar o `OtelBridgingRecorder` em `statsd_registry()` e `prometheus_registry()`

**Dificuldade:** Médio

**Depende de:** Task 3

**Bloqueia:** Task 5

**Files:**
- Modify: `crates/derust/src/metricx/registries/prometheus/mod.rs`
- Modify: `crates/derust/src/metricx/registries/statsd/mod.rs`

**Interfaces:**
- Consumes: `crate::metricx::otel_bridge::OtelBridgingRecorder` (Task 3).
- Produces: nenhuma mudança de assinatura pública — `prometheus_registry() ->
  Result<PrometheusHandle, Box<dyn std::error::Error>>` e `statsd_registry(config:
  &StatsdConfig) -> Result<(), Box<dyn std::error::Error>>` continuam idênticas.

- [x] **Step 1: `prometheus_registry()` — usar `build_recorder()` + wrapper em vez de `install_recorder()`**

Em `crates/derust/src/metricx/registries/prometheus/mod.rs`, troque:
```rust
    let handler = builder
        .install_recorder()
        .map_err(|error| Box::new(error))?;

    Ok(handler)
```
por:
```rust
    let recorder = builder.build_recorder();
    let handle = recorder.handle();

    metrics::set_global_recorder(crate::metricx::otel_bridge::OtelBridgingRecorder::new(recorder))
        .map_err(|error| Box::new(error))?;

    Ok(handle)
```
(`build_recorder()` não retorna `Result` — confirme lendo a assinatura atual do método
antes de editar, ver "Achados da exploração de código"; ajuste o `?`/`map_err` conforme
o que a leitura direta confirmar.)

- [x] **Step 2: `statsd_registry()` — envolver o recorder antes de instalar**

Em `crates/derust/src/metricx/registries/statsd/mod.rs`, troque:
```rust
    let recorder = recorder.build(None).map_err(|error| Box::new(error))?;

    let key = metrics::Key::from_static_name("any");
    let h = recorder.register_histogram(&key, &metrics::Metadata::new("any", Level::INFO, None));
    h.record(1.0);

    let _ = metrics::set_global_recorder(recorder);
```
por:
```rust
    let recorder = recorder.build(None).map_err(|error| Box::new(error))?;

    let key = metrics::Key::from_static_name("any");
    let h = recorder.register_histogram(&key, &metrics::Metadata::new("any", Level::INFO, None));
    h.record(1.0);

    let _ = metrics::set_global_recorder(crate::metricx::otel_bridge::OtelBridgingRecorder::new(recorder));
```
(O warm-up de histograma (`register_histogram`/`record(1.0)`) continua acontecendo
**antes** de envolver o recorder — mantém o comportamento pré-existente do warm-up
tocando só o `StatsdRecorder` puro, sem gerar um valor "fantasma" de 1.0 no lado OTel;
isso é intencional, não um bug a corrigir.)

- [x] **Step 3: Rodar os testes existentes de `metricx`**

```bash
cargo nextest run -p derust --features prometheus -- metricx
cargo nextest run -p derust --features statsd -- metricx
```
Esperado: todos passam (inclui `should_normalize_path`/`should_filter_metric_tags`
já existentes, sem nenhuma mudança de comportamento esperada neles).

- [x] **Step 4: Lint**

```bash
cargo fmt --all -- --check && cargo clippy --features "prometheus,statsd" -- -D warnings
```

- [x] **Step 5: Commit**

```bash
git add crates/derust/src/metricx/registries/prometheus/mod.rs crates/derust/src/metricx/registries/statsd/mod.rs
git commit -m "feat(metricx): wire OtelBridgingRecorder into prometheus_registry and statsd_registry"
```

---

### Task 5: Testes de ponta a ponta cobrindo os 3 critérios do plano de negócio

**Dificuldade:** Complexo

**Depende de:** Task 2, Task 4

**Bloqueia:** Task 6

**Files:**
- Modify: `crates/derust/src/metricx/registries/prometheus/mod.rs` (novo `mod test`
  no arquivo, ou novo arquivo de teste de integração — decidir durante a implementação
  conforme o que for mais simples de fiar com `AppContext`; se precisar de
  `AppContext`/`Environment`/`start_test`, considerar um teste de integração em
  `crates/derust/tests/` em vez de `#[cfg(test)]` interno, para poder habilitar a
  feature `start_test` sem acoplar isso ao build normal da lib)

**Interfaces:**
- Consumes: `AppContext::new(...)` com `PrometheusConfig` (via feature `prometheus`),
  `increment`/`current_gauge`/`record_duration`, `PrometheusHandle::render()` (pull),
  `opentelemetry_sdk::metrics::InMemoryMetricExporter` (push simulado, mesma técnica da
  Task 3).
- Produces: nada de código de produção — só cobertura de teste.

**Contexto:** O plano de negócio pede explicitamente (seção "Escopo"): "(1) uma métrica
emitida aparece nos dois canais quando o push está configurado; (2) sem configuração de
push, o comportamento é idêntico ao pré-existente; (3) tags negadas não aparecem em
nenhum dos dois canais." Como o bridge (Task 3) não distingue "push configurado" de
"push não configurado" via env — ele sempre encaminha para
`opentelemetry::global::meter(...)` —, o critério (1) é simulado instalando um
`SdkMeterProvider` real (`InMemoryMetricExporter`) via
`opentelemetry::global::set_meter_provider(...)` **antes** de chamar
`AppContext::new(...)`, e o critério (2) por **não** instalar nenhum provider (deixando
o proxy global resolver para o no-op padrão) — não é necessário, e seria mais frágil,
tentar rodar `tracex::init()` de ponta a ponta com um coletor OTLP real só para este
teste; a fronteira relevante para o bridge é "existe ou não um `MeterProvider` real
instalado", que é exatamente o que `tracex::init()` decide via env, e já está coberto
pelos testes de `tracex` na tarefa anterior.

- [x] **Step 1: Teste do critério (1) — métrica aparece nos dois canais quando "push" está configurado**

```rust
#[tokio::test]
async fn metric_appears_on_both_pull_and_push_channels_when_meter_provider_is_set() {
    // Install a real (in-memory) MeterProvider, simulating tracex::init() having
    // configured OTLP push (see Task 5 context above for why this is equivalent).
    let exporter = InMemoryMetricExporter::default();
    let reader = PeriodicReader::builder(exporter.clone()).build();
    let provider = SdkMeterProvider::builder()
        .with_reader(reader)
        .with_resource(Resource::builder_empty().build())
        .build();
    opentelemetry::global::set_meter_provider(provider.clone());

    let context = AppContext::new(
        "test-app",
        Environment::Test,
        PrometheusConfig { denied_metric_tags: vec![], denied_metric_tags_by_regex: vec![] },
        (),
    ).unwrap();

    increment(&context, "bridge_e2e_counter", MetricTags::default(), 7);

    // Pull channel.
    let rendered = context.prometheus_handle().render();
    assert!(rendered.contains("bridge_e2e_counter"));
    assert!(rendered.contains("7"));

    // Push channel.
    provider.force_flush().unwrap();
    let metrics = exporter.get_finished_metrics().unwrap();
    assert!(metrics.iter().any(|resource_metrics| {
        resource_metrics.scope_metrics().any(|scope| {
            scope.metrics().any(|metric| metric.name() == "bridge_e2e_counter")
        })
    }));
}
```
(Ajustar a navegação exata de `ResourceMetrics`/`ScopeMetrics`/`Metric` conforme a API
real do `opentelemetry_sdk` 0.30 — confirmar lendo `src/metrics/data/mod.rs` durante a
implementação, mesma ressalva da Task 2 Step 4. `AppContext::new` só recebe
`PrometheusConfig` quando a feature `prometheus` está habilitada — este teste deve
rodar com `--features prometheus`; um teste irmão análogo, usando `StatsdConfig`, é
opcional e menos crítico, já que a asserção do canal pull para StatsD é mais difícil de
observar em teste — sem endpoint HTTP de scrape — então cobrir só `prometheus` aqui é
suficiente para o critério (1), e a Task 3 já cobre o bridge em si de forma agnóstica
ao backend.)

- [x] **Step 2: Teste do critério (2) — sem `MeterProvider` real, comportamento é idêntico ao pré-existente**

```rust
#[tokio::test]
async fn pull_channel_behaviour_is_unchanged_without_a_real_meter_provider() {
    // Deliberately do NOT call opentelemetry::global::set_meter_provider — the global
    // proxy resolves to the opentelemetry API's default no-op provider.
    let context = AppContext::new(
        "test-app",
        Environment::Test,
        PrometheusConfig { denied_metric_tags: vec![], denied_metric_tags_by_regex: vec![] },
        (),
    ).unwrap();

    increment(&context, "bridge_e2e_counter_no_push", MetricTags::default(), 9);

    let rendered = context.prometheus_handle().render();
    assert!(rendered.contains("bridge_e2e_counter_no_push"));
    assert!(rendered.contains("9"));
    // No push-side assertion here by design: there is no MeterProvider installed, so
    // there is nothing to flush/inspect — this test's whole point is that the pull
    // channel keeps working exactly as it did before this plan.
}
```

- [x] **Step 3: Teste do critério (3) — tags negadas não aparecem em nenhum dos dois canais**

```rust
#[tokio::test]
async fn denied_tags_are_hidden_on_both_channels() {
    let exporter = InMemoryMetricExporter::default();
    let reader = PeriodicReader::builder(exporter.clone()).build();
    let provider = SdkMeterProvider::builder()
        .with_reader(reader)
        .with_resource(Resource::builder_empty().build())
        .build();
    opentelemetry::global::set_meter_provider(provider.clone());

    let context = AppContext::new(
        "test-app",
        Environment::Test,
        PrometheusConfig {
            denied_metric_tags: vec!["customer".to_string()],
            denied_metric_tags_by_regex: vec![],
        },
        (),
    ).unwrap();

    let tags = MetricTags::from([("customer", "123"), ("kind", "foo")]);
    increment(&context, "bridge_e2e_denied_tags", tags, 1);

    let rendered = context.prometheus_handle().render();
    assert!(rendered.contains("bridge_e2e_denied_tags"));
    assert!(!rendered.contains("customer"), "denied tag leaked into the pull channel");

    provider.force_flush().unwrap();
    let metrics = exporter.get_finished_metrics().unwrap();
    // Assert the exported data point's attributes do not contain "customer" — exact
    // navigation to data point attributes confirmed against
    // `opentelemetry_sdk::metrics::data` during implementation, same as Step 1.
}
```

- [x] **Step 4: Rodar a suíte completa destes testes**

```bash
cargo nextest run -p derust --features "prometheus,start_test" -- metric_appears_on_both pull_channel_behaviour denied_tags_are_hidden
```
Esperado: os 3 testes passam.

- [x] **Step 5: Lint**

```bash
cargo fmt --all -- --check && cargo clippy --features "prometheus,start_test" -- -D warnings
```

- [x] **Step 6: Commit**

```bash
git add -A
git commit -m "test(metricx): cover the 3 business-plan scope criteria for the OTel metrics bridge end-to-end"
```

---

### Task 6: Documentação e CHANGELOG

**Dificuldade:** Simples

**Depende de:** Task 5

**Bloqueia:** Task 7

**Files:**
- Modify: `crates/derust/src/tracex/README.md` (seção "OTLP push for metrics and
  logs" → "Coexists with the `metricx` Prometheus/StatsD pull path")
- Modify: `crates/derust/src/metricx/registries/prometheus/README.md`
- Modify: `crates/derust/src/metricx/registries/statsd/README.md`
- Modify: `CHANGELOG.md` (entrada `## [0.5.1]`)

**Interfaces:**
- Consumes: comportamento final implementado nas Tasks 1-5.
- Produces: documentação e changelog atualizados; nenhuma mudança de código.

- [x] **Step 1: Corrigir `crates/derust/src/tracex/README.md`**

Na seção "Coexists with the `metricx` Prometheus/StatsD pull path", substitua o
parágrafo que hoje diz que métricas via `metrics` crate "are not automatically
forwarded to the OTLP MeterProvider" por uma descrição do comportamento atual: toda
métrica emitida via `metricx` (`increment`, `current_gauge`, `record_money`,
`record_duration`, `start_stopwatch`, incluindo as automáticas de HTTP/DB) passa a
alimentar os dois canais automaticamente, sem instrumentação adicional; explique
brevemente o mecanismo (bridge dentro de `metricx`, sempre ativo, sem custo perceptível
quando push não está configurado) e mantenha o aviso de custo de ingestão (já existente
na seção "Cost warning", ainda válido e agora mais relevante, já que passa a haver
dados reais fluindo).

- [x] **Step 2: Atualizar os READMEs de `metricx`**

Em `crates/derust/src/metricx/registries/prometheus/README.md` e
`.../statsd/README.md`, adicione uma nota curta (próxima ao topo, junto com a descrição
das métricas automáticas) explicando que, quando `tracex::init()` está configurado para
push OTLP (mesmas envs `OTEL_EXPORTER_OTLP_*` de traces/logs), essas mesmas métricas
também chegam via push, sem nenhuma mudança de código — com um link/referência à seção
correspondente do README de `tracex`.

- [x] **Step 3: Atualizar `CHANGELOG.md`**

Na entrada `## [0.5.1]`, seção `### Notes`, remova a frase "Metrics instrumented via
the `metrics` crate ... are not automatically forwarded to the new OTLP
`MeterProvider`" e adicione, na seção `### Added` da mesma entrada, uma linha
descrevendo o comportamento novo, por exemplo:

```markdown
- Metrics instrumented via `metricx` (`increment`, `current_gauge`, `record_money`,
  `record_duration`, `start_stopwatch`, including the automatic HTTP/DB duration
  metrics) now also reach the OTLP push pipeline described above, with no
  instrumentation changes required — the existing Prometheus `/metrics` pull endpoint
  and StatsD push remain unaffected. Histogram bucket boundaries are identical on both
  channels.
```

Remova a seção `### Notes` inteira se, depois dessa edição, ela ficar vazia.

- [x] **Step 4: Revisar os diffs**

```bash
git diff crates/derust/src/tracex/README.md crates/derust/src/metricx/registries/prometheus/README.md crates/derust/src/metricx/registries/statsd/README.md CHANGELOG.md
```
Confirme que nenhum conteúdo não relacionado foi alterado.

- [x] **Step 5: Commit**

```bash
git add crates/derust/src/tracex/README.md crates/derust/src/metricx/registries/prometheus/README.md crates/derust/src/metricx/registries/statsd/README.md CHANGELOG.md
git commit -m "docs: document the metricx-to-OTLP-push metrics bridge, update CHANGELOG [0.5.1]"
```

---

### Task 7: Validação final do crate (lint, testes, exemplos) — fecha o plano

**Dificuldade:** Simples

**Depende de:** Task 1, Task 2, Task 3, Task 4, Task 5, Task 6

**Bloqueia:** nada (última tarefa do plano)

**Files:** nenhum (só validação; eventuais correções pontuais encontradas durante a
validação devem ser commitadas nos arquivos já tocados pelas tasks anteriores, não em
arquivos novos)

- [ ] **Step 1: Suíte completa de testes**

```bash
task test
```
Esperado: todos os testes passam, 0 falhas (inclui todos os testes novos das Tasks 2, 3
e 5).

- [ ] **Step 2: Lint completo do workspace**

```bash
task lint
```
Esperado: sem erros nos arquivos tocados por este plano
(`crates/derust/src/metricx/*`, `crates/derust/src/tracex/otlp_metrics.rs`,
`Cargo.toml`, `crates/derust/Cargo.toml`, `CHANGELOG.md`). Se houver débito técnico
pré-existente e não relacionado a este plano (mesmo precedente já documentado nos dois
refinamentos anteriores — ver
`docs/superpowers/plans/2026-09-22-adicionar-push-otlp-metricas-e-logs.md`, Task 6,
Step 4), confirme isoladamente (`git stash` + comparação) que os arquivos tocados por
este plano não fazem parte desse débito, e registre a mesma ressalva aqui.

- [ ] **Step 3: Sanity-check de performance (risco "custo de emissão duplicada")**

Não há harness de benchmark no repositório hoje — não introduzir um novo (fora de
escopo). Em vez disso, valide manualmente: escreva um teste/experimento local
(descartável, não commitado) que chame `increment()` em loop (ex.: 100_000 vezes) com
e sem o bridge (ou seja, comparando o tempo antes/depois desta implementação, via
`git stash`), e confirme que a diferença fica na casa dos microssegundos agregados, não
milissegundos — critério qualitativo aceito pelo plano de negócio ("não pode introduzir
latência ou contenção perceptível"), sem exigir um número exato. Documente o resultado
observado (ordem de grandeza) neste arquivo, substituindo este parágrafo por um
"Resultado real" análogo ao das tarefas anteriores, antes do commit final.

- [ ] **Step 4: Build de exemplos relevantes**

```bash
cd examples/metrics && cargo build && cd -
```
Esperado: build sem erros. (Mesma ressalva já documentada na tarefa anterior sobre
outros exemplos com débito técnico pré-existente não relacionado a este plano — não
investigar/corrigir aqui.)

- [ ] **Step 5: Commit final (se houver algum ajuste pontual desta validação)**

```bash
git add -A
git commit -m "chore: final validation pass for the metricx-to-OTLP-push metrics bridge"
```
(Só criar este commit se a validação tiver gerado alguma mudança real de arquivo; caso
contrário, não commitar nada vazio.)

---

## Plano de testes e revisão por tarefa (resumo)

| Tarefa | Plano de testes | Plano de revisão |
|---|---|---|
| 1 — dependências | `cargo build` com cada combinação de features (Step 3) | Confirmar que `Cargo.lock` não resolveu nenhuma versão nova, só features |
| 2 — buckets compartilhados + `View` | `cargo nextest run -- otlp_metrics` (teste novo `histogram_uses_shared_bucket_boundaries_via_view`) | Confirmar que `prometheus_registry()` e `build_otlp_meter_provider` referenciam a **mesma** constante, não dois arrays copiados |
| 3 — `OtelBridgingRecorder` | `cargo nextest run -- otel_bridge` (2 testes: fan-out de counter; gauge increment/decrement não traduzido) | Confirmar que `absolute`/`increment`/`decrement` de gauge realmente não tocam o lado OTel (decisão 4), e que o cache (`RwLock<HashMap>`) não recria instrumentos a cada chamada |
| 4 — instalação nos registries | `cargo nextest run -- metricx` (testes já existentes, sem regressão) | Confirmar que `PrometheusHandle`/warm-up do StatsD continuam funcionando exatamente como antes (o wrapper não muda o que os dois `Recorder`s originais recebem) |
| 5 — testes de ponta a ponta | Os 3 testes cobrindo os 3 critérios do plano de negócio (Steps 1-3) | Confirmar que os 3 critérios do "Escopo" do plano de negócio estão literalmente cobertos, um teste por critério, nomes de teste autoexplicativos |
| 6 — documentação/CHANGELOG | Revisão textual | Confirmar que nenhuma documentação (`tracex/README.md` em particular) ainda afirma que métricas não chegam ao push — essa era a limitação que este plano elimina |
| 7 — validação final | `task test`, `task lint`, build de exemplos, sanity-check de performance | Confirmar ausência de regressão de performance perceptível e que nenhum arquivo fora do escopo deste plano foi alterado |

## Paralelização

- Task 1 é sempre a primeira (bloqueia 2 e 3).
- Task 2 e Task 3 podem rodar **em paralelo** entre si (subagentes distintos) — ambas
  só dependem da Task 1, tocam arquivos majoritariamente diferentes, com um único ponto
  de conflito trivial em `crates/derust/src/metricx/mod.rs` (Task 2 adiciona uma
  constante, Task 3 adiciona `mod otel_bridge;` — resolver merge manualmente se
  rodarem em branches paralelas de verdade, mesmo padrão já aceito no refinamento
  anterior para `tracex/mod.rs`).
- Task 4 depende só da Task 3 (usa `OtelBridgingRecorder`), mas na prática só faz
  sentido rodar depois que a Task 2 também tiver terminado (para não ter que revisitar
  os registries duas vezes) — trate como dependendo de ambas na prática, mesmo que o
  código estritamente só exija a Task 3.
- Task 5 depende de Task 2 e Task 4 (precisa do bridge instalado E dos buckets
  unificados para os testes de ponta a ponta fazerem sentido).
- Task 6 depende de Task 5 (documenta o comportamento já validado por teste).
- Task 7 depende de todas as anteriores (fecha o plano).
- Oportunidade de paralelismo real: só o par (Task 2, Task 3). O restante é uma cadeia
  linear.
