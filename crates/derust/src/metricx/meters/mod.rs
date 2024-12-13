pub(crate) mod counter;
pub(crate) mod gauge;
pub(crate) mod money;
pub(crate) mod tags;
pub(crate) mod timer;

pub use counter::*;
pub use gauge::*;
use metrics::Histogram;
pub use money::*;
pub use tags::*;
pub use timer::*;

pub trait Registry: Clone {
    fn timer(&self, name: &str, metric_tags: MetricTags) -> Histogram;
}
