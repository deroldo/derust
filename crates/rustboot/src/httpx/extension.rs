use crate::envx::Environment;
use crate::httpx::health;
use crate::httpx::middlewares::log::{local_log_request, log_request};
use crate::httpx::middlewares::{compression, error_handler, sensitive_headers, timeout};
use axum::http::{header, HeaderName};
use axum::routing::get;
use axum::{middleware, Router};
use axum_tracing_opentelemetry::middleware::{OtelAxumLayer, OtelInResponseLayer};
use lazy_static::lazy_static;
use std::sync::Arc;
use tower_http::trace::TraceLayer;

lazy_static! {
    static ref DEFAULT_REQUEST_ID: HeaderName = HeaderName::from_static("x-request-id");
    static ref DEFAULT_SENSITIVE_HEADERS: Arc<[HeaderName]> =
        vec![header::AUTHORIZATION, header::COOKIE].into();
}

pub trait HttpMiddlewares<S> {
    fn using_httpx(self, state: S, env: Environment) -> Router;
}

impl<S> HttpMiddlewares<S> for Router<S>
where
    S: Clone + Send + Sync + 'static,
{
    fn using_httpx(self, state: S, env: Environment) -> Router {
        apply_middlewares(self, state, env)
    }
}

fn apply_middlewares<S>(router: Router<S>, state: S, env: Environment) -> Router<()>
where
    S: Clone + Send + Sync + 'static,
{
    let mut builder = router
        .route(health::HEALTH_PATH, get(health::route()))
        .layer(sensitive_headers::request_headers())
        .layer(error_handler::panic_catcher())
        .layer(sensitive_headers::response_headers())
        .layer(TraceLayer::new_for_http())
        .layer(OtelInResponseLayer)
        .layer(OtelAxumLayer::default())
        .layer(timeout::timeouts())
        .layer(compression::compression());

    if env.is_local() {
        builder = builder.layer(middleware::from_fn_with_state(
            state.clone(),
            local_log_request::<S>,
        ));
    } else {
        builder = builder.layer(middleware::from_fn_with_state(
            state.clone(),
            log_request::<S>,
        ));
    }

    builder.with_state(state)
}
