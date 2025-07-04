use crate::httpx::{HttpError, HttpTags};
use axum::http::StatusCode;
use serde_json::Value;
use std::time::Duration;
use growthbook_rust_sdk::client::GrowthBookClient;
use growthbook_rust_sdk::model_public::GrowthBookAttribute;

pub struct GrowthBookConfig {
    pub growth_book_url: String,
    pub sdk_key: String,
    pub update_interval: Option<Duration>,
    pub http_timeout: Option<Duration>,
}

pub async fn initialize(
    config: &GrowthBookConfig,
) -> Result<GrowthBookClient, Box<dyn std::error::Error>> {
    GrowthBookClient::new(
        &config.growth_book_url,
        &config.sdk_key,
        config.update_interval,
        config.http_timeout,
    )
    .await
    .map_err(|error| Box::new(error) as Box<dyn std::error::Error>)
}

pub fn growth_book_attributes(
    value: Value,
    tags: &HttpTags,
) -> Result<Vec<GrowthBookAttribute>, HttpError> {
    GrowthBookAttribute::from(value).map_err(|error| {
        HttpError::without_body(
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to parse growth book_attributes: {error}"),
            tags.clone(),
        )
    })
}
