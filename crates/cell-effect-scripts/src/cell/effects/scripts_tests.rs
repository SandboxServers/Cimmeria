//! Tests for the single-shot damage and chip scripts in `scripts.rs`.

use super::*;
use crate::cell::effects::test_fixtures::{effect_with_nvp, make_mgr_with_target};
use cimmeria_entity::abilities::EffectDef;
use std::collections::HashMap;

#[test]
fn melee_damage_applies_health_damage() {
    let mut mgr = make_mgr_with_target();
    let effect = effect_with_nvp("HealthDamage", "20");
    let mut ctx = EffectContext {
        source_id: 1,
        target_id: 1,
        effect: &effect,
        space_mgr: &mut mgr,
    };
    MeleeDamage.on_apply(&mut ctx);
    // 50 - 20 = 30
    let hp = ctx
        .space_mgr
        .get_entity(1)
        .unwrap()
        .stats
        .get(HEALTH)
        .unwrap()
        .cur;
    assert_eq!(hp, 30);
}

#[test]
fn melee_damage_clamps_at_zero() {
    let mut mgr = make_mgr_with_target();
    // Player at low health
    if let Some(e) = mgr.get_entity_mut(1) {
        if let Some(s) = e.stats.get_mut(HEALTH) {
            s.update(0, 5, 100);
        }
    }
    let effect = effect_with_nvp("HealthDamage", "999");
    let mut ctx = EffectContext {
        source_id: 1,
        target_id: 1,
        effect: &effect,
        space_mgr: &mut mgr,
    };
    MeleeDamage.on_apply(&mut ctx);
    let hp = ctx
        .space_mgr
        .get_entity(1)
        .unwrap()
        .stats
        .get(HEALTH)
        .unwrap()
        .cur;
    assert_eq!(hp, 0, "must clamp at zero, not go negative");
}

#[test]
fn missing_target_is_noop() {
    let mut mgr = make_mgr_with_target();
    let effect = effect_with_nvp("HealPercentage", "35.00");
    let mut ctx = EffectContext {
        source_id: 1,
        target_id: 99999, // missing
        effect: &effect,
        space_mgr: &mut mgr,
    };
    HealHealth.on_apply(&mut ctx); // must not panic
    HealFocus.on_apply(&mut ctx);
    MeleeDamage.on_apply(&mut ctx);
    // Original target unchanged
    let hp = mgr.get_entity(1).unwrap().stats.get(HEALTH).unwrap().cur;
    assert_eq!(hp, 50);
}

#[test]
fn suppression_chips_health_by_nvp_amount() {
    let mut mgr = make_mgr_with_target();
    // Player starts at 50/100. Suppression with HealthDamage=8.
    let mut params = HashMap::new();
    params.insert("HealthDamage".to_string(), "8".to_string());
    let effect = EffectDef {
        effect_id: 700,
        ability_id: 1,
        params,
        ..Default::default()
    };
    let mut ctx = EffectContext {
        source_id: 1,
        target_id: 1,
        effect: &effect,
        space_mgr: &mut mgr,
    };
    Suppression.on_apply(&mut ctx);
    let hp = ctx
        .space_mgr
        .get_entity(1)
        .unwrap()
        .stats
        .get(HEALTH)
        .unwrap()
        .cur;
    assert_eq!(hp, 42, "50 - 8 chip = 42");
}

// ── RangedPhysicalDamage ──────────────────────────────────────────

fn effect_with_two_nvps(name1: &str, val1: &str, name2: &str, val2: &str) -> EffectDef {
    let mut params = HashMap::new();
    params.insert(name1.to_string(), val1.to_string());
    params.insert(name2.to_string(), val2.to_string());
    EffectDef {
        effect_id: 641,
        ability_id: 579,
        params,
        ..Default::default()
    }
}

/// **Shield absorbs everything → no health damage.** Mirror of
/// `if remaining_dmg_percent > 0` in the legacy Python: when Focus
/// fully absorbs the requested damage, the script returns without
/// touching HEALTH. This is the load-bearing difference vs. the
/// legacy NVP fallback (which applies both pools independently).
/// Reverting the gate (always applying HealthDamage) would fail
/// this test.
#[test]
fn ranged_physical_full_focus_absorbs_no_health_damage() {
    let mut mgr = make_mgr_with_target();
    // Focus 200/1000 in fixture; FocusDamage 100 fits entirely.
    let effect = effect_with_two_nvps("FocusDamage", "100", "HealthDamage", "10");
    let mut ctx = EffectContext {
        source_id: 1,
        target_id: 1,
        effect: &effect,
        space_mgr: &mut mgr,
    };
    RangedPhysicalDamage.on_apply(&mut ctx);
    let e = ctx.space_mgr.get_entity(1).unwrap();
    assert_eq!(
        e.stats.get(FOCUS).unwrap().cur,
        100,
        "100 focus damage out of 200 must drain to 100"
    );
    assert_eq!(
        e.stats.get(HEALTH).unwrap().cur,
        50,
        "shield held → no HEALTH damage, even though HealthDamage NVP = 10"
    );
}

/// **Partial absorb → spillover lands as health damage.** Focus
/// 30/1000, FocusDamage 100 → 30 applied to focus, 70 overflow,
/// spillover = 70/3 = 23, plus HealthDamage 10 = 33 health damage.
#[test]
fn ranged_physical_partial_absorb_spills_to_health() {
    let mut mgr = make_mgr_with_target();
    if let Some(e) = mgr.get_entity_mut(1) {
        if let Some(s) = e.stats.get_mut(FOCUS) {
            s.update(0, 30, 1000);
        }
    }
    let effect = effect_with_two_nvps("FocusDamage", "100", "HealthDamage", "10");
    let mut ctx = EffectContext {
        source_id: 1,
        target_id: 1,
        effect: &effect,
        space_mgr: &mut mgr,
    };
    RangedPhysicalDamage.on_apply(&mut ctx);
    let e = ctx.space_mgr.get_entity(1).unwrap();
    assert_eq!(
        e.stats.get(FOCUS).unwrap().cur,
        0,
        "focus drained to 0 (30 was less than 100)"
    );
    // Overflow = 70. Spillover = 70/3 = 23 (integer division).
    // Final health damage = 23 + 10 = 33. HP was 50 → 17.
    assert_eq!(
        e.stats.get(HEALTH).unwrap().cur,
        17,
        "HP 50 - (spillover 23 + HealthDamage 10) = 17"
    );
}

/// **No focus at all → full overflow.** With FOCUS = 0, the entire
/// FocusDamage is overflow; spillover = 100/3 = 33; + HealthDmg 10
/// = 43 HP loss. HP 50 → 7.
#[test]
fn ranged_physical_no_focus_takes_full_overflow_spillover() {
    let mut mgr = make_mgr_with_target();
    if let Some(e) = mgr.get_entity_mut(1) {
        if let Some(s) = e.stats.get_mut(FOCUS) {
            s.update(0, 0, 1000);
        }
    }
    let effect = effect_with_two_nvps("FocusDamage", "100", "HealthDamage", "10");
    let mut ctx = EffectContext {
        source_id: 1,
        target_id: 1,
        effect: &effect,
        space_mgr: &mut mgr,
    };
    RangedPhysicalDamage.on_apply(&mut ctx);
    let e = ctx.space_mgr.get_entity(1).unwrap();
    assert_eq!(e.stats.get(FOCUS).unwrap().cur, 0);
    assert_eq!(
        e.stats.get(HEALTH).unwrap().cur,
        7,
        "HP 50 - (spillover 33 + HealthDamage 10) = 7"
    );
}

/// **FocusDamage = 0 → no Focus mutation AND no Health damage.**
/// The spillover gate trips on `focus_overflow == 0`. With no
/// Focus damage configured, overflow is also 0, so the script
/// returns before touching HEALTH. This pins that the script is
/// genuinely Focus-driven — HealthDamage alone shouldn't fire.
#[test]
fn ranged_physical_zero_focus_damage_skips_health_too() {
    let mut mgr = make_mgr_with_target();
    let effect = effect_with_two_nvps("FocusDamage", "0", "HealthDamage", "10");
    let mut ctx = EffectContext {
        source_id: 1,
        target_id: 1,
        effect: &effect,
        space_mgr: &mut mgr,
    };
    RangedPhysicalDamage.on_apply(&mut ctx);
    let e = ctx.space_mgr.get_entity(1).unwrap();
    assert_eq!(e.stats.get(FOCUS).unwrap().cur, 200, "no focus mutation");
    assert_eq!(
        e.stats.get(HEALTH).unwrap().cur,
        50,
        "no health damage when Focus damage is zero"
    );
}

/// **Missing target is a graceful no-op.**
#[test]
fn ranged_physical_missing_target_is_noop() {
    let mut mgr = make_mgr_with_target();
    let effect = effect_with_two_nvps("FocusDamage", "100", "HealthDamage", "10");
    let mut ctx = EffectContext {
        source_id: 1,
        target_id: 999, // doesn't exist
        effect: &effect,
        space_mgr: &mut mgr,
    };
    RangedPhysicalDamage.on_apply(&mut ctx);
    let e = ctx.space_mgr.get_entity(1).unwrap();
    assert_eq!(e.stats.get(FOCUS).unwrap().cur, 200);
    assert_eq!(e.stats.get(HEALTH).unwrap().cur, 50);
}

// ── RangedEnergyDamage ────────────────────────────────────────────

/// **Energy damage hits both pools simultaneously — no gating.**
/// The structural difference vs. RangedPhysicalDamage: even if the
/// target's Focus could absorb the requested damage, Health still
/// takes the HealthDamage NVP. This is what makes Energy weapons
/// the "ignore shields" counterpart to Physical.
#[test]
fn ranged_energy_applies_both_pools_in_parallel() {
    let mut mgr = make_mgr_with_target();
    // Focus 200/1000, HP 50/100 in fixture.
    let effect = effect_with_two_nvps("FocusDamage", "30", "HealthDamage", "15");
    let mut ctx = EffectContext {
        source_id: 1,
        target_id: 1,
        effect: &effect,
        space_mgr: &mut mgr,
    };
    RangedEnergyDamage.on_apply(&mut ctx);
    let e = ctx.space_mgr.get_entity(1).unwrap();
    assert_eq!(
        e.stats.get(FOCUS).unwrap().cur,
        170,
        "200 - 30 = 170 (no gating — applied independently)"
    );
    assert_eq!(
        e.stats.get(HEALTH).unwrap().cur,
        35,
        "50 - 15 = 35 (applied even though Focus could have absorbed)"
    );
}

/// Zero-NVP edge: nothing applied either way.
#[test]
fn ranged_energy_zero_nvps_is_noop() {
    let mut mgr = make_mgr_with_target();
    let effect = effect_with_two_nvps("FocusDamage", "0", "HealthDamage", "0");
    let mut ctx = EffectContext {
        source_id: 1,
        target_id: 1,
        effect: &effect,
        space_mgr: &mut mgr,
    };
    RangedEnergyDamage.on_apply(&mut ctx);
    let e = ctx.space_mgr.get_entity(1).unwrap();
    assert_eq!(e.stats.get(FOCUS).unwrap().cur, 200);
    assert_eq!(e.stats.get(HEALTH).unwrap().cur, 50);
}

/// Pool clamps at 0 — Focus damage exceeding cur drains to 0, not
/// below. (For Energy there's no spillover so the excess just
/// disappears.)
#[test]
fn ranged_energy_drain_clamps_at_zero() {
    let mut mgr = make_mgr_with_target();
    if let Some(e) = mgr.get_entity_mut(1) {
        if let Some(s) = e.stats.get_mut(FOCUS) {
            s.update(0, 20, 1000);
        }
    }
    let effect = effect_with_two_nvps("FocusDamage", "100", "HealthDamage", "5");
    let mut ctx = EffectContext {
        source_id: 1,
        target_id: 1,
        effect: &effect,
        space_mgr: &mut mgr,
    };
    RangedEnergyDamage.on_apply(&mut ctx);
    let e = ctx.space_mgr.get_entity(1).unwrap();
    assert_eq!(e.stats.get(FOCUS).unwrap().cur, 0, "focus clamps at 0");
    assert_eq!(e.stats.get(HEALTH).unwrap().cur, 45, "50 - 5 = 45");
}

/// Pins the legacy two-step truncation: with FocusDamage=80,
/// overflow=3 → `remaining_pct = 3*100/80 = 3` (truncated from 3.75)
/// → `spillover = 3*80/300 = 0` (truncated from 0.8). Zero spillover.
/// A regression to `overflow / 3` would compute `3/3 = 1` and over-
/// damage small overflows.
#[test]
fn ranged_physical_small_overflow_truncates_to_zero_spillover() {
    let mut mgr = make_mgr_with_target();
    if let Some(e) = mgr.get_entity_mut(1) {
        if let Some(s) = e.stats.get_mut(FOCUS) {
            s.update(0, 77, 1000); // 80 dmg → applied 77 → overflow 3
        }
    }
    let effect = effect_with_two_nvps("FocusDamage", "80", "HealthDamage", "10");
    let mut ctx = EffectContext {
        source_id: 1,
        target_id: 1,
        effect: &effect,
        space_mgr: &mut mgr,
    };
    RangedPhysicalDamage.on_apply(&mut ctx);
    let e = ctx.space_mgr.get_entity(1).unwrap();
    assert_eq!(e.stats.get(FOCUS).unwrap().cur, 0);
    // Spillover = 0, so final health damage is just HealthDamage = 10.
    // HP 50 - 10 = 40. With `overflow / 3` (1 spillover) it would be 39.
    assert_eq!(
        e.stats.get(HEALTH).unwrap().cur,
        40,
        "small overflow (3) truncates to zero spillover; only base \
         HealthDamage applies. Regression to overflow/3 would give 39."
    );
}

/// `param_i32` returns whatever the NVP parses to; the script
/// `.max(0)`s it, so a negative NVP value (content authoring
/// mistake) is clamped to 0 and treated as zero damage on that
/// pool — never produces "negative damage" healing.
#[test]
fn ranged_physical_negative_nvps_clamp_to_zero() {
    let mut mgr = make_mgr_with_target();
    let effect = effect_with_two_nvps("FocusDamage", "-50", "HealthDamage", "-20");
    let mut ctx = EffectContext {
        source_id: 1,
        target_id: 1,
        effect: &effect,
        space_mgr: &mut mgr,
    };
    RangedPhysicalDamage.on_apply(&mut ctx);
    let e = ctx.space_mgr.get_entity(1).unwrap();
    assert_eq!(e.stats.get(FOCUS).unwrap().cur, 200, "no focus change");
    assert_eq!(e.stats.get(HEALTH).unwrap().cur, 50, "no health change");
}

// ── MeleePhysicalDamage ──────────────────────────────────────────

/// Shield holds against a melee Focus-gated hit (Strike with
/// Focus 200, FocusDamage 100 → no overflow, no HEALTH bleed).
/// Mirror of `ranged_physical_full_focus_absorbs_no_health_damage`
/// for the melee script. Reverting the early-return after the
/// `focus_overflow == 0` gate would land a 10 HP hit here, failing
/// the assertion.
#[test]
fn melee_physical_full_focus_absorbs_no_health_damage() {
    let mut mgr = make_mgr_with_target();
    let effect = effect_with_two_nvps("FocusDamage", "100", "HealthDamage", "10");
    let mut ctx = EffectContext {
        source_id: 1,
        target_id: 1,
        effect: &effect,
        space_mgr: &mut mgr,
    };
    MeleePhysicalDamage.on_apply(&mut ctx);
    let e = ctx.space_mgr.get_entity(1).unwrap();
    assert_eq!(
        e.stats.get(FOCUS).unwrap().cur,
        100,
        "100 focus damage out of 200 must drain to 100"
    );
    assert_eq!(
        e.stats.get(HEALTH).unwrap().cur,
        50,
        "shield held → no HEALTH damage, even though HealthDamage NVP = 10"
    );
}

/// Strike-canonical NVPs (FocusDamage 100, HealthDamage 10) on a
/// target with Focus pre-depleted to 30 → 70 overflow, spillover
/// `(70*100/100)*100/300 = 23`, + HealthDamage 10 = 33 total HP
/// damage. HP 50 → 17. Same math the ranged twin uses, by design.
#[test]
fn melee_physical_partial_absorb_spills_to_health() {
    let mut mgr = make_mgr_with_target();
    if let Some(e) = mgr.get_entity_mut(1) {
        if let Some(s) = e.stats.get_mut(FOCUS) {
            s.update(0, 30, 1000);
        }
    }
    let effect = effect_with_two_nvps("FocusDamage", "100", "HealthDamage", "10");
    let mut ctx = EffectContext {
        source_id: 1,
        target_id: 1,
        effect: &effect,
        space_mgr: &mut mgr,
    };
    MeleePhysicalDamage.on_apply(&mut ctx);
    let e = ctx.space_mgr.get_entity(1).unwrap();
    assert_eq!(e.stats.get(FOCUS).unwrap().cur, 0);
    assert_eq!(
        e.stats.get(HEALTH).unwrap().cur,
        17,
        "HP 50 - (spillover 23 + HealthDamage 10) = 17"
    );
}

/// No focus at all → entire FocusDamage is overflow; spillover
/// `(100*100/100)*100/300 = 33`, + 10 = 43 HP loss. HP 50 → 7.
/// Symmetric to the ranged-side coverage so a refactor that
/// drops the empty-pool branch trips here too.
#[test]
fn melee_physical_no_focus_takes_full_overflow_spillover() {
    let mut mgr = make_mgr_with_target();
    if let Some(e) = mgr.get_entity_mut(1) {
        if let Some(s) = e.stats.get_mut(FOCUS) {
            s.update(0, 0, 1000);
        }
    }
    let effect = effect_with_two_nvps("FocusDamage", "100", "HealthDamage", "10");
    let mut ctx = EffectContext {
        source_id: 1,
        target_id: 1,
        effect: &effect,
        space_mgr: &mut mgr,
    };
    MeleePhysicalDamage.on_apply(&mut ctx);
    let e = ctx.space_mgr.get_entity(1).unwrap();
    assert_eq!(e.stats.get(FOCUS).unwrap().cur, 0);
    assert_eq!(e.stats.get(HEALTH).unwrap().cur, 7);
}

/// Missing target — return silently, no panic. The script-side
/// missing-target branch in the ranged twin had a regression
/// where an unwrap was reintroduced; pin the same shape here.
#[test]
fn melee_physical_missing_target_is_noop() {
    let mut mgr = make_mgr_with_target();
    let effect = effect_with_two_nvps("FocusDamage", "100", "HealthDamage", "10");
    let mut ctx = EffectContext {
        source_id: 1,
        target_id: 9999,
        effect: &effect,
        space_mgr: &mut mgr,
    };
    MeleePhysicalDamage.on_apply(&mut ctx);
    let e = ctx.space_mgr.get_entity(1).unwrap();
    assert_eq!(e.stats.get(FOCUS).unwrap().cur, 200);
    assert_eq!(e.stats.get(HEALTH).unwrap().cur, 50);
}

/// Both NVPs at zero → no work, no log. Coverage parity with the
/// ranged twin's zero-effect short-circuit.
#[test]
fn melee_physical_zero_nvps_skips_both_pools() {
    let mut mgr = make_mgr_with_target();
    let effect = effect_with_two_nvps("FocusDamage", "0", "HealthDamage", "0");
    let mut ctx = EffectContext {
        source_id: 1,
        target_id: 1,
        effect: &effect,
        space_mgr: &mut mgr,
    };
    MeleePhysicalDamage.on_apply(&mut ctx);
    let e = ctx.space_mgr.get_entity(1).unwrap();
    assert_eq!(e.stats.get(FOCUS).unwrap().cur, 200);
    assert_eq!(e.stats.get(HEALTH).unwrap().cur, 50);
}

/// **Edge case from PR #493 review**: `FocusDamage = 0` with
/// `HealthDamage > 0` returns early via the
/// `focus_overflow == 0` shields-held gate — no Health damage
/// applied, even though `HealthDamage` is non-zero.
///
/// This is the documented contract: with no Focus component,
/// there is no shield to pierce, so the spillover-only HEALTH
/// bleed never fires. An effect that wants flat raw melee damage
/// with no Focus interaction should use the `MeleeDamage` script
/// (reads `HealthDamage` alone), not this one.
///
/// Reverting the early-return after the `focus_overflow == 0`
/// gate would land a 10 HP hit here, failing the assertion.
#[test]
fn melee_physical_zero_focus_damage_skips_health_too() {
    let mut mgr = make_mgr_with_target();
    let effect = effect_with_two_nvps("FocusDamage", "0", "HealthDamage", "10");
    let mut ctx = EffectContext {
        source_id: 1,
        target_id: 1,
        effect: &effect,
        space_mgr: &mut mgr,
    };
    MeleePhysicalDamage.on_apply(&mut ctx);
    let e = ctx.space_mgr.get_entity(1).unwrap();
    assert_eq!(
        e.stats.get(FOCUS).unwrap().cur,
        200,
        "no Focus damage requested → Focus pool untouched"
    );
    assert_eq!(
        e.stats.get(HEALTH).unwrap().cur,
        50,
        "shields-held gate fires when there's nothing to absorb — \
         use the `MeleeDamage` script for flat-raw-HP melee damage"
    );
}

/// Negative NVPs (content authoring mistake) clamp to zero and
/// the script becomes a no-op — never produces "negative damage"
/// healing. Same `max(0)` discipline the ranged twin uses.
#[test]
fn melee_physical_negative_nvps_clamp_to_zero() {
    let mut mgr = make_mgr_with_target();
    let effect = effect_with_two_nvps("FocusDamage", "-50", "HealthDamage", "-20");
    let mut ctx = EffectContext {
        source_id: 1,
        target_id: 1,
        effect: &effect,
        space_mgr: &mut mgr,
    };
    MeleePhysicalDamage.on_apply(&mut ctx);
    let e = ctx.space_mgr.get_entity(1).unwrap();
    assert_eq!(e.stats.get(FOCUS).unwrap().cur, 200);
    assert_eq!(e.stats.get(HEALTH).unwrap().cur, 50);
}

/// Missing target on the energy path returns silently with a debug
/// log — companion to `ranged_physical_missing_target_is_noop`.
#[test]
fn ranged_energy_missing_target_is_noop() {
    let mut mgr = make_mgr_with_target();
    let effect = effect_with_two_nvps("FocusDamage", "30", "HealthDamage", "15");
    let mut ctx = EffectContext {
        source_id: 1,
        target_id: 9999,
        effect: &effect,
        space_mgr: &mut mgr,
    };
    RangedEnergyDamage.on_apply(&mut ctx);
    let e = ctx.space_mgr.get_entity(1).unwrap();
    assert_eq!(e.stats.get(FOCUS).unwrap().cur, 200);
    assert_eq!(e.stats.get(HEALTH).unwrap().cur, 50);
}

/// The first captured row with `event = <event>`.
fn row(logs: &[crate::test_support::Captured], event: &str) -> crate::test_support::Captured {
    logs.iter()
        .find(|c| c.has_field("event", event))
        .cloned()
        .unwrap_or_else(|| panic!("no `{event}` row in {logs:#?}"))
}

/// **Regression guard (AB-T2, damage scripts).** A damage script whose
/// target is gone returned without a row. It now logs
/// `effect_script_skipped` under `abilities.effect` with the script and a
/// stable reason.
#[test]
fn a_damage_script_with_no_target_logs_the_skip() {
    let mut mgr = make_mgr_with_target();
    let effect = effect_with_two_nvps("FocusDamage", "100", "HealthDamage", "10");
    let logs = crate::test_support::LogCapture::install();
    let mut ctx = EffectContext {
        source_id: 1,
        target_id: 404,
        effect: &effect,
        space_mgr: &mut mgr,
    };
    RangedPhysicalDamage.on_apply(&mut ctx);

    let r = row(&logs.all(), "effect_script_skipped");
    assert_eq!(r.target, "abilities.effect");
    assert_eq!(r.level, tracing::Level::DEBUG);
    assert!(r.has_field("script", "RangedPhysicalDamage"), "{r:?}");
    assert!(r.has_field("reason", "target_gone"), "{r:?}");
    assert!(r.has_field("target_id", "404"), "{r:?}");
}

/// **Regression guard (AB-T2, damage scripts).** A zero-NVP effect is a
/// skip with its own reason, not a silent return.
#[test]
fn a_zero_nvp_damage_script_logs_no_damage_nvp() {
    let mut mgr = make_mgr_with_target();
    let effect = effect_with_nvp("HealthDamage", "0");
    let logs = crate::test_support::LogCapture::install();
    let mut ctx = EffectContext {
        source_id: 1,
        target_id: 1,
        effect: &effect,
        space_mgr: &mut mgr,
    };
    MeleeDamage.on_apply(&mut ctx);

    let r = row(&logs.all(), "effect_script_skipped");
    assert!(r.has_field("script", "MeleeDamage"), "{r:?}");
    assert!(r.has_field("reason", "no_damage_nvp"), "{r:?}");
}

/// **Regression guard (AB-T2).** The per-hit damage rows are DEBUG: at INFO
/// every shot of every fight filled the default log index.
#[test]
fn a_damage_scripts_per_hit_row_is_debug() {
    let mut mgr = make_mgr_with_target();
    if let Some(e) = mgr.get_entity_mut(1) {
        if let Some(s) = e.stats.get_mut(FOCUS) {
            s.update(0, 30, 1000);
        }
    }
    let effect = effect_with_two_nvps("FocusDamage", "100", "HealthDamage", "10");
    let logs = crate::test_support::LogCapture::install();
    let mut ctx = EffectContext {
        source_id: 1,
        target_id: 1,
        effect: &effect,
        space_mgr: &mut mgr,
    };
    RangedPhysicalDamage.on_apply(&mut ctx);

    let all = logs.all();
    let r = row(&all, "ranged_physical_damage");
    assert!(
        r.has_field("health_damage_applied", "33"),
        "the bleed row: {r:?}"
    );
    assert_eq!(r.level, tracing::Level::DEBUG, "{r:?}");
    assert!(
        all.iter().all(|c| c.level != tracing::Level::INFO),
        "no per-hit INFO row: {all:#?}"
    );
}
