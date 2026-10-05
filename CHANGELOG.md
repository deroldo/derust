# Changelog

All notable changes to `derust` are documented in this file.

## [0.6.0]

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
