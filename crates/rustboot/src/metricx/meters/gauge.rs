use crate::httpx::AppContext;
use crate::metricx::meters::MetricTags;

pub fn current<S>(context: AppContext<S>, metric_name: String, metric_tags: MetricTags, value: f64)
where
    S: Clone,
{
    metrics::gauge!(metric_name, metric_tags.to_labels(context)).set(value);
}
