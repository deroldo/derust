use crate::commonx::{DerustError, DerustTags};
use crate::httpx::text::TextResponse;
use crate::httpx::AppContext;
use axum::extract::State;
use axum::http::StatusCode;

pub const PROMETHEUS_METRICS_PATH: &str = "/metrics";

pub async fn route<S>(State(context): State<AppContext<S>>) -> Result<TextResponse, DerustError>
where
    S: Clone,
{
    let metrics = context.prometheus_handle().render();

    Ok(TextResponse::new(
        StatusCode::OK,
        metrics,
        DerustTags::default(),
    ))
}
