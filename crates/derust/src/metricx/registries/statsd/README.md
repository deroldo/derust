# derust - statsd

Automatic HTTP requests duration metrics as `http_server_seconds`

Automatic duration metrics for features:
- `http_client` as `http_client_seconds`
- `postgres` as:
  - `repository_transaction_seconds`
  - `repository_query_seconds`

## [Example](https://github.com/deroldo/derust/tree/main/examples/metrics/statsd)

```toml
# Cargo.toml

[package]
# ...

[dependencies]
derust = { version = "0.1.0", features = ["statsd"] }

# ...
```

```rust
// main.rs

// ...
use derust::metricx::{current_gauge, increment, increment_one, record_duration, record_money, start_stopwatch, MetricTags, StatsdConfig};
// ...

#[derive(Clone)]
pub struct AppState {}


#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // ...

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
        denied_metric_tags: vec!["customer_id".to_string()], // any high cardinality http tags (log tags)
    };

    // easy way to get application context things, like your application state struct
    let context = AppContext::new(application_name, env, statsd_config, app_state)?;

    // start as the basic 
    // ... 
}
```

```rust
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
```