use axum::http::StatusCode;

pub const HEALTH_PATH: &str = "/health";

pub fn route() -> StatusCode {
    StatusCode::OK
}
