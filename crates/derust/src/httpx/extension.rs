use crate::httpx::middlewares::log::{local_log_request, log_request};
use crate::httpx::middlewares::{compression, error_handler, sensitive_headers, timeout};
use crate::httpx::{health, AppContext};
use axum::http::{header, HeaderName};
use axum::routing::get;
use axum::{middleware, Router};
use axum_tracing_opentelemetry::middleware::{OtelAxumLayer, OtelInResponseLayer};
use lazy_static::lazy_static;
use std::sync::Arc;
use tower_http::trace::TraceLayer;

#[cfg(feature = "prometheus")]
use crate::httpx::prometheus;

lazy_static! {
    static ref DEFAULT_REQUEST_ID: HeaderName = HeaderName::from_static("x-request-id");
    static ref DEFAULT_SENSITIVE_HEADERS: Arc<[HeaderName]> =
        vec![header::AUTHORIZATION, header::COOKIE].into();
}

pub(crate) fn apply_middlewares<S>(
    router: Router<AppContext<S>>,
    context: AppContext<S>,
) -> Router<()>
where
    S: Clone + Send + Sync + 'static,
{
    let mut builder = router.route(health::HEALTH_PATH, get(health::route));

    #[cfg(feature = "prometheus")]
    {
        builder = builder.nest(
            prometheus::PROMETHEUS_METRICS_PATH,
            Router::new().route("/", get(prometheus::route)),
        );
    }

    builder = builder
        .layer(sensitive_headers::request_headers())
        .layer(error_handler::panic_catcher())
        .layer(sensitive_headers::response_headers())
        .layer(TraceLayer::new_for_http())
        .layer(OtelInResponseLayer)
        .layer(OtelAxumLayer::default())
        .layer(timeout::timeouts())
        .layer(compression::compression());

    if context.env().is_local() {
        builder = builder.layer(middleware::from_fn_with_state(
            context.clone(),
            local_log_request::<S>,
        ));
    } else {
        builder = builder.layer(middleware::from_fn_with_state(
            context.clone(),
            log_request::<S>,
        ));
    }

    builder.with_state(context)
}
