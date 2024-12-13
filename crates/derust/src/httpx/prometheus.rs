use crate::httpx::text::TextResponse;
use crate::httpx::{AppContext, HttpError, HttpTags};
use axum::extract::State;
use axum::http::StatusCode;

pub const PROMETHEUS_METRICS_PATH: &str = "/metrics";

pub async fn route<S>(State(context): State<AppContext<S>>) -> Result<TextResponse, HttpError>
where
    S: Clone,
{
    let metrics = context.prometheus_handle().render();

    Ok(TextResponse::new(
        StatusCode::OK,
        metrics,
        HttpTags::default(),
    ))
}
