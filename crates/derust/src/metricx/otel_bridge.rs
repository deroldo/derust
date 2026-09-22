use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use metrics::{
    Counter, CounterFn, Gauge, GaugeFn, Histogram, HistogramFn, Key, KeyName, Metadata, Recorder,
    SharedString, Unit,
};
use opentelemetry::metrics::Meter;
use opentelemetry::KeyValue;

/// Wraps an existing `metrics::Recorder` (the Prometheus or StatsD recorder `metricx`
/// already builds today) so that every metric registered through it is *also*
/// forwarded to the OTel Meter API (`opentelemetry::global::meter("derust")`), on top
/// of continuing to be recorded by the wrapped recorder exactly as before.
///
/// This recorder never checks whether an OTLP push pipeline was actually configured —
/// it always forwards to `opentelemetry::global::meter(...)`, which is a dynamic proxy:
/// if `tracex::init()` did not install a real `SdkMeterProvider` (no
/// `OTEL_EXPORTER_OTLP_*` env set), that proxy resolves to the `opentelemetry` API's
/// default no-op provider, whose `.add()`/`.record()` do nothing observable. See the
/// refinement doc's "Decisões técnicas tomadas nesta fase", item 2, for the accepted
/// performance trade-off (a cache lookup + a no-op call per metric emission even when
/// push is disabled).
///
/// `Counter::absolute()` and `Gauge::increment()`/`Gauge::decrement()` are **not**
/// translated to the OTel side (only the wrapped recorder sees them) — `metricx`'s own
/// instrumentation (`metricx/meters/*.rs`) never calls these three methods today (only
/// `Counter::increment()`, `Gauge::set()` and `Histogram::record()`), and OTel's
/// `Counter<u64>`/`Gauge<f64>` have no "set absolute value" / "increment"/"decrement"
/// operations to map them onto. See the refinement doc, decision 4, for the full
/// justification.
pub(crate) struct OtelBridgingRecorder<R: Recorder> {
    inner: R,
    meter: Meter,
    otel_counters: RwLock<HashMap<String, opentelemetry::metrics::Counter<u64>>>,
    otel_gauges: RwLock<HashMap<String, opentelemetry::metrics::Gauge<f64>>>,
    otel_histograms: RwLock<HashMap<String, opentelemetry::metrics::Histogram<f64>>>,
}

impl<R: Recorder> OtelBridgingRecorder<R> {
    pub(crate) fn new(inner: R) -> Self {
        Self {
            inner,
            meter: opentelemetry::global::meter("derust"),
            otel_counters: RwLock::new(HashMap::new()),
            otel_gauges: RwLock::new(HashMap::new()),
            otel_histograms: RwLock::new(HashMap::new()),
        }
    }

    /// Returns the cached OTel counter for `name`, building (and caching) it on first
    /// use. The `opentelemetry::metrics::Meter` docs recommend caching instruments
    /// instead of recreating them on every emission, hence the `RwLock<HashMap<_>>`.
    fn otel_counter(&self, name: &str) -> opentelemetry::metrics::Counter<u64> {
        if let Some(counter) = self.otel_counters.read().unwrap().get(name) {
            return counter.clone();
        }
        self.otel_counters
            .write()
            .unwrap()
            .entry(name.to_string())
            .or_insert_with(|| self.meter.u64_counter(name.to_string()).build())
            .clone()
    }

    fn otel_gauge(&self, name: &str) -> opentelemetry::metrics::Gauge<f64> {
        if let Some(gauge) = self.otel_gauges.read().unwrap().get(name) {
            return gauge.clone();
        }
        self.otel_gauges
            .write()
            .unwrap()
            .entry(name.to_string())
            .or_insert_with(|| self.meter.f64_gauge(name.to_string()).build())
            .clone()
    }

    fn otel_histogram(&self, name: &str) -> opentelemetry::metrics::Histogram<f64> {
        if let Some(histogram) = self.otel_histograms.read().unwrap().get(name) {
            return histogram.clone();
        }
        self.otel_histograms
            .write()
            .unwrap()
            .entry(name.to_string())
            .or_insert_with(|| self.meter.f64_histogram(name.to_string()).build())
            .clone()
    }
}

/// Converts a `metrics::Key`'s labels directly into OTel attributes. Denied tags are
/// already filtered out upstream, in `MetricTags::to_labels()`, before the `Recorder`
/// ever sees the `Key` — so this is a plain 1:1 conversion, with no additional
/// filtering logic here.
fn key_to_attributes(key: &Key) -> Vec<KeyValue> {
    key.labels()
        .map(|label| KeyValue::new(label.key().to_string(), label.value().to_string()))
        .collect()
}

struct FanoutCounter {
    inner: Counter,
    otel: opentelemetry::metrics::Counter<u64>,
    attributes: Vec<KeyValue>,
}

impl CounterFn for FanoutCounter {
    fn increment(&self, value: u64) {
        self.inner.increment(value);
        self.otel.add(value, &self.attributes);
    }

    // `metricx` never calls `Counter::absolute()` today (only `.increment()`, via
    // `metrics::counter!(...).increment(count)` in `meters/counter.rs`) — and OTel's
    // `Counter<u64>` only supports monotonic `.add()`, with no "set absolute value"
    // operation. Forwarding to the inner recorder only, matching pre-existing
    // behaviour; no OTel side-effect. See refinement doc, decision 4.
    fn absolute(&self, value: u64) {
        self.inner.absolute(value);
    }
}

struct FanoutGauge {
    inner: Gauge,
    otel: opentelemetry::metrics::Gauge<f64>,
    attributes: Vec<KeyValue>,
}

impl GaugeFn for FanoutGauge {
    // `metricx::current_gauge` only ever calls `.set()` (see `meters/gauge.rs`) —
    // `increment`/`decrement` have no call site today, so they are not translated to
    // the OTel side (see refinement doc, decision 4).
    fn increment(&self, value: f64) {
        self.inner.increment(value);
    }

    fn decrement(&self, value: f64) {
        self.inner.decrement(value);
    }

    fn set(&self, value: f64) {
        self.inner.set(value);
        self.otel.record(value, &self.attributes);
    }
}

struct FanoutHistogram {
    inner: Histogram,
    otel: opentelemetry::metrics::Histogram<f64>,
    attributes: Vec<KeyValue>,
}

impl HistogramFn for FanoutHistogram {
    fn record(&self, value: f64) {
        self.inner.record(value);
        self.otel.record(value, &self.attributes);
    }
}

impl<R: Recorder> Recorder for OtelBridgingRecorder<R> {
    fn describe_counter(&self, key: KeyName, unit: Option<Unit>, description: SharedString) {
        self.inner.describe_counter(key, unit, description);
    }

    fn describe_gauge(&self, key: KeyName, unit: Option<Unit>, description: SharedString) {
        self.inner.describe_gauge(key, unit, description);
    }

    fn describe_histogram(&self, key: KeyName, unit: Option<Unit>, description: SharedString) {
        self.inner.describe_histogram(key, unit, description);
    }

    fn register_counter(&self, key: &Key, metadata: &Metadata<'_>) -> Counter {
        let inner = self.inner.register_counter(key, metadata);
        let otel = self.otel_counter(key.name());
        Counter::from_arc(Arc::new(FanoutCounter {
            inner,
            otel,
            attributes: key_to_attributes(key),
        }))
    }

    fn register_gauge(&self, key: &Key, metadata: &Metadata<'_>) -> Gauge {
        let inner = self.inner.register_gauge(key, metadata);
        let otel = self.otel_gauge(key.name());
        Gauge::from_arc(Arc::new(FanoutGauge {
            inner,
            otel,
            attributes: key_to_attributes(key),
        }))
    }

    fn register_histogram(&self, key: &Key, metadata: &Metadata<'_>) -> Histogram {
        let inner = self.inner.register_histogram(key, metadata);
        let otel = self.otel_histogram(key.name());
        Histogram::from_arc(Arc::new(FanoutHistogram {
            inner,
            otel,
            attributes: key_to_attributes(key),
        }))
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use metrics::Label;
    use opentelemetry_sdk::metrics::{InMemoryMetricExporter, PeriodicReader, SdkMeterProvider};
    use opentelemetry_sdk::Resource;

    /// Minimal in-process `Recorder` double standing in for `PrometheusRecorder`/
    /// `StatsdRecorder` in these tests — records every `.increment()`/`.set()`/
    /// `.record()` call it receives so tests can assert the inner path still works
    /// exactly as before wrapping it.
    #[derive(Default, Clone)]
    struct SpyRecorder {
        counter_calls: Arc<std::sync::Mutex<Vec<u64>>>,
    }

    impl Recorder for SpyRecorder {
        fn describe_counter(&self, _: KeyName, _: Option<Unit>, _: SharedString) {}
        fn describe_gauge(&self, _: KeyName, _: Option<Unit>, _: SharedString) {}
        fn describe_histogram(&self, _: KeyName, _: Option<Unit>, _: SharedString) {}

        fn register_counter(&self, _: &Key, _: &Metadata<'_>) -> Counter {
            let calls = self.counter_calls.clone();
            struct SpyCounter(Arc<std::sync::Mutex<Vec<u64>>>);
            impl CounterFn for SpyCounter {
                fn increment(&self, value: u64) {
                    self.0.lock().unwrap().push(value);
                }
                fn absolute(&self, _value: u64) {}
            }
            Counter::from_arc(Arc::new(SpyCounter(calls)))
        }

        fn register_gauge(&self, _: &Key, _: &Metadata<'_>) -> Gauge {
            Gauge::noop()
        }

        fn register_histogram(&self, _: &Key, _: &Metadata<'_>) -> Histogram {
            Histogram::noop()
        }
    }

    /// Installs a real `SdkMeterProvider` backed by an `InMemoryMetricExporter` as the
    /// global OTel meter provider, so `opentelemetry::global::meter(...)` resolves to
    /// it. Returns both the provider (needed to call `force_flush()` before reading the
    /// exporter) and the exporter (read after the flush).
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

    #[test]
    fn forwards_counter_increments_to_both_inner_recorder_and_otel() {
        let (provider, exporter) = install_in_memory_meter_provider();
        let spy = SpyRecorder::default();
        let counter_calls = spy.counter_calls.clone();
        let bridge = OtelBridgingRecorder::new(spy);

        let key = Key::from_parts(
            "bridge_counter_metric",
            vec![Label::new("app_name", "test")],
        );
        let counter =
            bridge.register_counter(&key, &Metadata::new("test", metrics::Level::INFO, None));
        counter.increment(3);

        assert_eq!(
            *counter_calls.lock().unwrap(),
            vec![3],
            "inner recorder must still receive the increment"
        );

        provider
            .force_flush()
            .expect("force_flush must succeed against the in-memory exporter");

        let metrics = exporter
            .get_finished_metrics()
            .expect("in-memory exporter must return finished metrics after a flush");
        assert!(
            !metrics.is_empty(),
            "expected at least one exported metric after forcing a flush"
        );

        let found = metrics.iter().any(|resource_metrics| {
            resource_metrics.scope_metrics().any(|scope_metrics| {
                scope_metrics
                    .metrics()
                    .any(|metric| metric.name() == "bridge_counter_metric")
            })
        });
        assert!(
            found,
            "expected the exported metrics to contain bridge_counter_metric"
        );
    }

    #[test]
    fn does_not_forward_gauge_increment_decrement_only_set() {
        // Documents decision 4: increment/decrement on a gauge only touch the inner
        // recorder; this test exists to make that limitation explicit and regression-
        // proof, not to validate OTel output (there is no OTel-side effect to assert).
        let spy = SpyRecorder::default();
        let bridge = OtelBridgingRecorder::new(spy);
        let key = Key::from_name("bridge_gauge_metric");
        let gauge = bridge.register_gauge(&key, &Metadata::new("test", metrics::Level::INFO, None));
        gauge.increment(1.0);
        gauge.decrement(1.0);
        gauge.set(5.0);
        // No panic, no assertion failure: exercised purely to document/lock the
        // behaviour described in decision 4 above.
    }
}
