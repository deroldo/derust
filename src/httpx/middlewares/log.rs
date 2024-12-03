use axum::body::Body;
use axum::http::{Method, Request, StatusCode, Uri};

use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use bytes::Bytes;

use tracing::log::{log_enabled, Level};
use tracing::{debug, error, info};

pub async fn log_request(
    req: Request<Body>,
    next: Next,
) -> Result<impl IntoResponse, (StatusCode, String)> {
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

    let (parts, res_body) = res.into_parts();
    let bytes = buffer_and_print(method, uri, parts.status, res_body, req_bytes).await?;
    let res = Response::from_parts(parts, Body::from(bytes));

    Ok(res)
}

async fn buffer_and_print(
    method: Method,
    uri: Uri,
    status: StatusCode,
    res_body: Body,
    req_bytes: Bytes,
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

    let request_body_string =
        std::str::from_utf8(&req_bytes).unwrap_or("Could not convert request bytes into string");

    if status.is_server_error() {
        let res_body_str = std::str::from_utf8(&response_bytes)
            .unwrap_or("Could not convert response bytes into string");
        error!(
            request = request_body_string,
            "{method} {uri} -> {} :: response :: {res_body_str}",
            status.as_u16(),
        );
    } else if log_enabled!(Level::Debug) {
        let res_body_str = std::str::from_utf8(&response_bytes)
            .unwrap_or("Could not convert response bytes into string");
        debug!(
            request = request_body_string,
            "{} {} -> {} :: {}",
            method,
            uri,
            status.as_u16(),
            res_body_str,
        );
    } else {
        info!(
            request = request_body_string,
            "{method} {uri} -> {}",
            status.as_u16(),
        );
    }

    Ok(response_bytes)
}
