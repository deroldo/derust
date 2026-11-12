use crate::commonx::{DerustError, DerustTags};
use crate::httpx::HttpResponse;
use crate::sduix::stacx::common::Validate;
use axum::http::StatusCode;
use axum_tracing_opentelemetry::tracing_opentelemetry_instrumentation_sdk::find_current_trace_id;

/// JSON response carrying a server-driven UI screen (works with any Stac major).
///
/// The screen is validated and serialized **once**, in [`SduiResponse::try_new`]; a screen that
/// cannot be validated or serialized yields a [`DerustError`] (HTTP 500) instead of a partial
/// response.
#[derive(Clone)]
pub struct SduiResponse<T>
where
    T: serde::Serialize + Validate + Send + Sync,
{
    status_code: StatusCode,
    screen: T,
    body: String,
    response_headers: Option<Vec<(String, String)>>,
    tags: DerustTags,
}

impl<T> SduiResponse<T>
where
    T: serde::Serialize + Validate + Send + Sync,
{
    /// Validates and serializes `screen`.
    pub fn try_new(
        status_code: StatusCode,
        screen: T,
        tags: DerustTags,
    ) -> Result<Self, DerustError> {
        let issues = screen.validate();
        if !issues.is_empty() {
            let details = issues
                .iter()
                .map(|issue| issue.to_string())
                .collect::<Vec<_>>()
                .join("; ");
            return Err(DerustError::without_body(
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("failed to serialize SDUI screen: invalid screen: {details}"),
                tags,
            ));
        }

        let body = serde_json::to_string(&screen).map_err(|error| {
            DerustError::without_body(
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("failed to serialize SDUI screen: {error}"),
                tags.clone(),
            )
        })?;

        Ok(Self {
            status_code,
            screen,
            body,
            response_headers: None,
            tags,
        })
    }

    /// Extra response headers (`Content-Type: application/json` is added when absent).
    pub fn with_headers(mut self, response_headers: Vec<(String, String)>) -> Self {
        self.response_headers = Some(response_headers);
        self
    }

    /// The screen that was serialized.
    pub fn screen(&self) -> &T {
        &self.screen
    }
}

impl<T> HttpResponse for SduiResponse<T>
where
    T: serde::Serialize + Validate + Send + Sync,
{
    fn status_code(&self) -> StatusCode {
        self.status_code
    }

    fn error_message(&self) -> Option<String> {
        None
    }

    fn response_body(&self) -> Option<String> {
        Some(self.body.clone())
    }

    fn response_headers(&self) -> Option<Vec<(String, String)>> {
        let mut headers: Vec<(String, String)> = self.response_headers.clone().unwrap_or_default();

        if !headers
            .iter()
            .any(|(name, _)| name.to_uppercase() == "Content-Type".to_uppercase())
        {
            headers.push(("Content-Type".to_string(), "application/json".to_string()));
        }

        Some(headers)
    }

    fn tags(&self) -> DerustTags {
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

#[cfg(all(test, feature = "sdui_stac_1"))]
mod tests {
    use super::*;
    use crate::sduix::stacx::common::{Path, ValidationIssue};
    use crate::sduix::stacx::v1::prelude::*;
    use axum::response::IntoResponse;
    use serde::ser::{Error as _, Serializer};

    fn tags() -> DerustTags {
        DerustTags::from([("screen", "home")])
    }

    #[test]
    fn serializes_valid_screen_with_headers_and_tags() {
        let screen = Widget::from(column().children([Widget::from(text("hi"))]));
        let response = SduiResponse::try_new(StatusCode::OK, screen, tags())
            .unwrap()
            .with_headers(vec![("X-Custom".to_string(), "1".to_string())]);

        assert_eq!(response.status_code(), StatusCode::OK);
        assert_eq!(response.error_message(), None);
        let body: serde_json::Value =
            serde_json::from_str(&response.response_body().unwrap()).unwrap();
        assert_eq!(
            body,
            serde_json::json!({"type": "column", "children": [{"type": "text", "data": "hi"}]})
        );
        let headers = response.response_headers().unwrap();
        assert!(headers
            .iter()
            .any(|(k, v)| k == "Content-Type" && v == "application/json"));
        assert!(headers.iter().any(|(k, _)| k == "X-Custom"));
        assert!(response
            .tags()
            .values()
            .iter()
            .any(|(k, v)| k == "screen" && v == "home"));
    }

    #[test]
    fn keeps_explicit_content_type() {
        let response = SduiResponse::try_new(StatusCode::OK, Widget::from(text("a")), tags())
            .unwrap()
            .with_headers(vec![("content-type".to_string(), "text/plain".to_string())]);
        let headers = response.response_headers().unwrap();
        assert_eq!(headers.len(), 1);
    }

    #[test]
    fn invalid_screen_is_an_error_not_a_panic() {
        let screen = Widget::from(badge().count(-1));
        let error = SduiResponse::try_new(StatusCode::OK, screen, tags())
            .err()
            .expect("invalid screen must be rejected");
        assert_eq!(error.status_code(), StatusCode::INTERNAL_SERVER_ERROR);
        assert!(error.error_message().unwrap().contains("$.count"));
    }

    struct FailsToSerialize;

    impl serde::Serialize for FailsToSerialize {
        fn serialize<S: Serializer>(&self, _: S) -> Result<S::Ok, S::Error> {
            Err(S::Error::custom("boom"))
        }
    }

    impl Validate for FailsToSerialize {
        fn validate_at(&self, _: &Path<'_>, _: &mut Vec<ValidationIssue>) {}
    }

    #[test]
    fn serialization_failure_is_an_error_not_a_panic() {
        let error = SduiResponse::try_new(StatusCode::OK, FailsToSerialize, tags())
            .err()
            .expect("serialization failure must be reported");
        assert_eq!(error.status_code(), StatusCode::INTERNAL_SERVER_ERROR);
        assert!(error.error_message().unwrap().contains("boom"));
    }

    #[tokio::test]
    async fn into_response_carries_body_headers_and_tags() {
        let response = SduiResponse::try_new(StatusCode::OK, Widget::from(text("a")), tags())
            .unwrap()
            .into_response();
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(
            response.headers().get("content-type").unwrap(),
            "application/json"
        );
        assert!(response.extensions().get::<DerustTags>().is_some());
        let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        assert_eq!(&bytes[..], br#"{"type":"text","data":"a"}"#);
    }
}
