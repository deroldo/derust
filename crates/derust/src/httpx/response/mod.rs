use crate::commonx::DerustTags;
use axum::http::StatusCode;

pub mod json;
pub mod text;

pub trait HttpResponse: Send + Sync {
    fn status_code(&self) -> StatusCode;
    fn error_message(&self) -> Option<String>;
    fn response_body(&self) -> Option<String>;
    fn response_headers(&self) -> Option<Vec<(String, String)>>;
    fn tags(&self) -> DerustTags;
}
