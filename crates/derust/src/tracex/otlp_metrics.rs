use opentelemetry_otlp::MetricExporter;
use opentelemetry_sdk::metrics::SdkMeterProvider;
use opentelemetry_sdk::Resource;

/// Builds an OTLP push pipeline for metrics (`SdkMeterProvider` backed by a
/// `PeriodicReader`, created internally by `with_periodic_exporter`), reading the same
/// `OTEL_EXPORTER_OTLP_*` env vars already used for traces. Returns `None` (after
/// logging a warning) when no exporter can be built — this must never fail
/// `tracex::init()`'s boot, mirroring the existing behaviour for traces.
pub(crate) fn build_otlp_meter_provider(resource: Resource) -> Option<SdkMeterProvider> {
    let protocol = infer_metrics_protocol();

    let exporter = match protocol.as_deref() {
        Some("http/protobuf") => match MetricExporter::builder().with_http().build() {
            Ok(exporter) => Some(exporter),
            Err(error) => {
                tracing::warn!(
                    "failed to build OTLP metric exporter (http/protobuf): {error}; no OTLP metrics will be pushed"
                );
                None
            }
        },
        Some("grpc") => match MetricExporter::builder().with_tonic().build() {
            Ok(exporter) => Some(exporter),
            Err(error) => {
                tracing::warn!(
                    "failed to build OTLP metric exporter (grpc): {error}; no OTLP metrics will be pushed"
                );
                None
            }
        },
        Some(other) => {
            tracing::warn!(
                "unknown OTEL_EXPORTER_OTLP_PROTOCOL '{other}'; no OTLP metric exporter will be created"
            );
            None
        }
        None => {
            tracing::warn!(
                "no OTEL_EXPORTER_OTLP_ENDPOINT/OTEL_EXPORTER_OTLP_PROTOCOL set; no OTLP metric exporter will be created"
            );
            None
        }
    }?;

    Some(
        SdkMeterProvider::builder()
            .with_periodic_exporter(exporter)
            .with_resource(resource)
            .build(),
    )
}

/// Mirrors `init_tracing_opentelemetry::otlp::infer_protocol`'s decision (private in
/// that crate): explicit `OTEL_EXPORTER_OTLP_PROTOCOL` wins; otherwise, infer from the
/// endpoint's default port (`:4317` => grpc, anything else with an endpoint set =>
/// http/protobuf); no endpoint and no protocol => `None`.
fn infer_metrics_protocol() -> Option<String> {
    if let Ok(protocol) = std::env::var("OTEL_EXPORTER_OTLP_PROTOCOL") {
        return Some(protocol);
    }
    let endpoint = std::env::var("OTEL_EXPORTER_OTLP_ENDPOINT").ok()?;
    if endpoint.contains(":4317") {
        Some("grpc".to_string())
    } else {
        Some("http/protobuf".to_string())
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use std::env;
    use std::sync::Mutex;

    // Mirrors the ENV_LOCK pattern in `initialize.rs` — these tests mutate
    // process-wide env vars.
    static ENV_LOCK: Mutex<()> = Mutex::new(());

    fn reset_env() {
        env::remove_var("OTEL_EXPORTER_OTLP_PROTOCOL");
        env::remove_var("OTEL_EXPORTER_OTLP_ENDPOINT");
    }

    #[test]
    fn builds_meter_provider_with_http_protobuf_env() {
        let _guard = ENV_LOCK.lock().unwrap();
        reset_env();
        env::set_var("OTEL_EXPORTER_OTLP_PROTOCOL", "http/protobuf");
        env::set_var("OTEL_EXPORTER_OTLP_ENDPOINT", "http://localhost:4318");

        let resource = Resource::builder_empty().build();
        let result = build_otlp_meter_provider(resource);

        assert!(
            result.is_some(),
            "expected build_otlp_meter_provider to return Some(_) with OTEL_EXPORTER_OTLP_PROTOCOL=http/protobuf"
        );

        reset_env();
    }

    #[test]
    fn returns_none_without_any_otlp_env() {
        let _guard = ENV_LOCK.lock().unwrap();
        reset_env();

        let resource = Resource::builder_empty().build();
        let result = build_otlp_meter_provider(resource);

        assert!(
            result.is_none(),
            "expected build_otlp_meter_provider to return None when no OTLP env is set"
        );

        reset_env();
    }
}
