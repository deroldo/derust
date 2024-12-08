use crate::httpx::AppContext;
use crate::metricx::meters::MetricTags;

pub fn record<S>(context: &AppContext<S>, metric_name: &str, metric_tags: MetricTags, value: f64)
where
    S: Clone,
{
    metrics::histogram!(metric_name.to_string(), metric_tags.to_labels(context)).record(value);
}
