use crate::envx::Environment;

#[cfg(any(feature = "postgres", feature = "outbox"))]
use crate::databasex::PostgresDatabase;
#[cfg(feature = "prometheus")]
use crate::metricx::{prometheus_registry, PrometheusConfig};
#[cfg(feature = "statsd")]
use crate::metricx::{statsd_registry, StatsdConfig};
#[cfg(feature = "growthbook")]
use growthbook_rust_sdk::client::GrowthBookClient;
#[cfg(feature = "prometheus")]
use metrics_exporter_prometheus::PrometheusHandle;
use regex::Regex;

#[derive(Clone)]
pub struct AppContext<S>
where
    S: Clone,
{
    app_name: String,
    env: Environment,
    #[cfg(any(feature = "postgres", feature = "outbox"))]
    database: PostgresDatabase,
    #[cfg(any(feature = "statsd", feature = "prometheus"))]
    denied_metric_tags: Vec<String>,
    #[cfg(any(feature = "statsd", feature = "prometheus"))]
    denied_metric_tags_by_regex: Vec<Regex>,
    #[cfg(feature = "prometheus")]
    prometheus_handle: PrometheusHandle,
    ignore_log_for_paths: Vec<String>,
    #[cfg(feature = "growthbook")]
    growth_book: GrowthBookClient,
    state: S,
}

impl<S> AppContext<S>
where
    S: Clone,
{
    pub fn new(
        app_name: &str,
        env: Environment,
        #[cfg(any(feature = "postgres", feature = "outbox"))] database: PostgresDatabase,
        #[cfg(feature = "statsd")] statsd_config: StatsdConfig,
        #[cfg(feature = "prometheus")] prometheus_config: PrometheusConfig,
        #[cfg(feature = "growthbook")] growth_book: GrowthBookClient,
        state: S,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        #[cfg(feature = "statsd")]
        statsd_registry(&statsd_config)?;
        #[cfg(feature = "statsd")]
        let denied_metric_tags = statsd_config.denied_metric_tags;
        #[cfg(feature = "statsd")]
        let denied_metric_tags_by_regex = statsd_config.denied_metric_tags_by_regex;

        #[cfg(feature = "prometheus")]
        let prometheus_handle = prometheus_registry()?;
        #[cfg(feature = "prometheus")]
        let denied_metric_tags = prometheus_config.denied_metric_tags;
        #[cfg(feature = "prometheus")]
        let denied_metric_tags_by_regex = prometheus_config.denied_metric_tags_by_regex;

        Ok(Self {
            app_name: app_name.to_string(),
            env,
            #[cfg(any(feature = "postgres", feature = "outbox"))]
            database,
            #[cfg(any(feature = "statsd", feature = "prometheus"))]
            denied_metric_tags,
            #[cfg(any(feature = "statsd", feature = "prometheus"))]
            denied_metric_tags_by_regex,
            #[cfg(feature = "prometheus")]
            prometheus_handle,
            ignore_log_for_paths: vec!["/metrics".to_string()],
            #[cfg(feature = "growthbook")]
            growth_book,
            state,
        })
    }

    pub fn app_name(&self) -> &String {
        &self.app_name
    }

    pub fn with_ignore_log_for_paths(mut self, paths: Vec<String>) -> Self {
        self.ignore_log_for_paths = paths;
        self
    }

    pub fn ignore_log_for_paths(&self) -> &Vec<String> {
        &self.ignore_log_for_paths
    }

    pub fn env(&self) -> &Environment {
        &self.env
    }

    pub fn state(&self) -> &S {
        &self.state
    }

    #[cfg(any(feature = "postgres", feature = "outbox"))]
    pub fn database(&self) -> &PostgresDatabase {
        &self.database
    }

    #[cfg(any(feature = "statsd", feature = "prometheus"))]
    pub fn denied_metric_tags(&self) -> &[String] {
        &self.denied_metric_tags
    }

    #[cfg(any(feature = "statsd", feature = "prometheus"))]
    pub fn denied_metric_tags_by_regex(&self) -> &[Regex] {
        &self.denied_metric_tags_by_regex
    }

    #[cfg(feature = "prometheus")]
    pub fn prometheus_handle(&self) -> &PrometheusHandle {
        &self.prometheus_handle
    }

    #[cfg(feature = "growthbook")]
    pub fn growth_book(&self) -> &GrowthBookClient {
        &self.growth_book
    }
}
