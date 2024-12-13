use axum::http::{header, HeaderName};
use lazy_static::lazy_static;
use std::sync::Arc;
use tower_http::sensitive_headers::{
    SetSensitiveRequestHeadersLayer, SetSensitiveResponseHeadersLayer,
};

lazy_static! {
    static ref DEFAULT_SENSITIVE_HEADERS: Arc<[HeaderName]> =
        vec![header::AUTHORIZATION, header::COOKIE].into();
}

pub fn request_headers() -> SetSensitiveRequestHeadersLayer {
    SetSensitiveRequestHeadersLayer::from_shared(DEFAULT_SENSITIVE_HEADERS.clone())
}

pub fn response_headers() -> SetSensitiveResponseHeadersLayer {
    SetSensitiveResponseHeadersLayer::from_shared(DEFAULT_SENSITIVE_HEADERS.clone())
}
