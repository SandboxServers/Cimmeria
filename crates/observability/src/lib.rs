//! Metrics facade — thin macro layer over the OpenTelemetry SDK's
//! metrics API.
//!
//! Why a facade instead of using the OTel API directly:
//!
//! - **Lazy instrument registration.** A process-global
//!   `OnceLock<RwLock<HashMap<&'static str, Instrument>>>` per
//!   instrument kind holds the cached `Counter` / `Histogram` /
//!   `UpDownCounter` handles. On first emission for a given name we
//!   take a write lock to build and insert; every subsequent emission
//!   takes a read lock and returns the cached clone. Without this,
//!   every callsite would walk the global `Meter` to look up the
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
//!
//! # Cache key is the instrument name, not (name, labels)
//!
//! OpenTelemetry instruments are **label-agnostic**: the same
//! `Counter<u64>` accepts every label-set at `.add()` time, so the
//! cache is keyed only by the metric name. A future contributor who
//! thinks they need a per-label-set cache is misreading the OTel API.
//!
//! # Resource attributes
//!
//! The OTel SDK attaches resource attributes (e.g. `service.name`,
//! `deployment.environment`) once at provider init — not here. The
//! server wires `CIMMERIA_DEPLOY_ENV` → `deployment.environment` in
//! `crates/server/src/otel.rs`; this facade is downstream of that.

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
#[must_use = "init failure leaves the metrics facade in no-op mode — log and decide"]
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
// it threads through a HashMap — so each named instrument is cached
// once per (name × process) pair in an `OnceLock<RwLock<HashMap>>`.
//
// Why RwLock not Mutex: emissions read this map on every hot-path
// callsite (`movement_validation_rejects_total`, NPC AI per-tick
// counters, every cover state transition). A Mutex would serialize
// readers from independent threads on the common cache-hit path.
// RwLock lets readers proceed concurrently and only serializes the
// rare "first emission for this name" path against the writer.
//
// Why poisoning recovery: if a panic unwinds while holding the lock,
// `lock().ok()?` would silently fail every subsequent emission for
// that instrument kind — a silent telemetry blackout. We recover via
// `unwrap_or_else(|e| e.into_inner())` so a panicked thread doesn't
// poison the metrics pipeline for the rest of the process.

use opentelemetry::metrics::{Counter, Histogram, UpDownCounter};
use std::collections::HashMap;
use std::sync::{RwLock, RwLockReadGuard, RwLockWriteGuard};

/// Recover from `PoisonError` by extracting the inner guard — a
/// panicked thread shouldn't blackhole telemetry for the rest of the
/// process. Standard pattern for read-mostly caches.
fn read_unpoisoned<T>(lock: &RwLock<T>) -> RwLockReadGuard<'_, T> {
    lock.read().unwrap_or_else(|e| e.into_inner())
}

fn write_unpoisoned<T>(lock: &RwLock<T>) -> RwLockWriteGuard<'_, T> {
    lock.write().unwrap_or_else(|e| e.into_inner())
}

/// Internal cache of registered Counter<u64> instruments by name.
/// Lazily populated on first emission per name.
#[doc(hidden)]
pub fn get_or_register_counter_u64(name: &'static str) -> Option<Counter<u64>> {
    static CACHE: OnceLock<RwLock<HashMap<&'static str, Counter<u64>>>> = OnceLock::new();
    let cache = CACHE.get_or_init(|| RwLock::new(HashMap::new()));
    let meter = meter()?;
    // Fast path: read lock, return cloned handle if cached.
    if let Some(c) = read_unpoisoned(cache).get(name) {
        return Some(c.clone());
    }
    // Slow path: take write lock and re-check (another thread may have
    // raced us between the read drop and write acquire), then insert.
    let mut guard = write_unpoisoned(cache);
    if let Some(c) = guard.get(name) {
        return Some(c.clone());
    }
    let counter = meter.u64_counter(name).build();
    guard.insert(name, counter.clone());
    Some(counter)
}

#[doc(hidden)]
pub fn get_or_register_histogram_f64(name: &'static str) -> Option<Histogram<f64>> {
    static CACHE: OnceLock<RwLock<HashMap<&'static str, Histogram<f64>>>> = OnceLock::new();
    let cache = CACHE.get_or_init(|| RwLock::new(HashMap::new()));
    let meter = meter()?;
    if let Some(h) = read_unpoisoned(cache).get(name) {
        return Some(h.clone());
    }
    let mut guard = write_unpoisoned(cache);
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
    // a true synchronous point-in-time gauge doesn't exist in the SDK
    // (only async observable gauges). For the use cases we have (cover
    // slots held, in-flight trades), an up/down counter that callers
    // `add(+1)` on reserve and `add(-1)` on release is the right shape.
    //
    // Caveat: on process restart the counter resets to 0; observability
    // dashboards should chart the rate of change, not the absolute
    // value, if they need restart-resilience.
    static CACHE: OnceLock<RwLock<HashMap<&'static str, UpDownCounter<i64>>>> = OnceLock::new();
    let cache = CACHE.get_or_init(|| RwLock::new(HashMap::new()));
    let meter = meter()?;
    if let Some(g) = read_unpoisoned(cache).get(name) {
        return Some(g.clone());
    }
    let mut guard = write_unpoisoned(cache);
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
    /// not deadlock. The instrument cache uses an `RwLock`, so readers
    /// from independent threads on the common cache-hit path proceed
    /// concurrently; only the rare first-registration write contends.
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

    /// Regression guard for Clara G1 (silent telemetry blackout via
    /// mutex poisoning). Originally the cache used a `Mutex` with
    /// `.lock().ok()?`, so a panicked thread would poison the lock and
    /// every subsequent emission would silently no-op — invisible
    /// telemetry loss. The fix moved to `RwLock` and unpoisons via
    /// `unwrap_or_else(|e| e.into_inner())`.
    ///
    /// This test induces a poison and asserts the next emission still
    /// goes through (does not panic, does not silently drop).
    ///
    /// Bug shape: revert the `read_unpoisoned` / `write_unpoisoned`
    /// helpers to `.read().ok()?` / `.write().ok()?` and observe the
    /// `try_emit_after_poison` thread silently fail to register a new
    /// instrument.
    #[test]
    fn poisoned_lock_recovers_via_unpoison() {
        use std::sync::{Arc, RwLock};

        // We test the unpoison helper directly against a synthetic
        // RwLock since the cache statics in the facade are private.
        // The helper's behavior is what guards the silent-blackout
        // failure mode — once the helper unpoisons, the facade's
        // cache lookups behave the same as a never-poisoned cache.
        let lock: Arc<RwLock<HashMap<&'static str, u32>>> = Arc::new(RwLock::new(HashMap::new()));
        let lock_clone = Arc::clone(&lock);

        let _ = std::thread::spawn(move || {
            let _guard = lock_clone.write().expect("acquire write lock");
            panic!("intentional panic to poison the lock");
        })
        .join();

        // Direct .read() / .write() would return Err(PoisonError).
        assert!(lock.read().is_err(), "lock must be poisoned");

        // The unpoison helpers must recover transparently.
        let read_guard = read_unpoisoned(&lock);
        assert!(read_guard.is_empty(), "read after unpoison must succeed");
        drop(read_guard);

        let mut write_guard = write_unpoisoned(&lock);
        write_guard.insert("after_poison", 42);
        drop(write_guard);

        // The map should now carry the post-poison write.
        assert_eq!(
            read_unpoisoned(&lock).get("after_poison").copied(),
            Some(42),
            "write through unpoisoned guard must persist — without the \
             unpoison helpers this assertion would never run because \
             the poisoned .write() returns Err and the test would \
             abort on the unwrap"
        );
    }
}
