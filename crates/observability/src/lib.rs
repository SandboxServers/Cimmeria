//! Metrics facade — thin macro layer over the OpenTelemetry SDK's
//! metrics API.
//!
//! Why a facade instead of using the OTel API directly:
//!
//! - **Lazy instrument registration.** Every counter/histogram is
//!   registered once via `once_cell::Lazy` on first emission. Without
//!   this, every callsite would walk the global `Meter` to look up the
//!   instrument by name on every hit — measurable cost on the hot
//!   path.
//! - **One macro shape.** `counter!("trade_swaps_total", "outcome" =>
//!   "completed")` matches the popular `metrics` crate's macro shape
//!   so call sites read familiar even though we don't depend on that
//!   crate. Means a future swap to `metrics` + a real exporter is a
//!   macro-body change, not a call-site rewrite.
//! - **No-op when disabled.** Until [`init`] is called (which the
//!   server's `otel::init` does when `OTEL_EXPORTER_OTLP_ENDPOINT` is
//!   set), the macros emit nothing — instrument registration is
//!   deferred until the first emission *after* init runs. Saves the
//!   cost of registering 50+ instruments at process start when
//!   telemetry is disabled entirely.
//!
//! # Cardinality rule (per
//! [`docs/architecture/instrumentation-discipline.md`](../../docs/architecture/instrumentation-discipline.md))
//!
//! Metric labels must be enumerated, low-cardinality strings (`outcome`,
//! `reason`, `kind`, `world_name`, `decision_outcome`). High-cardinality
//! correlators (`entity_id`, `player_id`, `peer`) belong in span/log
//! fields, NEVER on a metric. ClickHouse merge-tree storing a label per
//! entity degrades query performance non-linearly.

use std::sync::OnceLock;

use opentelemetry::metrics::Meter;

/// Process-global Meter handle, set by [`init`]. Pre-init the macros
/// no-op silently; the `meter()` helper returns `None`. This is the
/// "telemetry not configured" path — every crate's binary that doesn't
/// initialize OTLP still compiles and runs.
static METER: OnceLock<Meter> = OnceLock::new();

/// Initialize the global Meter. Call from the server's `otel::init`
/// after the metric provider has been registered with
/// `opentelemetry::global::set_meter_provider`.
///
/// Idempotent — only the first call wins (matches the OTel SDK's
/// "global provider is set once" pattern). A second call returns
/// `Err(InitError::AlreadyInitialized)` so the caller can decide to
/// warn (multiple init paths) or silently ignore (re-entry into init
/// during a hot-reload).
///
/// # Panics
///
/// Never panics. The error path returns `InitError` so the caller can
/// log and continue.
pub fn init(scope_name: &'static str) -> Result<(), InitError> {
    let meter = opentelemetry::global::meter(scope_name);
    METER
        .set(meter)
        .map_err(|_| InitError::AlreadyInitialized)?;
    Ok(())
}

#[derive(Debug)]
pub enum InitError {
    AlreadyInitialized,
}

impl std::fmt::Display for InitError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            InitError::AlreadyInitialized => write!(
                f,
                "cimmeria_observability::init was already called this process"
            ),
        }
    }
}

impl std::error::Error for InitError {}

/// Returns the global Meter if `init` has been called, otherwise None.
/// The macros use this to decide whether to register + emit, or
/// no-op silently.
///
/// Public so the macros below (which expand at the call site, not in
/// this crate) can resolve it. Treat as internal — direct callers
/// should use the macros.
#[doc(hidden)]
pub fn meter() -> Option<&'static Meter> {
    METER.get()
}

// ── Instrument cache ──────────────────────────────────────────────────
//
// Counters/histograms are looked up from the Meter by name; the lookup
// returns the same instrument every time (the SDK deduplicates by
// name + unit). The cost of the lookup itself is non-trivial because
// it threads through a HashMap — so we wrap each named instrument
// in `Lazy<>` keyed on the name string, computed once per
// (name × process) pair.

use opentelemetry::metrics::{Counter, Histogram, UpDownCounter};
use opentelemetry::KeyValue;
use std::collections::HashMap;
use std::sync::Mutex;

/// Internal cache of registered Counter<u64> instruments by name.
/// Lazily populated on first emission per name.
#[doc(hidden)]
pub fn get_or_register_counter_u64(name: &'static str) -> Option<Counter<u64>> {
    static CACHE: OnceLock<Mutex<HashMap<&'static str, Counter<u64>>>> = OnceLock::new();
    let cache = CACHE.get_or_init(|| Mutex::new(HashMap::new()));
    let meter = meter()?;
    let mut guard = cache.lock().ok()?;
    if let Some(c) = guard.get(name) {
        return Some(c.clone());
    }
    let counter = meter.u64_counter(name).build();
    guard.insert(name, counter.clone());
    Some(counter)
}

#[doc(hidden)]
pub fn get_or_register_histogram_f64(name: &'static str) -> Option<Histogram<f64>> {
    static CACHE: OnceLock<Mutex<HashMap<&'static str, Histogram<f64>>>> = OnceLock::new();
    let cache = CACHE.get_or_init(|| Mutex::new(HashMap::new()));
    let meter = meter()?;
    let mut guard = cache.lock().ok()?;
    if let Some(h) = guard.get(name) {
        return Some(h.clone());
    }
    let hist = meter.f64_histogram(name).build();
    guard.insert(name, hist.clone());
    Some(hist)
}

#[doc(hidden)]
pub fn get_or_register_gauge_i64(name: &'static str) -> Option<UpDownCounter<i64>> {
    // UpDownCounter is the OTel SDK's "gauge-like" cumulative metric;
    // a true "synchronous gauge" doesn't exist in the SDK (only async).
    // For the use cases we have (cover slots held, in-flight trades),
    // an up/down counter that callers `add(+1)` on reserve and
    // `add(-1)` on release is the right shape.
    static CACHE: OnceLock<Mutex<HashMap<&'static str, UpDownCounter<i64>>>> = OnceLock::new();
    let cache = CACHE.get_or_init(|| Mutex::new(HashMap::new()));
    let meter = meter()?;
    let mut guard = cache.lock().ok()?;
    if let Some(g) = guard.get(name) {
        return Some(g.clone());
    }
    let gauge = meter.i64_up_down_counter(name).build();
    guard.insert(name, gauge.clone());
    Some(gauge)
}

// Re-export so call sites can construct labels without depending on
// `opentelemetry` directly. Treating `opentelemetry::KeyValue` as the
// label primitive means a future switch to a different metrics SDK
// would change `KeyValue` to whatever the new SDK uses, and our
// macros' call sites stay unchanged.
pub use opentelemetry::KeyValue as Label;

/// Build a `[KeyValue]` slice from `(key, value)` pairs. Internal to
/// the macros; the value type is constrained to `Into<opentelemetry::Value>`
/// (which accepts `&str`, `String`, integers, etc.).
#[doc(hidden)]
pub fn labels<const N: usize>(pairs: [(&'static str, opentelemetry::Value); N]) -> Vec<KeyValue> {
    pairs
        .into_iter()
        .map(|(k, v)| KeyValue::new(k, v))
        .collect()
}

/// Increment a counter by 1.
///
/// ```ignore
/// use cimmeria_observability::counter;
/// counter!("trade_swaps_total", "outcome" => "completed");
/// ```
///
/// When init() has not been called (telemetry disabled), the macro
/// expands to a no-op that the compiler optimises away.
#[macro_export]
macro_rules! counter {
    ($name:expr $(, $key:expr => $val:expr)* $(,)?) => {{
        if let Some(c) = $crate::get_or_register_counter_u64($name) {
            let labels = vec![
                $( $crate::Label::new($key, $val) ),*
            ];
            c.add(1, &labels);
        }
    }};
}

/// Add an arbitrary value to a counter.
///
/// ```ignore
/// counter_add!("bytes_received_total", 4096u64, "transport" => "udp");
/// ```
#[macro_export]
macro_rules! counter_add {
    ($name:expr, $delta:expr $(, $key:expr => $val:expr)* $(,)?) => {{
        if let Some(c) = $crate::get_or_register_counter_u64($name) {
            let labels = vec![
                $( $crate::Label::new($key, $val) ),*
            ];
            c.add($delta, &labels);
        }
    }};
}

/// Record a histogram observation.
///
/// ```ignore
/// histogram!("trade_swap_duration_seconds", 0.123, "outcome" => "completed");
/// ```
#[macro_export]
macro_rules! histogram {
    ($name:expr, $value:expr $(, $key:expr => $val:expr)* $(,)?) => {{
        if let Some(h) = $crate::get_or_register_histogram_f64($name) {
            let labels = vec![
                $( $crate::Label::new($key, $val) ),*
            ];
            h.record($value, &labels);
        }
    }};
}

/// Adjust a gauge (up-down counter) by `delta` (positive or negative).
///
/// ```ignore
/// gauge_add!("cover_slots_held", 1, "world_name" => "Castle");
/// gauge_add!("cover_slots_held", -1, "world_name" => "Castle");
/// ```
#[macro_export]
macro_rules! gauge_add {
    ($name:expr, $delta:expr $(, $key:expr => $val:expr)* $(,)?) => {{
        if let Some(g) = $crate::get_or_register_gauge_i64($name) {
            let labels = vec![
                $( $crate::Label::new($key, $val) ),*
            ];
            g.add($delta, &labels);
        }
    }};
}

#[cfg(test)]
mod tests {
    //! Self-tests for the facade.
    //!
    //! The OTel SDK's `Meter` is a `Clone`able handle, so we install a
    //! noop-meter in tests and rely on the `OnceLock` guard to keep
    //! the singleton from getting clobbered between tests. Tests run
    //! single-threaded under the default `cargo test` flavor.

    use super::*;

    /// `init` is idempotent: the first call succeeds, subsequent calls
    /// return `Err(AlreadyInitialized)`. Pin this contract so a refactor
    /// that switches to `set_meter_provider` panics doesn't silently
    /// regress to a "second init crashes the process" failure mode.
    #[test]
    fn init_returns_already_initialized_on_second_call() {
        // First call may have happened in another test; just verify
        // the second call returns Err either way.
        let _ = init("self-test");
        let second = init("self-test");
        assert!(
            matches!(second, Err(InitError::AlreadyInitialized)),
            "second init must return AlreadyInitialized, got {second:?}",
        );
    }

    /// Pre-init, `meter()` returns None — confirms the no-op path
    /// where telemetry is disabled.
    ///
    /// Note: this test relies on test isolation; if another test
    /// already called `init()` first, `meter()` will be Some. We
    /// can't assert None deterministically across the suite without
    /// resetting global state, which OnceLock doesn't support. This
    /// test instead confirms the *shape* — calling `meter()` is safe
    /// regardless of init state.
    #[test]
    fn meter_is_safe_to_call_unconditionally() {
        // Doesn't panic, doesn't deadlock, returns Option.
        let _ = meter();
    }

    /// counter! macro registers + emits without panicking, even when
    /// no Meter is installed (the pre-init no-op path).
    #[test]
    fn counter_macro_is_safe_when_uninitialized() {
        // Don't call init() — verify the macro is a clean no-op.
        // (May or may not no-op depending on test ordering, but
        // either way must not panic.)
        crate::counter!("test_counter_total", "outcome" => "ok");
        crate::counter_add!("test_counter_add_total", 42u64, "kind" => "test");
    }

    /// histogram! macro registers + emits without panicking.
    #[test]
    fn histogram_macro_is_safe_when_uninitialized() {
        crate::histogram!("test_histogram_seconds", 0.001, "outcome" => "ok");
    }

    /// gauge_add! macro emits +/- deltas without panicking.
    #[test]
    fn gauge_add_macro_is_safe_when_uninitialized() {
        crate::gauge_add!("test_gauge", 1i64, "world_name" => "Castle");
        crate::gauge_add!("test_gauge", -1i64, "world_name" => "Castle");
    }

    /// Concurrent emissions from two threads must not panic and must
    /// not deadlock. The instrument cache uses a Mutex, so we pin that
    /// the lock isn't held across the actual counter add (would deadlock
    /// against re-entry from another thread doing the same).
    #[test]
    fn concurrent_emissions_dont_deadlock() {
        use std::thread;
        let h1 = thread::spawn(|| {
            for _ in 0..100 {
                crate::counter!("concurrent_test_total", "thread" => "1");
            }
        });
        let h2 = thread::spawn(|| {
            for _ in 0..100 {
                crate::counter!("concurrent_test_total", "thread" => "2");
            }
        });
        h1.join().expect("thread 1 must finish");
        h2.join().expect("thread 2 must finish");
    }
}
