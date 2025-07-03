use crate::httpx::{HttpError, HttpTags};
use axum::{
    extract::{rejection::JsonRejection, FromRequest, Request},
    http::StatusCode,
    response::IntoResponse,
};
use serde_json::{json, Value};

pub async fn handler(JsonRequest(value): JsonRequest<Value>) -> impl IntoResponse {
    JsonRequest(dbg!(value));
}

pub struct JsonRequest<T>(pub T);

impl<S, T> FromRequest<S> for JsonRequest<T>
where
    axum::Json<T>: FromRequest<S, Rejection = JsonRejection>,
    S: Send + Sync,
{
    type Rejection = HttpError;

    async fn from_request(req: Request, state: &S) -> Result<Self, Self::Rejection> {
        let (parts, body) = req.into_parts();

        let req = Request::from_parts(parts, body);

        match axum::Json::<T>::from_request(req, state).await {
            Ok(value) => Ok(Self(value.0)),
            Err(rejection) => Err(HttpError::with_body(
                StatusCode::BAD_REQUEST,
                format!("Invalid JSON: {:?}", rejection),
                json!({
                    "message": rejection.body_text(),
                })
                .to_string(),
                HttpTags::default(),
            )),
        }
    }
}
