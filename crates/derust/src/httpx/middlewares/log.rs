use axum::body::Body;
use axum::extract::State;
use axum::http::{Method, Request, StatusCode, Uri};

use crate::httpx::{AppContext, HttpTags};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use bytes::Bytes;
use flate2::read::GzDecoder;
use std::io::Read;

use tracing::log::{log_enabled, Level};
use tracing::{error, info};

#[cfg(any(feature = "statsd", feature = "prometheus"))]
use regex::Regex;
use serde_json::Value;
#[cfg(any(feature = "statsd", feature = "prometheus"))]
use crate::metricx::{timer, MetricTags, Stopwatch};

pub async fn local_log_request<S>(
    State(context): State<AppContext<S>>,
    req: Request<Body>,
    next: Next,
) -> Result<impl IntoResponse, (StatusCode, String)>
where
    S: Clone + Send + Sync + 'static,
{
    log(&context, req, next, true).await
}

pub async fn log_request<S>(
    State(context): State<AppContext<S>>,
    req: Request<Body>,
    next: Next,
) -> Result<impl IntoResponse, (StatusCode, String)>
where
    S: Clone + Send + Sync + 'static,
{
    log(&context, req, next, false).await
}

async fn log<S>(
    context: &AppContext<S>,
    req: Request<Body>,
    next: Next,
    local: bool,
) -> Result<impl IntoResponse, (StatusCode, String)>
where
    S: Clone + Send + Sync + 'static,
{
    #[cfg(any(feature = "statsd", feature = "prometheus"))]
    let stopwatch = start_stopwatch(&context, &req);

    let (req_parts, req_body) = req.into_parts();
    let method = req_parts.method.clone();
    let uri = req_parts.uri.clone();
    let req_bytes = axum::body::to_bytes(req_body, usize::MAX)
        .await
        .map_err(|err| {
            (
                StatusCode::BAD_REQUEST,
                format!("failed to read body: {}", err),
            )
        })?;

    let mut req = Request::from_parts(req_parts, Body::from(req_bytes.clone()));
    req.extensions_mut().insert(context.clone());

    let res = next.run(req).await;
    let tags = res
        .extensions()
        .get::<HttpTags>()
        .cloned()
        .unwrap_or(HttpTags::default());

    let request_body_string = if local || log_enabled!(Level::Debug) {
        let payload = std::str::from_utf8(&req_bytes)
            .unwrap_or("Could not convert request bytes into string");
        
        let result_value = serde_json::from_str::<Value>(payload);
        match result_value {
            Ok(value) => serde_json::to_string(&value).unwrap_or_default(),
            Err(_) => payload.to_string()
        }
    } else {
        String::new()
    };

    let (parts, res_body) = res.into_parts();
    let bytes = buffer_and_print(
        context,
        method,
        uri,
        parts.status,
        res_body,
        request_body_string,
        tags,
        local,
    )
    .await?;
    let res = Response::from_parts(parts, Body::from(bytes));

    #[cfg(any(feature = "statsd", feature = "prometheus"))]
    stopwatch.record(MetricTags::from([(
        "status",
        res.status().as_u16().to_string(),
    )]));

    Ok(res)
}

async fn buffer_and_print<S>(
    context: &AppContext<S>,
    method: Method,
    uri: Uri,
    status: StatusCode,
    res_body: Body,
    request_body_string: String,
    tags: HttpTags,
    local: bool,
) -> Result<Bytes, (StatusCode, String)>
where
    S: Clone,
{
    let response_bytes = match axum::body::to_bytes(res_body, usize::MAX).await {
        Ok(bytes) => bytes,
        Err(err) => {
            return Err((
                StatusCode::BAD_REQUEST,
                format!("failed to read body: {}", err),
            ));
        }
    };

    if context.ignore_log_for_paths().contains(&uri.to_string()) {
        return Ok(response_bytes);
    }

    if local || log_enabled!(Level::Debug) {
        let res_body_str = match decompress_gzip(&response_bytes) {
            Ok(decompressed) => std::str::from_utf8(&decompressed)
                .unwrap_or("Could not convert response bytes into string after decompression")
                .to_string(),
            Err(_) => {
                if is_binary(&response_bytes) {
                    "<binary data>".to_string()
                } else {
                    std::str::from_utf8(&response_bytes)
                        .unwrap_or("Could not convert response bytes into string")
                        .to_string()
                }
            }
        };

        if status.is_server_error() {
            error!(
                tags = ?tags.values(),
                "{method} {uri} -> {} :: request={request_body_string} :: response={res_body_str}",
                status.as_u16(),
            );
        } else {
            info!(
                tags = ?tags.values(),
                "{method} {uri} -> {} :: request={request_body_string} :: response={res_body_str}",
                status.as_u16(),
            );
        }
    } else if status.is_server_error() {
        error!(
            tags = ?tags.values(),
            "{method} {uri} -> {}", status.as_u16(),
        );
    } else {
        info!(
            tags = ?tags.values(),
            "{method} {uri} -> {}", status.as_u16(),
        );
    }

    Ok(response_bytes)
}

#[cfg(any(feature = "statsd", feature = "prometheus"))]
fn start_stopwatch<S>(context: &AppContext<S>, req: &Request<Body>) -> Stopwatch<S>
where
    S: Clone,
{
    let metric_tags = MetricTags::htt_server(req.uri(), req.method());
    timer::start_stopwatch(&context, "http_server_seconds", metric_tags)
}

fn decompress_gzip(data: &[u8]) -> Result<Vec<u8>, &'static str> {
    let mut decoder = GzDecoder::new(data);
    let mut decompressed = Vec::new();

    match decoder.read_to_end(&mut decompressed) {
        Ok(_) => Ok(decompressed),
        Err(_) => Err("Failed to decompress GZIP data"),
    }
}

fn is_binary(data: &[u8]) -> bool {
    data.iter()
        .any(|&byte| (byte < 32 || byte > 126) && !matches!(byte, b'\n' | b'\r' | b'\t'))
}
