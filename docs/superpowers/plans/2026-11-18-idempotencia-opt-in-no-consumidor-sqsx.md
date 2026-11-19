# Idempotência opt-in no consumo SQS (sqsx) — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Issue:** TAP-99 (Linear). **Branch/worktree:** `idempotencia_opt_in_no_consumidor_sqsx` (`.worktrees/idempotencia_opt_in_no_consumidor_sqsx`).
**Spec:** plano de negócio aprovado no vault Obsidian: `Projects/derust/plans/idempotencia_opt_in_no_consumidor_sqsx.md` (decisões D1 a D10 aprovadas; este documento só decide o "como").

**Goal:** `SqsQueueConsumer` ganha idempotência opt-in: para uma mesma chave, o handler tem efeito no máximo uma vez com sucesso, mesmo com duplicata/concorrência. Sem configuração, comportamento, assinaturas e chamadas são idênticos aos atuais (R1).

**Tech stack:** Rust 1.96, `aws-sdk-sqs`, `sqlx` 0.9 (já no workspace), `tokio`. **Nenhuma dependência nova** em nenhuma feature.

---

## Estado atual do código (leia antes de implementar)

Tudo em `crates/derust/src/awsx/sqsx/` (módulo existe só com a feature `sqs` = `aws` + `wg` + `rand`):

- `consumer.rs`: `Message` (wrapper do `SdkMessage`; `message_id()`, `message_attribute(name)`, `receive_count()`), `SqsQueueConsumer<S>` com campos `pub(crate) queue_url`, `handler: HandlerFn<S>` (`Arc<dyn Fn(AppContext<S>, Message, DerustTags) -> Pin<Box<dyn Future<Output=Result<(),DerustError>>+Send>>>`), `retry_policy: Option<RetryPolicy>`; `new()` e `with_retry_policy()`.
- `runner.rs`: `run(wg, context, consumers)` cria cliente SQS e faz `tokio::spawn(poll_queue)` por consumer. `poll_queue` faz `receive_message` pedindo `ApproximateReceiveCount` e o atributo `x-trace-id` (`TRACE_ID_ATTRIBUTE`). `process_message` chama o handler, em `Ok` loga e faz `delete_message` best-effort, em `Err` loga e aplica `apply_retry_backoff` se houver `retry_policy`; no fim grava a métrica `sqs_consumer_seconds` com tag `success` (só com `statsd`/`prometheus`).
- Testes de `process_message` usam um SQS fake (`fake_sqs`, HTTP em memória) e `AppContext::new("test", Environment::Test, ())`; só compilam quando **não** há `statsd/prometheus/postgres/outbox/growthbook` (o construtor do `AppContext` muda com as features).
- `databasex` (`crates/derust/src/databasex/postgresx/database.rs`): `PostgresDatabase { read_write: Pool, read_only: Option<Pool> }`; `begin_transaction` **sempre** usa `read_write`. `PostgresTransaction` tem **duas formas**: com `statsd`/`prometheus` é `PostgresTransaction<'a, S>` (e `begin_transaction(context, operation_name, tags)`); sem elas é `PostgresTransaction<'a>` (e `begin_transaction(context, tags)`). Ambas têm `pub transaction: sqlx::Transaction<'a, Postgres>` e `commit_transaction(self, &tags)`.
- Outbox: o derust **não cria** a tabela em runtime; o serviço aplica a migration (exemplo em `examples/outbox/database/migrations/1_outbox_pattern_table.sql`). D9 segue o mesmo caminho.
- Convenção de release (ver commits recentes e `CHANGELOG.md`): feature aditiva = bump de patch em `Cargo.toml` raiz (`workspace`), `crates/derust/Cargo.toml`, tabela de versões do README (`| 0.7.3 | sdui_stac_1 | ...`), `Cargo.lock`, mais seção em `CHANGELOG.md`. Esta entrega vira **0.7.4** (nada existente quebra: a "quebra de assinatura" aceita em D8 vale só para o handler da entrada nova `new_transactional`).

## Decisões de mecânica (o "como") — não reabrir sem `AskUserQuestion`

**M1. Features.** Contrato, config, motor, store em memória e modo não transacional vivem sob `feature = "sqs"` apenas (sem sqlx). Store Postgres, SQL, modo transacional e limpeza sob `all(feature = "sqs", feature = "postgres")`. Nenhuma entrada nova em `[features]`/`[dependencies]`. Builds sem `postgres` não compilam nada de sqlx neste módulo. O contrato do store **não usa `async-trait`** (não está na feature `sqs`): métodos retornam `Pin<Box<dyn Future<Output = ...> + Send + '_>>` (alias `BoxFuture`).

**M2. Contrato do store (não transacional)**, `trait IdempotencyStore: Send + Sync + 'static`:
- `try_acquire(&self, scope: &str, key: &str, in_progress_ttl: Duration) -> BoxFuture<Result<Acquire, StoreError>>`, `enum Acquire { Acquired, AlreadyCompleted, InProgress }`.
- `complete(&self, scope, key, completed_ttl) -> BoxFuture<Result<(), StoreError>>`
- `release(&self, scope, key) -> BoxFuture<Result<(), StoreError>>`
- `cleanup_expired(&self, batch_size: u32) -> BoxFuture<Result<u64, StoreError>>` (default: `Ok(0)`).
- `StoreError(String)` simples (Display + Error). `scope` = `queue_url` (D3). Chave completa = `(scope, key)`.
- Store em memória público `InMemoryIdempotencyStore` (std `Mutex<HashMap<(String,String), Entry>>`, `Instant` para expiração) para testes de quem usa o derust e para os testes do motor. Aceita relógio injetável só internamente (`pub(crate)`) para testar TTL sem `sleep`.

**M3. Configuração** `IdempotencyConfig` (builder, `Default`): `key_attribute` = `"x-idempotent-key"` (D1), `completed_ttl` = **7 dias**, `in_progress_ttl` = **5 minutos** (D6; docs: deve ser maior que o tempo máximo do handler e alinhado ao visibility timeout; `completed_ttl` deve cobrir a janela de reentrega/republicação, retenção SQS até 14 dias), `cleanup: Option<CleanupConfig>` = `None` (D7, desligada), `CleanupConfig { interval: 1h, batch_size: 1000 }`. Setters `with_key_attribute`, `with_completed_ttl`, `with_in_progress_ttl`, `with_cleanup`. Valida: TTLs > 0, `key_attribute` não vazio, `batch_size` > 0 (retorna `Result<_, DerustError>` ou panica? **Decisão: `with_*` infalíveis; a validação ocorre em `SqsQueueConsumer::with_idempotency`/`new_transactional`, que retornam `Result<Self, IdempotencyConfigError>`**).

**M4. API pública.** `SqsQueueConsumer::with_idempotency(self, store: Arc<dyn IdempotencyStore>, config: IdempotencyConfig) -> Result<Self, IdempotencyConfigError>` (modo não transacional, qualquer backend). `SqsQueueConsumer::new_transactional(queue_url, config: PostgresIdempotencyConfig, handler) -> Result<Self, IdempotencyConfigError>` (modo transacional, só Postgres). `with_idempotency` em consumer transacional é erro de compilação evitado por tipo: `new_transactional` produz o mesmo `SqsQueueConsumer<S>`, e `with_idempotency` retorna `Err(IdempotencyConfigError::AlreadyTransactional)` se o consumer já for transacional.

**M5. Chave (D1/D2).** `resolve_key(msg, key_attribute) -> Resolved { key, source: Attribute | MessageIdFallback }`: atributo String presente e não vazio (após `trim`) vence; senão `message_id`. Fallback loga `debug`/`info` (`idempotency key fallback to message_id`). Se nem atributo nem `message_id` existirem (não ocorre na SQS real): fail-closed (não executa, não apaga, loga `error`, conta como falha de store). **`poll_queue` só pede o atributo da chave em `message_attribute_names` quando há idempotência configurada** (sem isso o SQS não o devolve e o dedup cairia sempre no fallback); sem idempotência a requisição é byte a byte a de hoje (R1).

**M6. Fluxo não transacional** (`process_message`, só quando `consumer.idempotency` for `Some(NonTransactional)`):
1. resolve chave; `try_acquire(queue_url, key, in_progress_ttl)`.
2. `Err(store)` → fail-closed (D5): loga `error`, não executa, não apaga, **não** aplica backoff (não é falha do handler), métrica `success=false` + tag `idempotency=store_error`.
3. `AlreadyCompleted` → não executa, `delete_message` best-effort, log `info` "duplicate discarded", métrica `success=true` + tag `duplicate=true` (D10).
4. `InProgress` → não executa, **não apaga** (D4a), log `warn`/`info` "duplicate deferred", métrica `success=true` + tag `duplicate=in_progress`; sem backoff.
5. `Acquired` → executa o handler **dentro de `tokio::spawn`** (a future do handler já é `Send + 'static`) e aguarda o `JoinHandle`: `JoinError` com `is_panic()` equivale a falha (assim a chave é liberada em panic sem depender de `catch_unwind`/`futures`). `Ok` → `complete(...)` (falha do `complete` só loga `error`: o efeito já ocorreu; a mensagem é apagada mesmo assim, a chave expira pelo TTL de em andamento) e `delete_message`. `Err`/panic → `release(...)` (falha só loga; expira pelo TTL), **comportamento atual de falha** (não apaga, `apply_retry_backoff` se houver política). Panic: logar `error` e tratar como `Err` (500) — o consumer não cai.
O caminho sem idempotência **não** passa por `spawn` nem por nenhum código novo.

**M7. Modo transacional (D8) — assinatura escolhida: transação por valor, devolvida no `Ok`.** Handler: `Fn(AppContext<S>, Message, DerustTags, IdempotentTx<S>) -> Fut`, `Fut: Future<Output = Result<IdempotentTx<S>, DerustError>> + Send + 'static`. `Ok(tx)` devolve a transação ao consumer (que faz o commit — **o handler não commita**); `Err` (ou panic) derruba a transação (rollback no drop). Motivo: `&mut` + closure async exige HRTB (`for<'a> FnOnce(&'a mut Tx) -> BoxFuture<'a, ..>`), que o compilador infere mal em closures `async move`; por valor funciona com closures normais e permite `tokio::spawn` (captura de panic igual a M6). `IdempotentTx` é um alias com duas definições `cfg` (blocos separados): com `statsd|prometheus`, `pub type IdempotentTx<S> = PostgresTransaction<'static, S>;`; sem elas, `pub type IdempotentTx<S = ()> = PostgresTransaction<'static>;` **não compila** (parâmetro não usado em alias), então nessa forma o alias é `pub type IdempotentTx = PostgresTransaction<'static>;` e o resto do código escreve o tipo via uma macro/`cfg` local (`idem_tx!(S)`) ou duplica o bound sob `cfg`. A Task 4 Step 1 (spike) valida a forma final nas três combinações de features; se o alias sem parâmetro atrapalhar a ergonomia, é decisão de mecânica do implementador, desde que a API pública documentada no README seja a mesma.
   **Pré-requisito de lifetime:** hoje `begin_transaction` devolve `PostgresTransaction<'_, S>` com lifetime elidido preso ao `&self`, o que impede `'static`/`spawn`. Task 4 muda o tipo de retorno para `PostgresTransaction<'static, S>` (e `PostgresTransaction<'static>`); é compatível com chamadores existentes (`'static` é aceito onde se esperava `'a`) — validar com o build do workspace e dos exemplos.
   Fluxo: `context.database().begin_transaction(...)` (operation name `"sqs_idempotent_consumer"` nas features de métrica) → `SET LOCAL lock_timeout = '<ms>ms'` → `INSERT INTO <t> (queue_url, idempotency_key, status, expires_at) VALUES ($1,$2,'completed', now() + make_interval(secs => $3)) ON CONFLICT (queue_url, idempotency_key) DO UPDATE SET status='completed', expires_at = EXCLUDED.expires_at WHERE <t>.expires_at <= now() RETURNING 1`. Linha retornada = chave nova (ou expirada reaproveitada) → executa handler; **sem linha = duplicata concluída** (outra tx já commitou) → rollback, apaga a mensagem, `success=true` + `duplicate=true`. Duplicata concorrente com tx aberta da mesma chave **espera o lock da linha** (comportamento padrão do `ON CONFLICT` em READ COMMITTED, que é o default do Postgres; documentar que o isolamento deve ser READ COMMITTED): se a primeira commita vira duplicata concluída; se dá rollback, a segunda insere e executa. `Ok(tx)` → `tx.commit_transaction(&tags)` (chave + efeitos no mesmo commit) → `delete_message`. Falha no commit → fail-closed (não apaga, deixa reentregar, `success=false`, `idempotency=store_error`). Handler `Err`/panic → rollback, comportamento atual de retry/backoff.
   **Lock timeout:** `PostgresIdempotencyConfig::lock_timeout`, padrão **10 s** (menor que o visibility timeout típico de 30 s; documentar). Erro SQLSTATE `55P03` (`lock_not_available`) no INSERT = duplicata em andamento "adiada": rollback, **não apaga**, sem backoff, `success=true` + `duplicate=in_progress` (mesma semântica do M6.4, SQS reentrega). Qualquer outro erro SQL/de pool = falha de store (fail-closed). Nesse modo não existe estado "em andamento" nem `in_progress_ttl`; só `completed_ttl`.
   **Pool leitura/escrita:** transação e store usam **sempre `read_write`** (réplica de leitura introduz lag e quebraria o dedup); `PostgresIdempotencyStore::new` recebe `&PostgresDatabase` e clona `read_write`; `read_only` é ignorado e isso é documentado. `new_transactional` exige que o `AppContext` tenha banco (garantido pelo construtor com a feature `postgres`).

**M8. Postgres (D9, estilo outbox).** Tabela padrão `sqs_idempotency` (nome configurável, aceita `schema.tabela`; validado por regex `^[A-Za-z_][A-Za-z0-9_]{0,62}(\.[A-Za-z_][A-Za-z0-9_]{0,62})?$` porque é interpolado no SQL — nunca interpolar outra coisa; valores sempre por bind). **Sem criação em runtime** (mesmo caminho do outbox): o derust publica o SQL oficial versionado em `crates/derust/src/awsx/sqsx/idempotency/postgres_migration.sql` (placeholder `{table}`), exposto por `PostgresIdempotencyStore::migration_sql(table) -> Result<String, IdempotencyConfigError>` e copiado como exemplo em `examples/sqs_idempotency/database/migrations/1_sqs_idempotency.sql`. Schema:
```sql
create table sqs_idempotency
(
    queue_url       text        not null,
    idempotency_key text        not null,
    status          text        not null check (status in ('in_progress', 'completed')),
    created_at      timestamptz not null default now(),
    expires_at      timestamptz not null,
    primary key (queue_url, idempotency_key)
);
create index idx_sqs_idempotency_by_expires_at on sqs_idempotency (expires_at);
```
Operações do store não transacional (todas atômicas, uma query cada, relógio do banco via `now()`):
- `try_acquire`: `INSERT ... VALUES ($1,$2,'in_progress', now()+make_interval(secs=>$3)) ON CONFLICT (queue_url,idempotency_key) DO UPDATE SET status='in_progress', expires_at=EXCLUDED.expires_at, created_at=now() WHERE <t>.expires_at <= now() RETURNING 1`; com linha = `Acquired`; sem linha = `SELECT status FROM <t> WHERE ...`: `completed` → `AlreadyCompleted`, `in_progress` ou linha sumida (corrida com release) → `InProgress` (seguro: a SQS reentrega).
- `complete`: `UPDATE <t> SET status='completed', expires_at=now()+make_interval(secs=>$3) WHERE queue_url=$1 AND idempotency_key=$2`.
- `release`: `DELETE ... WHERE queue_url=$1 AND idempotency_key=$2 AND status='in_progress'`.
- `cleanup_expired(n)`: `DELETE FROM <t> WHERE ctid IN (SELECT ctid FROM <t> WHERE expires_at < now() LIMIT $1 FOR UPDATE SKIP LOCKED)` — seguro com várias instâncias, sem lock global.

**M9. Limpeza em background (D7).** Quando `IdempotencyConfig::cleanup`/`PostgresIdempotencyConfig::cleanup` for `Some`, `run()` faz `tokio::spawn` de uma task por consumer que, a cada `interval`, chama `cleanup_expired(batch_size)` em laço enquanto devolver `batch_size` (drena), loga `info` o total e `error` em falha (sem parar), e encerra no `shutdown_signal()`. `run()` também deve aguardar essa task como aguarda os consumers. Desligada por padrão: sem `with_cleanup` nenhuma task é criada. No modo transacional a limpeza usa `PostgresIdempotencyStore` criado de `context.database()` (apenas para o `cleanup_expired`).

**M10. Observabilidade.** Log `info`/`warn`/`error` com `queue`, `message_id`, `trace_id`, `idempotency_key`, `key_source` e `outcome` ∈ `processed | duplicate_discarded | duplicate_deferred | store_error | handler_failed`. Métrica `sqs_consumer_seconds`: `success` como hoje + tag extra **somente** quando houver idempotência: `duplicate=true` (descartada), `duplicate=in_progress` (adiada), `idempotency=store_error`. Sem idempotência as tags continuam exatamente `queue_url` + `success`.

**M11. Testes com Postgres real.** O repositório não tem infraestrutura de banco nos testes. Adotar: `crates/derust/tests/` ou módulo de testes `#[cfg(all(feature="sqs", feature="postgres"))]` que lê `DERUST_TEST_DATABASE_URL`; **sem a variável o teste se auto-ignora** (retorna cedo imprimindo aviso), para `task test` local continuar verde sem banco. Cada teste cria um nome de tabela único (`sqs_idem_<uuid sem hífen>`), aplica o SQL oficial (`migration_sql`) e dropa no fim. O CI (`.github/workflows/ci.yml`) ganha `services: postgres:16` e a variável, e roda os passos `--features sqs,postgres` abaixo. Concorrência real = `tokio::join!`/`spawn` de N tarefas com `Barrier` disputando a mesma chave, com handler que dorme alguns ms e incrementa um contador atômico.

---

## Mapa de dependências

```
T1 (contrato+config+memória) ──┬─> T2 (motor não transacional, runner) ──┐
                               └─> T3 (store Postgres + SQL + harness)  ─┼─> T4 (modo transacional) ─> T5 (limpeza background) ─> T6 (docs/exemplo/CHANGELOG/versão) ─> T7 (gates finais)
```
- T2 e T3 podem rodar **em paralelo** (arquivos disjuntos: T2 mexe em `consumer.rs`/`runner.rs`; T3 só em `idempotency/postgres.rs` + SQL + CI).
- T4 e T5 editam `runner.rs`/`consumer.rs`: **sequenciais** após T2 e T3.
- Cada tarefa termina com `cargo fmt --all` e os gates de lint/teste indicados, e com commit próprio.

---

### Task 1 — Contrato, configuração, chave e store em memória  `[médio]`

**Contexto:** só tipos e lógica pura, sob `feature = "sqs"`, sem tocar no runner. Arquivos novos em `crates/derust/src/awsx/sqsx/idempotency/`: `mod.rs` (re-exports + `BoxFuture`), `store.rs` (M2), `config.rs` (M3, `IdempotencyConfigError`, `CleanupConfig`), `key.rs` (M5 `resolve_key`, `KeySource`), `memory.rs` (`InMemoryIdempotencyStore`). Registrar `mod idempotency;` em `sqsx/mod.rs` e re-exportar: `IdempotencyStore, Acquire, StoreError, IdempotencyConfig, CleanupConfig, IdempotencyConfigError, InMemoryIdempotencyStore, BoxFuture`.

- [x] Step 1: testes de `resolve_key` (atributo presente; vazio; só espaços; ausente → `message_id`; sem nenhum → `None`), de `IdempotencyConfig` (defaults 7d/5min/`x-idempotent-key`/cleanup `None`; setters; validação rejeita TTL 0, atributo vazio, `batch_size` 0).
- [x] Step 2: testes do `InMemoryIdempotencyStore` com relógio injetado: acquire novo → `Acquired`; segundo → `InProgress`; após `complete` → `AlreadyCompleted`; após `release` → `Acquired`; in-progress expirado → `Acquired`; concluída expirada → `Acquired`; escopos diferentes não colidem; `cleanup_expired` remove só expirados e devolve a contagem.
- [x] Step 3: implementar até passar. Testar `try_acquire` concorrente no store em memória (o `Mutex` deve tornar a decisão atômica: com 50 tasks, exatamente uma `Acquired`).
- [x] Step 4: gates: `cargo fmt --all -- --check`; `cargo clippy --all-targets --package derust --no-default-features --features sqs -- -D warnings`; `cargo nextest run --package derust --features sqs`; `cargo clippy -- -D warnings` (default continua verde).
- [x] Step 5: commit `feat(sqsx): idempotency store contract, config and in-memory store`.

**Plano de testes/revisão:** unitários puros e determinísticos (sem `sleep`). Revisão: confirmar que nada novo aparece em `Cargo.toml`; que `cargo tree -p derust --no-default-features --features sqs` não ganhou `sqlx`.

---

### Task 2 — Motor de idempotência não transacional no consumer/runner  `[médio]`  (depende de T1; paralela a T3)

**Contexto:** implementar M4 (`with_idempotency`), M5 (pedido do atributo no `receive_message`), M6 e M10.
- `consumer.rs`: novo campo `pub(crate) idempotency: Option<IdempotencyRuntime>` (enum, por ora só `NonTransactional { store: Arc<dyn IdempotencyStore>, config }`, preparado para a variante transacional da T4). `new()` inicializa `None`.
- `runner.rs`: em `poll_queue`, acrescentar `.message_attribute_names(<key_attribute>)` **apenas se** `consumer.idempotency` for `Some`. Em `process_message`, antes de chamar o handler, ramificar conforme M6. Extrair o fluxo em funções pequenas (`resolve`, `run_handler_guarded` com `tokio::spawn`, `finish_*`) para testar sem SQS real. Reutilizar `apply_retry_backoff`/`retry_delay_secs` e a lógica de delete existentes; **não** alterar o ramo sem idempotência (o `match &result` atual deve ser preservado para esse caso).
- Métrica: `stopwatch.record` ganha tags extras só quando `idempotency` ativo (M10).

- [ ] Step 1 (TDD, módulo `process` de testes existente, usando `InMemoryIdempotencyStore` e `fake_sqs`): (a) sem `with_idempotency` nenhum teste existente muda e o fake não vê chamadas novas; (b) chave nova + sucesso: handler 1x, `DeleteMessage`, chave `AlreadyCompleted` depois; (c) mesma chave reentregue após sucesso: handler 0x, `DeleteMessage` feito; (d) handler `Err`: chave liberada, nenhum `DeleteMessage`, com política `ChangeMessageVisibility`, próxima entrega executa o handler; (e) handler com `panic!`: `process_message` não propaga o panic, chave liberada, tratado como falha; (f) chave `InProgress` (pré-adquirida no store): handler 0x e **nenhuma** chamada SQS; (g) store que devolve `Err` (store de teste que falha): handler 0x, nenhuma chamada SQS, sem backoff; (h) `complete` falhando: ainda apaga a mensagem; (i) atributo ausente/vazio → usa `message_id` (verificar pela chave gravada no store); atributo presente → usa o atributo, e duas mensagens com `message_id` diferentes e mesmo atributo deduplicam; (j) escopo por fila: mesma chave em dois consumers de `queue_url` distintos processa nos dois; (k) `with_idempotency` com config inválida retorna `Err`.
- [ ] Step 2: teste de que `receive_message` pede o atributo só com idempotência (extrair a montagem dos nomes de atributos para uma função pura `message_attribute_names(consumer) -> Vec<String>` e testá-la).
- [ ] Step 3: implementar até verde. Ao rodar com `statsd`/`prometheus`/`postgres` esses testes não compilam (limitação já existente); cobrir as tags de métrica de M10 com teste unitário da função pura que monta as tags (`idempotency_metric_tags(outcome)`), compilável em qualquer feature.
- [ ] Step 4: gates: `cargo fmt --all -- --check`; clippy `--all-targets` nas combinações `sqs`, default, `sqs,statsd`, `sqs,prometheus`, `sqs,postgres`; `cargo nextest run --package derust --features sqs` e `--all-features`.
- [ ] Step 5: commit `feat(sqsx): opt-in idempotency for SQS consumers (non-transactional)`.

**Plano de testes/revisão:** regressão (sem idempotência = zero chamadas novas) é o critério principal; revisar que o ramo antigo de `process_message` ficou intacto (diff pequeno nele) e que `spawn` só ocorre no caminho com idempotência.

---

### Task 3 — Store Postgres, SQL oficial e harness de testes com banco  `[médio]`  (depende de T1; paralela a T2)

**Contexto:** M8 e M11, sob `all(feature="sqs", feature="postgres")`. Arquivos: `idempotency/postgres.rs`, `idempotency/postgres_migration.sql` (via `include_str!`, placeholder `{table}`), `idempotency/table.rs` (validação do nome, M8), re-exportar `PostgresIdempotencyStore`, `DEFAULT_TABLE_NAME`.
- `PostgresIdempotencyStore::new(database: &PostgresDatabase, table: &str) -> Result<Self, IdempotencyConfigError>` (clona só `read_write`); `with_default_table(database)`; `migration_sql(table)`; implementa `IdempotencyStore` com as queries de M8 (strings montadas **uma vez** no `new`, com o nome da tabela já validado; valores por bind; `sqlx::query`, não macros `query!`, para não exigir banco em compile time).
- Harness: helper `test_support` (cfg test) que lê `DERUST_TEST_DATABASE_URL`, devolve `Option<PgPool>` (None → teste retorna cedo com `eprintln!`), cria tabela única via `migration_sql` e dropa no `Drop`/fim. Atualizar `.github/workflows/ci.yml`: `services.postgres` (postgres:16, health check) + `env: DERUST_TEST_DATABASE_URL` + passos `Clippy (sqs + postgres)` e `Tests (sqs + postgres)` (`--package derust --features sqs,postgres`). Atualizar `Taskfile.yml` `test:default` e `lint:default` com as mesmas combinações (documentar a variável no CLAUDE.md, seção Commands/testes).

- [ ] Step 1: testes sem banco: validação de nome de tabela (aceita `sqs_idempotency`, `app.sqs_idempotency`; rejeita `x; drop table y`, vazio, aspas, 64+ chars, começando com dígito); `migration_sql` troca o placeholder e o resultado contém o PK `(queue_url, idempotency_key)`.
- [ ] Step 2: testes com banco real (auto-ignorados sem a variável): acquire/complete/release/expiração (usar TTL de 1 s e `tokio::time::sleep(1.2s)` — única exceção de `sleep`, ou `UPDATE expires_at` direto na tabela para forçar expiração de forma determinística, **preferir o UPDATE**); escopos diferentes; `cleanup_expired` apaga só expirados e respeita `batch_size`; **concorrência real**: 20 tasks com `Barrier` chamando `try_acquire` da mesma chave → exatamente 1 `Acquired`, 19 `InProgress`; duas rodadas de `cleanup_expired` concorrentes não erram nem apagam duas vezes (soma das contagens = linhas expiradas).
- [ ] Step 3: implementar; rodar localmente contra um Postgres (`docker run -e POSTGRES_PASSWORD=... -p 5432:5432 postgres:16`) para validar de fato (o CI cobre o resto).
- [ ] Step 4: gates: `cargo clippy --all-targets --package derust --features sqs,postgres -- -D warnings`; `cargo nextest run --package derust --features sqs,postgres` (com `DERUST_TEST_DATABASE_URL` definida) e sem ela; confirmar que `cargo tree` com só `sqs` não puxa sqlx.
- [ ] Step 5: commit `feat(sqsx): Postgres idempotency store with official migration SQL`.

**Plano de testes/revisão:** o teste de concorrência é o aceite de R3 no nível do store. Revisão de segurança obrigatória: nenhuma interpolação além do nome de tabela validado.

---

### Task 4 — Modo transacional (`new_transactional`)  `[complexo]`  (depende de T2 e T3)

**Contexto:** M7 inteiro. Arquivos: `databasex/postgresx/database.rs` (retorno `'static`), `consumer.rs` (tipos de handler e `new_transactional`), `runner.rs` (ramo transacional de `process_message`), `idempotency/transactional.rs` (config `PostgresIdempotencyConfig` — `table`, `key_attribute`, `completed_ttl`, `lock_timeout` (10 s), `cleanup` — e a query de reserva/erro 55P03), `idempotency/mod.rs` (`IdempotentTx`).
Este é o ponto de maior risco de compilação: as duas formas de `PostgresTransaction` (com/sem métricas) exigem `cfg` duplicado em quatro lugares (alias `IdempotentTx`, bound do handler, chamada de `begin_transaction`, `commit_transaction`). Não tentar unificar com genéricos: duplicar sob `cfg` como o resto do `databasex` já faz.

- [ ] Step 1: **spike de compilação primeiro** (antes de qualquer teste de comportamento): mudar o retorno de `begin_transaction` para `'static`, definir `IdempotentTx`, o `TxHandlerFn<S>` (`Arc<dyn Fn(AppContext<S>, Message, DerustTags, IdempotentTx<S>) -> Pin<Box<dyn Future<Output = Result<IdempotentTx<S>, DerustError>> + Send>> + Send + Sync>`) e `new_transactional` aceitando uma closure `async move` real; provar que compila **nas três combinações** `sqs,postgres`, `sqs,postgres,statsd`, `sqs,postgres,prometheus` e que `cargo build` dos exemplos (`examples/*`) e de `--all-features` continua verde. Se o `'static` não for possível sem quebrar chamadores existentes, **parar e usar `AskUserQuestion`** (a alternativa é um `begin_transaction_owned` novo, aditivo).
- [ ] Step 2: testes de comportamento com Postgres real (auto-ignorados sem `DERUST_TEST_DATABASE_URL`; precisam de `AppContext` com banco, montado conforme o construtor da feature `postgres`, e `fake_sqs`): (a) chave nova + sucesso: handler 1x, linha `completed` existe após o commit, `DeleteMessage`; (b) o efeito do handler (INSERT em tabela de teste via a tx recebida) e a chave são atômicos: se o handler devolve `Err` depois de inserir, **nem o efeito nem a chave** existem (rollback), sem `DeleteMessage`, com política há `ChangeMessageVisibility`, e a próxima entrega executa o handler; (c) panic no handler: rollback, não propaga; (d) duplicata de chave concluída: handler 0x, `DeleteMessage`, `success=true` + `duplicate=true`; (e) **concorrência real**: 2 `process_message` simultâneos da mesma chave (handler dorme ~300 ms): um executa, o outro espera o lock e depois descarta como duplicata concluída (handler total = 1x, ambos apagam); variante em que o primeiro falha: o segundo, após o rollback, executa e conclui (handler 2x no total, 1 sucesso); (f) `lock_timeout` curto (ex. 200 ms) com o primeiro lento: o segundo recebe 55P03 → não executa, **não apaga**, sem backoff; (g) chave expirada (UPDATE `expires_at` para o passado) é reaproveitada; (h) atributo vs `message_id` como na T2; (i) falha de commit simulada (fechar a conexão/pool antes do commit) → não apaga; (j) `with_idempotency` em consumer transacional → `Err(AlreadyTransactional)`.
- [ ] Step 3: implementar o ramo transacional de `process_message`; o `spawn` do handler devolve `Result<IdempotentTx<S>, DerustError>`; commit e delete fora do `spawn`.
- [ ] Step 4: gates: clippy `--all-targets` e nextest em `sqs,postgres`, `sqs,postgres,statsd`, `sqs,postgres,prometheus` e `--all-features` (com a variável do banco); `cargo build` de `examples/outbox` e `examples/database` (consumidores existentes de `begin_transaction`).
- [ ] Step 5: commit `feat(sqsx): transactional idempotent consumer (new_transactional)`.

**Plano de testes/revisão:** os testes (e) e (f) são o aceite de "duplicata concorrente espera o lock". Revisão cuidadosa de: (1) o handler nunca commita; (2) toda saída que não é commit termina em rollback (drop) e **não** deixa conexão presa no pool (checar `pool.size()/num_idle()` ao fim de um teste de falha); (3) o aviso de READ COMMITTED na doc.

---

### Task 5 — Limpeza em background (D7)  `[médio]`  (depende de T4)

**Contexto:** M9. `runner.rs::run()` e `idempotency/cleanup.rs`. A função `run_cleanup_loop(store: Arc<dyn IdempotencyStore>, cfg: CleanupConfig, queue_url: String)` é genérica sobre o contrato (serve ao store em memória e ao Postgres); o ponto de extensão para o modo transacional cria um `PostgresIdempotencyStore` a partir de `context.database()`.

- [ ] Step 1: testes com `InMemoryIdempotencyStore` e `tokio::time::pause()`/`advance`: sem `with_cleanup` nenhuma task é criada (função pura `cleanup_plan(&consumers) -> Vec<...>` vazia); com cleanup roda a cada `interval`, drena em laço enquanto o lote vem cheio, continua após erro do store (store de teste que falha uma vez), para quando o sinal de parada é disparado (injetar um `Future` de shutdown em vez de chamar `shutdown_signal()` diretamente, para testar).
- [ ] Step 2: teste com Postgres real: 2 loops concorrentes sobre a mesma tabela não geram erro e removem todas as linhas expiradas sem tocar nas válidas (SKIP LOCKED).
- [ ] Step 3: implementar e ligar em `run()` (spawn + await no fim junto dos consumers).
- [ ] Step 4: gates como na T4. Commit `feat(sqsx): optional background cleanup of expired idempotency records`.

**Plano de testes/revisão:** garantir que `run()` sem cleanup não muda (mesmo número de tasks). Verificar que o `wg.done()` só ocorre depois de a task de limpeza terminar.

---

### Task 6 — Documentação, exemplo, CHANGELOG e versão  `[simples]`  (depende de T5)

**Contexto:** R10 e a convenção de release (ver "Estado atual").
- [ ] Step 1: `crates/derust/README.md` (e `README.md` da raiz, que espelha a tabela de versões): nova seção "SQS consumer idempotency" (junto de "SQS consumer retry...": mesmo estilo, exemplos `with_idempotency` e `new_transactional`) cobrindo: opt-in; chave (`x-idempotent-key`, fallback `message_id` e seu risco, produtores devem enviar o atributo — o outbox já tem o slot); escopo `(queue_url, key)`; ciclo de vida e tabela de resultados (processada / duplicata descartada / adiada / falha de store); fail-closed; TTLs padrão 7 d / 5 min e como escolher (`in_progress_ttl` > handler máximo e alinhado ao visibility timeout; `completed_ttl` ≥ janela de reentrega/republicação, retenção SQS até 14 d); risco de DLQ por `maxReceiveCount` com duplicata em andamento (D4) e que o modo transacional não tem o problema; modo transacional (assinatura do handler, "não commite", READ COMMITTED, lock timeout, `read_write` apenas); limitação do modo não transacional (efeito do handler fora do store não é atômico: falha de `complete` após sucesso); limpeza opcional; SQL oficial e como aplicar a migration (o derust não cria a tabela); tags de métrica; **registrar que a limpeza nesta entrega pode se sobrepor à TAP-31**.
- [ ] Step 2: exemplo `examples/sqs_idempotency/` (workspace próprio como os demais exemplos: `Cargo.toml` com `derust = { path = "../../crates/derust", features = ["sqs", "postgres"] }`, `src/main.rs` mostrando os dois modos, `database/migrations/1_sqs_idempotency.sql` idêntico ao SQL oficial, `docker-compose.yaml` com postgres + localstack no padrão de `examples/outbox`). Deve compilar (`cargo build` dentro do exemplo).
- [ ] Step 3: `CHANGELOG.md` seção `## [0.7.4]` (`### Added`: idempotência, `new_transactional`, store Postgres, SQL, limpeza opcional, store em memória; `### Changed`: `begin_transaction` devolve `PostgresTransaction<'static, ..>`, compatível). Bump para `0.7.4` em `Cargo.toml` (workspace, se aplicável), `crates/derust/Cargo.toml`, `Cargo.lock` (via `cargo build`), tabela de versões nos dois READMEs. Atualizar a tabela de features do `CLAUDE.md` (`sqs` + `postgres` habilita store Postgres/modo transacional) e a seção de testes (variável `DERUST_TEST_DATABASE_URL`).
- [ ] Step 4: commit `docs(sqsx): idempotency README, example, CHANGELOG (bump to 0.7.4)`.

**Plano de testes/revisão:** o exemplo compila; os trechos do README são os mesmos do exemplo (copiar, não reescrever); link/ortografia do CHANGELOG no mesmo padrão das entradas anteriores.

---

### Task 7 — Verificação final dos gates  `[simples]`  (depende de T6)

- [ ] `cargo fmt --all -- --check`
- [ ] `cargo clippy -- -D warnings`, `cargo clippy --workspace -- -D warnings`, `cargo clippy --all-targets -- -D warnings`
- [ ] clippy `--all-targets --package derust` com: `--no-default-features --features sqs`, `--features sqs,postgres`, `--features sqs,postgres,statsd`, `--features sqs,postgres,prometheus`, `--features cryptox,aws`, `--all-features` (todos `-D warnings`)
- [ ] `cargo nextest run` (default), `--package derust --features sqs`, `--features sqs,postgres` (com e sem `DERUST_TEST_DATABASE_URL`), `--all-features`
- [ ] Prova de R1/R5: `cargo tree -p derust --no-default-features --features sqs | grep -c sqlx` = 0; `git diff main -- crates/derust/Cargo.toml` só muda `version`; nenhum teste anterior foi alterado, só acrescentados.
- [ ] Commit só se algo foi ajustado; `git push -u origin idempotencia_opt_in_no_consumidor_sqsx`.

---

## Pontos a alinhar fora do código (não resolvidos aqui, não alterar issues)

- **TAP-99 x plano:** a issue TAP-99 deixa a limpeza da tabela de idempotência fora do escopo (TAP-31), mas o plano aprovado (D7, decisão do usuário) mantém uma rotina opcional de limpeza, desligada por padrão. Alinhar no Linear se a TAP-31 deve reaproveitar `cleanup_expired`/`CleanupConfig` ou ser reescopada.
- A issue fala de "tabela de eventos processados por consumidor" e "mesma transação do efeito": o modo transacional (T4) atende; o modo não transacional é um complemento para outros backends.
