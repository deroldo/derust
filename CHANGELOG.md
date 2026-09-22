# Changelog

All notable changes to `derust` are documented in this file.

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
