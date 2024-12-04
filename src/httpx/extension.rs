use crate::httpx::health;
use crate::httpx::middlewares::log::log_request;
use crate::httpx::middlewares::{
    compression, error_handler, request_id, sensitive_headers, timeout, tracer,
};
use axum::http::{header, HeaderName};
use axum::routing::get;
use axum::{middleware, Router};
use lazy_static::lazy_static;
use std::sync::Arc;

lazy_static! {
    static ref DEFAULT_REQUEST_ID: HeaderName = HeaderName::from_static("x-request-id");
    static ref DEFAULT_SENSITIVE_HEADERS: Arc<[HeaderName]> =
        vec![header::AUTHORIZATION, header::COOKIE].into();
}

pub trait MiddlewaresGenericExtension<S> {
    fn using_httpx(self, state: S, routes: Vec<(&str, Router<S>)>) -> Router;
}

pub trait MiddlewaresExtension {
    fn using_httpx(self, routes: Vec<(&str, Router)>) -> Router;
}

impl<S> MiddlewaresGenericExtension<S> for Router<S>
where
    S: Clone + Send + Sync + 'static,
{
    fn using_httpx(self, state: S, routes: Vec<(&str, Router<S>)>) -> Router {
        apply_middlewares(self, state, routes)
    }
}

impl MiddlewaresExtension for Router {
    fn using_httpx(self, routes: Vec<(&str, Router)>) -> Router {
        apply_middlewares(self, (), routes)
    }
}

fn apply_middlewares<S>(router: Router<S>, state: S, routes: Vec<(&str, Router<S>)>) -> Router<()>
where
    S: Clone + Send + Sync + 'static,
{
    let router = routes
        .into_iter()
        .fold(router, |router, (path, r)| router.nest(path, r));

    router
        .route(health::HEALTH_PATH, get(health::route()))
        .layer(sensitive_headers::request_headers())
        .layer(error_handler::panic_catcher())
        .layer(tracer::trace())
        .layer(sensitive_headers::response_headers())
        .layer(request_id::set_request_id())
        .layer(request_id::propagate_request_id())
        .layer(tracer::otel_in_response())
        .layer(timeout::timeouts())
        .layer(compression::compression())
        .layer(middleware::from_fn(log_request))
        .with_state(state)
}