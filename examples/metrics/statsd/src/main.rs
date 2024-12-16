use axum::extract::State;
use axum::http::StatusCode;
use axum::routing::get;
use axum::Router;
use derust::envx::Environment;
use derust::httpx::json::JsonResponse;
use derust::httpx::{start, AppContext, HttpError, HttpTags};
use derust::metricx::{current_gauge, increment, increment_one, record_duration, record_money, start_stopwatch, MetricTags, StatsdConfig};

#[derive(Clone)]
pub struct AppState {}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let env = Environment::detect().ok().unwrap_or(Environment::Local);

    // any cloneable struct
    let app_state = AppState {};

    let application_name = "sample";

    // statsd
    let statsd_config = StatsdConfig {
        agent_host: "localhost".to_string(),
        agent_port: Some(9125),
        queue_size: None,
        buffer_size: None,
        default_tags: MetricTags::default(), // any low cardinality key value - app_name and env already are default
        denied_metric_tags: vec!["customer".to_string()], // any high cardinality http tags (log tags)
        denied_metric_tags_by_regex: vec![Regex::new(".+_id$").unwrap()], // any high cardinality http tags regex (log tags)
    };

    // easy way to get application context things, like your application state struct
    let context = AppContext::new(application_name, env, statsd_config, app_state)?;

    let port = 3000;
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
    let tags = HttpTags::from([
        ("kind", "foo"),
        ("customer_id", "1"), // will be ignored with that configuration by `denied_metric_tags`
    ]);

    increment(&context, "counter_metric_name", MetricTags::from(&tags), 10);
    increment_one(&context, "counter_metric_name", MetricTags::from(&tags));
    current_gauge(&context, "gauge_metric_name", MetricTags::from(&tags), 100.0);
    record_money(&context, "money_metric_name", MetricTags::from(&tags), 100.0);
    record_duration(&context, "duration_metric_name", MetricTags::from(&tags), 100.0);

    let stopwatch = start_stopwatch(&context,"duration_metric_name", MetricTags::from(&tags));
    // ...
    stopwatch.record(MetricTags::from(&tags));

    Ok(JsonResponse::new(
        StatusCode::OK,
        FooResponse { foo: "".to_string() },
        HttpTags::default(),
    ))
}
