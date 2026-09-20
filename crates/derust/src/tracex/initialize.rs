use init_tracing_opentelemetry::tracing_subscriber_ext::{build_otel_layer, TracingGuard};
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::{fmt, EnvFilter};

pub fn init() -> Result<TracingGuard, Box<dyn std::error::Error>> {
    let subscriber = tracing_subscriber::registry()
        .with(build_loglevel_filter_layer())
        .with(fmt::layer());

    let _guard = tracing::subscriber::set_default(subscriber);

    let (layer, guard) = build_otel_layer()?;

    let subscriber = tracing_subscriber::registry()
        .with(layer)
        .with(build_loglevel_filter_layer())
        .with(fmt::layer());

    tracing::subscriber::set_global_default(subscriber)?;

    Ok(guard)
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
}
