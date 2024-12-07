use axum::body::Body;
use axum::extract::State;
use axum::http::{Method, Request, StatusCode, Uri};

use crate::httpx::{AppContext, HttpTags};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use bytes::Bytes;

use tracing::log::{log_enabled, Level};
use tracing::{error, info};

#[cfg(any(feature = "statsd", feature = "prometheus"))]
use regex::Regex;

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
    log(context, req, next, true).await
}

pub async fn log_request<S>(
    State(context): State<AppContext<S>>,
    req: Request<Body>,
    next: Next,
) -> Result<impl IntoResponse, (StatusCode, String)>
where
    S: Clone + Send + Sync + 'static,
{
    log(context, req, next, false).await
}

async fn log<S>(
    context: AppContext<S>,
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

    let req = Request::from_parts(req_parts, Body::from(req_bytes.clone()));

    let res = next.run(req).await;
    let mut tags = res
        .extensions()
        .get::<HttpTags>()
        .cloned()
        .unwrap_or(HttpTags::default());

    if local || log_enabled!(Level::Debug) {
        let request_body_string = std::str::from_utf8(&req_bytes)
            .unwrap_or("Could not convert request bytes into string");
        tags.add("request-body", request_body_string);
    }

    let (parts, res_body) = res.into_parts();
    let bytes =
        buffer_and_print(&context, method, uri, parts.status, res_body, tags, local).await?;
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
        let res_body_str = std::str::from_utf8(&response_bytes)
            .unwrap_or("Could not convert response bytes into string");

        if status.is_server_error() {
            error!(
                tags = ?tags.values(),
                "{method} {uri} -> {} :: response :: {res_body_str}",
                status.as_u16(),
            );
        } else {
            info!(
                tags = ?tags.values(),
                "{method} {uri} -> {} :: response :: {res_body_str}",
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
    let metric_tags =
        if let Ok(regex) = Regex::new("(/[a-fA-F0-9-]{36})|(/\\d+/)|(/\\d+$|/\\d+\\?)") {
            let normalized_path = regex
                .replace_all(req.uri().path(), "/{path_param}")
                .to_string();

            let path = normalized_path
                .split("?")
                .next()
                .unwrap_or("failed_to_clean_up_path");

            MetricTags::from([("method", req.method().as_str()), ("path", &path)])
        } else {
            MetricTags::default()
        };

    timer::start_stopwatch(&context, "http_server_income", metric_tags)
}
