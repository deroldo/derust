use tokio::signal;

pub mod envx;

#[cfg(feature = "http")]
pub mod httpx;

#[cfg(feature = "http")]
pub mod tracex;

#[cfg(feature = "http_client")]
pub mod http_clientx;

pub async fn shutdown_signal() {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;
    };

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }

    println!("signal received, starting graceful shutdown");
}