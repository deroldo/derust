#![cfg(feature = "sdui_stac_1")]

use axum::body::Body;
use axum::http::{Request, StatusCode};
use axum::routing::get;
use axum::Router;
use derust::commonx::{DerustError, DerustTags};
use derust::httpx::sdui::SduiResponse;
use derust::sduix::stacx::common::ValidationReport;
use derust::sduix::stacx::v1::prelude::*;
use serde::Serialize;
use tower::ServiceExt;

/// A widget of a custom parser registered in the Flutter app.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct StateText {
    state_key: String,
}

impl Validate for StateText {
    fn validate_at(&self, path: &Path<'_>, issues: &mut Vec<ValidationIssue>) {
        if self.state_key.is_empty() {
            issues.push(ValidationIssue::new(
                &path.key("stateKey"),
                "must not be empty",
            ));
        }
    }
}

impl CustomWidget for StateText {
    const TYPE_NAME: &'static str = "stateText";
}

async fn home() -> Result<SduiResponse<Widget>, DerustError> {
    let tags = DerustTags::from([("screen", "home")]);
    let screen = Widget::from(column().children([Widget::from(text(StateRef::new("greeting")))]));
    SduiResponse::try_new(StatusCode::OK, screen, tags)
}

async fn broken() -> Result<SduiResponse<Widget>, DerustError> {
    let tags = DerustTags::from([("screen", "broken")]);
    SduiResponse::try_new(StatusCode::OK, Widget::from(badge().count(-1)), tags)
}

async fn custom(state_key: String) -> Result<SduiResponse<Widget>, DerustError> {
    let tags = DerustTags::from([("screen", "custom")]);
    let screen = Widget::from(column().children([Widget::from(StateText { state_key })]));
    SduiResponse::try_new(StatusCode::OK, screen, tags)
}

fn app() -> Router {
    Router::new()
        .route("/home", get(home))
        .route("/broken", get(broken))
        .route("/custom", get(|| custom("user.name".into())))
        .route("/custom-broken", get(|| custom(String::new())))
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

#[tokio::test]
async fn custom_widgets_are_served_and_validated() {
    let response = app()
        .oneshot(Request::get("/custom").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let body: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(
        body,
        serde_json::json!({
            "type": "column",
            "children": [{"type": "stateText", "stateKey": "user.name"}]
        })
    );

    let response = app()
        .oneshot(Request::get("/custom-broken").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
}

async fn reactive() -> Result<SduiResponse<Widget>, DerustError> {
    let tags = DerustTags::from([("screen", "reactive")]);
    let screen = Widget::from(
        column()
            .state(StateBlock::new().with("count", 0))
            .children([Widget::from(
                elevated_button().on_pressed(increment_state("count")),
            )]),
    );
    SduiResponse::try_new(StatusCode::OK, screen, tags)
}

async fn reactive_broken() -> Result<SduiResponse<Widget>, DerustError> {
    let tags = DerustTags::from([("screen", "reactive-broken")]);
    let screen = Widget::from(
        column()
            .state(StateBlock::new().with("count", 0))
            .children([Widget::from(
                elevated_button().on_pressed(increment_state("missing")),
            )]),
    );
    SduiResponse::try_new(StatusCode::OK, screen, tags)
}

fn reactive_app() -> Router {
    Router::new()
        .route("/reactive", get(reactive))
        .route("/reactive-broken", get(reactive_broken))
}

#[tokio::test]
async fn reactive_screen_is_served() {
    let response = reactive_app()
        .oneshot(Request::get("/reactive").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let tags = response.extensions().get::<DerustTags>().unwrap().clone();
    assert!(tags
        .values()
        .iter()
        .any(|(key, value)| key == "screen" && value == "reactive"));
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let body: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(
        body,
        serde_json::json!({
            "type": "column",
            "state": {"count": 0},
            "children": [{"type": "elevatedButton", "onPressed": {
                "actionType": "incrementState", "key": "count"}}]
        })
    );
}

#[tokio::test]
async fn reactive_context_error_follows_the_error_path_with_tags() {
    let response = reactive_app()
        .oneshot(
            Request::get("/reactive-broken")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
    let tags = response.extensions().get::<DerustTags>().unwrap().clone();
    assert!(tags
        .values()
        .iter()
        .any(|(key, value)| key == "screen" && value == "reactive-broken"));
}

/// Cost of the extra `Value` pass for a large screen (2000 nodes); printed with `--nocapture`.
#[test]
fn large_screen_is_checked() {
    let children: Vec<Widget> = (0..2000)
        .map(|i| Widget::from(text(format!("row {i}"))))
        .collect();
    let screen = Widget::from(column().children(children));

    let started = std::time::Instant::now();
    let value = serde_json::to_value(&screen).unwrap();
    let to_value = started.elapsed();
    let started = std::time::Instant::now();
    let mut report = ValidationReport::new();
    screen.validate_serialized(&value, &mut report);
    let checked = started.elapsed();
    let started = std::time::Instant::now();
    let _ = serde_json::to_string(&screen).unwrap();
    let direct = started.elapsed();
    assert!(report.is_ok());
    println!("2000 nodes: to_value {to_value:?}, check {checked:?}, direct to_string {direct:?}");

    let response = SduiResponse::try_new(StatusCode::OK, screen, DerustTags::from([("a", "b")]));
    assert!(response.is_ok());
}
