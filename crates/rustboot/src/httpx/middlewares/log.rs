use axum::body::Body;
use axum::extract::State;
use axum::http::{Method, Request, StatusCode, Uri};

use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use bytes::Bytes;

use crate::httpx::Tags;
use tracing::log::{log_enabled, Level};
use tracing::{error, info};

pub async fn local_log_request<S>(
    State(state): State<S>,
    req: Request<Body>,
    next: Next,
) -> Result<impl IntoResponse, (StatusCode, String)>
where
    S: Clone + Send + Sync + 'static,
{
    log(state, req, next, true).await
}


pub async fn log_request<S>(
    State(state): State<S>,
    req: Request<Body>,
    next: Next,
) -> Result<impl IntoResponse, (StatusCode, String)>
where
    S: Clone + Send + Sync + 'static,
{
    log(state, req, next, false).await
}

async fn log<S>(
    _state: S,
    req: Request<Body>,
    next: Next,
    local: bool,
) -> Result<impl IntoResponse, (StatusCode, String)>
where
    S: Clone + Send + Sync + 'static,
{
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
        .get::<Tags>()
        .cloned()
        .unwrap_or(Tags::ok());

    if local || log_enabled!(Level::Debug) {
        let request_body_string = std::str::from_utf8(&req_bytes)
            .unwrap_or("Could not convert request bytes into string");
        tags.insert("request-body", request_body_string);
    }

    let (parts, res_body) = res.into_parts();
    let bytes = buffer_and_print(method, uri, parts.status, res_body, tags, local).await?;
    let res = Response::from_parts(parts, Body::from(bytes));

    Ok(res)
}

async fn buffer_and_print(
    method: Method,
    uri: Uri,
    status: StatusCode,
    res_body: Body,
    tags: Tags,
    local: bool,
) -> Result<Bytes, (StatusCode, String)> {
    let response_bytes = match axum::body::to_bytes(res_body, usize::MAX).await {
        Ok(bytes) => bytes,
        Err(err) => {
            return Err((
                StatusCode::BAD_REQUEST,
                format!("failed to read body: {}", err),
            ));
        }
    };

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
