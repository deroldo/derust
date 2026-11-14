# Changelog

All notable changes to `derust` are documented in this file.

## [0.7.0]

### Added

- Feature `sdui_stac_1` (module `derust::sduix::stacx::v1`): typed model of the Stac `1.6.0` JSON
  vocabulary (widgets, actions, auxiliary types and `StacTheme`) with builders, a DSL (`v1::dsl`,
  `v1::prelude`) and tree validation (`Validate`). One feature per Stac major; not part of `default`.
- `httpx::sdui::SduiResponse` (needs `http_server`): validates and serializes a screen once, failing with
  a `DerustError` (500) instead of a broken response.
- `v1/inventory.json` (coverage of every Stac component), JSON fixtures with Rust tests and a Flutter
  harness (`tools/stac-compat/v1`) that parses the fixtures with the real Stac; GitHub Actions CI.
- Extension points for applications: reusable compositions via `From<YourType> for Widget`, and custom
  widgets/actions of parsers registered in the Flutter app via `CustomWidget` / `CustomAction` (the
  `Custom` variant of `Widget` / `Action`).
- Example `examples/sdui`.
- Reactive state for SDUI (vocabulary of the `derust-stac` fork, branch `estado_compartilhado_reativo`,
  contract read at commit `92457f25`, no pinned version; the official Stac does not understand it):
  `Reactive<T>` on every leaf property (value, `$state` binding or `$expr` expression), the typed
  expression language `Expr` and `StatePath`, `Data`, the widget envelope (`visible`, `enabled`, `bind`,
  `state`, `globalState`, `onInit`), `condition` on every action, the state actions (`setState`,
  `incrementState`, `decrementState`, `toggleState`, `resetState`, `appendState`, `removeFromState`,
  `toggleItem`, `cancelRequest`, `none`), `networkRequest` callbacks/`requestKey`/`bodyFromState`,
  `navigate` `params`/`result`/`onResult`/`onCancel`, list templates (`ItemTemplate`), and
  `Slider::bound`.
- Contextual validation of the serialized screen in `SduiResponse::try_new` (errors answer 500 with the
  JSON path; warnings are logged), `Validate::validate_serialized` and `ValidationReport`.
- `v1/inventory_state.json`, reactive fixtures, harness `tools/stac-compat/v1-reactive`
  (CI job `stac-compat-v1-reactive`, `continue-on-error` until the fork is pinned) and example
  `examples/sdui_reactive`.

- Feature `cryptox` (module `derust::cryptox`, not in `default`, independent of `http_server`): EC P-256 keys
  (PKCS#8/SPKI PEM) with `kid` and JWKS; ES256 JWT (`JwtSigner`, `JwtVerifier`, `TokenIssuer`) with
  `token_type` access/refresh and mandatory `iss`/`aud` (no default TTL); compact JWE `ECDH-ES` + `A256GCM`
  (`JweSealer`, `JweOpener`, `Sealer`); Argon2id `PasswordHasher`; HMAC-SHA256 `CodeHasher` with pepper and
  `generate_numeric_code`. Unrelated to the existing `httpx::protect_endpoints_core` extractor.
- `cryptox` configuration (`KeySource` from env, file or AWS Secrets Manager, `JwtVerifierConfig`,
  `TokenIssuerConfig`) and Axum integration `cryptox::http` (`AuthConfig`, `Authenticated<C>`,
  `RequireClaims<C>`; 401 with `WWW-Authenticate: Bearer`, 403, 500), available with `http_server`.

### Breaking (`sdui_stac_1`, unreleased feature)

- Public fields of the generated structs change from `Option<T>` to `Option<Reactive<T>>`; setters take
  `impl IntoReactive<T>` (plain values keep working).
- The native `visible` / `enabled` fields of `Visibility`, `SliverVisibility`, `BackdropFilter`,
  `DropdownMenu`, `ListTile`, `Radio`, `TextField` and `TextFormField` are no longer fields: they live in
  the envelope (same JSON key, same setter name).
- `NavigateAction.result` and the former `Map` / `Value` fields (`body`, `headers`, `queryParameters`,
  `arguments`, ...) are now `Data`.
- `Dyn<T>` gained the `Bind` variant.
- `Slider.value` is optional (`value` or `bind`); `Slider::new(value)` is unchanged.
- The `bind` setter of every widget takes `impl Into<BindKey>`.
- `onResult` and `onCancel` of `navigate` may be set together.

### Changed

- `derust` now compiles with `--no-default-features` (the `shutdown_signal`/`StatusCode` exports are
  gated by `http_server`).

## [0.6.0]

### Breaking

- The `SqsQueueConsumer` handler now receives `DerustTags` (by value) as a third argument:
  `Fn(AppContext<S>, Message, DerustTags)`. Migration: change `|ctx, msg|` to `|ctx, msg, _tags|`.

### Added

- Consumed SQS messages now get `DerustTags` containing `x-trace-id`, read from the `x-trace-id`
  message attribute; a new trace id is generated when it is missing or invalid.
- `Message::message_attribute()` / `MessageBuilder::message_attribute()`.
- The SQS consumer's `receive_message` now requests the `x-trace-id` message attribute.
- Consumer logs (consumed, failed, retry) include `trace_id`. It is not added as a metric tag.

### Changed

- Outbox SQS, SNS and HTTP sends add the current trace as the `x-trace-id` header when there is
  one. A caller-provided `x-trace-id` (any case) is never overwritten.
- For SQS/SNS the trace is delivered as a MessageAttribute; with the 10-attribute limit
  (`x-idempotent-key` counts as one) the trace is silently omitted when there is no free slot.
- For HTTP the header is `x-trace-id`; strict destinations that reject unknown headers may be affected.

### Changed (BREAKING)

- `httpx::HttpTags` is now `commonx::DerustTags`.
- `httpx::HttpError` is now `commonx::DerustError`.
- The old names were removed outright (no `#[deprecated]` aliases) and `httpx` no longer re-exports
  the new names.
- The new `commonx` module is available with the `http_server` feature (default).
- `HttpResponse`, `HttpClient*`, `JsonResponse` and `TextResponse` are unchanged.
- No behavior change: logs, metrics and HTTP responses are identical.

### Migration guide

| Before | After |
|---|---|
| `derust::httpx::HttpTags` | `derust::commonx::DerustTags` |
| `derust::httpx::HttpError` | `derust::commonx::DerustError` |

```rust
// before
use derust::httpx::{HttpError, HttpTags};
// after
use derust::commonx::{DerustError, DerustTags};
```

Signatures that mention these types change nominally: SQS handlers
(`Result<(), DerustError>`), `databasex` repositories, `outboxx`, `http_clientx`, and any
implementation of `HttpResponse::tags() -> DerustTags`.

## [0.5.2]

### Added

- `sqsx::RetryPolicy` and `SqsQueueConsumer::with_retry_policy` (opt-in): failed SQS messages are
  rescheduled via `ChangeMessageVisibility` with exponential backoff and full jitter
  (defaults: base 5s, max 15min, factor 2.0; capped at the 12h SQS limit).
- `Message::receive_count()` / `MessageBuilder::receive_count()`.
- The SQS consumer's `receive_message` now requests `ApproximateReceiveCount`.
- New optional dependency `rand` under the `sqs` feature.

## [0.5.1]

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
- Metrics instrumented via `metricx` (`increment`, `increment_one`, `current_gauge`,
  `record_money`, `record_duration`, `start_stopwatch`, including the automatic
  HTTP/DB duration metrics) now also reach the OTLP push pipeline described above,
  with no instrumentation changes required — the existing Prometheus `/metrics` pull
  endpoint and StatsD push remain unaffected. Histogram bucket boundaries are
  identical on both channels.
