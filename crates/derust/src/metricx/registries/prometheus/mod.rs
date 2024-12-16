use metrics_exporter_prometheus::{PrometheusBuilder, PrometheusHandle};
use regex::Regex;

#[derive(Clone)]
pub struct PrometheusConfig {
    pub denied_metric_tags: Vec<String>,
    pub denied_metric_tags_by_regex: Vec<Regex>,
}

pub fn prometheus_registry() -> Result<PrometheusHandle, Box<dyn std::error::Error>> {
    let builder = PrometheusBuilder::new()
        .set_buckets(&[
            0.01, 0.25, 0.05, 0.75, 0.1, 0.15, 0.25, 0.5, 1.0, 2.5, 5.0, 10.0,
        ])
        .map_err(|error| Box::new(error))?;

    let handler = builder
        .install_recorder()
        .map_err(|error| Box::new(error))?;

    Ok(handler)
}
