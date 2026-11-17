#![cfg(all(feature = "sdui_stac_1", feature = "http_server"))]

use axum::body::Body;
use axum::http::{Request, StatusCode};
use axum::routing::get;
use axum::Router;
use derust::commonx::{DerustError, DerustTags};
use derust::httpx::stac_error::StacErrorResponse;
use derust::sduix::stacx::v1::prelude::*;
use tower::ServiceExt;

async fn expired() -> Result<StacErrorResponse, DerustError> {
    StacErrorResponse::new(StatusCode::UNAUTHORIZED)
        .code("TOKEN_EXPIRED")
        .tags(DerustTags::from([("route", "expired")]))
        .recovery(Recovery::new().recover(Action::from(set_state("refreshing", true))))
        .build()
}

async fn invalid() -> Result<StacErrorResponse, DerustError> {
    StacErrorResponse::new(StatusCode::OK).build()
}

async fn from_error() -> Result<StacErrorResponse, DerustError> {
    let error = DerustError::without_body(
        StatusCode::FORBIDDEN,
        "forbidden".to_string(),
        DerustTags::from([("route", "forbidden")]),
    );
    StacErrorResponse::from_derust_error(error)
        .screen(Widget::from(text("no access")))
        .build()
}

fn app() -> Router {
    Router::new()
        .route("/expired", get(expired))
        .route("/invalid", get(invalid))
        .route("/forbidden", get(from_error))
}

async fn call(path: &str) -> axum::response::Response {
    app()
        .oneshot(Request::get(path).body(Body::empty()).unwrap())
        .await
        .unwrap()
}

#[tokio::test]
async fn handler_returns_the_error_envelope() {
    let response = call("/expired").await;
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    assert_eq!(
        response.headers().get("content-type").unwrap(),
        "application/vnd.stac.error+json"
    );
    assert_eq!(response.headers().get("cache-control").unwrap(), "no-store");
    let tags = response.extensions().get::<DerustTags>().unwrap().clone();
    assert!(tags
        .values()
        .iter()
        .any(|(k, v)| k == "route" && v == "expired"));
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let value: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(value["version"], 1);
    assert_eq!(value["code"], "TOKEN_EXPIRED");
    assert_eq!(value["recovery"]["recover"]["actionType"], "setState");
}

#[tokio::test]
async fn invalid_status_is_an_internal_error() {
    let response = call("/invalid").await;
    assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
}

#[tokio::test]
async fn converted_derust_error_keeps_status_and_tags() {
    let response = call("/forbidden").await;
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
    let tags = response.extensions().get::<DerustTags>().unwrap().clone();
    assert!(tags
        .values()
        .iter()
        .any(|(k, v)| k == "route" && v == "forbidden"));
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let value: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(value["message"], "forbidden");
    assert_eq!(value["screen"]["data"], "no access");
}
