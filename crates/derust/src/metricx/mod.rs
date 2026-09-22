mod meters;
mod otel_bridge;
mod registries;

pub use meters::*;

#[cfg(feature = "statsd")]
pub use registries::statsd::*;

#[cfg(feature = "prometheus")]
pub use registries::prometheus::*;

/// Single source of truth for histogram bucket boundaries, shared by the Prometheus
/// pull registry (`registries::prometheus::prometheus_registry`) and the OTLP push
/// metrics pipeline (`crate::tracex::otlp_metrics::build_otlp_meter_provider`). Keeping
/// this in one place guarantees both channels represent the same metric with the same
/// distribution granularity.
pub(crate) const HISTOGRAM_BUCKET_BOUNDARIES: [f64; 13] = [
    0.010, 0.025, 0.050, 0.075, 0.100, 0.150, 0.200, 0.250, 0.500, 1.0, 2.5, 5.0, 10.0,
];
