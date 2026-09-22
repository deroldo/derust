use opentelemetry_otlp::LogExporter;
use opentelemetry_sdk::logs::SdkLoggerProvider;
use opentelemetry_sdk::Resource;

/// Builds an OTLP push pipeline for logs (`SdkLoggerProvider` backed by a batch log
/// processor), reading the same `OTEL_EXPORTER_OTLP_*` env vars already used for
/// traces. Returns `None` (after logging a warning) when no exporter can be built —
/// this must never fail `tracex::init()`'s boot, mirroring the existing behaviour for
/// traces.
pub(crate) fn build_otlp_logger_provider(resource: Resource) -> Option<SdkLoggerProvider> {
    let protocol = infer_logs_protocol();

    let exporter = match protocol.as_deref() {
        Some("http/protobuf") => match LogExporter::builder().with_http().build() {
            Ok(exporter) => Some(exporter),
            Err(error) => {
                tracing::warn!(
                    "failed to build OTLP log exporter (http/protobuf): {error}; no OTLP logs will be pushed"
                );
                None
            }
        },
        Some("grpc") => match LogExporter::builder().with_tonic().build() {
            Ok(exporter) => Some(exporter),
            Err(error) => {
                tracing::warn!(
                    "failed to build OTLP log exporter (grpc): {error}; no OTLP logs will be pushed"
                );
                None
            }
        },
        Some(other) => {
            tracing::warn!(
                "unknown OTEL_EXPORTER_OTLP_PROTOCOL '{other}'; no OTLP log exporter will be created"
            );
            None
        }
        None => {
            tracing::warn!(
                "no OTEL_EXPORTER_OTLP_ENDPOINT/OTEL_EXPORTER_OTLP_PROTOCOL set; no OTLP log exporter will be created"
            );
            None
        }
    }?;

    Some(
        SdkLoggerProvider::builder()
            .with_batch_exporter(exporter)
            .with_resource(resource)
            .build(),
    )
}

/// Mirrors `otlp_metrics::infer_metrics_protocol` — duplicated intentionally (small,
/// signal-specific, and each function only needs to know its own signal's env
/// fallback semantics; the alternative of sharing one generic helper was rejected to
/// keep each file readable on its own, consistent with the existing traces code in
/// `init_tracing_opentelemetry::otlp`, which does the same per-signal inference).
fn infer_logs_protocol() -> Option<String> {
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

    static ENV_LOCK: Mutex<()> = Mutex::new(());

    fn reset_env() {
        env::remove_var("OTEL_EXPORTER_OTLP_PROTOCOL");
        env::remove_var("OTEL_EXPORTER_OTLP_ENDPOINT");
    }

    #[test]
    fn builds_logger_provider_with_http_protobuf_env() {
        let _guard = ENV_LOCK.lock().unwrap();
        reset_env();
        env::set_var("OTEL_EXPORTER_OTLP_PROTOCOL", "http/protobuf");
        env::set_var("OTEL_EXPORTER_OTLP_ENDPOINT", "http://localhost:4318");

        let resource = Resource::builder_empty().build();
        let result = build_otlp_logger_provider(resource);

        assert!(
            result.is_some(),
            "expected build_otlp_logger_provider to return Some(_) with OTEL_EXPORTER_OTLP_PROTOCOL=http/protobuf"
        );

        reset_env();
    }

    #[test]
    fn returns_none_without_any_otlp_env() {
        let _guard = ENV_LOCK.lock().unwrap();
        reset_env();

        let resource = Resource::builder_empty().build();
        let result = build_otlp_logger_provider(resource);

        assert!(
            result.is_none(),
            "expected build_otlp_logger_provider to return None when no OTLP env is set"
        );

        reset_env();
    }
}
