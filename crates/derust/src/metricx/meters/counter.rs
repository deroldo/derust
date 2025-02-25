use crate::httpx::AppContext;
use crate::metricx::meters::MetricTags;

pub fn increment<S>(context: &AppContext<S>, metric_name: &str, metric_tags: MetricTags, count: u64)
where
    S: Clone,
{
    metrics::counter!(metric_name.to_string(), metric_tags.to_labels(
        context.app_name(),
        context.env(),
        context.denied_metric_tags(),
        context.denied_metric_tags_by_regex(),
    )).increment(count);
}

pub fn increment_one<S>(context: &AppContext<S>, metric_name: &str, metric_tags: MetricTags)
where
    S: Clone,
{
    increment(context, metric_name, metric_tags, 1);
}
