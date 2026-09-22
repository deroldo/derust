use init_tracing_opentelemetry::resource::DetectResource;
use init_tracing_opentelemetry::tracing_subscriber_ext::{build_otel_layer, TracingGuard};
use opentelemetry_appender_tracing::layer::OpenTelemetryTracingBridge;
use opentelemetry_sdk::logs::SdkLoggerProvider;
use opentelemetry_sdk::metrics::SdkMeterProvider;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::{fmt, EnvFilter};

use crate::tracex::otlp_logs::build_otlp_logger_provider;
use crate::tracex::otlp_metrics::build_otlp_meter_provider;

pub fn init() -> Result<Guard, Box<dyn std::error::Error>> {
    let subscriber = tracing_subscriber::registry()
        .with(build_loglevel_filter_layer())
        .with(fmt::layer());

    let _guard = tracing::subscriber::set_default(subscriber);

    let (trace_layer, trace_guard) = build_otel_layer()?;

    let resource = DetectResource::default().build();
    let meter_provider = build_otlp_meter_provider(resource.clone());
    if let Some(meter_provider) = &meter_provider {
        opentelemetry::global::set_meter_provider(meter_provider.clone());
    }
    let logger_provider = build_otlp_logger_provider(resource);
    let otlp_log_layer = logger_provider
        .as_ref()
        .map(OpenTelemetryTracingBridge::new);

    let subscriber = tracing_subscriber::registry()
        .with(trace_layer)
        .with(otlp_log_layer)
        .with(build_loglevel_filter_layer())
        .with(fmt::layer());

    tracing::subscriber::set_global_default(subscriber)?;

    Ok(Guard {
        trace_guard,
        meter_provider,
        logger_provider,
    })
}

/// On Drop, flushes and shuts down every OTLP pipeline that was actually created
/// (trace is always created — see `build_otel_layer()` — metrics/logs are `Option`
/// because they are opt-in by env, per this crate's `http_server`/`tracex` design).
/// Errors from `force_flush`/`shutdown` are intentionally ignored here, mirroring
/// `TracingGuard`'s existing `Drop` behaviour for traces (`init-tracing-opentelemetry`
/// v0.29.0, `tracing_subscriber_ext.rs:129-132`): shutdown must never panic during
/// application termination.
#[must_use = "Recommend holding with 'let _guard = ' pattern to ensure final traces/metrics/logs are sent to the server"]
pub struct Guard {
    trace_guard: TracingGuard,
    meter_provider: Option<SdkMeterProvider>,
    logger_provider: Option<SdkLoggerProvider>,
}

impl Guard {
    /// The wrapped trace guard (kept for backward-compatible access to the tracer
    /// provider, same accessor pattern as the previous `TracingGuard::tracer_provider()`).
    pub fn trace_guard(&self) -> &TracingGuard {
        &self.trace_guard
    }
}

impl Drop for Guard {
    fn drop(&mut self) {
        if let Some(meter_provider) = &self.meter_provider {
            let _ = meter_provider.force_flush();
            let _ = meter_provider.shutdown();
        }
        if let Some(logger_provider) = &self.logger_provider {
            let _ = logger_provider.force_flush();
            let _ = logger_provider.shutdown();
        }
        // `self.trace_guard` is dropped automatically right after this block, which
        // triggers its own `Drop` impl (force_flush + shutdown of the trace pipeline) —
        // no explicit call needed here.
    }
}

const DERUST_OTEL_DEBUG_ENV_NAME: &str = "DERUST_OTEL_DEBUG";

fn build_loglevel_filter_layer() -> EnvFilter {
    let host_log_level = std::env::var("RUST_LOG")
        .or_else(|_| std::env::var("OTEL_LOG_LEVEL"))
        .unwrap_or_else(|_| "info".to_string());

    let otel_debug_enabled = std::env::var(DERUST_OTEL_DEBUG_ENV_NAME)
        .map(|value| value.eq_ignore_ascii_case("true"))
        .unwrap_or(false);

    let filter = if otel_debug_enabled {
        format!("{host_log_level},derust=info,tower_http::trace=off,otel::tracing=trace,otel=debug")
    } else {
        format!("{host_log_level},derust=info,tower_http::trace=off,otel::tracing=trace")
    };

    std::env::set_var("RUST_LOG", filter);

    EnvFilter::from_default_env()
}

#[cfg(test)]
mod test {
    use super::*;
    use std::env;
    use std::sync::Mutex;

    // These tests mutate process-wide env vars, so they must not run concurrently
    // with each other (nextest runs each test in its own process by default, but
    // this guards against future changes to that behaviour within this binary).
    static ENV_LOCK: Mutex<()> = Mutex::new(());

    fn reset_env() {
        env::remove_var("RUST_LOG");
        env::remove_var("OTEL_LOG_LEVEL");
        env::remove_var(DERUST_OTEL_DEBUG_ENV_NAME);
        env::remove_var("OTEL_EXPORTER_OTLP_PROTOCOL");
        env::remove_var("OTEL_EXPORTER_OTLP_ENDPOINT");
    }

    #[test]
    fn does_not_force_otel_debug_by_default() {
        let _guard = ENV_LOCK.lock().unwrap();
        reset_env();
        env::set_var("RUST_LOG", "info");

        let _ = build_loglevel_filter_layer();

        let result = env::var("RUST_LOG").unwrap();
        assert!(!result.contains("otel=debug"));
        assert!(result.contains("info"));
        assert!(result.contains("derust=info"));
        assert!(result.contains("tower_http::trace=off"));
        assert!(result.contains("otel::tracing=trace"));

        reset_env();
    }

    #[test]
    fn includes_otel_debug_when_opted_in() {
        let _guard = ENV_LOCK.lock().unwrap();
        reset_env();
        env::set_var("RUST_LOG", "info");
        env::set_var(DERUST_OTEL_DEBUG_ENV_NAME, "true");

        let _ = build_loglevel_filter_layer();

        let result = env::var("RUST_LOG").unwrap();
        assert!(result.contains("otel=debug"));

        reset_env();
    }

    #[test]
    fn respects_host_application_rust_log() {
        let _guard = ENV_LOCK.lock().unwrap();
        reset_env();
        env::set_var("RUST_LOG", "warn,my_app=debug");

        let _ = build_loglevel_filter_layer();

        let result = env::var("RUST_LOG").unwrap();
        assert!(result.starts_with("warn,my_app=debug,"));
        assert!(!result.contains("otel=debug"));

        reset_env();
    }

    #[test]
    fn falls_back_to_otel_log_level_then_info() {
        let _guard = ENV_LOCK.lock().unwrap();
        reset_env();
        env::set_var("OTEL_LOG_LEVEL", "error");

        let _ = build_loglevel_filter_layer();

        let result = env::var("RUST_LOG").unwrap();
        assert!(result.starts_with("error,"));

        reset_env();
    }

    // Regression test for a Cargo feature-unification bug: opentelemetry-otlp's
    // "reqwest-client" and "reqwest-blocking-client" features were both being
    // enabled at the same time (one declared directly by derust, the other
    // pulled in transitively via init-tracing-opentelemetry's "otlp" feature).
    // Because opentelemetry-otlp's HTTP client selection requires exactly one
    // of those features to be enabled, having both present left it with no
    // client at all, and building the OTLP HTTP exporter failed at *runtime*
    // with `ExporterBuildError(NoHttpClient)` — but only once an app actually
    // configured `OTEL_EXPORTER_OTLP_PROTOCOL=http/protobuf`, so it never
    // showed up at compile time. This test exercises that exact code path.
    #[test]
    fn builds_otlp_http_protobuf_exporter_without_client_conflict() {
        let _guard = ENV_LOCK.lock().unwrap();
        reset_env();
        env::set_var("OTEL_EXPORTER_OTLP_PROTOCOL", "http/protobuf");
        env::set_var("OTEL_EXPORTER_OTLP_ENDPOINT", "http://localhost:4318");

        let result = init();

        assert!(
            result.is_ok(),
            "expected tracex::init() to succeed with OTEL_EXPORTER_OTLP_PROTOCOL=http/protobuf, got: {:?}",
            result.err().map(|error| error.to_string())
        );

        reset_env();
    }

    #[test]
    fn builds_otlp_metrics_pipeline_without_error_with_http_protobuf_env() {
        let _guard = ENV_LOCK.lock().unwrap();
        reset_env();
        env::set_var("OTEL_EXPORTER_OTLP_PROTOCOL", "http/protobuf");
        env::set_var("OTEL_EXPORTER_OTLP_ENDPOINT", "http://localhost:4318");

        let result = init();

        assert!(
            result.is_ok(),
            "expected tracex::init() to succeed when OTLP metrics env is set, got: {:?}",
            result.err().map(|error| error.to_string())
        );

        reset_env();
    }

    #[test]
    fn builds_otlp_logs_pipeline_without_error_with_http_protobuf_env() {
        let _guard = ENV_LOCK.lock().unwrap();
        reset_env();
        env::set_var("OTEL_EXPORTER_OTLP_PROTOCOL", "http/protobuf");
        env::set_var("OTEL_EXPORTER_OTLP_ENDPOINT", "http://localhost:4318");

        let result = init();

        assert!(
            result.is_ok(),
            "expected tracex::init() to succeed when OTLP logs env is set, got: {:?}",
            result.err().map(|error| error.to_string())
        );

        reset_env();
    }

    #[test]
    fn does_not_break_boot_when_no_otlp_env_is_set() {
        let _guard = ENV_LOCK.lock().unwrap();
        reset_env();

        let result = init();

        assert!(
            result.is_ok(),
            "expected tracex::init() to succeed (Ok, with all OTLP pipelines as None) when no OTLP env is set, got: {:?}",
            result.err().map(|error| error.to_string())
        );

        reset_env();
    }
}
