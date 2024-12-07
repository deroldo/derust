use crate::metricx::meters::MetricTags;
use metrics::{histogram, Level, LocalRecorderGuard, Recorder};
use metrics_exporter_statsd::StatsdBuilder;
use tracing::info;

const DEFAULT_HOST: &str = "127.0.0.1";
const DEFAULT_PORT: u16 = 8125;
const DEFAULT_QUEUE_SIZE: usize = 5000;
const DEFAULT_BUFFER_SIZE: usize = 256;
const CLIENT_UDP_HOST: &str = "0.0.0.0";

#[derive(Clone)]
pub struct StatsdConfig {
    pub agent_host: String,
    pub agent_port: Option<u16>,
    pub prefix: String,
    pub queue_size: Option<usize>,
    pub buffer_size: Option<usize>,
    pub default_tags: MetricTags,
    pub denied_metric_tags: Vec<String>,
}

pub fn statsd_registry(config: &StatsdConfig) -> Result<(), Box<dyn std::error::Error>> {
    let host = if config.agent_host.is_empty() || config.agent_host == "localhost" {
        DEFAULT_HOST
    } else {
        &config.agent_host
    };

    let port = config.agent_port.unwrap_or(DEFAULT_PORT);

    let mut recorder = StatsdBuilder::from(host.clone(), port)
        .with_queue_size(config.queue_size.unwrap_or(DEFAULT_QUEUE_SIZE))
        .with_buffer_size(config.buffer_size.unwrap_or(DEFAULT_BUFFER_SIZE));

    for metric_tag in config.default_tags.vec() {
        recorder = recorder.with_default_tag(metric_tag.key(), metric_tag.value());
    }

    let recorder = recorder.build(None).map_err(|error| Box::new(error))?;

    let key = metrics::Key::from_static_name("any");
    let h = recorder.register_histogram(&key, &metrics::Metadata::new("any", Level::INFO, None));
    h.record(1.0);

    let _ = metrics::set_global_recorder(recorder);

    info!("StatsD registry configured on {}:{}", host, port);

    Ok(())
}
