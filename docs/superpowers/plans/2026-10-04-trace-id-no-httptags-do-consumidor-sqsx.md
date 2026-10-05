# Trace id no HttpTags do consumidor SQS (sqsx) e propagação no outbox — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** (1) No envio via outbox (SQS, SNS e HTTP) o trace atual passa a viajar sempre nos headers, sob a chave `x-trace-id`. (2) No consumo via `sqsx` o handler passa a receber `HttpTags` (por valor) já contendo `x-trace-id`, lido do MessageAttribute de mesmo nome (ou um trace novo se ausente/inválido). Versão 0.6.0 (breaking: nova assinatura do handler).

**Spec:** Plano de negócio aprovado no vault Obsidian "Deroldo": `Projects/derust/plans/trace_id_no_httptags_do_consumidor_sqsx.md`. Branch/worktree: `trace_id_no_httptags_do_consumidor_sqsx` (`.worktrees/trace_id_no_httptags_do_consumidor_sqsx`).

**Tech Stack:** Rust 1.96. Sem novas dependências: `rand` (já na feature `sqs`), `axum-tracing-opentelemetry` (já em `http_server`), `aws-sdk-sqs` 1.74.

## Decisões de negócio já fechadas (não reabrir)

1. Versão 0.6.0; breaking aceito: `HandlerFn` recebe também `HttpTags`, por valor.
2. Chave do trace: `x-trace-id` (a mesma de `httpx/response/{json,text,error}.rs`). `http_clientx`/`traceparent` não mudam.
3. Transporte: header do outbox -> MessageAttribute String do SQS de mesmo nome (o processor faz a conversão).
4. Mensagem sem trace (ou inválido) no consumo: gera trace novo.
5. Envio, para SQS, SNS e HTTP, mesma precedência: header do chamador (qualquer caixa) vale e não é sobrescrito; senão o trace atual é adicionado; `None` + trace -> headers só com o trace; sem trace -> inalterado (inclusive `None`).
6. SQS/SNS (limite de 10 MessageAttributes): headers do usuário têm preferência; sem slot, o trace é omitido sem erro.
7. Fora de escopo: spans/exportação, `traceparent` do `http_clientx`, outras tags no consumidor, retry/backoff/`receive_count`.

## Achados da exploração (contexto para todas as tarefas)

- **Outbox hoje**: `crates/derust/src/outboxx/{sqs,sns,http}.rs` só repassam `headers: Option<HashMap<String,String>>` a `Outbox::sqs/sns/http_post_json` (de `outbox-pattern-processor` 0.4.0) e chamam `insert_outbox`. Cada função existe em duas variantes (`send_to_x` e `send_to_x_with_delay`). `sqs.rs`/`sns.rs` são gated por `aws` (em `outboxx/mod.rs`); `http.rs` não.
- **Processor 0.4.0** (`sqs_notification_service.rs`/`sns_notification_service.rs`): para cada outbox monta o entry com `message_attributes("x-idempotent-key", ...)` **mais** um atributo (`data_type = "String"`) por header. Logo o `x-idempotent-key` ocupa 1 dos 10 slots de MessageAttributes (SQS e SNS iguais): **o teto efetivo é `headers do usuário + 1 (x-idempotent-key) + trace <= 10`**, ou seja, o trace só cabe com até 8 headers do usuário (9 se o usuário mesmo já enviou `x-idempotent-key`, pois a chave é a mesma e é sobrescrita). `Outbox::http_post_json` apenas faz `extend` com `Content-Type`. O destino HTTP converte valores `{{ENV_NAME}}` em env var; um trace id nunca casa com esse padrão.
- **Trace atual**: `axum_tracing_opentelemetry::tracing_opentelemetry_instrumentation_sdk::find_current_trace_id()` (usado em `httpx/response/json.rs` linhas ~60-75, que primeiro olha se `tags` já tem `x-trace-id` em qualquer caixa e só então chama `find_current_trace_id()`). `HttpTags` (`httpx/tags.rs`): `add(k,v)`, `values() -> HashMap`, `Default`.
- **Consumidor hoje**: `awsx/sqsx/consumer.rs` tem `HandlerFn<S> = Arc<dyn Fn(AppContext<S>, Message) -> Pin<Box<dyn Future<Output=Result<(), HttpError>> + Send>> + Send + Sync>`, `Message(SdkMessage)`, `MessageBuilder`, `SqsQueueConsumer::new`. `awsx/sqsx/runner.rs`: `poll_queue` chama `receive_message()...message_system_attribute_names(ApproximateReceiveCount)`; `process_message` extrai `message_id/body/receipt_handle/receive_count`, chama `(consumer.handler)(context.clone(), msg)`, loga `info!/error!` e chama `apply_retry_backoff(...)` (que também loga). A métrica `sqs_consumer_seconds` usa só as tags `queue_url` e `success`. O lote é processado sequencialmente num `for`.
- **SNS -> SQS**: sem raw message delivery, os atributos chegam encapsulados no corpo (envelope JSON). O consumo só considera MessageAttribute direto.
- **Compilação por feature**: `outboxx` só compila com uma feature de métrica (pré-existente). Verificações: `cargo nextest run -p derust --features sqs` e `cargo nextest run -p derust --features outbox,sqs,postgres,prometheus`; lint: `cargo clippy -p derust --features outbox,sqs,postgres,prometheus -- -D warnings` e `--features sqs`. Os testes de `process_message` ficam em `mod process`, que só compila sem `statsd/prometheus/postgres/outbox/growthbook` (use `--features sqs` para rodá-los).
- Há warnings pré-existentes no crate; não os corrija. `cargo fmt` apenas nos arquivos tocados.

## Decisões técnicas (a mecânica — "como")

1. **Ordem dos argumentos do handler**: `Fn(AppContext<S>, Message, HttpTags)`. `HttpTags` entra por último (coerente com `outboxx`, onde `tags` é o último parâmetro) e por valor.
2. **Header do trace no outbox HTTP: `x-trace-id`** (resolve o ponto em aberto do plano). Motivos: é a chave de negócio já decidida; o `traceparent` do `http_clientx` não segue o formato W3C (`{flags}-{trace}-{span}-{flags}`) e exigiria span id, que não existe quando o processor envia depois, fora do span; usar `traceparent` poderia conflitar com o que o destino já interpreta. Risco residual (destino que rejeita header desconhecido) vai para README/CHANGELOG.
3. **Resolução do trace no envio** (`outboxx/trace.rs`): primeiro `x-trace-id` já presente em `tags: &HttpTags` (qualquer caixa, valor não vazio) — espelha `httpx/response/*` e faz o trace do consumidor (que vive só no `HttpTags`, sem span) atravessar um outbox chamado de dentro do handler —, senão `find_current_trace_id()`. Sem nenhum dos dois, nada é adicionado.
4. **Lógica pura e testável** em `crates/derust/src/outboxx/trace.rs`:
   ```rust
   pub(crate) const TRACE_ID_HEADER: &str = "x-trace-id";
   pub(crate) const SQS_SNS_MAX_ATTRIBUTES: usize = 10;
   pub(crate) fn resolve_trace_id(tags: &HttpTags) -> Option<String>;
   pub(crate) fn with_trace_header(headers: Option<HashMap<String,String>>, trace_id: Option<&str>) -> Option<HashMap<String,String>>;
   pub(crate) fn with_trace_header_limited(headers: Option<HashMap<String,String>>, trace_id: Option<&str>, reserved_slots: usize, max: usize) -> Option<HashMap<String,String>>;
   ```
   `with_trace_header_limited` só adiciona se `headers.len() + reserved + 1 <= max`; `reserved` = 1 (`x-idempotent-key`) a menos que `headers` já contenha a chave exata `x-idempotent-key` (então 0). Comparação de `x-trace-id` é case-insensitive (`eq_ignore_ascii_case`); chaves são preservadas como o chamador enviou.
5. **Consumo** (`awsx/sqsx/trace.rs`, novo): 
   - `receive_message` passa a pedir `.message_attribute_names("x-trace-id")` (nome exato, não `All`).
   - `Message::message_attribute(&self, name) -> Option<&str>` (pub) lê `string_value()` de `message_attributes()`; `MessageBuilder::message_attribute(name, value)` (pub, para testes de handler; usa `MessageAttributeValue::builder().data_type("String").string_value(v).build()`; se o `build()` falhar, ignora).
   - Trace válido = após `trim()` não vazio, `len() <= 128` e só ASCII imprimível sem espaços (`is_ascii_graphic`). Inválido/ausente -> `new_trace_id()` = `format!("{:032x}", rand::random::<u128>())` (mesmo formato de 32 hex do trace id OTel; refaz o sorteio se der 0). Inválido (presente mas rejeitado) loga `warn!` com `message_id`, sem o valor bruto.
   - `HttpTags` é criado **por mensagem** dentro de `process_message` (nenhum estado compartilhado -> sem vazamento no lote). O valor do trace usado em log é uma `String` mantida fora do `tags` movido ao handler.
6. **Logs**: todas as linhas de `process_message` (`consumed`, `failed`) e as de `apply_retry_backoff` (`retry scheduled`, `Failed to apply retry delay`) ganham o sufixo ` :: trace_id=<id>`. `apply_retry_backoff` recebe `trace_id: &str` como novo parâmetro. **Nenhuma tag de métrica nova**: `sqs_consumer_seconds` segue só com `queue_url` e `success` (cardinalidade).
7. **SNS encapsulado**: apenas documentado (README): para o trace chegar ao consumidor SQS via SNS é preciso habilitar raw message delivery na subscription; sem isso o consumo gera trace novo. Não se faz parsing do envelope.
8. **Reentrega**: o trace vem do atributo da própria mensagem, então é estável entre tentativas; só o trace gerado (atributo ausente) varia — documentar.

## Mapa de dependências e paralelismo

```
T1 -> T2 ---------\
                   >-> T5
T3 -> T4 ---------/
```
- Trilha A (outbox: T1 -> T2) e trilha B (sqsx: T3 -> T4) são independentes (arquivos disjuntos) e **podem rodar em paralelo**.
- T5 (docs, versão) depende de T2 e T4; é sequencial e final.

| Tarefa | Dificuldade | Depende de | Arquivos |
|---|---|---|---|
| T1 helper puro de trace/headers do outbox | médio | — | `outboxx/trace.rs` (novo), `outboxx/mod.rs` |
| T2 aplicar nos 6 pontos de envio | simples | T1 | `outboxx/{sqs,sns,http}.rs` |
| T3 atributos de mensagem + helper de trace do consumo | médio | — | `awsx/sqsx/{consumer,trace,mod}.rs` |
| T4 assinatura do handler, runner, logs | complexo | T3 | `awsx/sqsx/{consumer,runner}.rs` |
| T5 README, CHANGELOG, bump 0.6.0 | simples | T2, T4 | docs, `crates/derust/Cargo.toml`, `Cargo.lock` |

Commits pequenos, um por tarefa, mensagens em inglês (conventional commits), terminando com as linhas de `Co-Authored-By` exigidas pelo ambiente.

---

## Task 1 — Helper puro de trace/headers do outbox (médio)

**Contexto**: ver "Achados" (processor, slots) e decisões 2-4. Nada de I/O; só funções puras + `resolve_trace_id`.

**Arquivos**: criar `crates/derust/src/outboxx/trace.rs`; em `outboxx/mod.rs` adicionar `mod trace;` (sem `pub use`).

- [x] **Step 1 (testes primeiro)**: em `trace.rs`, `#[cfg(test)] mod tests` cobrindo (todas puras, sem span):
  - `with_trace_header`: chamador com `x-trace-id` / `X-Trace-Id` / `X-TRACE-ID` -> inalterado, sem duplicar, mesmo valor do chamador (inclusive vazio); sem a chave + trace -> adiciona `x-trace-id` mantendo os demais; `None` + trace -> `Some({x-trace-id})`; `None` sem trace -> `None`; `Some(h)` sem trace -> `Some(h)` idêntico; `Some(vazio)` + trace -> só o trace.
  - `with_trace_header_limited(.., reserved=1, max=10)`: 7 e 8 headers de usuário -> trace entra (8 -> total 10); 9 e 10 -> sem trace e sem erro; 9 headers incluindo `x-idempotent-key` do usuário -> trace entra (reserved=0); `None` + trace -> entra; usuário com `X-Trace-Id` + 10 headers -> inalterado.
  - `resolve_trace_id`: tags com `X-Trace-Id` não vazio -> esse valor; tags com valor vazio/ausente -> cai em `find_current_trace_id()` (sem span em teste = `None`).
- [x] **Step 2**: implementar conforme assinaturas da decisão 4 (`with_trace_header` = `with_trace_header_limited` sem limite, via função interna comum). `resolve_trace_id` usa `HttpTags::values()` e `find_current_trace_id()`.
- [x] **Step 3**: `cargo nextest run -p derust --features outbox,sqs,postgres,prometheus trace` e clippy verdes.

**Plano de testes**: unitário determinístico (acima); a parte de `find_current_trace_id` com span real é exercida só indiretamente (retorna `None` sem span) — aceitável, a função é do crate externo.
**Revisão**: conferir a conta de slots (`len + reserved + 1 <= 10`), que as chaves do chamador não mudam de caixa, ausência de `unwrap`.

## Task 2 — Aplicar nos pontos de envio (simples) — depende de T1

**Contexto**: seis funções públicas, mesma lógica em cada uma. Assinaturas públicas **não mudam**.

**Arquivos**: `outboxx/sqs.rs`, `outboxx/sns.rs`, `outboxx/http.rs`.

- [x] **Step 1**: em cada função (`send_to_sqs`, `send_to_sqs_with_delay`, `send_to_sns`, `send_to_sns_with_delay`, `send_to_http`, `send_to_http_with_delay`), antes de construir o `Outbox`, calcular `let trace_id = trace::resolve_trace_id(tags);` e substituir `headers` por: SQS/SNS -> `with_trace_header_limited(headers, trace_id.as_deref(), 1, SQS_SNS_MAX_ATTRIBUTES)`; HTTP -> `with_trace_header(headers, trace_id.as_deref())`. (Acesso via `super::trace::...`/`crate::outboxx::trace`.)
- [x] **Step 2**: `cargo check`/clippy nas duas combinações de features; `cargo nextest run` completo.

**Plano de testes**: sem DB real no crate para testar os `send_*`; a lógica está coberta em T1. Aqui o teste é compilação + revisão: as 6 funções usam o helper correto (SQS/SNS limitado, HTTP sem limite). Se existir infraestrutura de teste de outbox com Postgres em `examples/outbox`, rodar manualmente é opcional.
**Revisão**: grep por `Outbox::sqs|sns|http_post_json` garantindo que nenhuma chamada usa `headers` cru.

## Task 3 — Atributos de mensagem + helper de trace do consumo (médio)

**Contexto**: decisões 5 e 8. Não toca o handler nem o runner (isso é T4).

**Arquivos**: `awsx/sqsx/consumer.rs` (só `Message`/`MessageBuilder`), criar `awsx/sqsx/trace.rs`, `awsx/sqsx/mod.rs` (`mod trace;`).

- [x] **Step 1 (testes primeiro)**:
  - `consumer.rs`: `Message::message_attribute` retorna `None` sem atributos; retorna o valor posto via `MessageBuilder::message_attribute`; nome diferente -> `None`; atributo sem `string_value` -> `None`.
  - `trace.rs`: `trace_id_from(&Message) -> (String, bool)` ou equivalente (`fn resolve(msg:&Message) -> TraceResolution { id, generated }` — formato livre, mas testável): atributo válido -> mesmo valor, `generated=false`; ausente -> id novo de 32 hex; vazio, só espaços, > 128 chars, com espaço/controle/não-ASCII -> id novo; duas chamadas sem atributo geram ids diferentes; id gerado nunca é só zeros.
- [x] **Step 2**: implementar `Message::message_attribute`, `MessageBuilder::message_attribute`, `TRACE_ID_ATTRIBUTE = "x-trace-id"`, validação, `new_trace_id()`, e `pub(crate) fn tags_for(msg: &Message) -> (HttpTags, String)` que monta `HttpTags` com `x-trace-id` (`tags.add("x-trace-id", &id)`) e devolve o id também como `String`; `warn!` quando o atributo está presente e inválido (sem logar o valor).
- [x] **Step 3**: `cargo nextest run -p derust --features sqs` e clippy verdes.

**Plano de testes**: unitários (acima), todos sem rede.
**Revisão**: nenhum `unwrap`; `HttpTags` novo a cada chamada; o valor bruto inválido não é logado.

## Task 4 — Assinatura do handler, runner, logs (complexo) — depende de T3

**Contexto**: decisões 1, 5, 6. Breaking change contido em `sqsx`.

**Arquivos**: `awsx/sqsx/consumer.rs`, `awsx/sqsx/runner.rs`.

- [x] **Step 1**: `consumer.rs`: `HandlerFn<S>` -> `Fn(AppContext<S>, Message, HttpTags)`; `SqsQueueConsumer::new` aceita `H: Fn(AppContext<S>, Message, HttpTags) -> Fut`; `Arc::new(move |ctx, msg, tags| Box::pin(handler(ctx, msg, tags)))`. Importar `HttpTags`. Atualizar os testes existentes (`|_c, _m|` -> `|_c, _m, _t|`).
- [x] **Step 2 (testes primeiro, em `runner.rs` `mod process`)**: atualizar `failing()/succeeding()/handler_sees_receive_count` para a nova assinatura e adicionar:
  - handler recebe `HttpTags` com `x-trace-id` igual ao atributo da mensagem (helper `sdk_msg` ganha parâmetro opcional de trace via `MessageBuilder::message_attribute`);
  - mensagem sem atributo -> handler recebe `x-trace-id` de 32 hex;
  - atributo inválido (vazio) -> id novo, mensagem ainda consumida (`DeleteMessage` chamado);
  - lote sequencial: msg A com trace `aaa`, msg B sem atributo -> B não recebe `aaa`; msg C com `ccc` recebe `ccc`;
  - reentrega: duas chamadas com a mesma mensagem (mesmo atributo) -> mesmo trace nas duas;
  - falha do handler com política continua chamando `ChangeMessageVisibility`; sucesso continua `DeleteMessage` (ack/retry inalterados);
  - `apply_retry_backoff` atualizado com o parâmetro `trace_id` (testes existentes ajustados).
- [x] **Step 3**: `runner.rs`: em `poll_queue` acrescentar `.message_attribute_names("x-trace-id")` (usar a constante de `trace.rs`); em `process_message` chamar `let (tags, trace_id) = trace::tags_for(&msg);` **antes** de mover `msg`, passar `tags` ao handler e usar `trace_id` nos logs (decisão 6); `apply_retry_backoff(..., &trace_id)` com sufixo nos dois logs. Não adicionar tag de métrica.
- [x] **Step 4**: `cargo nextest run -p derust --features sqs` (inclui `mod process`), `cargo nextest run -p derust --features outbox,sqs,postgres,prometheus` (compila `server.rs` que usa `SqsQueueConsumer`), clippy nas duas combinações, `cargo build` sem `sqs`.

**Plano de testes**: unitários/integração em memória com o cliente SQS fake já existente (`fake_sqs`); verificar também, no teste de `receive_message`, que o request leva `MessageAttributeNames=["x-trace-id"]` se houver como interceptar (o fake registra o corpo JSON; `poll_queue` é infinito, então testar apenas se extrair o builder do request for simples — caso contrário, validar por revisão e teste manual).
**Revisão**: conferir que o `tags` é movido ao handler e o log usa a `String` separada; que nenhuma tag de métrica contém o trace; que logs de sucesso/falha/retry todos têm `trace_id`.

## Task 5 — Docs, CHANGELOG e versão 0.6.0 (simples) — depende de T2 e T4

**Arquivos**: `crates/derust/README.md`, `crates/derust/src/outboxx/README.md`, `CHANGELOG.md`, `crates/derust/Cargo.toml` (`version = "0.6.0"`), `Cargo.lock` (via `cargo check`), exemplos que referenciem a versão, se houver.

- [x] **Step 1**: `CHANGELOG.md` nova seção `## [0.6.0]` acima de `[0.5.2]`: **Breaking** (nova assinatura `Fn(AppContext<S>, Message, HttpTags)` com antes/depois e guia de migração: handlers existentes passam `|ctx, msg, _tags|`); **Added** (`HttpTags` com `x-trace-id` no consumo, `Message::message_attribute`/`MessageBuilder::message_attribute`, `receive_message` pede o atributo `x-trace-id`); **Changed** (outbox SQS/SNS/HTTP adiciona `x-trace-id` quando há trace; precedência do chamador; limite de 10 atributos com `x-idempotent-key` contado, trace omitido sem erro; HTTP usa `x-trace-id` e destinos estritos podem ser afetados).
- [x] **Step 2**: `crates/derust/README.md`: seção do consumidor SQS com a nova assinatura e exemplo lendo `tags.values().get("x-trace-id")`; documentar: trace novo quando ausente/inválido (pode variar entre reentregas), trace omitido por falta de slot rompe a correlação, SNS exige raw message delivery, trace não vira tag de métrica. Atualizar o exemplo existente de `with_retry_policy` (`|ctx, msg|` -> `|ctx, msg, tags|`). `outboxx/README.md`: comportamento de `x-trace-id` nos headers e o limite de slots.
- [x] **Step 3**: bump de versão; `cargo check -p derust --features sqs` atualiza o lock; `task lint` nos arquivos tocados e `cargo nextest run` final nas duas combinações.

**Plano de testes**: sem código novo; verificar que os trechos Rust do README compilam por inspeção (nenhum doctest roda neles) e que o lock só mudou a versão do `derust`.
**Revisão**: CHANGELOG e README coerentes com as decisões 2, 6, 7, 8; nada de linguagem de decisão de negócio nova.
