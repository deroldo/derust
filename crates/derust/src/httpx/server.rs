use axum::Router;
use std::net::{Ipv4Addr, SocketAddr};
use tokio::net::TcpListener;

use crate::httpx::extension::apply_middlewares;
use crate::httpx::AppContext;
use crate::shutdown_signal;
use tracing::info;
use wg::WaitGroup;

#[cfg(feature = "outbox")]
use crate::outboxx;
#[cfg(feature = "outbox")]
use outbox_pattern_processor::outbox_resources::OutboxProcessorResources;

pub async fn start<T>(
    port: u16,
    context: AppContext<T>,
    router: Router<AppContext<T>>,
    enable_web_socket: bool,
    #[cfg(feature = "outbox")] outbox_processor_resources: OutboxProcessorResources,
    #[cfg(feature = "outbox")] outbox_metrics_monitor_enabled: bool,
    #[cfg(feature = "outbox")] outbox_metrics_monitor_interval_in_secs: Option<u64>,
) -> Result<(), Box<dyn std::error::Error>>
where
    T: Clone + Send + Sync + 'static,
{
    let wg = WaitGroup::new();
    let http_router = apply_middlewares(router, context.clone());
    tokio::spawn(start_http_server(wg.add(1), port, http_router, enable_web_socket));

    #[cfg(feature = "outbox")]
    tokio::spawn(outboxx::run(wg.add(1), context.clone(), outbox_processor_resources, outbox_metrics_monitor_enabled, outbox_metrics_monitor_interval_in_secs));

    wg.wait();

    info!("Shutdown completed!");

    Ok(())
}

#[cfg(feature = "start_test")]
pub async fn start_test<T>(
    context: AppContext<T>,
    router: Router<AppContext<T>>,
    listener: TcpListener,
) -> std::io::Result<()>
where
    T: Clone + Send + Sync + 'static,
{
    let http_router = apply_middlewares(router, context);

    axum::serve(listener, http_router.into_make_service())
        .with_graceful_shutdown(shutdown_signal())
        .await
}

async fn start_http_server(wg: WaitGroup, port: u16, router: Router<()>, enable_web_socket: bool) {
    info!("Started http server on port {}", port);

    let addr = SocketAddr::from((Ipv4Addr::UNSPECIFIED, port));

    if let Ok(listener) = TcpListener::bind(addr).await {
        if enable_web_socket {
            let _ = axum::serve(listener, router.into_make_service_with_connect_info::<SocketAddr>())
                .with_graceful_shutdown(shutdown_signal())
                .await;
        } else {
            let _ = axum::serve(listener, router.into_make_service())
                .with_graceful_shutdown(shutdown_signal())
                .await;
        }
    }

    wg.done();

    info!("Http server stopped!");
}
