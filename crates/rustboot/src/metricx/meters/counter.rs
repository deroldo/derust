use crate::httpx::AppContext;
use crate::metricx::meters::MetricTags;

pub fn increment<S>(
    context: AppContext<S>,
    metric_name: String,
    metric_tags: MetricTags,
    count: u64,
) where
    S: Clone,
{
    metrics::counter!(metric_name, metric_tags.to_labels(context)).increment(count);
}

pub fn increment_one<S>(context: AppContext<S>, metric_name: String, metric_tags: MetricTags)
where
    S: Clone,
{
    increment(context, metric_name, metric_tags, 1);
}
