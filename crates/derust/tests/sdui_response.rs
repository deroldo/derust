#![cfg(feature = "sdui_stac_1")]

use axum::body::Body;
use axum::http::{Request, StatusCode};
use axum::routing::get;
use axum::Router;
use derust::commonx::{DerustError, DerustTags};
use derust::httpx::sdui::SduiResponse;
use derust::sduix::stacx::v1::prelude::*;
use tower::ServiceExt;

async fn home() -> Result<SduiResponse<Widget>, DerustError> {
    let tags = DerustTags::from([("screen", "home")]);
    let screen = Widget::from(column().children([Widget::from(text(StateRef::new("greeting")))]));
    SduiResponse::try_new(StatusCode::OK, screen, tags)
}

async fn broken() -> Result<SduiResponse<Widget>, DerustError> {
    let tags = DerustTags::from([("screen", "broken")]);
    SduiResponse::try_new(StatusCode::OK, Widget::from(badge().count(-1)), tags)
}

fn app() -> Router {
    Router::new()
        .route("/home", get(home))
        .route("/broken", get(broken))
}

#[tokio::test]
async fn handler_returns_screen_json_with_tags() {
    let response = app()
        .oneshot(Request::get("/home").body(Body::empty()).unwrap())
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        response.headers().get("content-type").unwrap(),
        "application/json"
    );
    let tags = response.extensions().get::<DerustTags>().unwrap().clone();
    assert!(tags
        .values()
        .iter()
        .any(|(key, value)| key == "screen" && value == "home"));

    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let body: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(
        body,
        serde_json::json!({
            "type": "column",
            "children": [{"type": "text", "data": "{{greeting}}"}]
        })
    );
}

#[tokio::test]
async fn invalid_screen_follows_the_error_path() {
    let response = app()
        .oneshot(Request::get("/broken").body(Body::empty()).unwrap())
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
    assert!(response.extensions().get::<DerustTags>().is_some());
}
