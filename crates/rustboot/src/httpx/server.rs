use axum::Router;
use std::net::{Ipv4Addr, SocketAddr};
use tokio::net::TcpListener;

use tracing::info;
use crate::shutdown_signal;

pub async fn start(port: u16, router: Router<()>) -> Result<(), Box<dyn std::error::Error>> {
    info!("Starting http server on port {}", port);

    let addr = SocketAddr::from((Ipv4Addr::UNSPECIFIED, port));
    let listener = TcpListener::bind(addr).await?;

    axum::serve(listener, router.into_make_service())
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    Ok(())
}
