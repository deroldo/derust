use tokio::signal;
use tracing::info;

#[cfg(feature = "env")]
pub mod envx;

#[cfg(feature = "http")]
pub mod httpx;

#[cfg(feature = "http")]
pub mod tracex;

#[cfg(feature = "http_client")]
pub mod http_clientx;

#[cfg(feature = "outbox")]
pub mod outboxx;

#[cfg(feature = "aws")]
pub mod awsx;

#[cfg(any(feature = "statsd", feature = "prometheus"))]
pub mod metricx;

#[cfg(any(feature = "postgres", feature = "outbox"))]
pub mod databasex;

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
