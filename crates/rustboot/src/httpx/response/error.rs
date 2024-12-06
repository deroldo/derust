use crate::httpx::tags::Tags;
use crate::httpx::HttpResponse;
use axum::http::StatusCode;
use axum_tracing_opentelemetry::tracing_opentelemetry_instrumentation_sdk::find_current_trace_id;
use serde_json::Value;

pub struct HttpError {
    status_code: StatusCode,
    error_message: String,
    response_body: Option<String>,
    response_headers: Option<Vec<(String, String)>>,
    tags: Tags,
}

impl HttpError {
    pub fn with_body(
        status_code: StatusCode,
        error_message: String,
        response_body: String,
        tags: Tags,
    ) -> Self {
        Self {
            status_code,
            error_message,
            response_body: Some(response_body),
            response_headers: None,
            tags,
        }
    }

    pub fn with_json(
        status_code: StatusCode,
        error_message: String,
        response_body: Value,
        tags: Tags,
    ) -> Self {
        let headers = vec![("Content-Type".to_string(), "application/json".to_string())];

        Self {
            status_code,
            error_message,
            response_body: Some(response_body.to_string()),
            response_headers: Some(headers),
            tags,
        }
    }

    pub fn without_body(status_code: StatusCode, error_message: String, tags: Tags) -> Self {
        Self {
            status_code,
            error_message,
            response_body: None,
            response_headers: None,
            tags,
        }
    }

    pub fn with_headers(mut self, response_headers: Vec<(String, String)>) -> Self {
        let mut headers: Vec<(String, String)> = self.response_headers.unwrap_or_default();

        for (key, value) in response_headers {
            headers.push((key, value));
        }

        self.response_headers = Some(headers);

        self
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
        self.response_body.clone()
    }

    fn response_headers(&self) -> Option<Vec<(String, String)>> {
        self.response_headers.clone()
    }

    fn tags(&self) -> Tags {
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
