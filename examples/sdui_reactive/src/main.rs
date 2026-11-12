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
    let context = AppContext::new("sdui-reactive-sample", env, AppState)?;

    // routes are defined by the application; derust only adds /health
    let router = Router::new()
        .route("/screens/form", get(form))
        .route("/screens/list", get(list))
        .route("/screens/network", get(network))
        .route("/screens/navigation", get(navigation))
        .route("/screens/invalid", get(invalid));

    start(3000, context, router, false).await
}

fn ok(name: &'static str, screen: Widget) -> Result<SduiResponse<Widget>, DerustError> {
    SduiResponse::try_new(StatusCode::OK, screen, DerustTags::from([("screen", name)]))
}

async fn form() -> Result<SduiResponse<Widget>, DerustError> {
    ok("form", screen::form_screen())
}

async fn list() -> Result<SduiResponse<Widget>, DerustError> {
    ok("list", screen::list_screen())
}

async fn network() -> Result<SduiResponse<Widget>, DerustError> {
    ok("network", screen::network_screen())
}

async fn navigation() -> Result<SduiResponse<Widget>, DerustError> {
    ok("navigation", screen::navigation_screen())
}

/// Fails the contextual validation: `try_new` returns a `DerustError` (HTTP 500).
async fn invalid() -> Result<SduiResponse<Widget>, DerustError> {
    ok("invalid", screen::invalid_screen())
}
