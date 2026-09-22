use metrics_exporter_prometheus::{PrometheusBuilder, PrometheusHandle};
use regex::Regex;

#[derive(Clone)]
pub struct PrometheusConfig {
    pub denied_metric_tags: Vec<String>,
    pub denied_metric_tags_by_regex: Vec<Regex>,
}

pub fn prometheus_registry() -> Result<PrometheusHandle, Box<dyn std::error::Error>> {
    let builder = PrometheusBuilder::new()
        .set_buckets(&crate::metricx::HISTOGRAM_BUCKET_BOUNDARIES)
        .map_err(|error| Box::new(error))?;

    let handler = builder
        .install_recorder()
        .map_err(|error| Box::new(error))?;

    Ok(handler)
}
