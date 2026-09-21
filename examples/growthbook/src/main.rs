use axum::extract::State;
use axum::http::StatusCode;
use axum::routing::get;
use axum::Router;
use derust::envx::Environment;
use derust::growthbookx::{GrowthBookAttribute, GrowthBookClient, GrowthBookClientTrait};
use derust::httpx::json::JsonResponse;
use derust::httpx::{start, AppContext, HttpError, HttpTags};
use rand::Rng;
use serde_json::json;

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
    // API 100% nativa da growthbook-rust (SDK oficial): sem casca própria do derust.
    let growthbook = GrowthBookClient::new(
        "http://localhost:3100",
        // change it with your created sdk-key
        "sdk-key",
        None, // update_interval
        None, // http_timeout
    )
    .await
    .map_err(|error| Box::new(error) as Box<dyn std::error::Error>)?;

    let application_name = "sample";

    // easy way to get application context things, like your application state struct
    let context = AppContext::new(application_name, env, growthbook, app_state)?;

    let port = 3001;
    let router = Router::new().nest("/foo", Router::new().route("/", get(handler)));

    // automatic health-check route
    // automatic route response status code log
    start(port, context, router, false).await
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

    // creating growthbook attributes to match conditions, direto da SDK nativa
    let attrs = GrowthBookAttribute::from(json!({
        "pair": pair,
    }))
    .map_err(|error| {
        HttpError::without_body(
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to parse growth book attributes: {error}"),
            tags.clone(),
        )
    })?;

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
