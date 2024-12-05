use crate::httpx::{HttpError, HttpResponse};
use axum::http::{HeaderMap, HeaderName, HeaderValue};
use axum::response::{IntoResponse, Response};
use std::str::FromStr;
use crate::httpx::json::JsonResponse;

impl IntoResponse for Box<dyn HttpResponse> {
    // TODO: remover os unwraps
    fn into_response(self) -> Response {
        let headers_vec = self.response_headers().unwrap_or_default();

        let mut headers = HeaderMap::new();
        for (name, value) in headers_vec {
            headers.insert(HeaderName::from_str(&name).unwrap(), HeaderValue::from_str(&value).unwrap());
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
    // TODO: remover os unwraps
    fn into_response(self) -> Response {
        let headers_vec = self.response_headers().unwrap_or_default();

        let mut headers = HeaderMap::new();
        for (name, value) in headers_vec {
            headers.insert(HeaderName::from_str(&name).unwrap(), HeaderValue::from_str(&value).unwrap());
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
    // TODO: remover os unwraps
    fn into_response(self) -> Response {
        let headers_vec = self.response_headers().unwrap_or_default();

        let mut headers = HeaderMap::new();
        for (name, value) in headers_vec {
            headers.insert(HeaderName::from_str(&name).unwrap(), HeaderValue::from_str(&value).unwrap());
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