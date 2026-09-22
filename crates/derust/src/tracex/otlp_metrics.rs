use opentelemetry_otlp::MetricExporter;
use opentelemetry_sdk::metrics::SdkMeterProvider;
use opentelemetry_sdk::Resource;

/// Builds an OTLP push pipeline for metrics (`SdkMeterProvider` backed by a
/// `PeriodicReader`, created internally by `with_periodic_exporter`), reading the same
/// `OTEL_EXPORTER_OTLP_*` env vars already used for traces. Returns `None` (after
/// logging a warning) when no exporter can be built — this must never fail
/// `tracex::init()`'s boot, mirroring the existing behaviour for traces.
pub(crate) fn build_otlp_meter_provider(resource: Resource) -> Option<SdkMeterProvider> {
    let protocol = infer_metrics_protocol();

    let exporter = match protocol.as_deref() {
        Some("http/protobuf") => match MetricExporter::builder().with_http().build() {
            Ok(exporter) => Some(exporter),
            Err(error) => {
                tracing::warn!(
                    "failed to build OTLP metric exporter (http/protobuf): {error}; no OTLP metrics will be pushed"
                );
                None
            }
        },
        Some("grpc") => match MetricExporter::builder().with_tonic().build() {
            Ok(exporter) => Some(exporter),
            Err(error) => {
                tracing::warn!(
                    "failed to build OTLP metric exporter (grpc): {error}; no OTLP metrics will be pushed"
                );
                None
            }
        },
        Some(other) => {
            tracing::warn!(
                "unknown OTEL_EXPORTER_OTLP_PROTOCOL '{other}'; no OTLP metric exporter will be created"
            );
            None
        }
        None => {
            tracing::warn!(
                "no OTEL_EXPORTER_OTLP_ENDPOINT/OTEL_EXPORTER_OTLP_PROTOCOL set; no OTLP metric exporter will be created"
            );
            None
        }
    }?;

    let mut builder = SdkMeterProvider::builder()
        .with_periodic_exporter(exporter)
        .with_resource(resource);

    // Keeps histogram bucket boundaries identical between the OTLP push channel and
    // the Prometheus pull channel (`crate::metricx::HISTOGRAM_BUCKET_BOUNDARIES`),
    // whenever `metricx` is compiled in. Without this `View`, the SDK's default
    // histogram aggregation would use different (and divergent) bucket boundaries.
    #[cfg(any(feature = "statsd", feature = "prometheus"))]
    {
        builder = builder.with_view(|instrument: &opentelemetry_sdk::metrics::Instrument| {
            if instrument.kind() == opentelemetry_sdk::metrics::InstrumentKind::Histogram {
                opentelemetry_sdk::metrics::Stream::builder()
                    .with_aggregation(
                        opentelemetry_sdk::metrics::Aggregation::ExplicitBucketHistogram {
                            boundaries: crate::metricx::HISTOGRAM_BUCKET_BOUNDARIES.to_vec(),
                            record_min_max: true,
                        },
                    )
                    .build()
                    .ok()
            } else {
                None
            }
        });
    }

    Some(builder.build())
}

/// Mirrors `init_tracing_opentelemetry::otlp::infer_protocol`'s decision (private in
/// that crate): explicit `OTEL_EXPORTER_OTLP_PROTOCOL` wins; otherwise, infer from the
/// endpoint's default port (`:4317` => grpc, anything else with an endpoint set =>
/// http/protobuf); no endpoint and no protocol => `None`.
fn infer_metrics_protocol() -> Option<String> {
    if let Ok(protocol) = std::env::var("OTEL_EXPORTER_OTLP_PROTOCOL") {
        return Some(protocol);
    }
    let endpoint = std::env::var("OTEL_EXPORTER_OTLP_ENDPOINT").ok()?;
    if endpoint.contains(":4317") {
        Some("grpc".to_string())
    } else {
        Some("http/protobuf".to_string())
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use std::env;
    use std::sync::Mutex;

    // Mirrors the ENV_LOCK pattern in `initialize.rs` — these tests mutate
    // process-wide env vars.
    static ENV_LOCK: Mutex<()> = Mutex::new(());

    fn reset_env() {
        env::remove_var("OTEL_EXPORTER_OTLP_PROTOCOL");
        env::remove_var("OTEL_EXPORTER_OTLP_ENDPOINT");
    }

    #[test]
    fn builds_meter_provider_with_http_protobuf_env() {
        let _guard = ENV_LOCK.lock().unwrap();
        reset_env();
        env::set_var("OTEL_EXPORTER_OTLP_PROTOCOL", "http/protobuf");
        env::set_var("OTEL_EXPORTER_OTLP_ENDPOINT", "http://localhost:4318");

        let resource = Resource::builder_empty().build();
        let result = build_otlp_meter_provider(resource);

        assert!(
            result.is_some(),
            "expected build_otlp_meter_provider to return Some(_) with OTEL_EXPORTER_OTLP_PROTOCOL=http/protobuf"
        );

        reset_env();
    }

    #[test]
    fn returns_none_without_any_otlp_env() {
        let _guard = ENV_LOCK.lock().unwrap();
        reset_env();

        let resource = Resource::builder_empty().build();
        let result = build_otlp_meter_provider(resource);

        assert!(
            result.is_none(),
            "expected build_otlp_meter_provider to return None when no OTLP env is set"
        );

        reset_env();
    }

    #[cfg(any(feature = "statsd", feature = "prometheus"))]
    #[tokio::test]
    async fn histogram_uses_shared_bucket_boundaries_via_view() {
        use opentelemetry::metrics::MeterProvider;
        use opentelemetry_sdk::metrics::data::{AggregatedMetrics, MetricData};
        use opentelemetry_sdk::metrics::{InMemoryMetricExporter, PeriodicReader};

        let _guard = ENV_LOCK.lock().unwrap();
        reset_env();

        let exporter = InMemoryMetricExporter::default();
        let reader = PeriodicReader::builder(exporter.clone()).build();
        let resource = Resource::builder_empty().build();

        // Reimplements just the "with_view" part (skipping `infer_metrics_protocol`,
        // already covered by other tests) so an `InMemoryMetricExporter` can be
        // injected instead of a real OTLP exporter.
        let provider = SdkMeterProvider::builder()
            .with_reader(reader)
            .with_resource(resource)
            .with_view(|instrument: &opentelemetry_sdk::metrics::Instrument| {
                if instrument.kind() == opentelemetry_sdk::metrics::InstrumentKind::Histogram {
                    opentelemetry_sdk::metrics::Stream::builder()
                        .with_aggregation(
                            opentelemetry_sdk::metrics::Aggregation::ExplicitBucketHistogram {
                                boundaries: crate::metricx::HISTOGRAM_BUCKET_BOUNDARIES.to_vec(),
                                record_min_max: true,
                            },
                        )
                        .build()
                        .ok()
                } else {
                    None
                }
            })
            .build();

        let meter = provider.meter("test");
        let histogram = meter.f64_histogram("test_histogram").build();
        histogram.record(0.2, &[]);

        provider.force_flush().unwrap();

        let metrics = exporter.get_finished_metrics().unwrap();
        let metric = metrics
            .iter()
            .flat_map(|resource_metrics| resource_metrics.scope_metrics())
            .flat_map(|scope_metrics| scope_metrics.metrics())
            .find(|metric| metric.name() == "test_histogram")
            .expect("expected a `test_histogram` metric to have been exported");

        let data_point_bounds: Vec<f64> = match metric.data() {
            AggregatedMetrics::F64(MetricData::Histogram(histogram)) => histogram
                .data_points()
                .next()
                .expect("expected at least one histogram data point")
                .bounds()
                .collect(),
            other => panic!("expected an f64 Histogram, got {other:?}"),
        };

        assert_eq!(
            data_point_bounds,
            crate::metricx::HISTOGRAM_BUCKET_BOUNDARIES.to_vec()
        );

        reset_env();
    }
}
