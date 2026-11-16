# Resposta de erro Stac e instrução de recuperação (derust) — Refinamento

**Objetivo:** o derust passa a montar a resposta HTTP de erro do Stac (`StacErrorResponse`: status, `Content-Type: application/vnd.stac.error+json`, envelope `{version, code?, message?, screen?, action?, recovery?}`), com a DSL tipada da instrução de recuperação (`Recovery`, `Retry`) e a action `recoveryOutcome`. O derust **não** conhece token, refresh, Authorization nem sessão: o handler/extrator do serviço decide se a falha é recuperável e monta a instrução. Contrato do envelope e semântica no runtime: plano principal `Projects/derust-stac/plans/sessao_expirada_renovacao_e_navegacao.md` (seções 4, 6 e apêndice A). Feature `sdui_stac_1` (a resposta HTTP exige também `http_server`). Branch `resposta_de_erro_stac_e_recuperacao`. Este refinamento não decide regra de negócio.

## Contexto do código (lido em 98e217a)

- `crates/derust/src/httpx/response/sdui.rs`: `SduiResponse::try_new(status, screen, tags) -> Result<Self, DerustError>` valida (`Validate::validate` + `validate_serialized`), serializa uma vez, erro vira `DerustError` 500 sem corpo parcial; implementa `HttpResponse` (status, body, headers com `Content-Type` default, tags + `x-trace-id`). `IntoResponse` em `crates/derust/src/httpx/axum.rs` (bloco `#[cfg(feature = "sdui_stac_1")]`). Seguir esse molde.
- `crates/derust/src/commonx/error.rs`: `DerustError` (`with_body`, `with_json`, `without_body`, `with_headers`); implementa `HttpResponse`.
- `crates/derust/src/sduix/stacx/v1/state/` (escrito à mão): `Reactive<T>` (literal ou `{"$state"}`/`{"$expr"}`), `Data` (JSON livre com bindings), `Binding`, `StatePath`, `Expr`, `Actions` (uma ação ou lista, serializa objeto/array), ações de estado em `actions.rs` com a macro `action_common!` (campo `condition: Option<Reactive<bool>>`, `TYPE`, `StacNode`, `Plain`), `route_name.rs` (`route_name_problem`: com `request`/`widgetJson`/`assetPath` exige `/screens/...`, query permitida).
- `enum Action` em `crates/derust/src/sduix/stacx/v1/actions/action.rs` é **gerado** por `tools/stac-inventory/gen_rust.py` (ações do fork listadas ~linha 116, ex.: `("CancelRequest", "cancelRequest", "crate::...::state::CancelRequest")`). Nunca editar o gerado à mão; regenerar (ver `tools/stac-inventory/README.md`, o gerador é idempotente).
- Inventário do fork: `v1/inventory_state.json` (fonte `fork`). Testes: `crates/derust/tests/sdui_stac_v1/` (`main.rs`, `reactive_actions.rs`, `navigate_route_name.rs`, fixtures em `fixtures_reactive/{actions,bindings,envelope,expressions,lists,screens}`), harness Dart `tools/stac-compat/v1-reactive/test/compat_reactive_test.dart` (percorre todos os fixtures). Precedente de refinamento/commits: `docs/superpowers/plans/2026-11-14-navigate-route-name.md`.
- Não existe `public_route` no derust (provavelmente é helper dos BFFs). Ver decisão D3.

## Decisões de mecânica (o "como")

- **M1.** `Recovery`/`Retry` aceitam valores como `Data` (já existe: literal JSON, `$state`, `$expr`); `Retry.header(nome, impl Into<Data>)` etc. `recover`/`on_failure`/`on_unavailable` são `Actions`.
- **M2.** `recoveryOutcome` serializa como `{"actionType":"recoveryOutcome","result":"success|failure|unavailable","condition"?}`, igual ao apêndice A.2. Tipo `RecoveryOutcome` + enum `RecoveryResult`, no mesmo molde de `CancelRequest` (`state/actions.rs`), registrado no gerador e no inventário (`source: fork`).
- **M3.** Builders infalíveis e encadeáveis; toda validação em `Validate` (tipada) e uma única verificação final em `StacErrorResponse::try_new(...) -> Result<Self, DerustError>` (500 sem corpo parcial, como `SduiResponse::try_new`). Nada de `panic`/`unwrap`. Erro de validação é erro de programação, pego no teste do handler.
- **M4.** `Retry` não tem campo de URL nem método (o tipo impede). `body(null)` remove a chave: `Retry::remove_body_key(k)` grava `Data` null.
- **M5.** `version` é constante `1` (não é campo público); `timeout_ms` entre 1 e 600000; `key` não vazio, ≤128 chars, sem controle; `no_coalesce()` serializa `"key": null`; sem chamar nada, a chave é omitida (o runtime usa o hash do `recover`).
- **M6.** Content-Type fixo `application/vnd.stac.error+json`; `Cache-Control: no-store` por padrão quando há `recovery` (sobrescrevível via `with_headers`); tags + `x-trace-id` iguais ao `SduiResponse`.
- **M7.** `screen` entra como `serde_json::Value` já validado: o builder recebe `impl Serialize + Validate` (widget) e valida na hora (`validate` + `validate_serialized`), porque o front o renderiza como tela própria. `action`, `recover`, `on_failure` e `on_unavailable` são validados **só pela parte tipada** (`Validate::validate`), sem `validate_serialized`/`check_screen`: o envelope não declara `state`/`globalState`, e as checagens contextuais (chave não declarada, `response` fora de callback) dariam falso erro/aviso em `global.*` que vive na tela do app. Isso permite reusar a validação existente sem acoplar o envelope a um tipo de widget.
- **M8.** Conversão `StacErrorResponse::from_derust_error(error, screen?, action?)`: preserva status, `error_message` (vira `message`) e tags; o corpo antigo do `DerustError` não é propagado.

## Tarefas

Dependências: D1, D2 e D5 em paralelo; D3 depende de D1 e D2; D4 depende de D3; D6 depende de D1 a D5.

### D1 — Action `recoveryOutcome` (simples)
Arquivos: `crates/derust/src/sduix/stacx/v1/state/actions.rs` (+ `mod.rs`), `tools/stac-inventory/gen_rust.py` (lista do fork), `inventory_state.json`, `action.rs` (regerado).
Contexto: tipos `RecoveryOutcome { condition, result }`, `RecoveryResult::{Success, Failure, Unavailable}` (serializa minúsculo). Funções livres no estilo `cancel_request()`: `recovery_outcome(RecoveryResult)`.
Testes: serialização com e sem `condition`, `condition` por `Expr`; o enum `Action` serializa/valida a variante; geração idempotente (rodar o gerador duas vezes sem diff); `inventory_state.json` aceito pelo teste `inventory_state.rs`. Fixture `fixtures_reactive/actions/recovery_outcome*.json`.
Revisão: sem edição manual de arquivo gerado; `cargo nextest run --features sdui_stac_1`; clippy.

### D2 — DSL `Recovery`, `Retry` (médio)
Arquivo novo `crates/derust/src/sduix/stacx/v1/state/recovery.rs` (re-export em `state/mod.rs` e `prelude`).
`Recovery::new().key(..)|.no_coalesce().recover(impl Into<Actions>).retry(Retry).on_failure(..).on_unavailable(..).timeout_ms(n)`; `Retry::new().header(n, v).query(n, v).body(n, v).remove_header(n)` serializam `{"retry":{"overrides":{"headers","queryParameters","body"},"removeHeaders"}}` (campos vazios omitidos). Validação (`Validate`): `recover` presente e não vazio (lista vazia recusada); `key`/`timeout_ms` (M5); nome de header válido (token HTTP); bindings validados via `Data`/`Binding`; actions aninhadas validadas recursivamente. Serialização falha → erro propagado, nunca `unwrap_or_default`.
Testes: snapshot de cada campo; casos inválidos recusados (recover vazio, key vazia, timeout 0, header com espaço); `no_coalesce` ⇒ `"key":null`; nenhum campo `url`/`method` possível (teste de compilação não necessário: ausência no JSON).

### D3 — `StacErrorResponse` (médio)
Arquivo novo `crates/derust/src/httpx/response/stac_error.rs` (`pub mod` em `response/mod.rs` sob `sdui_stac_1`; `IntoResponse` em `httpx/axum.rs`; exportar em `httpx::sdui`).
`StacErrorResponse::new(status)` + `.code(..)`, `.message(..)`, `.screen(widget)`, `.action(action)`, `.recovery(Recovery)`, `.tags(..)`, `.with_headers(..)`; `try_new`/`build()` fazem a validação única e a serialização (M3, M7); só aceita 4xx/5xx (o runtime só reconhece resposta não-2xx e o Dio só entrega ao interceptor de erro o que não é 2xx; 1xx/2xx/3xx não carregam envelope). Implementa `HttpResponse` (M6) e `IntoResponse` com extensão de tags (igual `SduiResponse`).
Testes: envelope mínimo/só `screen`/só `action`/completo com `recovery`; Content-Type e Cache-Control; tags no response extension; 1xx/2xx/3xx recusados; falha de validação ⇒ `DerustError` 500 sem corpo parcial; usado como `IntoResponse` num extrator real com `start_test` (teste de integração em `crates/derust/tests/`).

### D4 — Conversão de `DerustError` (simples)
Em `stac_error.rs`: M8. Testes: status e tags preservados, `message` copiada, `screen`/`action` opcionais aplicados.

### D5 — `public_route_with_query` (simples)
Em `state/route_name.rs` (pública). `public_route_with_query(path, &[(chave, valor)]) -> String`: percent-encoding de chave/valor (sem dependência nova), path validado pela regra existente (`/screens/...` sem esquema/host). Nome e escopo da função estão na pendência P1 (seção Reconciliação com o front). Testes: encoding de espaço, `&`, `=`, `?`, unicode; resultado passa em `route_name_problem(.., true)`; path inválido recusado.

### D6 — Docs, exemplo, fixtures e harness (médio)
README `sduix/stacx/README.md` (seção erro/recuperação; vocabulário só existe no runtime do fork `derust-stac`), CHANGELOG (Added, sob 0.7.0), matriz de compatibilidade; exemplo executável `examples/sdui_reactive` (ou novo `examples/stac_error`) com o envelope do apêndice A.2 só como exemplo de uso; fixtures `fixtures_reactive/envelope/recovery_*.json` (mínimo, só screen, só action, completo) usados tanto pelos testes Rust quanto pelo harness Dart (`flutter test` em `tools/stac-compat/v1-reactive`: se o runtime do fork ainda não tem o parser de `recoveryOutcome`, registrar como pendente, ver abaixo).

## Plano de testes e revisão (global)
`task lint`; `task test` (default e `sdui_stac_1`); gates `--no-default-features --features sdui_stac_1` (sem `http_server`, módulo compila sem a resposta) e `--features sdui_stac_1,http_server`. Revisão por tarefa: serialização por snapshot, validações negativas, sem `unwrap`.

## Dificuldade
D1 simples; D2 médio; D3 médio; D4 simples; D5 simples; D6 médio. Nenhuma complexa.

## Reconciliação com o front (derust-stac, commit 44abb606)

Fonte: `/Users/deroldo/workspace/derust-stac-recuperacao/docs/superpowers/plans/2026-11-16-recuperacao-e-repeticao-de-request.md`. Conferido contrato por contrato.

| Contrato | Front espera | Derust | Situação |
|---|---|---|---|
| Marcador | `Content-Type: application/vnd.stac.error+json`, ignora parâmetros (`; charset`); só não-2xx; `version` int == 1; corpo objeto | Content-Type fixo (M6), `version` 1, só 4xx/5xx | Alinhado. O derust emite o tipo sem parâmetro. O front aceita corpo já decodificado ou String JSON. |
| Envelope | `{version, code?, message?, screen?, action?, recovery?}`; `screen`/`action` JSON cru | Idem | Alinhado. |
| `recovery` | `{key? (ausente != null), recover (ação ou lista), retry?:{overrides?:{headers?,queryParameters?,body?}, removeHeaders?}, onFailure?, onUnavailable?, timeoutMs?}` | D2 serializa exatamente isso; campos vazios omitidos; `no_coalesce()` ⇒ `"key": null` | Alinhado. `Data` serve: o front resolve bindings dos overrides depois da recuperação; `null` remove a chave. |
| `recoveryOutcome` | `{actionType, result: success\|failure\|unavailable, condition?}`; tipo em `stac_core` (`ActionType.recoveryOutcome`); fora de uma recuperação é no-op com log | D1 idêntico (M2) | Alinhado. `actionType` no inventário do derust: `recoveryOutcome`. |
| Status do erro | Qualquer não-2xx | Só 4xx/5xx | Derust mais restritivo de propósito. |
| Variáveis `request`/`failure` em `recover` | Plano 4.2 do front: "variáveis somente-leitura" | Só existe `Root::Response` (válido em `onSuccess/onError/onFinally`); não há `Root::Request`/`Failure` | **Divergência**, ver P2. |
| `routeName` | `StacRouter` reconstrói por `GET <base><backendPrefix>/screens/x?q=1`; `screensPrefix` configurável | `route_name_problem` exige `/screens/` fixo; query permitida | Ver P1/P3. |
| Fixtures | Front: `packages/stac/test/recovery/fixtures/` (T10), "as mesmas que o derust deve produzir" | Derust: `crates/derust/tests/sdui_stac_v1/fixtures_reactive/error_envelope/` | Ver D6 e P4. |
| Runtime parseia `recoveryOutcome`? | **Ainda não.** O front só foi refinado (T4 cria o parser, T1 o reconhecedor); nada implementado | O harness Dart do derust (`tools/stac-compat/v1-reactive/pubspec.yaml`) usa o fork por branch, sem pin | Parte Dart da D6 bloqueada até o front mesclar T1/T3/T4/T10 na ref que o harness usa. |

### Ajustes feitos nas tarefas
- **D1:** fixtures do `recoveryOutcome` dentro de callback `onSuccess`/`onError` de `networkRequest` (posição do apêndice A.2); validação tipada apenas.
- **D2:** validação tipada apenas das ações aninhadas (M7); `Retry`/`overrides` aceitam `Data`; sem `request`/`failure` (P2).
- **D3:** só 4xx/5xx; `screen` com validação completa, demais ações só tipada; `Cache-Control: no-store` com `recovery`.
- **D5:** ver P1 e P3. Percent-encoding no estilo RFC 3986 (espaço vira `%20`, nunca `+`), compatível com o `Uri.parse` do Flutter.
- **D6:** fixtures em `fixtures_reactive/error_envelope/` (não em `envelope/`, que já guarda as propriedades comuns de widget): mínimo, só `screen`, só `action`, completo com `recovery`, versão desconhecida e o ciclo 401 do apêndice A.2. Os testes Rust comparam a serialização com os arquivos. O teste Dart novo (`recovery_envelope_test.dart`, reconhecedor + modelos do front, sem alterar `compat_reactive_test.dart`) fica **pendente** até o front publicar T1/T4 na ref do harness; sem isso a D6 entrega só a parte Rust e deixa o Dart documentado. Nomes e conteúdo dos arquivos idênticos aos do front; sincronização manual (P4).
- **Documentação (D6):** descrever o comportamento do runtime que afeta o autor Rust: uma só repetição; requests do `recover`, da repetição e de `onFailure`/`onUnavailable` são isentos de recuperação (G7-B); respostas de erro da própria recuperação não devem levar `recovery`; erro final = último erro observado; com `onFailure` rodando o chamador não renderiza o erro final (G6b); a `action` do envelope roda depois do `onError` (G13).

### Pendências e recomendações (decisão do usuário)
- **P1. Nome e escopo de `public_route_with_query`.** Não há `public_route` no derust, só nos BFFs. Recomendação: manter o nome e a função em `state/route_name.rs`, sem replicar o `public_route` dos BFFs.
- **P2. `request`/`failure` em `recover`.** O plano do front os cita como variáveis somente-leitura, mas nenhuma tarefa do front as define na gramática de binding, e o derust não as tem. Recomendação: não criar `Root::Request`/`Root::Failure` agora (o exemplo A.2 só usa `response` nos callbacks); o front confirma se existirão, e então se abre uma D7 no derust.
- **P3. Prefixo `/screens/`.** O front torna `screensPrefix` configurável; o derust fixa `/screens/` quando a ação carrega tela. Recomendação: manter fixo; revisitar só se um app usar outro prefixo.
- **P4. Fixtures duplicadas nos dois repos.** Recomendação: o derust é a fonte de verdade da serialização; mesmo nome e conteúdo nos dois repos, com nota no README de cada um. Sem automação entre repositórios por ora.
- **P5. Bloqueio da parte Dart da D6.** Recomendação: iniciar o derust sem esperar o front; só o teste Dart espera.
