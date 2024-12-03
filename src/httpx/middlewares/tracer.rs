use bytes::Bytes;
use datadog_tracing::axum::OtelInResponseLayer;
use lazy_static::lazy_static;
use std::time::Duration;
use tower_http::trace::{
    DefaultMakeSpan, DefaultOnRequest, DefaultOnResponse, HttpMakeClassifier, OnBodyChunk,
    TraceLayer,
};
use tower_http::LatencyUnit;
use tracing::{trace, Span};

lazy_static! {
    static ref DEFAULT_TRACE_SPAN: DefaultMakeSpan = DefaultMakeSpan::new()
        .include_headers(true)
        .level(tracing::Level::INFO);
    static ref DEFAULT_TRACE_ON_RESPONSE: DefaultOnResponse = DefaultOnResponse::new()
        .include_headers(true)
        .latency_unit(LatencyUnit::Micros);
}

#[derive(Copy, Clone)]
pub struct TraceBodyChunk;

impl OnBodyChunk<Bytes> for TraceBodyChunk {
    fn on_body_chunk(&mut self, chunk: &Bytes, latency: Duration, _: &Span) {
        trace!(size_bytes = chunk.len(), latency = ?latency, "body chunked");
    }
}

pub fn trace() -> TraceLayer<
    HttpMakeClassifier,
    DefaultMakeSpan,
    DefaultOnRequest,
    DefaultOnResponse,
    TraceBodyChunk,
> {
    TraceLayer::new_for_http()
        .on_body_chunk(TraceBodyChunk)
        .make_span_with(DEFAULT_TRACE_SPAN.clone())
        .on_response(DEFAULT_TRACE_ON_RESPONSE.clone())
}

pub fn otel_in_response() -> OtelInResponseLayer {
    OtelInResponseLayer
}
