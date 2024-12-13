use axum::extract::State;
use axum::http::StatusCode;
use axum::routing::get;
use axum::Router;
use rand::Rng;
use serde_json::json;
use derust::envx::Environment;
use derust::growthbookx;
use derust::growthbookx::{growth_book_attributes, GrowthBookConfig};
use derust::httpx::json::JsonResponse;
use derust::httpx::{start, AppContext, HttpError, HttpTags};

#[derive(Clone)]
pub struct AppState {
    pub bar_v1: String,
    pub bar_v2: String,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let env = Environment::detect().ok().unwrap_or(Environment::Local);

    // any cloneable struct
    let app_state = AppState {
        bar_v1: "bar".to_string(),
        bar_v2: "foobar".to_string(),
    };

    // required to access growthbook admin dashboard to create the sdk-key: http://localhost:3000
    let gb_config = GrowthBookConfig {
        growth_book_url: "http://localhost:3100".to_string(),
        sdk_key: "sdk-key".to_string(),
        update_interval: None,
        http_timeout: None,
    };
    let growthbook = growthbookx::initialize(&gb_config).await?;

    let application_name = "sample";

    // easy way to get application context things, like your application state struct
    let context = AppContext::new(application_name, env, growthbook, app_state)?;

    let port = 3001;
    let router = Router::new().nest("/foo", Router::new().route("/", get(handler)));

    // automatic health-check route
    // automatic route response status code log
    start(port, context, router).await
}

#[derive(serde::Serialize)]
pub struct FooResponse {
    pub foo: String,
}

async fn handler(
    State(context): State<AppContext<AppState>>,
) -> Result<JsonResponse<FooResponse>, HttpError> {
    let tags = HttpTags::default();

    let pair = rand::thread_rng().gen_range(0..100) % 2 == 0;

    // creating growhtbook attributes to match conditions
    let attrs = growth_book_attributes(json!({
        "pair": pair,
    }), &tags)?;

    // boolean condition
    // can you also get `feature_result` and parse to String or your struct type
    let bar = if context.growth_book().is_on("test", Some(attrs)) {
        context.state().bar_v1.clone()
    } else {
        context.state().bar_v2.clone()
    };

    Ok(JsonResponse::new(
        StatusCode::OK,
        FooResponse { foo: bar },
        tags,
    ))
}
