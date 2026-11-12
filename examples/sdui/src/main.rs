mod screen;

use axum::http::StatusCode;
use axum::routing::get;
use axum::Router;
use derust::commonx::{DerustError, DerustTags};
use derust::envx::Environment;
use derust::httpx::sdui::SduiResponse;
use derust::httpx::{start, AppContext};
use derust::sduix::stacx::v1::prelude::*;

#[derive(Clone)]
pub struct AppState;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let env = Environment::detect().ok().unwrap_or(Environment::Local);
    let context = AppContext::new("sdui-sample", env, AppState)?;

    // routes are defined by the application; derust only adds /health
    let router = Router::new()
        .route("/screens/home", get(home))
        .route("/screens/broken", get(broken));

    start(3000, context, router, false).await
}

async fn home() -> Result<SduiResponse<Widget>, DerustError> {
    let tags = DerustTags::from([("screen", "home")]);
    SduiResponse::try_new(StatusCode::OK, screen::home_screen(), tags)
}

/// A screen that violates a Stac constraint: `SduiResponse::try_new` returns a `DerustError`
/// (HTTP 500) instead of sending a broken screen.
async fn broken() -> Result<SduiResponse<Widget>, DerustError> {
    let tags = DerustTags::from([("screen", "broken")]);
    SduiResponse::try_new(StatusCode::OK, Widget::from(badge().count(-1)), tags)
}
