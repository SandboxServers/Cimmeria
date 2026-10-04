//! AB-T6 pinning tests: each metric's label set equals its enum, and each
//! enum equals the reasons the code logs.
//!
//! The call sites take the enums, so a sample can carry no other value.
//! What these tests catch is drift the other way: a row reason with no
//! metric value, or a metric value nothing emits.

use std::collections::BTreeSet;

use cimmeria_cell_world::cell::effects::ability_metrics::{
    self, PoolSample, DAMAGE_DEALT, HEAL_DONE, LEDGER_REMOVED_TOTAL,
};
use cimmeria_cell_world::cell::effects::stat_buff::StatBuffRemoval;
use cimmeria_entity::abilities::{
    RangeRefusal, RC_CRITICAL, RC_DOUBLE_CRITICAL, RC_GLANCING, RC_HIT, RC_MISS, RC_NONE,
};
use cimmeria_observability::testing::{counter_total, histogram_count, histogram_sum, install};

use super::*;
use crate::cell::abilities::effect_plan::{
    PATH_ALLY_FANOUT, PATH_LEDGER, PATH_NVP, PATH_PULSE, PATH_ROUTED_TO_USER, PATH_SCRIPT,
    PATH_SKIPPED,
};

fn npc_cast(world: &'static str) -> RefusedCast {
    RefusedCast {
        entity_id: 7,
        entity_name: None,
        ability_id: 99,
        who: cimmeria_entity::cell_entity::PlayerIdentity::UNKNOWN,
        caster: CasterKind::Npc,
        world,
    }
}

fn labels(all: impl IntoIterator<Item = &'static str>) -> BTreeSet<&'static str> {
    all.into_iter().collect()
}

/// Rule 4 hygiene for one label: non-empty, no duplicate value, at most 30
/// values, no whitespace.
fn assert_enumerated(name: &str, values: &[&'static str]) {
    let set = labels(values.iter().copied());
    assert!(!values.is_empty(), "{name}: empty label set");
    assert_eq!(
        set.len(),
        values.len(),
        "{name}: duplicate values {values:?}"
    );
    assert!(
        values.len() <= 30,
        "{name}: {} values, Rule 4's target is 30",
        values.len()
    );
    for v in values {
        assert!(
            !v.is_empty() && !v.contains(char::is_whitespace),
            "{name}: bad value {v:?}"
        );
    }
}

#[test]
fn every_label_set_is_enumerated() {
    let sets: [(&str, Vec<&'static str>); 9] = [
        (
            "outcome",
            CastOutcome::ALL.iter().map(|v| v.label()).collect(),
        ),
        (
            "caster",
            CasterKind::ALL.iter().map(|v| v.label()).collect(),
        ),
        (
            "reason",
            RefusalReason::ALL.iter().map(|v| v.label()).collect(),
        ),
        ("path", EffectPath::ALL.iter().map(|v| v.label()).collect()),
        ("result", QrOutcome::ALL.iter().map(|v| v.label()).collect()),
        (
            "message",
            WireMessage::ALL.iter().map(|v| v.label()).collect(),
        ),
        (
            "fire path",
            FirePath::ALL.iter().map(|v| v.label()).collect(),
        ),
        ("pool", Pool::ALL.iter().map(|v| v.label()).collect()),
        (
            "ledger reason",
            StatBuffRemoval::ALL.iter().map(|v| v.reason()).collect(),
        ),
    ];
    for (name, values) in &sets {
        assert_enumerated(name, values);
    }
}

/// `effect_planned`'s path consts are exactly `EffectPath`.
#[test]
fn effect_path_label_set_is_the_effect_plan_paths() {
    let consts = labels([
        PATH_SCRIPT,
        PATH_NVP,
        PATH_ROUTED_TO_USER,
        PATH_ALLY_FANOUT,
        PATH_LEDGER,
        PATH_PULSE,
        PATH_SKIPPED,
    ]);
    assert_eq!(
        consts,
        labels(EffectPath::ALL.iter().map(|p| p.label())),
        "a new effect path needs an EffectPath variant, and the reverse"
    );
    for p in EffectPath::ALL {
        assert_eq!(EffectPath::from_label(p.label()), Some(*p));
    }
    assert_eq!(EffectPath::from_label("not_a_path"), None);
}

/// `qr_rolled`'s `result` over every code is exactly `QrOutcome`.
#[test]
fn qr_label_set_is_every_result_code() {
    let from_codes = labels((0..=u8::MAX).map(|c| QrOutcome::from_code(c).label()));
    assert_eq!(from_codes, labels(QrOutcome::ALL.iter().map(|q| q.label())));
    for (code, want) in [
        (RC_NONE, "none"),
        (RC_HIT, "hit"),
        (RC_MISS, "miss"),
        (RC_CRITICAL, "critical"),
        (RC_DOUBLE_CRITICAL, "double_critical"),
        (RC_GLANCING, "glancing"),
    ] {
        assert_eq!(QrOutcome::from_code(code).label(), want);
    }
}

/// `wire_ledger::method_name` over every index, plus the two cell-to-base
/// messages, is exactly `WireMessage`.
#[test]
fn wire_message_label_set_is_the_ledger_method_names() {
    let mut names = labels((0..=u16::MAX).map(crate::cell::abilities::wire_ledger::method_name));
    names.insert(WireMessage::RefreshAppearance.label());
    names.insert(WireMessage::ContactListPresenceEvent.label());
    assert_eq!(names, labels(WireMessage::ALL.iter().map(|m| m.label())));
    for idx in 0..=u16::MAX {
        assert_eq!(
            WireMessage::from_method(idx).label(),
            crate::cell::abilities::wire_ledger::method_name(idx)
        );
    }
}

/// The range refusals map to their rows' reasons.
#[test]
fn range_refusals_keep_their_row_reasons() {
    for r in [RangeRefusal::TooFar, RangeRefusal::TooClose] {
        assert_eq!(RefusalReason::from_range(r).label(), r.reason());
    }
}

/// Every `RefusalReason` that is not a `LaunchRefusal` (those are pinned in
/// `use_ability::tests::metrics`) is named at a call site in production
/// code, so the enum holds no value nothing emits.
#[test]
fn every_refusal_reason_has_a_call_site() {
    let sources: String = crate::test_support::source_scan::rust_sources()
        .into_iter()
        .filter(|s| {
            s.crates_rel.starts_with("cell-combat/src/cell/abilities/")
                && !s.crates_rel.contains("/metrics/")
                && !s.is_test_path()
        })
        .map(|s| {
            let text = s.read();
            crate::test_support::source_scan::production_lines(&text)
                .into_iter()
                .map(|(_, l)| format!("{l}\n"))
                .collect::<String>()
        })
        .collect();
    let launch: BTreeSet<_> = labels([
        "caster_missing",
        "caster_dead",
        "already_warming",
        "not_known",
        "unknown_ability_id",
        "on_cooldown",
        "target_dead",
        "no_beneficial_target",
        "caster_missing_at_commit",
        "reload_in_flight",
        "no_ammo",
    ]);
    let mut missing = Vec::new();
    for r in RefusalReason::ALL {
        if launch.contains(r.label()) {
            continue;
        }
        let variant = format!("{r:?}");
        let named = sources.contains(&format!("RefusalReason::{variant}"))
            || (matches!(
                r,
                RefusalReason::TargetOutOfRange | RefusalReason::TargetTooClose
            ) && sources.contains("RefusalReason::from_range"));
        if !named {
            missing.push(variant);
        }
    }
    assert!(missing.is_empty(), "reasons nothing emits: {missing:?}");
}

/// `StatBuffRemoval::ALL` is every variant, each at its index.
#[test]
fn ledger_removal_all_is_complete() {
    for (i, r) in StatBuffRemoval::ALL.iter().enumerate() {
        assert_eq!(r.index(), i, "{r:?}");
    }
}

/// Each metric records under its name and labels once the meter is on.
/// Every sample uses a world no other test emits.
#[test]
fn samples_reach_the_meter_with_their_labels() {
    install();
    const W: &str = "Metrics_T6_Unit";
    let count = |name, l: &[(&str, &str)]| counter_total(name, l);

    let before = count(CAST_TOTAL, &[("outcome", "fired"), ("world", W)]);
    let h0 = histogram_count(PRESS_TO_FIRE_MS, &[("path", "warmup"), ("world", W)]);
    fired(
        FirePath::Warmup,
        Duration::from_millis(1500),
        CasterKind::Player,
        W,
    );
    assert_eq!(
        count(
            CAST_TOTAL,
            &[("outcome", "fired"), ("caster", "player"), ("world", W)]
        ) - before,
        1
    );
    assert_eq!(
        histogram_count(PRESS_TO_FIRE_MS, &[("path", "warmup"), ("world", W)]) - h0,
        1
    );

    let r0 = count(REFUSED_TOTAL, &[("reason", "shield_full"), ("world", W)]);
    let c0 = count(CAST_TOTAL, &[("outcome", "refused"), ("world", W)]);
    refused(RefusalReason::ShieldFull, npc_cast(W));
    assert_eq!(
        count(
            REFUSED_TOTAL,
            &[("reason", "shield_full"), ("caster", "npc"), ("world", W)]
        ) - r0,
        1
    );
    assert_eq!(
        count(CAST_TOTAL, &[("outcome", "refused"), ("world", W)]) - c0,
        1,
        "a refusal is also the cast's outcome"
    );

    let e0 = count(EFFECT_APPLIED_TOTAL, &[("path", "ledger"), ("world", W)]);
    effect_applied(EffectPath::Ledger, W);
    assert_eq!(
        count(EFFECT_APPLIED_TOTAL, &[("path", "ledger"), ("world", W)]) - e0,
        1
    );

    let q0 = count(QR_TOTAL, &[("result", "glancing"), ("world", W)]);
    qr(RC_GLANCING, W);
    assert_eq!(
        count(QR_TOTAL, &[("result", "glancing"), ("world", W)]) - q0,
        1
    );

    let w0 = count(
        WIRE_SEND_FAILED_TOTAL,
        &[("message", "onTimerUpdate"), ("world", W)],
    );
    wire_send_failed(WireMessage::OnTimerUpdate, W);
    assert_eq!(
        count(
            WIRE_SEND_FAILED_TOTAL,
            &[("message", "onTimerUpdate"), ("world", W)]
        ) - w0,
        1
    );

    let l0 = count(
        LEDGER_REMOVED_TOTAL,
        &[("reason", "cleansed"), ("world", W)],
    );
    ability_metrics::ledger_removed(StatBuffRemoval::Cleansed, W);
    assert_eq!(
        count(
            LEDGER_REMOVED_TOTAL,
            &[("reason", "cleansed"), ("world", W)]
        ) - l0,
        1
    );
}

/// A pool sample turns a fall into damage and a rise into a heal, per pool,
/// and records nothing for no change.
#[test]
fn pool_change_splits_into_damage_and_heal() {
    install();
    const W: &str = "Metrics_T6_Pools";
    let d = |pool| histogram_sum(DAMAGE_DEALT, &[("pool", pool), ("world", W)]);
    let h = |pool| histogram_sum(HEAL_DONE, &[("pool", pool), ("world", W)]);
    let (dh, df, hh, hf) = (d("health"), d("focus"), h("health"), h("focus"));
    let before = PoolSample {
        health: 100,
        focus: 50,
    };
    before.record_change(
        PoolSample {
            health: 70,
            focus: 65,
        },
        W,
    );
    assert_eq!(d("health") - dh, 30.0);
    assert_eq!(h("focus") - hf, 15.0);
    assert_eq!(d("focus") - df, 0.0, "a rise is not damage");
    assert_eq!(h("health") - hh, 0.0, "a fall is not a heal");
    let n = histogram_count(DAMAGE_DEALT, &[("world", W)]);
    before.record_change(before, W);
    assert_eq!(histogram_count(DAMAGE_DEALT, &[("world", W)]), n);
}

/// Every `abilities_refused_total` sample writes exactly one
/// `ability_refused` row whose `reason` is the metric's label, for every
/// reason: the refusals view and the dashboard count the same population.
#[test]
fn every_refusal_writes_one_ability_refused_row_with_its_label() {
    install();
    const W: &str = "Metrics_T6_RefusalRows";
    for reason in RefusalReason::ALL {
        let logs = crate::test_support::LogCapture::install();
        let label = [("reason", reason.label()), ("world", W)];
        let before = counter_total(REFUSED_TOTAL, &label);
        refused(*reason, npc_cast(W));
        let rows: Vec<_> = logs
            .all()
            .into_iter()
            .filter(|c| c.target == "abilities" && c.has_field("event", EVENT_ABILITY_REFUSED))
            .collect();
        assert_eq!(rows.len(), 1, "{reason:?}: {rows:#?}");
        assert!(rows[0].has_field("reason", reason.label()), "{rows:?}");
        assert!(rows[0].has_field("ability_id", "99"), "{rows:?}");
        assert_eq!(
            counter_total(REFUSED_TOTAL, &label) - before,
            1,
            "{reason:?}"
        );
    }
}

/// `abilities_refused_total` is counted in `metrics::refused` and nowhere
/// else, so no call site can count a refusal without its row.
#[test]
fn the_refusal_counter_has_one_call_site() {
    let users: Vec<_> = crate::test_support::source_scan::rust_sources()
        .into_iter()
        .filter(|s| !s.is_test_path() && s.crates_rel.starts_with("cell-"))
        .filter(|s| {
            let text = s.read();
            crate::test_support::source_scan::production_lines(&text)
                .iter()
                .any(|(_, l)| {
                    !l.trim_start().starts_with("//")
                        && (l.contains("REFUSED_TOTAL")
                            || l.contains("\"abilities_refused_total\""))
                })
        })
        .map(|s| s.crates_rel)
        .collect();
    assert_eq!(
        users,
        ["cell-combat/src/cell/abilities/metrics/mod.rs"],
        "count refusals through metrics::refused"
    );
}

/// cell-world counts the `abandoned` outcome on the same metric with the
/// same `caster` spelling as this crate's enums.
#[test]
fn abandoned_outcome_matches_the_cell_world_counter() {
    assert_eq!(
        CastOutcome::Abandoned.label(),
        ability_metrics::OUTCOME_ABANDONED
    );
    assert_eq!(CAST_TOTAL, ability_metrics::CAST_TOTAL);
    assert_eq!(
        CasterKind::Player.label(),
        ability_metrics::caster_label(true)
    );
    assert_eq!(
        CasterKind::Npc.label(),
        ability_metrics::caster_label(false)
    );
}
