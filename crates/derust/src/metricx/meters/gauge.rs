use crate::httpx::AppContext;
use crate::metricx::meters::MetricTags;

pub fn current_gauge<S>(
    context: &AppContext<S>,
    metric_name: &str,
    metric_tags: MetricTags,
    value: f64,
) where
    S: Clone,
{
    metrics::gauge!(
        metric_name.to_string(),
        metric_tags.to_labels(
            context.app_name(),
            context.env(),
            context.denied_metric_tags(),
            context.denied_metric_tags_by_regex(),
        )
    )
    .set(value);
}
