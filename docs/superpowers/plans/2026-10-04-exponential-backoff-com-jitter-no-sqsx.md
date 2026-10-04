# Exponential backoff com jitter no consumo SQS (sqsx) — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Quando o handler de uma mensagem SQS falha (`Err(HttpError)`), o reenvio passa
a ser progressivamente mais espaçado, com full jitter, em vez de usar o visibility
timeout fixo da fila. É opt-in (`SqsQueueConsumer::with_retry_policy`); sem política o
comportamento atual é preservado integralmente. O handler passa a poder consultar o
número da tentativa atual via `Message::receive_count()`.

**Architecture:** Três peças pequenas dentro de `crates/derust/src/awsx/sqsx/`:

1. `retry.rs` (novo): `RetryPolicy` com a matemática pura (sem I/O, sem relógio) do
   atraso. A parte aleatória é isolada em uma função que recebe a amostra `[0,1)` como
   parâmetro, para o cálculo ser 100% determinístico em teste.
2. `consumer.rs`: `Message::receive_count()` (lê o atributo de sistema
   `ApproximateReceiveCount`, ausente/inválido = 1), `MessageBuilder::receive_count(u32)`
   e o builder `SqsQueueConsumer::with_retry_policy(RetryPolicy)` (campo opcional
   `retry_policy: Option<RetryPolicy>`; `new()` não muda).
3. `runner.rs`: `receive_message` passa a pedir `ApproximateReceiveCount`; no ramo `Err`
   de `process_message`, se houver política, calcula o atraso e chama
   `change_message_visibility`; falha nessa chamada é só logada.

**Tech Stack:** Rust 1.96. `aws-sdk-sqs` 1.74 (já no workspace). Nova dependência opcional
`rand` 0.9 (já presente no `Cargo.lock` como 0.9.1, então sem novo download de versão),
declarada na feature `sqs`.

**Spec:** Plano de negócio aprovado no vault Obsidian "Deroldo":
`Projects/derust/plans/exponential_backoff_com_jitter_no_sqsx.md`

## Decisões de negócio já fechadas (não reabrir)

- Jitter: **full jitter**, `delay = random(0, min(max, base * fator^(n-1)))`, `n` = receive count (1 = primeira entrega).
- Defaults quando habilitada sem parâmetros: **base 5s, máximo 15min, fator 2.0**.
- **Opt-in**: sem `with_retry_policy`, nada muda.
- `receive_count` é API pública de `Message`; mensagem sem a informação = tentativa 1.
- Fora de escopo: erro transitório vs permanente (contrato do handler), backoff no erro de `receive_message` (continua `sleep` fixo de 5s), processamento paralelo do lote, gestão de DLQ/redrive.

## Achados da exploração de código (contexto para todas as tarefas)

- Feature `sqs` (`crates/derust/Cargo.toml`: `sqs = ["aws", "dep:wg"]`) gateia
  `pub mod sqsx` em `crates/derust/src/awsx/mod.rs`. Todo código novo vive sob esse gate.
- `crates/derust/src/awsx/sqsx/mod.rs` hoje:
  ```rust
  mod consumer;
  mod runner;
  pub use consumer::{Message, MessageBuilder, SqsQueueConsumer};
  pub use runner::run;
  ```
- `consumer.rs`: `Message(SdkMessage)` com `new` (pub(crate)), `builder()`, `body()`,
  `message_id()`, `receipt_handle()` (pub(crate)); `MessageBuilder(SdkMessageBuilder)` com
  `body`, `message_id`, `receipt_handle`, `build`; `SqsQueueConsumer<S>` com campos
  `pub(crate) queue_url: String` e `pub(crate) handler: HandlerFn<S>`, e `new(queue_url, handler)`.
- `runner.rs`: `poll_queue` faz `receive_message().queue_url(..).max_number_of_messages(10).wait_time_seconds(20)`;
  `process_message` constrói `Message::new(sdk_msg)`, extrai `message_id`/`body`/`receipt_handle`
  **antes** de mover `msg` para o handler, trata `Ok` (log + `delete_message`) e `Err` (só log
  `error!`). A métrica `sqs_consumer_seconds` (feature `statsd`/`prometheus`) é gravada no
  fim com `success=true|false`. O lote é processado sequencialmente.
- Verificado em `aws-sdk-sqs` 1.74: `ReceiveMessageFluentBuilder::message_system_attribute_names(MessageSystemAttributeName)`
  existe (usar esta, não a `attribute_names`, que é de `QueueAttributeName`);
  `types::Message::attributes() -> Option<&HashMap<MessageSystemAttributeName, String>>` e
  `types::builders::MessageBuilder::attributes(k, v)` existem;
  `ChangeMessageVisibilityFluentBuilder::visibility_timeout(i32)` recebe **segundos em `i32`**.
- Não há testes hoje em `sqsx` e não há README nem exemplo de SQS. `crates/derust/README.md`
  lista features em `## Features` (linha `- [aws](...)`); `CHANGELOG.md` está em `## [0.5.1]`
  (versão atual do crate em `crates/derust/Cargo.toml` é `0.5.1`).
- Rodar `task lint` e `task test` do crate: `cargo nextest run --features sqs` (sqsx só compila com `sqs`).
  Há diffs de `cargo fmt` pré-existentes em `awsx/sqsx/*` (ver planos anteriores): rode
  `cargo fmt` apenas nos arquivos tocados e confirme que o diff de formatação se limita às
  linhas alteradas por você; se `cargo fmt --check` acusar linhas que você não tocou, formate
  o arquivo inteiro em um commit separado `style(sqsx): cargo fmt`.

## Decisões técnicas (a mecânica — "como")

1. **Fonte de aleatoriedade**: crate `rand` 0.9 (`rand::random::<f64>()`, que é `[0,1)`). A
   amostra é injetada por parâmetro em `RetryPolicy::delay_with_sample`, permitindo teste determinístico.
2. **`RetryPolicy` é `Copy + Clone + Debug + PartialEq`**, imutável, com builder encadeado
   (`with_base_delay`, `with_max_delay`, `with_factor`) e `Default` = 5s/15min/2.0.
3. **Saneamento de entradas** (sem panic, sem decisão de negócio nova): `factor` não finito
   ou `< 1.0` vira `1.0` (atraso constante, nunca decrescente); `max < base` faz o teto efetivo
   ser `max`; `base == 0` gera atraso 0. O teto efetivo nunca excede 12h
   (`SQS_MAX_VISIBILITY_TIMEOUT = 43_200s`), mesmo que `max` configurado seja maior.
4. **Overflow**: o crescimento é calculado em `f64` com `powi`/`powf` saturando em `+inf`;
   a conversão para `Duration` passa por `min(teto_em_segundos_f64)` **antes** de
   `Duration::from_secs_f64` (que entra em panic com valores inf/NaN/negativos). Expoente
   `n-1` usa `saturating_sub` em `u32`.
5. **Conversão para o SDK**: `Duration` -> segundos inteiros (`as_secs()`, truncando) ->
   `i32` via `try_from` saturando em `43_200`. Atraso 0 é válido (mensagem volta imediatamente).
6. **Efeito no lote sequencial / DLQ**: `change_message_visibility` não altera o
   `ApproximateReceiveCount`, então a redrive policy/DLQ continua contando normalmente; o
   atraso é só um ajuste de visibilidade. O tempo do lote sequencial não é mitigado nem
   agravado (apenas documentado no README).
7. **Falha em `change_message_visibility`**: `warn!`/`error!` com queue, message_id, atraso e
   erro; sem retry, sem panic, o loop segue. A mensagem volta à fila pelo timeout padrão.
8. **Métrica**: `sqs_consumer_seconds` continua gravada ao final de `process_message` com
   `success=false`; a chamada `change_message_visibility` ocorre dentro do ramo `Err`, antes
   do registro, então seu tempo (ms) entra na métrica — aceitável e documentado em comentário.
9. **Testabilidade da chamada ao SDK**: extrair uma função
   `async fn apply_retry_backoff(sqs: &SqsClient, queue_url: &str, receipt_handle: &str, message_id: &str, delay_secs: i32)`
   e uma função pura `fn retry_delay_secs(policy: Option<&RetryPolicy>, receive_count: u32) -> Option<i32>`.
   A pura é testada por unidade; a de I/O é testada com um cliente SQS de teste (ver Task 4).

## Global Constraints

- Nenhuma mudança na assinatura pública existente: `SqsQueueConsumer::new`, `Message::{body,message_id}`, `MessageBuilder::{body,message_id,receipt_handle,build}`, `run`.
- Tudo novo sob a feature `sqs`. `cargo build` sem `--features sqs` continua passando.
- Sem `unwrap()`/`expect()` em caminho de produção; nenhum panic possível a partir de `RetryPolicy`.
- Sem política configurada: zero chamadas novas ao SDK e nenhum log novo no ramo `Err`. (A única diferença observável é o pedido do atributo `ApproximateReceiveCount` no `receive_message`, que é necessário para `Message::receive_count()` e inofensivo.)
- Commits pequenos, um por tarefa, mensagens em inglês estilo conventional commits, terminando com a linha `Co-Authored-By` exigida pelo ambiente.
- Verificação por tarefa: `cargo nextest run --features sqs` e `cargo clippy --features sqs -- -D warnings`.

## Mapa de arquivos

| Arquivo | Ação | Tarefas |
|---|---|---|
| `crates/derust/Cargo.toml` | Modify (`rand` opcional; feature `sqs`; dev-dep de teste, se necessária) | 1, 4 |
| `Cargo.toml` (workspace) | Modify (`rand` em `[workspace.dependencies]`) | 1 |
| `crates/derust/src/awsx/sqsx/retry.rs` | Create | 1 |
| `crates/derust/src/awsx/sqsx/mod.rs` | Modify (`mod retry; pub use retry::RetryPolicy;`) | 1 |
| `crates/derust/src/awsx/sqsx/consumer.rs` | Modify | 2, 3 |
| `crates/derust/src/awsx/sqsx/runner.rs` | Modify | 2, 4, 5 |
| `crates/derust/README.md` | Modify (seção SQS) | 6 |
| `CHANGELOG.md` | Modify (entrada em `[0.5.1]`/próxima) | 6 |

## Grafo de dependências

```
Task 1 (RetryPolicy) ──┬──> Task 3 (with_retry_policy) ──> Task 4 (backoff no runner) ──> Task 5 (testes e2e/regressão) ──> Task 6 (docs) ──> Task 7 (validação)
Task 2 (receive_count) ┴────────────────────────────────────┘
```

- **Paralelas**: Task 1 e Task 2 são independentes (arquivos diferentes; único ponto de
  contato trivial: `mod.rs`, que só Task 1 altera). Rodar em paralelo.
- **Sequenciais**: Task 3 depende de 1; Task 4 depende de 1, 2 e 3; Task 5 depende de 4;
  Task 6 depende de 5; Task 7 fecha.

---

### Task 1: `RetryPolicy` — cálculo do atraso (full jitter, saturação, teto 12h, defaults)

**Dificuldade:** média (matemática de borda; sem I/O).

**Depende de:** nada. **Paralela com:** Task 2.

**Files:**
- Modify: `Cargo.toml` (workspace) — adicionar `rand = { version = "0.9" }` em `[workspace.dependencies]`.
- Modify: `crates/derust/Cargo.toml` — `rand = { workspace = true, optional = true }` em `[dependencies]` e `"dep:rand"` na feature `sqs`.
- Create: `crates/derust/src/awsx/sqsx/retry.rs`
- Modify: `crates/derust/src/awsx/sqsx/mod.rs`

**API a implementar (assinaturas):**

```rust
use std::time::Duration;

/// Teto de visibility timeout da SQS (12h).
pub const SQS_MAX_VISIBILITY_TIMEOUT: Duration = Duration::from_secs(43_200);

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RetryPolicy { base_delay: Duration, max_delay: Duration, factor: f64 }

impl Default for RetryPolicy { /* base 5s, max 15min (900s), factor 2.0 */ }

impl RetryPolicy {
    pub fn new(base_delay: Duration, max_delay: Duration, factor: f64) -> Self;
    pub fn with_base_delay(self, base_delay: Duration) -> Self;
    pub fn with_max_delay(self, max_delay: Duration) -> Self;
    pub fn with_factor(self, factor: f64) -> Self;
    pub fn base_delay(&self) -> Duration;
    pub fn max_delay(&self) -> Duration;
    pub fn factor(&self) -> f64;

    /// min(max, base * factor^(n-1)), n = receive_count (0 tratado como 1), teto 12h. Sem jitter.
    pub fn max_delay_for(&self, receive_count: u32) -> Duration;

    /// Full jitter: sorteia em [0, max_delay_for(n)].
    pub fn delay_for(&self, receive_count: u32) -> Duration;

    /// Determinístico para teste: sample em [0,1) (valores fora do intervalo são clampados;
    /// NaN vira 0.0).
    pub(crate) fn delay_with_sample(&self, receive_count: u32, sample: f64) -> Duration;
}

/// Duration -> segundos i32 para o SDK (trunca; satura em 43_200).
pub(crate) fn to_visibility_timeout_secs(delay: Duration) -> i32;
```

Algoritmo de `max_delay_for` (seguir exatamente; evita panic de `Duration::from_secs_f64`):

```rust
let factor = if self.factor.is_finite() && self.factor >= 1.0 { self.factor } else { 1.0 };
let exp = receive_count.max(1).saturating_sub(1);          // u32
let grown = self.base_delay.as_secs_f64() * factor.powi(exp.min(i32::MAX as u32) as i32); // pode ser +inf
let cap = self.max_delay.min(SQS_MAX_VISIBILITY_TIMEOUT).as_secs_f64();
let secs = if grown.is_finite() { grown.min(cap) } else { cap };
Duration::from_secs_f64(secs.max(0.0))   // secs é finito e >= 0 aqui
```

`delay_with_sample`: `Duration::from_secs_f64(max_delay_for(n).as_secs_f64() * sample_clamped)`.
`delay_for`: `delay_with_sample(n, rand::random::<f64>())`.

- [x] **Step 1: Dependências.** Adicionar `rand` conforme acima; `cargo build --features sqs` deve compilar e `Cargo.lock` não deve resolver uma versão de `rand` além de uma 0.9.x já presente.

- [x] **Step 2: Escrever os testes primeiro** (`#[cfg(test)] mod tests` em `retry.rs`), todos falhando por `todo!()`/ausência:
  - `default_policy_is_5s_15min_factor_2` — `Default` retorna 5s/900s/2.0.
  - `max_delay_grows_exponentially_until_cap` — defaults: n=1→5s, n=2→10s, n=3→20s, n=8→640s, n=9→900s (capado), n=100→900s.
  - `receive_count_zero_is_treated_as_first_attempt` — n=0 igual a n=1.
  - `no_overflow_on_huge_receive_count` — `u32::MAX` retorna 900s sem panic (factor 2.0 vira `inf` em `powi`).
  - `max_delay_is_clamped_to_12_hours` — `max=100h` e n alto → exatamente 43_200s.
  - `factor_below_one_or_non_finite_behaves_as_constant` — factor 0.5, NaN, `INFINITY`, negativo → atraso = `min(base, max)` para qualquer n.
  - `zero_base_gives_zero_delay`.
  - `max_below_base_caps_at_max` — base 60s, max 10s → 10s.
  - `full_jitter_sample_bounds` — sample 0.0 → 0s; sample 0.5 → metade do teto; sample 0.999999 ≤ teto; sample 1.5 e -1.0 e NaN são clampados/zerados sem panic.
  - `delay_for_stays_within_zero_and_ceiling` — 1000 iterações com n=3 (teto 20s): sempre `<= 20s`.
  - `to_visibility_timeout_secs_truncates_and_saturates` — 1.9s→1; 0→0; 43_200s→43_200; 100h→43_200; `Duration::MAX`→43_200.

- [x] **Step 3: Implementar** até verde. `cargo nextest run --features sqs -- retry`.

- [x] **Step 4:** `mod retry;` + `pub use retry::RetryPolicy;` em `mod.rs` (a constante e a função de conversão ficam `pub(crate)`/`pub` dentro de `retry.rs`; só `RetryPolicy` é re-exportada). Rustdoc em cada item público explicando a fórmula e os defaults.

- [x] **Step 5: Lint e commit.** `cargo clippy --features sqs -- -D warnings`; commit `feat(sqsx): add RetryPolicy with full-jitter exponential backoff`.

**Plano de testes:** os 11 testes acima (puros, sem rede, sem relógio).
**Plano de revisão:** confirmar (a) nenhum caminho chama `Duration::from_secs_f64` com `inf`/NaN/negativo; (b) `powi` com expoente convertido sem overflow de `as i32`; (c) defaults exatamente 5s/900s/2.0; (d) `rand` só é usado em `delay_for`; (e) nenhum `unwrap`.

---

### Task 2: `Message::receive_count()` e pedido de `ApproximateReceiveCount` no `receive_message`

**Dificuldade:** simples.

**Depende de:** nada. **Paralela com:** Task 1.

**Files:**
- Modify: `crates/derust/src/awsx/sqsx/consumer.rs`
- Modify: `crates/derust/src/awsx/sqsx/runner.rs` (apenas a chamada `receive_message`)

**API:**

```rust
use aws_sdk_sqs::types::MessageSystemAttributeName;

impl Message {
    /// Número da tentativa atual (1 = primeira entrega). Lê o atributo de sistema
    /// `ApproximateReceiveCount`; ausente, não numérico ou 0 => 1.
    pub fn receive_count(&self) -> u32 {
        self.0.attributes()
            .and_then(|a| a.get(&MessageSystemAttributeName::ApproximateReceiveCount))
            .and_then(|v| v.parse::<u32>().ok())
            .filter(|n| *n >= 1)
            .unwrap_or(1)
    }
}

impl MessageBuilder {
    /// Para testes de handlers: define `ApproximateReceiveCount`.
    pub fn receive_count(self, count: u32) -> Self {
        Self(self.0.attributes(MessageSystemAttributeName::ApproximateReceiveCount, count.to_string()))
    }
}
```

Em `poll_queue`, adicionar à chamada `receive_message()`:
`.message_system_attribute_names(MessageSystemAttributeName::ApproximateReceiveCount)`.

- [x] **Step 1: Testes primeiro** (`#[cfg(test)] mod tests` em `consumer.rs`):
  - `receive_count_defaults_to_one_when_attribute_missing` — `Message::builder().body("x").build().receive_count() == 1`.
  - `receive_count_reads_builder_value` — `.receive_count(4)` → 4.
  - `receive_count_invalid_or_zero_is_one` — montar `SdkMessage::builder().attributes(ApproximateReceiveCount, "abc")` e `"0"` e `""` e `"-3"`, via `Message::new(..)` (é `pub(crate)`, ok em teste do mesmo crate) → 1.
  - `receive_count_large_value` — `"4294967295"` → `u32::MAX`; `"99999999999"` (overflow de u32) → 1.
- [x] **Step 2: Implementar**, rodar `cargo nextest run --features sqs -- receive_count`.
- [x] **Step 3: Ajustar o `receive_message`** e rodar `cargo build --features sqs` (compila é a verificação; o comportamento de rede é coberto na Task 5).
- [x] **Step 4: Lint e commit** `feat(sqsx): expose Message::receive_count and request ApproximateReceiveCount`.

**Plano de testes:** 4 testes unitários acima + build.
**Plano de revisão:** confirmar que `Message` não ganha campo novo (é derivado do SDK), que `MessageBuilder::receive_count` não quebra os métodos existentes e que o atributo é pedido via `message_system_attribute_names`, não `attribute_names`.

---

### Task 3: Builder `SqsQueueConsumer::with_retry_policy`

**Dificuldade:** simples.

**Depende de:** Task 1.

**Files:**
- Modify: `crates/derust/src/awsx/sqsx/consumer.rs`

**API:**

```rust
use super::retry::RetryPolicy;

pub struct SqsQueueConsumer<S> where S: Clone + Send + Sync + 'static {
    pub(crate) queue_url: String,
    pub(crate) handler: HandlerFn<S>,
    pub(crate) retry_policy: Option<RetryPolicy>,   // novo; None em new()
}

impl<S> SqsQueueConsumer<S> where S: Clone + Send + Sync + 'static {
    // new(): INALTERADA na assinatura; apenas inicializa retry_policy: None.
    pub fn with_retry_policy(mut self, policy: RetryPolicy) -> Self { self.retry_policy = Some(policy); self }
}
```

- [x] **Step 1: Testes primeiro** (`consumer.rs`):
  - `new_has_no_retry_policy` — `SqsQueueConsumer::<()>::new("q", |_c, _m| async { Ok(()) }).retry_policy.is_none()` (usar um estado `()`/struct `Clone` trivial).
  - `with_retry_policy_sets_policy` — `.with_retry_policy(RetryPolicy::default()).retry_policy == Some(RetryPolicy::default())`.
  - `with_retry_policy_keeps_queue_url_and_handler` — `queue_url` preservado.
- [x] **Step 2: Implementar** e rodar `cargo nextest run --features sqs -- retry_policy`.
- [x] **Step 3: Lint e commit** `feat(sqsx): add SqsQueueConsumer::with_retry_policy (opt-in)`.

**Plano de testes:** 3 testes unitários.
**Plano de revisão:** `new()` continua com a mesma assinatura (`git diff` não toca a linha `pub fn new<H, Fut>(queue_url: &str, handler: H) -> Self`); campo é `pub(crate)`, não público.

---

### Task 4: `change_message_visibility` no ramo `Err` do runner

**Dificuldade:** complexa (único ponto com I/O e interação com o SDK; exige teste com cliente simulado).

**Depende de:** Tasks 1, 2 e 3.

**Files:**
- Modify: `crates/derust/src/awsx/sqsx/runner.rs`
- Modify: `crates/derust/Cargo.toml` (`[dev-dependencies]`, se for usar mock)

**Mudanças em `process_message`:**

1. Antes de `let result = (consumer.handler)(..., msg)`, ler `let receive_count = msg.receive_count();` (o `msg` é movido para o handler; extrair junto de `message_id`/`body`/`receipt_handle`).
2. No ramo `Err(err)`, depois do `error!` existente (que permanece idêntico):

```rust
if let Some(delay_secs) = retry_delay_secs(consumer.retry_policy.as_ref(), receive_count) {
    apply_retry_backoff(sqs, &consumer.queue_url, &receipt_handle, &message_id, receive_count, delay_secs).await;
}
```

**Funções novas em `runner.rs`:**

```rust
/// None quando não há política (comportamento atual preservado).
fn retry_delay_secs(policy: Option<&RetryPolicy>, receive_count: u32) -> Option<i32> {
    policy.map(|p| to_visibility_timeout_secs(p.delay_for(receive_count)))
}

async fn apply_retry_backoff(
    sqs: &SqsClient, queue_url: &str, receipt_handle: &str,
    message_id: &str, receive_count: u32, delay_secs: i32,
) {
    match sqs.client.change_message_visibility()
        .queue_url(queue_url).receipt_handle(receipt_handle).visibility_timeout(delay_secs)
        .send().await
    {
        Ok(_) => info!("SQS message retry scheduled :: queue={} :: message_id={} :: receive_count={} :: retry_delay_secs={}", ...),
        Err(e) => error!("Failed to apply retry delay :: queue={} :: message_id={} :: receive_count={} :: retry_delay_secs={} :: error={:?}", ...),
    }
}
```

Nada pode usar `?`, `unwrap` ou `panic!` aqui; falha só é logada. Se `receipt_handle` for vazio
(`unwrap_or_default()` existente), ainda assim a chamada é feita e a eventual falha é só logada
(comportamento análogo ao `delete_message` atual, que ignora o resultado).

- [x] **Step 1: Testes puros primeiro** (`runner.rs`, `#[cfg(test)]`):
  - `retry_delay_secs_is_none_without_policy` — `retry_delay_secs(None, 5) == None`.
  - `retry_delay_secs_is_within_ceiling` — política default, n=3, 500 iterações: `Some(s)` com `0 <= s <= 20`.
  - `retry_delay_secs_never_exceeds_12h` — política `max=100h`, n=`u32::MAX`: `s <= 43_200` e `s >= 0`.
- [x] **Step 2: Teste de I/O com cliente simulado.** Montar um `aws_sdk_sqs::Client` de teste cuja camada HTTP é substituída (preferência: `aws-smithy-mocks` como dev-dependency; alternativa: `aws-smithy-http-client` com `test-util` e `StaticReplayClient`; checar qual compila com o `aws-sdk-sqs` 1.74 / `aws-config` 1.8 do workspace e, se nenhum compilar sem inflar o lockfile de forma desproporcional, parar e reportar em vez de improvisar). Construir `SqsClient { client }` (campo `pub`). Casos:
  - `apply_retry_backoff_sends_visibility_timeout` — o mock captura a requisição; assertar que a ação é `ChangeMessageVisibility`, com `QueueUrl`, `ReceiptHandle` e `VisibilityTimeout` iguais aos passados.
  - `apply_retry_backoff_swallows_service_error` — o mock responde erro (ex.: `ReceiptHandleIsInvalid`/HTTP 400); a função retorna normalmente (sem panic) e uma chamada subsequente funciona (consumidor não é derrubado).
- [x] **Step 3: Implementar** as duas funções e a integração no ramo `Err`; rodar `cargo nextest run --features sqs -- runner`.
- [x] **Step 4: Conferir métrica.** Com `--features sqs,prometheus` e `--features sqs,statsd`, `cargo build` ok; o bloco `#[cfg(any(feature = "statsd", feature = "prometheus"))]` do `stopwatch.record` permanece intacto.
- [x] **Step 5: Lint e commit** `feat(sqsx): apply exponential backoff with full jitter via ChangeMessageVisibility on handler failure`.

**Plano de testes:** 3 testes puros + 2 testes com cliente simulado.
**Plano de revisão:** (a) o `error!` existente do ramo `Err` e o ramo `Ok` não mudaram; (b) sem política, nenhuma chamada nova é feita (função pura devolve `None` e o `if let` não entra); (c) a falha do SDK nunca propaga; (d) o log do atraso contém `retry_delay_secs` e `receive_count`; (e) o `msg` não é usado após o move para o handler.

---

### Task 5: Testes de regressão e de comportamento ponta a ponta do ramo de falha

**Dificuldade:** média.

**Depende de:** Task 4.

**Files:**
- Modify: `crates/derust/src/awsx/sqsx/runner.rs` (módulo de testes) — reutilizar o cliente simulado da Task 4.

Se `process_message` exigir um `AppContext` real para ser chamado, construir um com os helpers de teste já usados em `crates/derust` (procurar `AppContext::new` em testes existentes, ex.: em `httpx`) — features mínimas `sqs` + `http_server`.

- [x] **Step 1: Testes** (cada um com o mock registrando as chamadas recebidas):
  - `failing_handler_with_policy_changes_visibility` — consumer com `with_retry_policy(RetryPolicy::default())`, handler `Err`, mensagem com `receive_count(3)`: exatamente 1 chamada `ChangeMessageVisibility` com `VisibilityTimeout` em `[0, 20]`, e **nenhuma** `DeleteMessage`.
  - `failing_handler_without_policy_makes_no_sqs_calls` — consumer só com `new()`: **zero** chamadas ao SDK (preserva o comportamento atual).
  - `successful_handler_with_policy_deletes_and_does_not_change_visibility` — `Ok(())`: 1 `DeleteMessage`, 0 `ChangeMessageVisibility`.
  - `visibility_failure_does_not_stop_processing` — o mock falha em `ChangeMessageVisibility`; `process_message` retorna e uma segunda mensagem do lote é processada normalmente.
  - `handler_sees_receive_count` — handler captura `msg.receive_count()` e confirma o valor vindo do atributo (e 1 quando ausente).
- [x] **Step 2:** `cargo nextest run --features sqs`; também `--features sqs,prometheus` (confirma que `sqs_consumer_seconds` continua compilando/rodando no mesmo caminho).
- [x] **Step 3: Commit** `test(sqsx): cover retry backoff and no-policy regression paths`.

**Plano de testes:** os 5 testes acima.
**Plano de revisão:** conferir que cada critério de sucesso do plano de negócio tem pelo menos um teste: crescimento limitado (Task 1), número da tentativa acessível (Task 2 e `handler_sees_receive_count`), sem política = igual a hoje (`failing_handler_without_policy_makes_no_sqs_calls`), falha ao ajustar atraso não interrompe (`visibility_failure_does_not_stop_processing`), log do atraso (Task 4).

---

### Task 6: Documentação (README, exemplo) e CHANGELOG

**Dificuldade:** simples.

**Depende de:** Task 5.

**Files:**
- Modify: `crates/derust/README.md` — nova seção `## SQS consumer retry with exponential backoff` antes de `## Features` (ou logo após `## Tests`).
- Modify: `CHANGELOG.md` — nova entrada.
- Modify: rustdoc em `retry.rs`/`consumer.rs` (já iniciado nas tarefas anteriores; revisar).

**Conteúdo do README (exemplo compilável conceitualmente):**

```rust
use derust::awsx::sqsx::{RetryPolicy, SqsQueueConsumer};
use std::time::Duration;

let consumer = SqsQueueConsumer::new(&queue_url, |ctx, msg| async move {
    tracing::info!("attempt {}", msg.receive_count());
    process(ctx, msg).await
})
.with_retry_policy(
    RetryPolicy::default()                       // base 5s, max 15min, factor 2.0
        .with_base_delay(Duration::from_secs(2)) // opcional
        .with_max_delay(Duration::from_secs(600)),
);
```

Deve explicar: opt-in; fórmula full jitter `random(0, min(max, base*fator^(n-1)))`; teto
de 12h da SQS; `receive_count()` (1 = primeira entrega; ausente = 1); log
`SQS message retry scheduled ... retry_delay_secs=` ; falha ao ajustar o atraso só gera log e a
mensagem volta pelo visibility timeout da fila; a redrive policy/DLQ continua valendo
(`maxReceiveCount` é contado pela SQS normalmente); o lote é processado sequencialmente, então
o tempo gasto nas mensagens anteriores consome visibility timeout das seguintes (dimensione o
timeout da fila); o backoff **não** se aplica ao erro de `receive_message`; sem
`with_retry_policy` nada muda; feature necessária: `sqs`.

**CHANGELOG** (`### Added` na entrada vigente, versão a seguir a convenção do arquivo; não
alterar o número da versão do crate sem decisão explícita):
- `sqsx::RetryPolicy` + `SqsQueueConsumer::with_retry_policy` (opt-in).
- `Message::receive_count()` / `MessageBuilder::receive_count()`.
- `receive_message` agora solicita `ApproximateReceiveCount`.
- Nova dependência opcional `rand` na feature `sqs`.

- [x] **Step 1:** Escrever README e CHANGELOG. **Step 2:** `cargo doc --features sqs --no-deps` sem warnings de rustdoc nos itens novos. **Step 3:** commit `docs(sqsx): document retry policy and receive_count, update CHANGELOG`.

**Plano de testes:** revisão textual + `cargo doc`; se o trecho do README for copiado como doc-test, `cargo test --doc --features sqs`.
**Plano de revisão:** README e CHANGELOG não prometem backoff no erro de `receive_message`, nem decisão de retry por tipo de erro, nem paralelismo; defaults documentados batem com `RetryPolicy::default()`.

---

### Task 7: Validação final — fecha o plano

**Dificuldade:** simples.

**Depende de:** todas.

- [x] `task lint` (fmt + clippy `-D warnings`) e `cargo clippy --features sqs,prometheus -- -D warnings`, `--features sqs,statsd`.
- [x] `task test` completo; mais `cargo nextest run --features sqs`.
- [x] `cargo build` (default) e `cargo build --features sqs`: confirma que o gate de feature está correto.
- [x] `git diff main --stat`: nenhum arquivo fora do mapa de arquivos acima; `Cargo.lock` só ganha `rand` (se já não estiver como dependência do crate) e eventuais dev-deps de teste, sem bump de versões existentes.
- [x] Marcar as tarefas como concluídas neste documento e commitar `docs(plan): mark tasks as completed`.

---

## Plano de testes e revisão por tarefa (resumo)

| Tarefa | Dificuldade | Testes | Revisão |
|---|---|---|---|
| 1 — `RetryPolicy` | média | 11 unitários puros (`nextest -- retry`) | sem panic em `from_secs_f64`; defaults 5s/15min/2.0; teto 12h |
| 2 — `receive_count` | simples | 4 unitários (`-- receive_count`) | ausente/inválido/0 = 1; atributo pedido via `message_system_attribute_names` |
| 3 — `with_retry_policy` | simples | 3 unitários | `new()` intacta; campo `pub(crate)` |
| 4 — backoff no runner | complexa | 3 puros + 2 com cliente simulado | falha não propaga; sem política = nenhuma chamada; log com atraso |
| 5 — regressão e2e | média | 5 testes com mock | cada critério de sucesso coberto |
| 6 — docs | simples | `cargo doc` / revisão | sem promessas fora de escopo |
| 7 — validação | simples | lint + testes + builds por feature | diff restrito ao escopo |

## Paralelização

- Task 1 e Task 2 em paralelo (subagentes distintos; arquivos disjuntos exceto `runner.rs`,
  que só a Task 2 toca nesta fase).
- Depois, cadeia linear: 3 -> 4 -> 5 -> 6 -> 7. Task 3 só precisa da Task 1, então pode
  começar assim que ela terminar, ainda com a Task 2 em andamento; a Task 4 espera as três.
