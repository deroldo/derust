use crate::commonx::{DerustError, DerustTags};
use crate::httpx::HttpResponse;
use crate::sduix::stacx::common::{Validate, ValidationIssue, ValidationReport};
use crate::sduix::stacx::v1::state::Recovery;
use axum::http::{HeaderName, HeaderValue, StatusCode};
use axum_tracing_opentelemetry::tracing_opentelemetry_instrumentation_sdk::find_current_trace_id;
use serde_json::{Map, Value};
use std::str::FromStr;

/// `Content-Type` of the Stac error envelope.
pub const STAC_ERROR_CONTENT_TYPE: &str = "application/vnd.stac.error+json";

/// Version of the envelope (constant, not configurable).
const ENVELOPE_VERSION: u8 = 1;

/// Stac error response: a 4xx/5xx status with the envelope
/// `{version, code?, message?, screen?, action?, recovery?}` and
/// `Content-Type: application/vnd.stac.error+json`.
///
/// Build it with [`StacErrorResponse::new`] (a builder) and finish with
/// [`StacErrorResponseBuilder::build`]. All validation happens once, there: `screen` gets the
/// full check (typed + serialized, it is rendered as a screen of its own); `action` and
/// `recovery` only the typed check (they run inside the app screen's state, which the envelope
/// does not declare). Any problem yields a [`DerustError`] (HTTP 500) without a partial body;
/// messages carry paths, never values.
///
/// The derust knows nothing about tokens or sessions: the handler decides whether a failure
/// is recoverable and builds the [`Recovery`].
#[derive(Clone, Debug)]
pub struct StacErrorResponse {
    status_code: StatusCode,
    body: String,
    response_headers: Vec<(String, String)>,
    tags: DerustTags,
}

/// Builder of a [`StacErrorResponse`]; every setter is infallible.
#[derive(Debug)]
pub struct StacErrorResponseBuilder {
    status_code: StatusCode,
    code: Option<String>,
    message: Option<String>,
    screen: Option<Value>,
    action: Option<Value>,
    recovery: Option<Value>,
    response_headers: Vec<(String, String)>,
    tags: DerustTags,
    issues: Vec<ValidationIssue>,
}

fn issue(path: &str, message: impl Into<String>) -> ValidationIssue {
    ValidationIssue {
        path: path.to_string(),
        message: message.into(),
    }
}

fn prefixed(
    prefix: &str,
    issues: impl IntoIterator<Item = ValidationIssue>,
) -> Vec<ValidationIssue> {
    issues
        .into_iter()
        .map(|i| ValidationIssue {
            path: format!("{prefix}{}", i.path.trim_start_matches('$')),
            message: i.message,
        })
        .collect()
}

impl StacErrorResponse {
    /// Starts an error response with `status_code` (must be 4xx or 5xx).
    #[allow(clippy::new_ret_no_self)]
    pub fn new(status_code: StatusCode) -> StacErrorResponseBuilder {
        StacErrorResponseBuilder {
            status_code,
            code: None,
            message: None,
            screen: None,
            action: None,
            recovery: None,
            response_headers: Vec::new(),
            tags: DerustTags::default(),
            issues: Vec::new(),
        }
    }

    /// Same as [`StacErrorResponseBuilder::build`].
    pub fn try_new(builder: StacErrorResponseBuilder) -> Result<Self, DerustError> {
        builder.build()
    }

    /// Starts from a [`DerustError`]: keeps its status, `error_message` (as `message`) and tags.
    /// The old body and headers of the error are not propagated.
    pub fn from_derust_error(error: DerustError) -> StacErrorResponseBuilder {
        let mut builder = Self::new(error.status_code()).tags(error.tags());
        builder.message = error.error_message();
        builder
    }
}

impl StacErrorResponseBuilder {
    /// Machine-readable error code.
    pub fn code(mut self, code: impl Into<String>) -> Self {
        self.code = Some(code.into());
        self
    }

    /// Human-readable message.
    pub fn message(mut self, message: impl Into<String>) -> Self {
        self.message = Some(message.into());
        self
    }

    /// Screen rendered by the client for this error (fully validated).
    pub fn screen<T: serde::Serialize + Validate>(mut self, screen: T) -> Self {
        let issues = screen.validate();
        if !issues.is_empty() {
            self.issues.extend(prefixed("$.screen", issues));
            return self;
        }
        match serde_json::to_value(&screen) {
            Ok(value) => {
                let mut report = ValidationReport::new();
                screen.validate_serialized(&value, &mut report);
                self.issues.extend(prefixed("$.screen", report.errors));
                for warning in &report.warnings {
                    tracing::warn!(
                        path = %warning.path,
                        message = %warning.message,
                        "Stac error screen warning"
                    );
                }
                self.screen = Some(value);
            }
            Err(error) => self
                .issues
                .push(issue("$.screen", format!("serialization failed: {error}"))),
        }
        self
    }

    /// Action (or list) the client runs after the error callbacks (typed validation only).
    pub fn action<T: serde::Serialize + Validate>(mut self, action: T) -> Self {
        self.action = self.typed_value("$.action", &action);
        self
    }

    /// Recovery instruction (typed validation only).
    pub fn recovery(mut self, recovery: Recovery) -> Self {
        self.recovery = self.typed_value("$.recovery", &recovery);
        self
    }

    /// Tags logged with the response.
    pub fn tags(mut self, tags: DerustTags) -> Self {
        self.tags = tags;
        self
    }

    /// Extra headers. `Content-Type` is always the envelope's; `Cache-Control: no-store` is
    /// added when there is a `recovery` and no `Cache-Control` was given.
    pub fn with_headers(mut self, response_headers: Vec<(String, String)>) -> Self {
        self.response_headers = response_headers;
        self
    }

    fn typed_value<T: serde::Serialize + Validate>(
        &mut self,
        prefix: &str,
        value: &T,
    ) -> Option<Value> {
        let issues = value.validate();
        if !issues.is_empty() {
            self.issues.extend(prefixed(prefix, issues));
            return None;
        }
        match serde_json::to_value(value) {
            Ok(value) => Some(value),
            Err(error) => {
                self.issues
                    .push(issue(prefix, format!("serialization failed: {error}")));
                None
            }
        }
    }

    /// Validates everything and serializes the envelope once.
    pub fn build(self) -> Result<StacErrorResponse, DerustError> {
        let Self {
            status_code,
            code,
            message,
            screen,
            action,
            recovery,
            response_headers,
            tags,
            mut issues,
        } = self;

        if !(status_code.is_client_error() || status_code.is_server_error()) {
            issues.push(issue("$.status", "must be a 4xx or 5xx status"));
        }
        for (name, value) in &response_headers {
            if HeaderName::from_str(name).is_err() || HeaderValue::from_str(value).is_err() {
                issues.push(issue("$.headers", "invalid header name or value"));
            }
        }

        let mut envelope = Map::new();
        envelope.insert("version".into(), Value::from(ENVELOPE_VERSION));
        if let Some(code) = code {
            envelope.insert("code".into(), Value::from(code));
        }
        if let Some(message) = message {
            envelope.insert("message".into(), Value::from(message));
        }
        let has_recovery = recovery.is_some();
        for (key, value) in [
            ("screen", screen),
            ("action", action),
            ("recovery", recovery),
        ] {
            if let Some(value) = value {
                envelope.insert(key.into(), value);
            }
        }

        let body = if issues.is_empty() {
            serde_json::to_string(&Value::Object(envelope))
                .map_err(|error| vec![issue("$", format!("serialization failed: {error}"))])
        } else {
            Err(issues)
        };
        let body = body.map_err(|issues| {
            let details = issues
                .iter()
                .map(|i| i.to_string())
                .collect::<Vec<_>>()
                .join("; ");
            DerustError::without_body(
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("failed to build Stac error response: {details}"),
                tags.clone(),
            )
        })?;

        let mut headers: Vec<(String, String)> = response_headers
            .into_iter()
            .filter(|(name, _)| !name.eq_ignore_ascii_case("content-type"))
            .collect();
        if has_recovery
            && !headers
                .iter()
                .any(|(name, _)| name.eq_ignore_ascii_case("cache-control"))
        {
            headers.push(("Cache-Control".to_string(), "no-store".to_string()));
        }
        headers.push((
            "Content-Type".to_string(),
            STAC_ERROR_CONTENT_TYPE.to_string(),
        ));

        Ok(StacErrorResponse {
            status_code,
            body,
            response_headers: headers,
            tags,
        })
    }
}

impl HttpResponse for StacErrorResponse {
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
        Some(self.response_headers.clone())
    }

    fn tags(&self) -> DerustTags {
        let mut tags = self.tags.clone();

        if !tags
            .values()
            .iter()
            .any(|(key, _)| key.eq_ignore_ascii_case("x-trace-id"))
        {
            if let Some(trace_id) = find_current_trace_id() {
                tags.add("x-trace-id", &trace_id);
            }
        }

        tags
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sduix::stacx::v1::prelude::*;
    use axum::response::IntoResponse;
    use serde_json::json;

    fn tags() -> DerustTags {
        DerustTags::from([("screen", "login")])
    }

    fn body(response: &StacErrorResponse) -> Value {
        serde_json::from_str(&response.response_body().unwrap()).unwrap()
    }

    fn header<'a>(headers: &'a [(String, String)], name: &str) -> Option<&'a str> {
        headers
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case(name))
            .map(|(_, v)| v.as_str())
    }

    fn recovery() -> Recovery {
        Recovery::new().recover(Action::from(set_state("x", 1)))
    }

    #[test]
    fn minimal_envelope() {
        let response = StacErrorResponse::new(StatusCode::NOT_FOUND)
            .build()
            .unwrap();
        assert_eq!(response.status_code(), StatusCode::NOT_FOUND);
        assert_eq!(body(&response), json!({"version": 1}));
        let headers = response.response_headers().unwrap();
        assert_eq!(
            header(&headers, "content-type"),
            Some("application/vnd.stac.error+json")
        );
        assert_eq!(header(&headers, "cache-control"), None);
    }

    #[test]
    fn screen_only_and_action_only() {
        let screen = Widget::from(text("oops"));
        let response = StacErrorResponse::new(StatusCode::BAD_REQUEST)
            .code("E1")
            .message("bad")
            .screen(screen)
            .build()
            .unwrap();
        assert_eq!(
            body(&response),
            json!({"version": 1, "code": "E1", "message": "bad",
                   "screen": {"type": "text", "data": "oops"}})
        );
        let response = StacErrorResponse::new(StatusCode::BAD_REQUEST)
            .action(Action::from(set_state("a", 1)))
            .build()
            .unwrap();
        assert_eq!(
            body(&response),
            json!({"version": 1, "action": {"actionType": "setState", "values": {"a": 1}}})
        );
    }

    #[test]
    fn full_envelope_with_recovery_is_no_store() {
        let response = StacErrorResponse::new(StatusCode::UNAUTHORIZED)
            .code("TOKEN_EXPIRED")
            .recovery(recovery())
            .build()
            .unwrap();
        let value = body(&response);
        assert_eq!(value["recovery"]["recover"]["actionType"], "setState");
        let headers = response.response_headers().unwrap();
        assert_eq!(header(&headers, "cache-control"), Some("no-store"));
    }

    #[test]
    fn cache_control_can_be_overridden_and_content_type_cannot() {
        let response = StacErrorResponse::new(StatusCode::UNAUTHORIZED)
            .recovery(recovery())
            .with_headers(vec![
                ("cache-control".into(), "private".into()),
                ("Content-Type".into(), "text/plain".into()),
                ("X-A".into(), "1".into()),
            ])
            .build()
            .unwrap();
        let headers = response.response_headers().unwrap();
        assert_eq!(header(&headers, "cache-control"), Some("private"));
        assert_eq!(
            header(&headers, "content-type"),
            Some("application/vnd.stac.error+json")
        );
        assert_eq!(header(&headers, "x-a"), Some("1"));
        assert_eq!(headers.len(), 3);
    }

    #[test]
    fn non_error_statuses_are_rejected() {
        for status in [StatusCode::CONTINUE, StatusCode::OK, StatusCode::FOUND] {
            let error = StacErrorResponse::new(status)
                .tags(tags())
                .build()
                .unwrap_err();
            assert_eq!(error.status_code(), StatusCode::INTERNAL_SERVER_ERROR);
            assert!(error.error_message().unwrap().contains("4xx or 5xx"));
            assert!(error.response_body().is_none());
        }
        assert!(StacErrorResponse::new(StatusCode::BAD_GATEWAY)
            .build()
            .is_ok());
    }

    #[test]
    fn invalid_parts_are_a_500_without_body() {
        let error = StacErrorResponse::new(StatusCode::BAD_REQUEST)
            .tags(tags())
            .screen(Widget::from(badge().count(-1)))
            .recovery(Recovery::new())
            .action(Action::from(set_state("a", f64::NAN)))
            .build()
            .unwrap_err();
        assert_eq!(error.status_code(), StatusCode::INTERNAL_SERVER_ERROR);
        let message = error.error_message().unwrap();
        assert!(message.contains("$.screen.count"), "{message}");
        assert!(message.contains("$.recovery.recover"), "{message}");
        assert!(message.contains("$.action"), "{message}");
        assert!(error.response_body().is_none());
        assert!(error
            .tags()
            .values()
            .iter()
            .any(|(k, v)| k == "screen" && v == "login"));
    }

    #[test]
    fn invalid_header_is_a_500() {
        let error = StacErrorResponse::new(StatusCode::BAD_REQUEST)
            .with_headers(vec![("bad name".into(), "x".into())])
            .build()
            .unwrap_err();
        assert_eq!(error.status_code(), StatusCode::INTERNAL_SERVER_ERROR);
    }

    #[test]
    fn from_derust_error_keeps_status_message_and_tags() {
        let derust = DerustError::with_json(
            StatusCode::UNAUTHORIZED,
            "expired".to_string(),
            json!({"secret": "body"}),
            tags(),
        );
        let response = StacErrorResponse::from_derust_error(derust)
            .screen(Widget::from(text("login")))
            .action(Action::from(set_state("a", 1)))
            .build()
            .unwrap();
        assert_eq!(response.status_code(), StatusCode::UNAUTHORIZED);
        let value = body(&response);
        assert_eq!(value["message"], "expired");
        assert_eq!(value["screen"]["data"], "login");
        assert_eq!(value["action"]["actionType"], "setState");
        assert!(!response.response_body().unwrap().contains("secret"));
        assert!(response
            .tags()
            .values()
            .iter()
            .any(|(k, v)| k == "screen" && v == "login"));
    }

    #[test]
    fn from_derust_error_with_a_non_error_status_is_rejected() {
        let derust = DerustError::without_body(StatusCode::OK, "x".to_string(), tags());
        assert!(StacErrorResponse::from_derust_error(derust)
            .build()
            .is_err());
    }

    #[tokio::test]
    async fn into_response_carries_status_body_headers_and_tags() {
        let response = StacErrorResponse::new(StatusCode::UNAUTHORIZED)
            .tags(tags())
            .recovery(recovery())
            .build()
            .unwrap()
            .into_response();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
        assert_eq!(
            response.headers().get("content-type").unwrap(),
            STAC_ERROR_CONTENT_TYPE
        );
        assert_eq!(response.headers().get("cache-control").unwrap(), "no-store");
        assert!(response.extensions().get::<DerustTags>().is_some());
        let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let value: Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(value["version"], 1);
    }
}
