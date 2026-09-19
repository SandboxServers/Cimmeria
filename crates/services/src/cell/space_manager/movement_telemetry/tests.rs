//! Unit guards for [`LogThrottle`] — the primitive both the
//! movement-reject log and the NPC path-failure log sit behind.
//!
//! The end-to-end shape (a burst through `report_movement_reject`
//! producing one row, then a `suppressed` count) is guarded in
//! `space_manager::tests::movement_validation::telemetry`; these pin the
//! arithmetic itself, which is where an off-by-one would silently
//! under- or over-report how much was elided.

use super::*;

const WINDOW: Duration = Duration::from_secs(1);

/// The first occurrence for an entity always emits, immediately. A
/// throttle that made the first one wait would delay the single most
/// useful row — the one that says a problem started.
#[test]
fn the_first_occurrence_emits_with_nothing_suppressed() {
    let mut t = LogThrottle::default();
    assert_eq!(t.admit(1, Instant::now(), WINDOW), Some(0));
}

/// Occurrences inside the window are counted, not written, and the next
/// one through carries the exact count.
#[test]
fn suppressed_count_is_exact_across_the_window() {
    let mut t = LogThrottle::default();
    let t0 = Instant::now();

    assert_eq!(t.admit(1, t0, WINDOW), Some(0));
    for i in 1..=7 {
        assert_eq!(
            t.admit(1, t0 + Duration::from_millis(i * 10), WINDOW),
            None,
            "occurrence {i} is inside the window and must not emit"
        );
    }
    assert_eq!(
        t.admit(1, t0 + Duration::from_secs(2), WINDOW),
        Some(7),
        "the next emitted row must account for all seven elided \
         occurrences — an off-by-one here misreports the magnitude of a \
         stuck-entity episode, which is the number an operator triages on"
    );
    assert_eq!(
        t.admit(1, t0 + Duration::from_secs(4), WINDOW),
        Some(0),
        "the counter resets after each emission; it is a per-gap count, \
         not a running total"
    );
}

/// The window boundary is inclusive — exactly `interval` later emits.
/// A strict `>` would make a perfectly periodic 1 Hz emitter (the shape
/// a tick loop produces) never emit again.
#[test]
fn exactly_one_interval_later_emits() {
    let mut t = LogThrottle::default();
    let t0 = Instant::now();
    assert_eq!(t.admit(1, t0, WINDOW), Some(0));
    assert_eq!(t.admit(1, t0 + Duration::from_millis(999), WINDOW), None);
    assert_eq!(t.admit(1, t0 + WINDOW, WINDOW), Some(1));
}

/// Entities are independent. Without this, one spamming client hides
/// every other player's first occurrence — strictly worse than no
/// throttle at all.
#[test]
fn entities_do_not_share_a_window() {
    let mut t = LogThrottle::default();
    let t0 = Instant::now();
    assert_eq!(t.admit(1, t0, WINDOW), Some(0));
    assert_eq!(t.admit(1, t0, WINDOW), None);
    assert_eq!(
        t.admit(2, t0, WINDOW),
        Some(0),
        "a second entity's first occurrence must emit regardless of the \
         first entity's open window"
    );
}

/// `forget` releases the slot, so the map is bounded by the live entity
/// population and a recycled id starts with a clean window.
#[test]
fn forget_releases_the_slot_and_resets_the_window() {
    let mut t = LogThrottle::default();
    let t0 = Instant::now();
    t.admit(1, t0, WINDOW);
    t.admit(1, t0, WINDOW);
    assert_eq!(t.tracked(), 1);

    t.forget(1);
    assert_eq!(t.tracked(), 0, "state must not outlive the entity");
    assert_eq!(
        t.admit(1, t0, WINDOW),
        Some(0),
        "a reused entity id must emit its first occurrence rather than \
         inherit the previous occupant's open window and suppression count"
    );
}

/// A non-monotonic sample (clock rewind, coarse platform timer) must not
/// panic. `Instant` subtraction does; `saturating_duration_since` is
/// what keeps a log-throttle decision from taking the cell task down.
#[test]
fn a_rewound_clock_suppresses_rather_than_panicking() {
    let mut t = LogThrottle::default();
    let t0 = Instant::now() + Duration::from_secs(10);
    assert_eq!(t.admit(1, t0, WINDOW), Some(0));
    assert_eq!(
        t.admit(1, t0 - Duration::from_secs(5), WINDOW),
        None,
        "a sample from before the last emission reads as zero elapsed, so \
         it suppresses — the safe direction"
    );
}

/// The two metric-label fallbacks are stable tokens.
///
/// `cimmeria_observability` no-ops without an initialised Meter, so a
/// counter *emission* cannot be observed from a unit test. What can be
/// pinned is the label vocabulary — and these two are the values a
/// SigNoz `sum by (world)` / `sum by (gate)` groups on for the series
/// that have neither.
#[test]
fn metric_label_fallbacks_are_stable_non_empty_tokens() {
    assert_eq!(UNKNOWN_WORLD, "unknown");
    assert_eq!(GATE_NOT_APPLICABLE, "n/a");
    // An empty string is indistinguishable from an absent label in
    // ClickHouse, which silently drops the series from a groupBy.
    assert!(!UNKNOWN_WORLD.is_empty() && !GATE_NOT_APPLICABLE.is_empty());
}

/// The sampling budget the instrumentation-discipline doc quotes.
/// Loosening either of these changes the rows/hour figure recorded
/// there, so they are pinned together.
#[test]
fn position_sample_budget_matches_the_documented_rate() {
    assert_eq!(POSITION_SAMPLE_MIN_INTERVAL, Duration::from_secs(5));
    assert_eq!(POSITION_SAMPLE_MIN_DISTANCE, 1.0);
    assert_eq!(REJECT_LOG_MIN_INTERVAL, Duration::from_secs(1));
}

/// `MovementTelemetry::forget` must clear **every** map it owns. A new
/// per-entity field added without a matching line in `forget` is the
/// leak this pins.
#[test]
fn movement_telemetry_forget_clears_every_map() {
    let mut mt = MovementTelemetry::default();
    let now = Instant::now();
    mt.reject_log.admit(42, now, WINDOW);
    mt.npc_path_fail_log.admit(42, now, WINDOW);
    mt.position_samples.insert(
        42,
        PositionSample {
            at: now,
            position: Vector3::new(0.0, 0.0, 0.0),
        },
    );
    assert_eq!(mt.tracked(), 3);

    mt.forget(42);
    assert_eq!(
        mt.tracked(),
        0,
        "forget must clear all three maps; a map missed here leaks one \
         slot per entity for the process lifetime"
    );
}
