use std::env;
use std::time::Duration;
use tower_http::timeout::TimeoutLayer;

const DEFAULT_TIMEOUT_ENV_NAME: &str = "SERVER_TIMEOUT_IN_MILLIS";
const DEFAULT_TIMEOUT_STR: &str = "10000";
const DEFAULT_TIMEOUT_U64: u64 = 10000;

pub fn timeouts() -> TimeoutLayer {
    let timeout = env::var(DEFAULT_TIMEOUT_ENV_NAME).unwrap_or(DEFAULT_TIMEOUT_STR.to_string());
    let duration = Duration::from_millis(timeout.parse().ok().unwrap_or(DEFAULT_TIMEOUT_U64));
    TimeoutLayer::new(duration)
}
