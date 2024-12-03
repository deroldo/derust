use datadog_tracing::axum::OtelAxumLayer;
use std::time::Duration;
use tower_http::timeout::TimeoutLayer;

const DEFAULT_TIMEOUT: Duration = Duration::from_secs(30);

pub fn timeouts() -> (OtelAxumLayer, TimeoutLayer) {
    (OtelAxumLayer::default(), TimeoutLayer::new(DEFAULT_TIMEOUT))
}
