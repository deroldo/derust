use crate::httpx::json::JsonResponse;
use crate::httpx::text::TextResponse;
use crate::httpx::{HttpError, HttpResponse};
use axum::http::{HeaderMap, HeaderName, HeaderValue};
use axum::response::{IntoResponse, Response};
use std::str::FromStr;

impl IntoResponse for Box<dyn HttpResponse> {
    fn into_response(self) -> Response {
        let headers_vec = self.response_headers().unwrap_or_default();

        let mut headers = HeaderMap::new();
        for (name, value) in headers_vec {
            headers.insert(
                HeaderName::from_str(&name).unwrap(),
                HeaderValue::from_str(&value).unwrap(),
            );
        }

        let mut response = Response::builder()
            .status(self.status_code())
            .body(self.response_body().unwrap_or_default().into())
            .unwrap();

        *response.headers_mut() = headers;
        response.extensions_mut().insert(self.tags());

        response
    }
}

impl IntoResponse for JsonResponse {
    fn into_response(self) -> Response {
        let headers_vec = self.response_headers().unwrap_or_default();

        let mut headers = HeaderMap::new();
        for (name, value) in headers_vec {
            headers.insert(
                HeaderName::from_str(&name).unwrap(),
                HeaderValue::from_str(&value).unwrap(),
            );
        }

        let mut response = Response::builder()
            .status(self.status_code())
            .body(self.response_body().unwrap_or_default().into())
            .unwrap();

        *response.headers_mut() = headers;
        response.extensions_mut().insert(self.tags());

        response
    }
}

impl IntoResponse for TextResponse {
    fn into_response(self) -> Response {
        let headers_vec = self.response_headers().unwrap_or_default();

        let mut headers = HeaderMap::new();
        for (name, value) in headers_vec {
            headers.insert(
                HeaderName::from_str(&name).unwrap(),
                HeaderValue::from_str(&value).unwrap(),
            );
        }

        let mut response = Response::builder()
            .status(self.status_code())
            .body(self.response_body().unwrap_or_default().into())
            .unwrap();

        *response.headers_mut() = headers;
        response.extensions_mut().insert(self.tags());

        response
    }
}

impl IntoResponse for HttpError {
    fn into_response(self) -> Response {
        let headers_vec = self.response_headers().unwrap_or_default();

        let mut headers = HeaderMap::new();
        for (name, value) in headers_vec {
            headers.insert(
                HeaderName::from_str(&name).unwrap(),
                HeaderValue::from_str(&value).unwrap(),
            );
        }

        let mut response = Response::builder()
            .status(self.status_code())
            .body(self.response_body().unwrap_or_default().into())
            .unwrap();

        *response.headers_mut() = headers;
        response.extensions_mut().insert(self.tags());

        response
    }
}
