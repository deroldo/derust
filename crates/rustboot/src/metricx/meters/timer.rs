use crate::httpx::AppContext;
use crate::metricx::meters::MetricTags;
use std::time::Instant;

pub struct Stopwatch<S>
where
    S: Clone,
{
    context: AppContext<S>,
    metric_name: String,
    metric_tags: MetricTags,
    start: Instant,
}

impl<S> Stopwatch<S>
where
    S: Clone,
{
    pub fn record(self, metric_tags: MetricTags) {
        let mut tags = self.metric_tags.vec();
        for metrics_tag in metric_tags.vec() {
            tags.push(metrics_tag)
        }

        record_duration(
            &self.context,
            &self.metric_name,
            MetricTags(tags),
            self.start.elapsed().as_secs_f64(),
        )
    }
}

pub fn start_stopwatch<S>(
    context: &AppContext<S>,
    metric_name: &str,
    metric_tags: MetricTags,
) -> Stopwatch<S>
where
    S: Clone,
{
    Stopwatch {
        context: context.clone(),
        metric_name: metric_name.to_string(),
        metric_tags,
        start: Instant::now(),
    }
}

pub fn record_duration<S>(
    context: &AppContext<S>,
    metric_name: &str,
    metric_tags: MetricTags,
    value: f64,
) where
    S: Clone,
{
    metrics::histogram!(metric_name.to_string(), metric_tags.to_labels(context)).record(value);
}
