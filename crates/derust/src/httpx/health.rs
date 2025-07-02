use std::error::Error;
use std::time::Duration;
use crate::httpx::json::JsonResponse;
use crate::httpx::{AppContext, HttpError, HttpTags};
use axum::extract::State;
use axum::http::StatusCode;
use opentelemetry_otlp::WithExportConfig;
use serde::Serialize;
use tokio::time::timeout;

pub const HEALTH_PATH: &str = "/health";

#[derive(Serialize, PartialEq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum HealthStatus {
    Ok,
    Failure,
}

impl HealthStatus {
    fn is_ok(&self) -> bool {
        self == &HealthStatus::Ok
    }
}

#[derive(Serialize)]
pub struct HealthResponseDto {
    status: HealthStatus,
    #[cfg(feature = "postgres")]
    database: DatabaseHealthResponseDto,
}

impl HealthResponseDto {
    fn new(
        #[cfg(feature = "postgres")]
        database: DatabaseHealthResponseDto,
    ) -> Self {
        let mut status = HealthStatus::Ok;

        #[cfg(feature = "postgres")]
        if !database.status.is_ok() {
            status = HealthStatus::Failure;
        }

        Self {
            status,
            #[cfg(feature = "postgres")]
            database,
        }
    }
}

#[cfg(feature = "postgres")]
#[derive(Serialize)]
pub struct DatabaseHealthResponseDto {
    status: HealthStatus,
}

pub async fn route<S>(
    State(state): State<AppContext<S>>,
) -> Result<JsonResponse<HealthResponseDto>, HttpError>
where
    S: Clone + Send + Sync + 'static,
{
    #[cfg(feature = "postgres")]
    let database = {
        let result = timeout(
            Duration::from_millis(100),
            sqlx::query_as::<_, (i32,)>("SELECT 1").fetch_one(&state.database().read_write)
        ).await;

        let database_status = if result.is_ok() {
            HealthStatus::Ok
        } else {
            HealthStatus::Failure
        };

        DatabaseHealthResponseDto {
            status: database_status,
        }
    };

    let response = HealthResponseDto::new(
        #[cfg(feature = "postgres")]
        database,
    );

    let http_status = if response.status.is_ok() {
        StatusCode::OK
    } else {
        StatusCode::SERVICE_UNAVAILABLE
    };

    Ok(JsonResponse::new(http_status, response, HttpTags::default()))
}
