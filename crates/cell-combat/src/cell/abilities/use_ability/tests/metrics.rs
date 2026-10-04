//! AB-T6 through the launch and the warmup tick: each cast lands one
//! `abilities_cast_total` outcome, a refusal its `abilities_refused_total`
//! reason, a fire its `abilities_press_to_fire_ms` sample, a hit its QR,
//! effect-path and damage samples, and a send the queue refused its
//! `abilities_wire_send_failed_total`.
//!
//! Each test runs in a world of its own, so its label set is one no other
//! test emits (the recording meter is process-wide under `cargo test`).

use std::time::{Duration, Instant};

use cimmeria_observability::testing::{counter_total, histogram_count, histogram_sum, install};

use super::super::gate_rows::LaunchRefusal;
use super::warmup::{warmup_mgr_in, INSTANT_ABILITY, WARMUP_ABILITY, WARMUP_SECS};
use super::*;
use crate::cell::abilities::metrics::{
    RefusalReason, CAST_TOTAL, EFFECT_APPLIED_TOTAL, PRESS_TO_FIRE_MS, QR_TOTAL, REFUSED_TOTAL,
    WIRE_SEND_FAILED_TOTAL,
};
use crate::cell::abilities::resolve_warmups;
use crate::test_support::NoContentEvents;
use cimmeria_cell_world::cell::effects::ability_metrics::DAMAGE_DEALT as DAMAGE_DEALT_NAME;

fn outcome(world: &str, outcome: &str) -> u64 {
    counter_total(
        CAST_TOTAL,
        &[("outcome", outcome), ("caster", "player"), ("world", world)],
    )
}

/// Every launch refusal counts under the reason its row logs.
#[test]
fn launch_refusals_count_under_their_row_reason() {
    for why in LaunchRefusal::ALL {
        assert_eq!(RefusalReason::from(why).label(), why.reason(), "{why:?}");
    }
}

/// A gate refusal (`LaunchRow::refused`) counts its reason and the cast's
/// `refused` outcome, nothing else.
#[tokio::test]
async fn a_cooldown_refusal_counts_refused_on_cooldown() {
    install();
    const W: &str = "Metrics_T6_Cooldown";
    let mut mgr = warmup_mgr_in(W);
    mgr.get_entity_mut(1)
        .unwrap()
        .abilities
        .start_ability_cooldown(INSTANT_ABILITY, Duration::from_secs(60));
    let (tx, _rx) = mpsc::channel(64);
    let reason = [
        ("reason", "on_cooldown"),
        ("caster", "player"),
        ("world", W),
    ];
    let (r0, c0, f0) = (
        counter_total(REFUSED_TOTAL, &reason),
        outcome(W, "refused"),
        outcome(W, "fired"),
    );

    assert!(!handle_use_ability(1, INSTANT_ABILITY, 2, &tx, &mut mgr).await);

    assert_eq!(counter_total(REFUSED_TOTAL, &reason) - r0, 1);
    assert_eq!(outcome(W, "refused") - c0, 1);
    assert_eq!(outcome(W, "fired") - f0, 0);
}

/// A refusal answered by another module (`LaunchRow::count`) counts too:
/// the range gate's reason is the row's (`target_out_of_range`).
#[tokio::test]
async fn an_out_of_range_refusal_counts_its_range_reason() {
    install();
    const W: &str = "Metrics_T6_Range";
    let mut mgr = warmup_mgr_in(W);
    mgr.get_entity_mut(2).unwrap().position.x = 200.0;
    let (tx, _rx) = mpsc::channel(64);
    let reason = [("reason", "target_out_of_range"), ("world", W)];
    let (r0, c0) = (counter_total(REFUSED_TOTAL, &reason), outcome(W, "refused"));

    assert!(!handle_use_ability(1, INSTANT_ABILITY, 2, &tx, &mut mgr).await);

    assert_eq!(counter_total(REFUSED_TOTAL, &reason) - r0, 1);
    assert_eq!(outcome(W, "refused") - c0, 1);
}

/// An instant cast at a hostile fires in the launch pass: one `fired`, one
/// `instant` press-to-fire sample, one QR result, one effect plan per
/// effect, and (on anything but a miss) the NVP damage it dealt.
#[tokio::test]
async fn an_instant_hit_counts_fire_qr_effect_and_damage() {
    install();
    const W: &str = "Metrics_T6_Instant";
    let mut mgr = warmup_mgr_in(W);
    let (tx, _rx) = mpsc::channel(256);
    let world = [("world", W)];
    let press = [("path", "instant"), ("world", W)];
    let health = [("pool", "health"), ("world", W)];
    let (f0, p0, q0, e0, m0, d0) = (
        outcome(W, "fired"),
        histogram_count(PRESS_TO_FIRE_MS, &press),
        counter_total(QR_TOTAL, &world),
        counter_total(EFFECT_APPLIED_TOTAL, &world),
        counter_total(QR_TOTAL, &[("result", "miss"), ("world", W)]),
        histogram_sum(DAMAGE_DEALT_NAME, &health),
    );

    assert!(handle_use_ability(1, INSTANT_ABILITY, 2, &tx, &mut mgr).await);

    assert_eq!(outcome(W, "fired") - f0, 1);
    assert_eq!(histogram_count(PRESS_TO_FIRE_MS, &press) - p0, 1);
    assert_eq!(counter_total(QR_TOTAL, &world) - q0, 1, "one roll per hit");
    assert_eq!(
        counter_total(EFFECT_APPLIED_TOTAL, &world) - e0,
        1,
        "one plan row for the fixture's one effect"
    );
    let missed = counter_total(QR_TOTAL, &[("result", "miss"), ("world", W)]) - m0 == 1;
    let dealt = histogram_sum(DAMAGE_DEALT_NAME, &health) - d0;
    let lost = 100_000
        - mgr
            .get_entity(2)
            .unwrap()
            .stats
            .get(cimmeria_entity::stats::HEALTH)
            .unwrap()
            .cur;
    assert_eq!(
        dealt,
        f64::from(lost),
        "the histogram is what the target lost"
    );
    assert_eq!(missed, lost == 0, "a miss deals nothing (AB-06)");
}

/// A warmed cast fires from the tick: its press-to-fire sample is at least
/// the warmup, measured on the tick's clock.
#[tokio::test]
async fn a_warmed_cast_records_press_to_fire_from_the_tick() {
    install();
    const W: &str = "Metrics_T6_Warmup";
    let mut mgr = warmup_mgr_in(W);
    let (tx, _rx) = mpsc::channel(256);
    let press = [("path", "warmup"), ("world", W)];
    let (f0, p0, s0) = (
        outcome(W, "fired"),
        histogram_count(PRESS_TO_FIRE_MS, &press),
        histogram_sum(PRESS_TO_FIRE_MS, &press),
    );

    assert!(handle_use_ability(1, WARMUP_ABILITY, 2, &tx, &mut mgr).await);
    assert_eq!(outcome(W, "fired") - f0, 0, "a warmup start is not a fire");
    let later = Instant::now() + Duration::from_secs_f32(WARMUP_SECS + 0.5);
    assert_eq!(
        resolve_warmups(later, &tx, &mut mgr, &NoContentEvents).await,
        1
    );

    assert_eq!(outcome(W, "fired") - f0, 1);
    assert_eq!(histogram_count(PRESS_TO_FIRE_MS, &press) - p0, 1);
    let ms = histogram_sum(PRESS_TO_FIRE_MS, &press) - s0;
    assert!(
        ms >= f64::from(WARMUP_SECS) * 1000.0,
        "press to fire {ms} ms is shorter than the {WARMUP_SECS} s warmup"
    );
}

/// A warmup broken by movement counts `interrupted`, never `fired`.
#[tokio::test]
async fn a_moved_warmup_counts_interrupted() {
    install();
    const W: &str = "Metrics_T6_Interrupt";
    let mut mgr = warmup_mgr_in(W);
    let (tx, _rx) = mpsc::channel(256);
    let (i0, f0) = (outcome(W, "interrupted"), outcome(W, "fired"));

    assert!(handle_use_ability(1, WARMUP_ABILITY, 2, &tx, &mut mgr).await);
    mgr.get_entity_mut(1).unwrap().position.x += 1.0;
    resolve_warmups(Instant::now(), &tx, &mut mgr, &NoContentEvents).await;

    assert_eq!(outcome(W, "interrupted") - i0, 1);
    assert_eq!(outcome(W, "fired") - f0, 0);
}

/// With the cell-to-base queue closed, the launch's cooldown timer is not
/// sent: `abilities_wire_send_failed_total{message=onTimerUpdate}` counts it.
#[tokio::test]
async fn a_refused_timer_send_counts_wire_send_failed() {
    install();
    const W: &str = "Metrics_T6_SendFailed";
    let mut mgr = warmup_mgr_in(W);
    let (tx, rx) = mpsc::channel(256);
    drop(rx);
    let label = [("message", "onTimerUpdate"), ("world", W)];
    let before = counter_total(WIRE_SEND_FAILED_TOTAL, &label);

    handle_use_ability(1, WARMUP_ABILITY, 2, &tx, &mut mgr).await;

    assert!(
        counter_total(WIRE_SEND_FAILED_TOTAL, &label) - before >= 1,
        "the cooldown timer's failed send is counted"
    );
}
