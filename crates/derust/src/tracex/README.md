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

**Important:** metrics instrumented today via the `metrics` crate (the macros used
internally by `metricx`, e.g. `counter!`/`histogram!`) are **not** automatically
forwarded to the OTLP `MeterProvider` — there is no maintained bridge between the
`metrics` crate and `opentelemetry::metrics` today. If you need custom metrics pushed
via OTLP, instrument them directly with the `opentelemetry::metrics` API (via
`opentelemetry::global::meter(...)`, after `tracex::init()` has run) — this means
double instrumentation if you also want the same metric on the Prometheus/StatsD path.
This is a known, documented limitation, not a bug.

### Logs

Every log emitted via `derust::tracex::log::*` (or any `tracing` macro) is
automatically captured by the OTLP logs pipeline once it is enabled — no code change
needed. Logs keep going to stdout as well (unchanged, via `fmt::layer()`); the two
outputs carry the same content and level.

### Cost warning

Enabling OTLP push for metrics and logs increases the volume of data sent to your
observability backend (e.g. Grafana Cloud). Review your backend's ingestion pricing
before enabling this in production.