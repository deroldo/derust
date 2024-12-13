use crate::httpx::{HttpResponse, HttpTags};
use axum::http::StatusCode;
use axum_tracing_opentelemetry::tracing_opentelemetry_instrumentation_sdk::find_current_trace_id;

#[derive(Clone)]
pub struct TextResponse {
    status_code: StatusCode,
    response_body: String,
    response_headers: Option<Vec<(String, String)>>,
    tags: HttpTags,
}

impl TextResponse {
    pub fn new(status_code: StatusCode, response_body: String, tags: HttpTags) -> Self {
        Self {
            status_code,
            response_body,
            response_headers: None,
            tags,
        }
    }

    pub fn with_headers(mut self, response_headers: Vec<(String, String)>) -> Self {
        self.response_headers = Some(response_headers);
        self
    }
}

impl HttpResponse for TextResponse {
    fn status_code(&self) -> StatusCode {
        self.status_code
    }

    fn error_message(&self) -> Option<String> {
        None
    }

    fn response_body(&self) -> Option<String> {
        Some(self.response_body.to_string())
    }

    fn response_headers(&self) -> Option<Vec<(String, String)>> {
        let mut headers: Vec<(String, String)> = self.response_headers.clone().unwrap_or_default();

        if !headers
            .iter()
            .any(|(name, _)| name.to_uppercase() == "Content-Type".to_uppercase())
        {
            headers.push(("Content-Type".to_string(), "text/plain".to_string()));
        }

        Some(headers)
    }

    fn tags(&self) -> HttpTags {
        let mut tags = self.tags.clone();

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
