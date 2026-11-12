#[cfg(feature = "http_server")]
use tokio::signal;
#[cfg(feature = "http_server")]
use tracing::info;

#[cfg(feature = "env")]
pub mod envx;

#[cfg(feature = "http_server")]
pub mod commonx;

#[cfg(feature = "http_server")]
pub mod httpx;

#[cfg(feature = "http_server")]
pub mod tracex;

#[cfg(feature = "http_client")]
pub mod http_clientx;

#[cfg(feature = "outbox")]
pub mod outboxx;

#[cfg(any(feature = "aws", feature = "env_from_secrets_manager"))]
pub mod awsx;

#[cfg(any(feature = "statsd", feature = "prometheus"))]
pub mod metricx;

#[cfg(any(feature = "postgres", feature = "outbox"))]
pub mod databasex;

#[cfg(feature = "growthbook")]
pub mod growthbookx;

#[cfg(feature = "http_server")]
pub use axum::http::StatusCode;

#[cfg(feature = "sdui_stac_1")]
pub mod sduix;

#[cfg(feature = "http_server")]
pub async fn shutdown_signal() {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("Failed to install Ctrl+C handler");
    };

    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("Failed to install signal handler")
            .recv()
            .await;
    };

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }

    info!("Starting graceful shutdown");
}
