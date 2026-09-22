# derust - trace

[Example](https://github.com/deroldo/derust/tree/main/examples/trace)

```toml
# Cargo.toml

[package]
# ...

[dependencies]
derust = { version = "<last-version>" }

# ...
```

```rust
// main.rs

// ...
use derust::tracex;
use derust::tracex::log::info;
// ...

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // automatic log subscribe and add b3 traceparent
    let _guard = tracex::init();
    
    // start as the basic 
    // ... 
}

async fn handler(
    State(context): State<AppContext<AppState>>,
    // automatic add tags into log
) -> Result<JsonResponse, HttpError> {
    // ...
    
    // tags to be added into log
    let tags = HttpTags::from([("customer_id", customer_id)]);
    
    // automatic add tags into log
    info("Request handler", &tags);
    
    // ...
}
```

## OTLP push for metrics and logs

Besides traces (already pushed via OTLP when `OTEL_EXPORTER_OTLP_ENDPOINT` /
`OTEL_EXPORTER_OTLP_PROTOCOL` are set), `tracex::init()` also builds OTLP push
pipelines for **metrics** and **logs**, using the exact same env vars — no extra
configuration needed:

- `OTEL_EXPORTER_OTLP_ENDPOINT`
- `OTEL_EXPORTER_OTLP_PROTOCOL` (`http/protobuf` or `grpc`)
- `OTEL_EXPORTER_OTLP_HEADERS` (read automatically by the exporter builders)

If these env vars are not set, `tracex::init()` behaves exactly as before: no OTLP
metrics/logs exporter is created, only a `tracing::warn!` is logged, and the
application boots normally.

### Coexists with the `metricx` Prometheus/StatsD pull path

This does **not** replace `metricx`'s `GET /metrics` (Prometheus) or StatsD push —
those keep working exactly as before, independently. You can enable OTLP push, the
existing pull/StatsD path, both, or neither.

**Metrics bridge:** every metric instrumented via `metricx` (`increment`,
`increment_one`, `current_gauge`, `record_money`, `record_duration`,
`start_stopwatch`, including the automatic HTTP/DB duration metrics) automatically
feeds **both** channels — the existing Prometheus `/metrics` pull endpoint (or StatsD
push) **and** the OTLP `MeterProvider` above — with no instrumentation changes
required. This works by wrapping the `Recorder` that `metricx` already builds
(`PrometheusRecorder`/`StatsdRecorder`) with an internal bridge that also forwards
every emission to `opentelemetry::global::meter("derust")`. The bridge is always
active (compiled in whenever `statsd`/`prometheus` is enabled) but has no observable
effect when no OTLP `MeterProvider` is configured — `opentelemetry::global::meter(...)`
then resolves to the API's no-op default, so the extra forwarding calls are cheap
lookups with no I/O. Histogram bucket boundaries are identical on both channels (a
single shared constant feeds both the Prometheus exporter and the OTLP `View`), so
`record_money`/`record_duration` distributions never diverge between the two.

### Logs

Every log emitted via `derust::tracex::log::*` (or any `tracing` macro) is
automatically captured by the OTLP logs pipeline once it is enabled — no code change
needed. Logs keep going to stdout as well (unchanged, via `fmt::layer()`); the two
outputs carry the same content and level.

### Cost warning

Enabling OTLP push for metrics and logs increases the volume of data sent to your
observability backend (e.g. Grafana Cloud). Review your backend's ingestion pricing
before enabling this in production.