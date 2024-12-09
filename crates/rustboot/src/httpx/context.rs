use crate::envx::Environment;

#[cfg(any(feature = "postgres", feature = "outbox"))]
use crate::databasex::PostgresDatabase;
#[cfg(feature = "prometheus")]
use crate::metricx::{prometheus_registry, PrometheusConfig};
#[cfg(feature = "statsd")]
use crate::metricx::{statsd_registry, StatsdConfig};
#[cfg(feature = "prometheus")]
use metrics_exporter_prometheus::PrometheusHandle;

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
    #[cfg(feature = "prometheus")]
    prometheus_handle: PrometheusHandle,
    ignore_log_for_paths: Vec<String>,
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
        state: S,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        #[cfg(feature = "statsd")]
        statsd_registry(&statsd_config)?;
        #[cfg(feature = "statsd")]
        let denied_metric_tags = statsd_config.denied_metric_tags;

        #[cfg(feature = "prometheus")]
        let prometheus_handle = prometheus_registry()?;
        #[cfg(feature = "prometheus")]
        let denied_metric_tags = prometheus_config.denied_metric_tags;

        Ok(Self {
            app_name: app_name.to_string(),
            env,
            #[cfg(any(feature = "postgres", feature = "outbox"))]
            database,
            #[cfg(any(feature = "statsd", feature = "prometheus"))]
            denied_metric_tags,
            #[cfg(feature = "prometheus")]
            prometheus_handle,
            ignore_log_for_paths: vec!["/metrics".to_string()],
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

    #[cfg(feature = "prometheus")]
    pub fn prometheus_handle(&self) -> &PrometheusHandle {
        &self.prometheus_handle
    }
}
