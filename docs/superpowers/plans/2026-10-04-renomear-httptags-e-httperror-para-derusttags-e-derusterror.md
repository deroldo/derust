# Renomear HttpTags/HttpError para DerustTags/DerustError — Plano de Implementação

> **Para quem executa:** use `superpowers:subagent-driven-development` ou `superpowers:executing-plans`. Cada tarefa é autocontida. Passos usam checkbox (`- [ ]`).

**Objetivo:** renomear `HttpTags` -> `DerustTags` e `HttpError` -> `DerustError`, movê-los de `httpx` para o novo módulo `commonx` e atualizar crate, exemplos, docs, versão (0.6.0) e CHANGELOG, sem alterar comportamento.

**Plano de negócio (fonte das regras):** vault Obsidian `Projects/derust/plans/renomear_httptags_e_httperror_para_derusttags_e_derusterror.md`. Nenhuma regra de negócio nova pode ser decidida aqui.

**Worktree/branch:** `.worktrees/renomear_httptags_e_httperror_para_derusttags_e_derusterror`, branch `renomear_httptags_e_httperror_para_derusttags_e_derusterror` (base `main`, em b907c3c).

## Decisões de mecânica (o "como")

1. **Layout do módulo:** `crates/derust/src/commonx/{mod.rs, tags.rs, error.rs}`. `tags.rs` = conteúdo de `httpx/tags.rs` renomeado; `error.rs` = conteúdo de `httpx/response/error.rs` renomeado. `commonx/mod.rs` declara `mod tags; mod error;` e faz `pub use tags::DerustTags; pub use error::DerustError;`. Caminho público: `derust::commonx::{DerustTags, DerustError}`.
2. **Feature-gate de `commonx`:** `#[cfg(feature = "http_server")]` (mesmo gate que os tipos têm hoje). Verificado em main: sem `http_server` nenhuma outra feature (`postgres`, `outbox`, `sqs`, `http_client`, `statsd`) compila hoje (`unresolved import crate::httpx`/`tokio`/`axum`). `DerustError` usa `axum` (`StatusCode`, `Response<Body>`), `serde_json` e `axum-tracing-opentelemetry`, todos deps de `http_server`; tornar `commonx` livre de feature exigiria mudar os tipos (fora de escopo). Logo, "disponível em toda combinação que hoje usa os tipos" é atendido e nada regride. Registrar no CHANGELOG/README que `commonx` vem com `http_server` (default).
3. **Dependência circular:** não é problema (mesma crate). `httpx::HttpResponse` permanece em `httpx/response/mod.rs`, passa a retornar `crate::commonx::DerustTags`. `commonx::error` faz `use crate::httpx::HttpResponse;`. `httpx` não reexporta os novos nomes; remover `mod tags;` e `pub use tags::*;` de `httpx/mod.rs`, e `pub mod error;` de `httpx/response/mod.rs` (o `pub use error::*` em `httpx/mod.rs` sai junto).
4. **Renomear só os dois tipos:** substituição por palavra inteira (`\bHttpTags\b`, `\bHttpError\b`). Não tocar `HttpResponse`, `HttpClient*`, `JsonResponse`, `TextResponse`, nem tipos de terceiros. Nomes de variáveis locais como `http_error` podem virar `derust_error` (opcional, só em `From<DerustError> for Response<Body>`).
5. **Imports:** onde havia `use crate::httpx::{AppContext, HttpError}` separar em `use crate::commonx::DerustError; use crate::httpx::AppContext;`. Para consumidores/exemplos: `use derust::commonx::{DerustError, DerustTags}; use derust::httpx::{start, AppContext};`.
6. **Colisões:** já verificado que `DerustTags`, `DerustError` e `commonx` não existem hoje no repositório.
7. **Arquivos com ocorrências** (contagem de linhas, base main): databasex (`repository.rs` 13, `postgresx/database.rs` 14, `pg_transaction_repository.rs` 19, `pg_transaction_repository_and_metrics.rs` 21, `pg_connection_repository.rs` 21, `pg_option_transaction_repository.rs` 13, `pg_option_transaction_repository_and_metrics.rs` 13), `http_clientx/client.rs` 19, outboxx (`mod.rs` 4, `runner.rs` 4, `http.rs` 5, `sqs.rs` 5, `sns.rs` 5), `awsx/sqsx/{runner,consumer}.rs` 3 cada, `tracex/log.rs` 6, `metricx/meters/tags.rs` 5, httpx (`health.rs`, `axum.rs`, `prometheus.rs`, `response/{json,text,error,mod}.rs`, `middlewares/log.rs`, `request/json_request.rs`, `tags.rs`). Exemplos: 10 `main.rs`.
8. **Conflitos de merge:** as branches `adicionar_push_otlp_metricas_e_logs` e `ponte_metrics_para_otel_meter` (worktrees abertas) tocam `tracex`/`metricx`; podem conflitar com a renomeação ao serem integradas. Não resolver aqui; apenas ciente.

## Dependências entre tarefas

- T1 (núcleo da crate) é atômica: a renomeação só compila quando todos os módulos mudam juntos; não paralelizar por módulo.
- T2 (exemplos) depende de T1.
- T3 (docs) e T4 (versão + CHANGELOG) são independentes de T1 e entre si: podem rodar em paralelo com T1.
- T5 (verificação final) depende de T1-T4.

```
T1 ─┬─> T2 ─┐
T3 ─┤       ├─> T5
T4 ─┴───────┘
```

---

## T1 — Núcleo da crate: criar `commonx` e renomear em todo `crates/derust/src` (dificuldade: média)

**Contexto:** hoje `HttpTags` está em `crates/derust/src/httpx/tags.rs` (newtype `HttpTags(HashMap<String,String>)`, com `error`, `error_message`, `add`, `values` e 3 impls `From<[(..);N]>`) e `HttpError` em `crates/derust/src/httpx/response/error.rs` (campos `status_code`, `error_message`, `response_body`, `response_headers`, `tags`; construtores `with_body`, `with_json`, `without_body`, `with_headers`, `response_json`; `impl HttpResponse`, `impl Display/Error`, `impl From<HttpError> for Response<Body>`). Não alterar nenhum corpo de função, campos, mensagens, nomes de tags de log/métrica.

**Arquivos:** criar `commonx/{mod.rs,tags.rs,error.rs}`; apagar `httpx/tags.rs` e `httpx/response/error.rs`; editar `lib.rs`, `httpx/mod.rs`, `httpx/response/mod.rs` e todos os arquivos listados na decisão 7.

- [x] Passo 1: `git mv` `httpx/tags.rs` -> `commonx/tags.rs` e `httpx/response/error.rs` -> `commonx/error.rs` (preserva histórico). Renomear os tipos dentro deles. Em `error.rs`: `use crate::commonx::tags::DerustTags;` (ou `super::tags`), `use crate::httpx::HttpResponse;`.
- [x] Passo 2: criar `commonx/mod.rs` (decisão 1). Em `lib.rs` adicionar, junto a `httpx`:
  ```rust
  #[cfg(feature = "http_server")]
  pub mod commonx;
  ```
- [x] Passo 3: editar `httpx/mod.rs` (remover `mod tags;`, `pub use tags::*;`, `pub use error::*;`) e `httpx/response/mod.rs` (remover `pub mod error;`; `use crate::commonx::DerustTags;` e `fn tags(&self) -> DerustTags;`).
- [x] Passo 4: substituição por palavra inteira em todo `crates/derust/src/**/*.rs` (exceto READMEs, tratados em T3) e ajustar os `use` conforme decisão 5. Atenção aos `use` do tipo `use crate::httpx::{..., HttpError, HttpTags}` e `use crate::httpx::tags::HttpTags` / `crate::httpx::response::error::...` se existirem.
- [x] Passo 5: `cargo fmt --all` (ordem de imports pode mudar).
- [x] Passo 6: busca residual: `grep -rnE '\bHttp(Tags|Error)\b' crates/derust/src --include='*.rs'` deve retornar vazio. Conferir com `git diff --stat` que `HttpResponse`/`HttpClient*` não foram alterados (`git diff | grep -E '^[-+].*HttpClient'` vazio).

**Testes/verificação (T1):**
- `cargo nextest run` (default) verde, mesma contagem de testes de antes (rodar em main antes para baseline: `cargo nextest run 2>&1 | tail -3`).
- Compilar combinações: `cargo check -p derust` e, para cada `F` em `postgres`, `outbox`, `sqs`, `aws`, `http_client`, `statsd`, `prometheus`, `growthbook`, `start_test`: `cargo check -p derust --features F`; e `cargo check -p derust --all-features`. Também `cargo nextest run -p derust --all-features` se compilar em main (confirmar baseline).
- `cargo clippy -- -D warnings` e `cargo fmt --all -- --check` passam.
- Teste novo (pequeno, em `commonx/error.rs` ou `tags.rs`, `#[cfg(test)]`): `DerustError::without_body(StatusCode::BAD_REQUEST, "x".into(), DerustTags::from([("a","b")]))` -> `HttpResponse::status_code()==400`, `tags().values()` contém `a` e `error_message`; e `DerustTags::error_message("m").values()["error"]=="m"`. Garante que o novo caminho funciona (testes existentes que usavam os tipos já cobrem o resto).

**Revisão (T1):** revisar o diff completo: apenas renomeações e imports; nenhuma mudança de lógica; nenhum `HttpResponse`/`HttpClient*` tocado; `httpx` não reexporta os novos nomes.

**Commit:** `refactor: rename HttpTags/HttpError to DerustTags/DerustError and move to commonx`.

---

## T2 — Exemplos (dificuldade: simples; depende de T1)

**Contexto:** `examples/` não faz parte do workspace (`exclude`), cada exemplo é uma crate com `path = "../../crates/derust"`; compilar à parte. Arquivos: `examples/{basic,outbox,http_client,trace,growthbook}/src/main.rs`, `examples/env/{basic,secrets-manager}/src/main.rs`, `examples/database/postgres/src/main.rs`, `examples/metrics/{prometheus,statsd}/src/main.rs`.

- [x] Passo 1: em cada `main.rs`, substituir `HttpError`/`HttpTags` por `DerustError`/`DerustTags` (palavra inteira) e mover do `use derust::httpx::{...}` para `use derust::commonx::{DerustError, DerustTags};` (manter o resto em `httpx`).
- [x] Passo 2: `grep -rnE '\bHttp(Tags|Error)\b' examples --include='*.rs'` vazio.

**Testes:** para cada diretório de exemplo acima: `cargo check --manifest-path examples/<dir>/Cargo.toml` deve passar (ou `cargo build`). `cargo fmt --manifest-path ... -- --check`. Se algum exemplo já falha em main por motivo alheio (ex.: dependência/versão), registrar como pré-existente (conferir com `git stash`/main) e não corrigir.

**Revisão:** diff só com renomeação/imports.

**Commit:** `docs(examples): use DerustTags and DerustError`.

---

## T3 — Documentação (dificuldade: simples; independente)

**Arquivos:** `README.md` e `crates/derust/README.md` (ocorrências nas linhas ~39, 76, 80, 248, 262 — `use derust::httpx::{start, AppContext, HttpError, HttpTags};` vira `use derust::commonx::{DerustError, DerustTags}; use derust::httpx::{start, AppContext};`), `CLAUDE.md` (linhas ~67 e ~69; descrever também o módulo `commonx` na seção Architecture/tabela: `commonx` vem com `http_server`, contém `DerustTags` e `DerustError`), e READMEs de módulo: `crates/derust/src/http_clientx/README.md`, `growthbookx/README.md` (6), `tracex/README.md`, `outboxx/README.md`, `databasex/postgresx/README.md`, `metricx/registries/prometheus/README.md`, `metricx/registries/statsd/README.md`.

- [x] Passo 1: substituir nomes (palavra inteira) e ajustar os `use` conforme decisão 5, em todos os arquivos acima. Se `README.md` raiz for cópia do da crate, manter idênticos.
- [x] Passo 2: `grep -rnE '\bHttp(Tags|Error)\b' README.md CLAUDE.md crates --include='*.md'` vazio.

**Testes:** busca residual acima; conferir manualmente que snippets de README ainda são coerentes (imports de `AppContext`, `start` continuam de `httpx`). Não editar `docs/superpowers/plans/*` históricos nem o CHANGELOG nesta tarefa.

**Revisão:** ler o diff dos READMEs verificando que nenhum snippet ficou com import quebrado.

**Commit:** `docs: update README, CLAUDE.md and module READMEs for DerustTags/DerustError`.

---

## T4 — Versão 0.6.0 e CHANGELOG (dificuldade: simples; independente)

- [x] Passo 1: `crates/derust/Cargo.toml` `version = "0.5.2"` -> `"0.6.0"`; atualizar `Cargo.lock` (entrada `name = "derust"` na raiz) via `cargo check -p derust` (ou `cargo update -p derust --offline`); conferir `git diff Cargo.lock` só muda a versão do derust. Verificar se algum exemplo/README referencia `0.5.2` (`grep -rn '0\.5\.2' --include='*.toml' --include='*.md' .` fora de `docs/superpowers` e CHANGELOG) e atualizar para `0.6.0` se for a versão de dependência recomendada.
- [x] Passo 2: em `CHANGELOG.md`, acima de `## [0.5.2]`, adicionar `## [0.6.0]` com:
  - `### Changed (BREAKING)`: `httpx::HttpTags` -> `commonx::DerustTags`; `httpx::HttpError` -> `commonx::DerustError`; os nomes antigos foram removidos (sem aliases `#[deprecated]`); `httpx` não reexporta os novos nomes; `commonx` está disponível com a feature `http_server` (default); `HttpResponse`, `HttpClient*`, `JsonResponse`, `TextResponse` não mudam; sem mudança de comportamento (logs, métricas, respostas HTTP).
  - `### Migration guide`: tabela de/para (nome e caminho) e exemplo `use derust::httpx::{HttpError, HttpTags};` -> `use derust::commonx::{DerustError, DerustTags};`; avisar que assinaturas de handlers SQS (`Result<(), DerustError>`), repositórios (`databasex`), `outboxx` e `http_clientx`, além de quem implementa `HttpResponse::tags() -> DerustTags`, mudam de tipo nominal.

**Testes:** `cargo check -p derust` ok com o lock atualizado; CHANGELOG lido para conferir de/para correto (as ocorrências de `HttpTags`/`HttpError` no CHANGELOG são o histórico permitido).

**Revisão:** conferir que o texto cobre nome + caminho + ausência de aliases.

**Commit:** `chore: bump derust to 0.6.0 and document breaking rename in CHANGELOG`.

---

## T5 — Verificação final (dificuldade: simples; depende de T1-T4)

- [x] `grep -rnE '\bHttp(Tags|Error)\b' . --include='*.rs' --include='*.md' --include='*.toml' | grep -v 'docs/superpowers/plans\|CHANGELOG.md\|^./target\|/target/'` vazio.
- [x] `task lint` e `task test` (equivale a `cargo fmt --check`, `cargo clippy -- -D warnings`, `cargo nextest run`) verdes; matriz de features de T1 e todos os exemplos de T2 compilam.
- [x] `git diff main --stat` revisado: nenhum arquivo inesperado; `HttpResponse`/`HttpClient*` inalterados.
- [x] Marcar as tarefas deste plano como concluídas e commitar.

**Critérios de aceite:** os do plano de negócio (tipos novos em `commonx`, nomes antigos inexistentes, comportamento idêntico, gates verdes, exemplos compilam, versão 0.6.0 + CHANGELOG).
