use crate::httpx::tags::HttpTags;
use crate::httpx::HttpResponse;
use axum::body::Body;
use axum::http::{Response, StatusCode};
use axum_tracing_opentelemetry::tracing_opentelemetry_instrumentation_sdk::find_current_trace_id;
use serde_json::Value;

pub struct HttpError {
    status_code: StatusCode,
    error_message: String,
    response_body: Box<Option<String>>,
    response_headers: Box<Option<Vec<(String, String)>>>,
    tags: HttpTags,
}

impl HttpError {
    pub fn with_body(
        status_code: StatusCode,
        error_message: String,
        response_body: String,
        tags: HttpTags,
    ) -> Self {
        Self {
            status_code,
            error_message,
            response_body: Box::new(Some(response_body)),
            response_headers: Box::new(None),
            tags,
        }
    }

    pub fn with_json(
        status_code: StatusCode,
        error_message: String,
        response_body: Value,
        tags: HttpTags,
    ) -> Self {
        let headers = vec![("Content-Type".to_string(), "application/json".to_string())];

        Self {
            status_code,
            error_message,
            response_body: Box::new(Some(response_body.to_string())),
            response_headers: Box::new(Some(headers)),
            tags,
        }
    }

    pub fn without_body(status_code: StatusCode, error_message: String, tags: HttpTags) -> Self {
        Self {
            status_code,
            error_message,
            response_body: Box::new(None),
            response_headers: Box::new(None),
            tags,
        }
    }

    pub fn with_headers(mut self, response_headers: Vec<(String, String)>) -> Self {
        let mut headers: Vec<(String, String)> = self.response_headers.unwrap_or_default();

        for (key, value) in response_headers {
            headers.push((key, value));
        }

        self.response_headers = Box::new(Some(headers));

        self
    }

    pub fn response_json(&self) -> Option<Value> {
        if let Some(body) = self.response_body() {
            return serde_json::from_str(&body).ok();
        }

        None
    }
}

impl HttpResponse for HttpError {
    fn status_code(&self) -> StatusCode {
        self.status_code
    }

    fn error_message(&self) -> Option<String> {
        Some(self.error_message.to_string())
    }

    fn response_body(&self) -> Option<String> {
        self.response_body.clone().map(|s| Some(s)).unwrap_or(None)
    }

    fn response_headers(&self) -> Option<Vec<(String, String)>> {
        self.response_headers
            .clone()
            .map(|v| Some(v))
            .unwrap_or(None)
    }

    fn tags(&self) -> HttpTags {
        let mut tags = self.tags.clone();
        tags.add("error_message", &self.error_message);

        if !tags
            .values()
            .iter()
            .any(|(key, _)| key.to_uppercase() == "X-TRACE-ID".to_uppercase())
        {
            if let Some(trace_id) = find_current_trace_id() {
                tags.add("x-trace-id", &trace_id);
            }
        }

        tags
    }
}

impl From<HttpError> for Response<Body> {
    fn from(http_error: HttpError) -> Self {
        let mut response = Response::builder().status(http_error.status_code());

        if let Some(headers) = http_error.response_headers() {
            for (key, value) in headers {
                response = response.header(&key, value);
            }
        }

        let body = http_error.response_body().unwrap_or_default();

        response.body(Body::from(body)).unwrap_or_else(|_| {
            Response::builder()
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .body(Body::empty())
                .unwrap()
        })
    }
}
