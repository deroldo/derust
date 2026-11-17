use crate::commonx::DerustError;
use crate::httpx::json::JsonResponse;
#[cfg(feature = "sdui_stac_1")]
use crate::httpx::sdui::SduiResponse;
#[cfg(feature = "sdui_stac_1")]
use crate::httpx::stac_error::StacErrorResponse;
use crate::httpx::text::TextResponse;
use crate::httpx::HttpResponse;
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

impl<T> IntoResponse for JsonResponse<T>
where
    T: serde::Serialize + Send + Sync,
{
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

#[cfg(feature = "sdui_stac_1")]
impl<T> IntoResponse for SduiResponse<T>
where
    T: serde::Serialize + crate::sduix::stacx::common::Validate + Send + Sync,
{
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

#[cfg(feature = "sdui_stac_1")]
impl IntoResponse for StacErrorResponse {
    fn into_response(self) -> Response {
        let mut headers = HeaderMap::new();
        // names and values were validated in `StacErrorResponseBuilder::build`
        for (name, value) in self.response_headers().unwrap_or_default() {
            if let (Ok(name), Ok(value)) =
                (HeaderName::from_str(&name), HeaderValue::from_str(&value))
            {
                headers.insert(name, value);
            }
        }

        let mut response = Response::new(self.response_body().unwrap_or_default().into());
        *response.status_mut() = self.status_code();
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

impl IntoResponse for DerustError {
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
