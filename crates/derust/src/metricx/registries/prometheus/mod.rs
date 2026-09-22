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

    let recorder = builder.build_recorder();
    let handle = recorder.handle();

    metrics::set_global_recorder(crate::metricx::otel_bridge::OtelBridgingRecorder::new(
        recorder,
    ))
    .map_err(|error| Box::new(error))?;

    Ok(handle)
}
