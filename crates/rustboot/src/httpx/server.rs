use axum::Router;
use std::net::{Ipv4Addr, SocketAddr};
use tokio::net::TcpListener;

use crate::shutdown_signal;
use tracing::info;
use wg::WaitGroup;

#[cfg(feature = "outbox")]
use crate::outboxx;
#[cfg(feature = "outbox")]
use outbox_pattern_processor::outbox_resources::OutboxProcessorResources;

pub async fn start(
    port: u16,
    router: Router<()>,
    #[cfg(feature = "outbox")] outbox_processor_resources: OutboxProcessorResources,
) -> Result<(), Box<dyn std::error::Error>> {
    let wg = WaitGroup::new();

    tokio::spawn(start_http_server(wg.add(1), port, router));

    #[cfg(feature = "outbox")]
    tokio::spawn(outboxx::run(wg.add(1), outbox_processor_resources));

    wg.wait();

    info!("Shutdown completed!");

    Ok(())
}

async fn start_http_server(
    wg: WaitGroup,
    port: u16,
    router: Router<()>,
) {
    info!("Starting http server on port {}", port);

    let addr = SocketAddr::from((Ipv4Addr::UNSPECIFIED, port));

    if let Ok(listener) = TcpListener::bind(addr).await {
        let _ = axum::serve(listener, router.into_make_service())
            .with_graceful_shutdown(shutdown_signal())
            .await;
    }

    wg.done();

    info!("Http server stopped!");
}


