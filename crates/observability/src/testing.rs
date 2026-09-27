//! A recording meter for tests, behind the `testing` feature.
//!
//! The facade's macros do nothing until [`crate::init`] has run, which no
//! test does, so a counter call cannot be observed by default. [`install`]
//! puts a process-global meter provider in place whose counters add every
//! measurement to an in-memory table, then calls [`crate::init`]. After that
//! [`counter_total`] reads what a code path emitted.
//!
//! Only `u64` counters are recorded; other instrument kinds fall back to the
//! API's no-op instruments.
//!
//! The table is process-wide. nextest runs each test in its own process;
//! under `cargo test` tests share it, so assert on the change across the
//! code under test ([`counter_total`] before and after) with labels no other
//! test emits.

use std::borrow::Cow;
use std::collections::HashMap;
use std::sync::{Arc, Mutex, OnceLock};

use opentelemetry::metrics::{
    Counter, InstrumentBuilder, InstrumentProvider, Meter, MeterProvider, SyncInstrument,
};
use opentelemetry::{InstrumentationScope, KeyValue};

/// `(metric name, sorted labels)` → running total.
type Table = Mutex<HashMap<(String, Vec<(String, String)>), u64>>;

fn table() -> &'static Table {
    static TABLE: OnceLock<Table> = OnceLock::new();
    TABLE.get_or_init(|| Mutex::new(HashMap::new()))
}

struct RecordingCounter {
    name: Cow<'static, str>,
}

impl SyncInstrument<u64> for RecordingCounter {
    fn measure(&self, measurement: u64, attributes: &[KeyValue]) {
        let mut labels: Vec<(String, String)> = attributes
            .iter()
            .map(|kv| (kv.key.to_string(), kv.value.to_string()))
            .collect();
        labels.sort();
        let mut table = table().lock().unwrap_or_else(|e| e.into_inner());
        *table.entry((self.name.to_string(), labels)).or_default() += measurement;
    }
}

struct RecordingInstruments;

impl InstrumentProvider for RecordingInstruments {
    fn u64_counter(&self, builder: InstrumentBuilder<'_, Counter<u64>>) -> Counter<u64> {
        Counter::new(Arc::new(RecordingCounter { name: builder.name }))
    }
}

struct RecordingProvider;

impl MeterProvider for RecordingProvider {
    fn meter_with_scope(&self, _scope: InstrumentationScope) -> Meter {
        Meter::new(Arc::new(RecordingInstruments))
    }
}

/// Install the recording meter once per process. Safe to call from every
/// test, in any order.
///
/// The facade's meter can be set only once per process, so in a test
/// binary that uses this module every [`crate::init`] must go through here
/// (the facade's own `init` self-test does); a direct `init` would take the
/// slot with whatever provider is global at the time.
///
/// # Panics
///
/// If the facade was already initialised with another meter, since every
/// later [`counter_total`] would then read zero.
pub fn install() {
    static INSTALLED: OnceLock<()> = OnceLock::new();
    INSTALLED.get_or_init(|| {
        opentelemetry::global::set_meter_provider(RecordingProvider);
        crate::init("cimmeria-test")
            .expect("the metrics facade was initialised before the recording meter");
    });
}

/// The total added to counter `name` over every label set that contains
/// all of `labels`.
pub fn counter_total(name: &str, labels: &[(&str, &str)]) -> u64 {
    let table = table().lock().unwrap_or_else(|e| e.into_inner());
    table
        .iter()
        .filter(|((metric, set), _)| {
            metric == name
                && labels
                    .iter()
                    .all(|&(k, v)| set.iter().any(|(sk, sv)| sk == k && sv == v))
        })
        .map(|(_, &total)| total)
        .sum()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A `counter!` call made after `install` is counted under its labels,
    /// and a subset of the labels selects every matching set.
    #[test]
    fn counter_macro_is_recorded_after_install() {
        install();
        let probe = [("probe", "testing_module")];
        let before = counter_total("observability_testing_probe_total", &probe);
        crate::counter!("observability_testing_probe_total", "probe" => "testing_module", "k" => "v");
        crate::counter!("observability_testing_probe_total", "probe" => "testing_module", "k" => "w");
        assert_eq!(
            counter_total("observability_testing_probe_total", &probe) - before,
            2
        );
        assert_eq!(
            counter_total(
                "observability_testing_probe_total",
                &[("k", "w"), ("probe", "testing_module")]
            ),
            counter_total(
                "observability_testing_probe_total",
                &[("probe", "testing_module"), ("k", "w")]
            ),
            "label order does not matter"
        );
    }
}
