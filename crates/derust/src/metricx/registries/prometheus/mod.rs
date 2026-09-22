use metrics_exporter_prometheus::{PrometheusBuilder, PrometheusHandle};
use regex::Regex;

#[derive(Clone)]
pub struct PrometheusConfig {
    pub denied_metric_tags: Vec<String>,
    pub denied_metric_tags_by_regex: Vec<Regex>,
}

pub fn prometheus_registry() -> Result<PrometheusHandle, Box<dyn std::error::Error>> {
    let builder = PrometheusBuilder::new()
        .set_buckets(&crate::metricx::HISTOGRAM_BUCKET_BOUNDARIES)
        .map_err(|error| Box::new(error))?;

    let recorder = builder.build_recorder();
    let handle = recorder.handle();

    metrics::set_global_recorder(crate::metricx::otel_bridge::OtelBridgingRecorder::new(
        recorder,
    ))
    .map_err(|error| Box::new(error))?;

    Ok(handle)
}

/// End-to-end coverage for the 3 business-plan scope criteria of the OTel metrics
/// bridge (see the plan's "Escopo" section): (1) a metric emitted while a real
/// `MeterProvider` is installed appears on both the pull (Prometheus) and push (OTLP)
/// channels; (2) without a real `MeterProvider` installed, the pull channel behaves
/// exactly as it did before this plan; (3) denied tags never leak onto either channel.
///
/// Each test here calls `AppContext::new(...)` with a `PrometheusConfig`, which
/// internally calls `prometheus_registry()` -> `metrics::set_global_recorder(...)`, a
/// process-wide global that can only be set once. `opentelemetry::global::set_meter_provider`
/// is similarly process-wide. `cargo nextest run` isolates every test in its own
/// process, so these tests are safe to run together only under nextest (never under
/// `cargo test`, which runs them in threads within a single process).
///
/// Gated on `not(feature = "statsd")`: when both `statsd` and `prometheus` are
/// enabled, `AppContext::new(...)` takes both a `StatsdConfig` and a
/// `PrometheusConfig` and calls both `statsd_registry()` and `prometheus_registry()`,
/// each of which calls `metrics::set_global_recorder(...)` — a process-wide global
/// that can only be set once, so the second call would always fail. That combination
/// is a pre-existing constraint of `AppContext::new` (see `httpx/context.rs`), not
/// something introduced by this bridge, so these prometheus-focused tests simply don't
/// run under it.
#[cfg(all(test, not(feature = "statsd")))]
mod test {
    use super::*;
    use crate::envx::Environment;
    use crate::httpx::AppContext;
    use crate::metricx::{increment, MetricTags};
    use opentelemetry_sdk::metrics::data::{AggregatedMetrics, MetricData};
    use opentelemetry_sdk::metrics::{InMemoryMetricExporter, PeriodicReader, SdkMeterProvider};
    use opentelemetry_sdk::Resource;

    /// Installs a real `SdkMeterProvider` backed by an `InMemoryMetricExporter` as the
    /// global OTel meter provider, simulating `tracex::init()` having configured OTLP
    /// push. Returns both the provider (to `force_flush()` before reading) and the
    /// exporter (to read the finished metrics after the flush).
    fn install_in_memory_meter_provider() -> (SdkMeterProvider, InMemoryMetricExporter) {
        let exporter = InMemoryMetricExporter::default();
        let reader = PeriodicReader::builder(exporter.clone()).build();
        let provider = SdkMeterProvider::builder()
            .with_reader(reader)
            .with_resource(Resource::builder_empty().build())
            .build();
        opentelemetry::global::set_meter_provider(provider.clone());
        (provider, exporter)
    }

    #[tokio::test]
    async fn metric_appears_on_both_pull_and_push_channels_when_meter_provider_is_set() {
        // Install a real (in-memory) MeterProvider, simulating tracex::init() having
        // configured OTLP push (see the plan's Task 5 context for why this is
        // equivalent to it, without needing a real OTLP collector in this test).
        let (provider, exporter) = install_in_memory_meter_provider();

        let context = AppContext::new(
            "test-app",
            Environment::Test,
            PrometheusConfig {
                denied_metric_tags: vec![],
                denied_metric_tags_by_regex: vec![],
            },
            (),
        )
        .unwrap();

        increment(&context, "bridge_e2e_counter", MetricTags::default(), 7);

        // Pull channel.
        let rendered = context.prometheus_handle().render();
        assert!(rendered.contains("bridge_e2e_counter"));
        assert!(rendered.contains('7'));

        // Push channel.
        provider
            .force_flush()
            .expect("force_flush must succeed against the in-memory exporter");
        let metrics = exporter
            .get_finished_metrics()
            .expect("in-memory exporter must return finished metrics after a flush");

        let found = metrics.iter().any(|resource_metrics| {
            resource_metrics.scope_metrics().any(|scope| {
                scope
                    .metrics()
                    .any(|metric| metric.name() == "bridge_e2e_counter")
            })
        });
        assert!(
            found,
            "expected bridge_e2e_counter to have been exported on the push channel too"
        );
    }

    #[tokio::test]
    async fn pull_channel_behaviour_is_unchanged_without_a_real_meter_provider() {
        // Deliberately do NOT call opentelemetry::global::set_meter_provider — the
        // global proxy resolves to the opentelemetry API's default no-op provider, so
        // the bridge's forwarding calls are no-ops and only the pull channel is
        // observable.
        let context = AppContext::new(
            "test-app",
            Environment::Test,
            PrometheusConfig {
                denied_metric_tags: vec![],
                denied_metric_tags_by_regex: vec![],
            },
            (),
        )
        .unwrap();

        increment(
            &context,
            "bridge_e2e_counter_no_push",
            MetricTags::default(),
            9,
        );

        let rendered = context.prometheus_handle().render();
        assert!(rendered.contains("bridge_e2e_counter_no_push"));
        assert!(rendered.contains('9'));
        // No push-side assertion here by design: there is no MeterProvider installed,
        // so there is nothing to flush/inspect — this test's whole point is that the
        // pull channel keeps working exactly as it did before this plan.
    }

    #[tokio::test]
    async fn denied_tags_are_hidden_on_both_channels() {
        let (provider, exporter) = install_in_memory_meter_provider();

        let context = AppContext::new(
            "test-app",
            Environment::Test,
            PrometheusConfig {
                denied_metric_tags: vec!["customer".to_string()],
                denied_metric_tags_by_regex: vec![],
            },
            (),
        )
        .unwrap();

        let tags = MetricTags::from([("customer", "123"), ("kind", "foo")]);
        increment(&context, "bridge_e2e_denied_tags", tags, 1);

        let rendered = context.prometheus_handle().render();
        assert!(rendered.contains("bridge_e2e_denied_tags"));
        assert!(
            !rendered.contains("customer"),
            "denied tag leaked into the pull channel"
        );

        provider
            .force_flush()
            .expect("force_flush must succeed against the in-memory exporter");
        let metrics = exporter
            .get_finished_metrics()
            .expect("in-memory exporter must return finished metrics after a flush");

        let metric = metrics
            .iter()
            .flat_map(|resource_metrics| resource_metrics.scope_metrics())
            .flat_map(|scope| scope.metrics())
            .find(|metric| metric.name() == "bridge_e2e_denied_tags")
            .expect("expected bridge_e2e_denied_tags to have been exported on the push channel");

        let attribute_keys: Vec<String> = match metric.data() {
            AggregatedMetrics::U64(MetricData::Sum(sum)) => sum
                .data_points()
                .flat_map(|data_point| data_point.attributes())
                .map(|kv| kv.key.to_string())
                .collect(),
            other => panic!("expected a u64 Sum (counter), got {other:?}"),
        };

        assert!(
            !attribute_keys.iter().any(|key| key == "customer"),
            "denied tag leaked into the push channel: {attribute_keys:?}"
        );
    }
}
