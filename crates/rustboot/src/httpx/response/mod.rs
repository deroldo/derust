use crate::httpx::Tags;
use axum::http::StatusCode;

pub mod error;
pub mod json;

pub trait HttpResponse: Send + Sync {
    fn status_code(&self) -> StatusCode;
    fn error_message(&self) -> Option<String>;
    fn response_body(&self) -> Option<String>;
    fn response_headers(&self) -> Option<Vec<(String, String)>>;
    fn tags(&self) -> Tags;
}
