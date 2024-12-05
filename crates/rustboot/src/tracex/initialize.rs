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

fn build_loglevel_filter_layer() -> tracing_subscriber::filter::EnvFilter {
    std::env::set_var(
        "RUST_LOG",
        format!(
            "{},tower_http::trace=off,otel::tracing=trace,otel=debug",
            std::env::var("RUST_LOG")
                .or_else(|_| std::env::var("OTEL_LOG_LEVEL"))
                .unwrap_or_else(|_| "info".to_string())
        ),
    );
    EnvFilter::from_default_env()
}
