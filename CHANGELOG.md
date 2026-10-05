# Changelog

All notable changes to `derust` are documented in this file.

## [0.6.0]

### Breaking

- The `SqsQueueConsumer` handler now receives `HttpTags` (by value) as a third argument:
  `Fn(AppContext<S>, Message, HttpTags)`. Migration: change `|ctx, msg|` to `|ctx, msg, _tags|`.

### Added

- Consumed SQS messages now get `HttpTags` containing `x-trace-id`, read from the `x-trace-id`
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
