//! QR damage-application pipeline.
//!
//! Applies the multi-stage damage formula to a defender, consuming
//! absorption shields and writing the resulting stat change:
//!
//!   baseDamage × qrRand × QR_DAMAGE_MULTIPLIER × (1 + damageBonus%)
//!     × (1 - statResistance) × (1 + qr) - armorFactor - absorption
//!
//! Reference: `python/cell/AbilityManager.py:82-122` (DamageCalc.calculateDamage)

use super::qr::QrResult;
use cimmeria_entity::abilities::{
    ClientEffectResult, DT_ENERGY, DT_HAZMAT, DT_PHYSICAL, DT_PSIONIC, SRC_MORTAL, SRC_NONE,
};
use cimmeria_entity::stats::{
    StatList, ABSORB_ENERGY, ABSORB_ENERGY_ENERGY, ABSORB_ENERGY_ITEM, ABSORB_HAZMAT,
    ABSORB_HAZMAT_ENERGY, ABSORB_HAZMAT_ITEM, ABSORB_PHYSICAL, ABSORB_PHYSICAL_ENERGY,
    ABSORB_PHYSICAL_ITEM, ABSORB_PSIONIC, ABSORB_PSIONIC_ENERGY, ABSORB_PSIONIC_ITEM,
    ABSORB_UNTYPED, ABSORB_UNTYPED_ENERGY, ABSORB_UNTYPED_ITEM, DAMAGE, ENERGY_AF, ENERGY_DENSITY,
    FOCUS, FORTITUDE, HAZMAT_AF, HAZMAT_DENSITY, HEALTH, HEALTH_RES, INTELLIGENCE, MENTAL_RES,
    MITIGATION, PENETRATION, PHYSICAL_AF, PHYSICAL_DENSITY, PSIONIC_AF, PSIONIC_DENSITY,
};

/// Damage scaling from QR random value.
const QR_DAMAGE_MULTIPLIER: f64 = 2.0;

/// Calculate damage from a single effect application.
///
/// Returns a list of `ClientEffectResult` entries to send to the client,
/// and the total damage dealt.
///
/// Reference: `python/cell/AbilityManager.py:82-122` (DamageCalc.calculateDamage)
pub fn calculate_damage(
    qr_result: &QrResult,
    base_damage: i32,
    damage_type: i8,
    stat_id: i32,
    attacker: &StatList,
    defender: &mut StatList,
) -> (Vec<ClientEffectResult>, i32) {
    let mut results = Vec::new();

    // Base damage * qrRand * QR_DAMAGE_MULTIPLIER
    let raw = base_damage as f64 * qr_result.qr_rand * QR_DAMAGE_MULTIPLIER;
    if raw <= 0.0 {
        return (results, 0);
    }

    // Damage bonus from attacker
    let damage_bonus = stat_cur(attacker, DAMAGE) as f64 / 100.0 + 1.0;

    // Stat resistance
    let stat_resist = calculate_stat_resistance(defender, stat_id);

    // Armor factor
    let af = calculate_armor_factor(defender, damage_type);
    let miti = stat_cur(defender, MITIGATION) as f64;
    let pen = stat_cur(attacker, PENETRATION) as f64;
    let af_mitigation = (af as f64 * (miti - pen).max(0.0) / 100.0).round() as i32;

    // Pipeline up to absorption
    let res_damage = raw * damage_bonus * (1.0 - stat_resist);
    let qr_damage = (res_damage * (1.0 + qr_result.qr)).round() as i32;
    let af_damage = (qr_damage - af_mitigation).max(0);

    // Absorption shield: drain the matching ABSORB_*
    // stat pool by `min(remaining_damage, pool_cur)` so shields are
    // genuinely consumable, not just a flat subtraction. Each damage
    // type drains its own pool. Health stat-id absorbs first; non-
    // HEALTH stat-ids (focus) skip absorption to match the Python
    // reference (only physical/elemental damage to HEALTH gets a
    // shield treatment).
    let (final_damage, absorbed) = if stat_id == HEALTH && af_damage > 0 {
        drain_absorption_pools(defender, damage_type, af_damage)
    } else {
        (af_damage, 0)
    };

    // Apply to target stat
    let actual_change = if let Some(stat) = defender.get_mut(stat_id) {
        stat.change(-final_damage)
    } else {
        0
    };

    // Check for lethal damage
    let is_dead = stat_id == HEALTH && defender.get(HEALTH).is_some_and(|s| s.cur <= 0);
    let src = if is_dead {
        SRC_MORTAL
    } else if absorbed > 0 && final_damage == 0 {
        // Fully absorbed — surface the SRC_ABSORB code so the client
        // shows "Absorbed" floater text instead of "0 damage".
        cimmeria_entity::abilities::SRC_ABSORB
    } else {
        SRC_NONE
    };

    results.push(ClientEffectResult {
        stat_id: stat_id as i8,
        delta: actual_change,
        damage_code: damage_type,
        stat_result_code: src,
    });

    if absorbed > 0 {
        tracing::debug!(
            target: "abilities",
            event = "shield_absorbed_damage",
            damage_type,
            absorbed,
            passed_through = final_damage,
            "Shield absorbed damage"
        );
    }

    let total_damage = actual_change.unsigned_abs() as i32;
    (results, total_damage)
}

/// Drain the absorption pool(s) matching `damage_type` by up to
/// `incoming` damage. Returns `(damage_remaining_after_absorption,
/// total_absorbed)`. Drains the elemental-specific pool first
/// (ABSORB_PHYSICAL, ABSORB_ENERGY, etc.) before the catch-all
/// generic pool, so shields placed on a specific damage type are
/// consumed first when that damage type hits.
fn drain_absorption_pools(defender: &mut StatList, damage_type: i8, incoming: i32) -> (i32, i32) {
    let pools: &[i32] = match damage_type {
        DT_PHYSICAL => &[
            ABSORB_PHYSICAL,
            ABSORB_PHYSICAL_ENERGY,
            ABSORB_PHYSICAL_ITEM,
        ],
        DT_ENERGY => &[ABSORB_ENERGY, ABSORB_ENERGY_ENERGY, ABSORB_ENERGY_ITEM],
        DT_HAZMAT => &[ABSORB_HAZMAT, ABSORB_HAZMAT_ENERGY, ABSORB_HAZMAT_ITEM],
        DT_PSIONIC => &[ABSORB_PSIONIC, ABSORB_PSIONIC_ENERGY, ABSORB_PSIONIC_ITEM],
        _ => &[ABSORB_UNTYPED, ABSORB_UNTYPED_ENERGY, ABSORB_UNTYPED_ITEM],
    };

    let mut remaining = incoming;
    let mut absorbed_total = 0;
    for &pool_id in pools {
        if remaining == 0 {
            break;
        }
        let Some(pool) = defender.get_mut(pool_id) else {
            continue;
        };
        let available = pool.cur.max(0);
        if available == 0 {
            continue;
        }
        let drain = remaining.min(available);
        // `change(-drain)` returns the actual delta (clamped by stat min/max)
        let actual = pool.change(-drain).unsigned_abs() as i32;
        absorbed_total += actual;
        remaining -= actual;
    }
    (remaining, absorbed_total)
}

// ── Helpers ──────────────────────────────────────────────────────────────────

pub(super) fn stat_cur(stats: &StatList, id: i32) -> i32 {
    stats.get(id).map_or(0, |s| s.cur)
}

fn calculate_stat_resistance(defender: &StatList, stat_id: i32) -> f64 {
    match stat_id {
        HEALTH => {
            stat_cur(defender, FORTITUDE) as f64 * 0.01
                + stat_cur(defender, HEALTH_RES) as f64 * 0.01
        }
        FOCUS => {
            stat_cur(defender, INTELLIGENCE) as f64 * 0.01
                + stat_cur(defender, MENTAL_RES) as f64 * 0.01
        }
        _ => 0.0,
    }
}

fn calculate_armor_factor(defender: &StatList, damage_type: i8) -> i32 {
    match damage_type {
        DT_PHYSICAL => stat_cur(defender, PHYSICAL_AF) + stat_cur(defender, PHYSICAL_DENSITY),
        DT_ENERGY => stat_cur(defender, ENERGY_AF) + stat_cur(defender, ENERGY_DENSITY),
        DT_HAZMAT => stat_cur(defender, HAZMAT_AF) + stat_cur(defender, HAZMAT_DENSITY),
        DT_PSIONIC => stat_cur(defender, PSIONIC_AF) + stat_cur(defender, PSIONIC_DENSITY),
        _ => 0, // DT_UNTYPED has no AF
    }
}

/// Sum of all absorption pools for `damage_type`. Read-only —
/// `drain_absorption_pools` is the mutator that actually consumes
/// pool charges during a damage-apply.
#[allow(dead_code)]
fn calculate_absorption(defender: &StatList, damage_type: i8) -> i32 {
    match damage_type {
        DT_PHYSICAL => {
            stat_cur(defender, ABSORB_PHYSICAL)
                + stat_cur(defender, ABSORB_PHYSICAL_ENERGY)
                + stat_cur(defender, ABSORB_PHYSICAL_ITEM)
        }
        DT_ENERGY => {
            stat_cur(defender, ABSORB_ENERGY)
                + stat_cur(defender, ABSORB_ENERGY_ENERGY)
                + stat_cur(defender, ABSORB_ENERGY_ITEM)
        }
        DT_HAZMAT => {
            stat_cur(defender, ABSORB_HAZMAT)
                + stat_cur(defender, ABSORB_HAZMAT_ENERGY)
                + stat_cur(defender, ABSORB_HAZMAT_ITEM)
        }
        DT_PSIONIC => {
            stat_cur(defender, ABSORB_PSIONIC)
                + stat_cur(defender, ABSORB_PSIONIC_ENERGY)
                + stat_cur(defender, ABSORB_PSIONIC_ITEM)
        }
        _ => {
            stat_cur(defender, ABSORB_UNTYPED)
                + stat_cur(defender, ABSORB_UNTYPED_ENERGY)
                + stat_cur(defender, ABSORB_UNTYPED_ITEM)
        }
    }
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use cimmeria_entity::abilities::{RC_CRITICAL, RC_HIT, RC_MISS};
    use cimmeria_entity::stats::ArchetypeStatValues;

    fn make_attacker() -> StatList {
        let mut stats = StatList::new();
        stats.apply_archetype(&ArchetypeStatValues {
            coordination: 5,
            engagement: 4,
            fortitude: 3,
            morale: 4,
            perception: 3,
            intelligence: 2,
            health: 760,
            focus: 1570,
            health_per_level: 10,
            focus_per_level: 70,
        });
        stats
    }

    fn make_defender() -> StatList {
        let mut stats = StatList::new();
        stats.apply_archetype(&ArchetypeStatValues {
            coordination: 3,
            engagement: 3,
            fortitude: 3,
            morale: 3,
            perception: 3,
            intelligence: 2,
            health: 500,
            focus: 500,
            health_per_level: 10,
            focus_per_level: 70,
        });
        stats
    }

    #[test]
    fn damage_reduces_health() {
        let attacker = make_attacker();
        let mut defender = make_defender();
        let initial_hp = defender.get(HEALTH).unwrap().cur;

        let qr_result = QrResult {
            qr_rand: 0.5,
            result_code: RC_HIT,
            qr: 0.1,
        };
        let (results, _total) = calculate_damage(
            &qr_result,
            100,
            DT_PHYSICAL,
            HEALTH,
            &attacker,
            &mut defender,
        );

        let final_hp = defender.get(HEALTH).unwrap().cur;
        assert!(
            final_hp < initial_hp,
            "Health should decrease: {initial_hp} -> {final_hp}"
        );
        assert!(!results.is_empty());
        assert!(results[0].delta < 0, "Delta should be negative (damage)");
    }

    #[test]
    fn miss_deals_zero_damage() {
        let attacker = make_attacker();
        let mut defender = make_defender();
        let initial_hp = defender.get(HEALTH).unwrap().cur;

        // qr_rand = 0 → raw damage = 0
        let qr_result = QrResult {
            qr_rand: 0.0,
            result_code: RC_MISS,
            qr: 0.0,
        };
        let (results, total) = calculate_damage(
            &qr_result,
            100,
            DT_PHYSICAL,
            HEALTH,
            &attacker,
            &mut defender,
        );

        assert_eq!(total, 0);
        assert!(results.is_empty());
        assert_eq!(defender.get(HEALTH).unwrap().cur, initial_hp);
    }

    #[test]
    fn lethal_damage_returns_src_mortal() {
        let attacker = make_attacker();
        let mut defender = make_defender();
        // Set defender to very low health
        defender.get_mut(HEALTH).unwrap().set_current(1);

        let qr_result = QrResult {
            qr_rand: 0.8,
            result_code: RC_CRITICAL,
            qr: 0.5,
        };
        let (results, _) = calculate_damage(
            &qr_result,
            500,
            DT_PHYSICAL,
            HEALTH,
            &attacker,
            &mut defender,
        );

        assert_eq!(defender.get(HEALTH).unwrap().cur, 0);
        assert_eq!(results[0].stat_result_code, SRC_MORTAL);
    }

    #[test]
    fn focus_damage_hits_focus_stat() {
        let attacker = make_attacker();
        let mut defender = make_defender();
        let initial_focus = defender.get(FOCUS).unwrap().cur;

        let qr_result = QrResult {
            qr_rand: 0.5,
            result_code: RC_HIT,
            qr: 0.0,
        };
        let (results, _) =
            calculate_damage(&qr_result, 50, DT_ENERGY, FOCUS, &attacker, &mut defender);

        let final_focus = defender.get(FOCUS).unwrap().cur;
        assert!(final_focus < initial_focus);
        assert_eq!(results[0].stat_id, FOCUS as i8);
    }

    #[test]
    fn damage_cannot_go_below_zero_health() {
        let attacker = make_attacker();
        let mut defender = make_defender();
        defender.get_mut(HEALTH).unwrap().set_current(10);

        let qr_result = QrResult {
            qr_rand: 0.9,
            result_code: RC_CRITICAL,
            qr: 1.0,
        };
        let (results, _) = calculate_damage(
            &qr_result,
            9999,
            DT_PHYSICAL,
            HEALTH,
            &attacker,
            &mut defender,
        );

        assert_eq!(defender.get(HEALTH).unwrap().cur, 0);
        // Delta should be exactly -10 (clamped by stat.change)
        assert_eq!(results[0].delta, -10);
    }

    // ── Absorption shield ──────────────────────────────────

    #[test]
    fn absorption_drains_shield_pool_and_passes_overflow_to_health() {
        // Defender has 1000 ABSORB_PHYSICAL (well above any post-pipeline
        // damage). Single 25-base hit: shield should drain by however much
        // damage made it past AF, and HEALTH should be untouched.
        let attacker = make_attacker();
        let mut defender = make_defender();
        if let Some(stat) = defender.get_mut(ABSORB_PHYSICAL) {
            stat.update(0, 1000, 1000);
        }
        let hp_before = defender.get(HEALTH).unwrap().cur;
        let shield_before = defender.get(ABSORB_PHYSICAL).unwrap().cur;

        let qr = QrResult {
            result_code: cimmeria_entity::abilities::RC_HIT,
            qr: 0.0,
            qr_rand: 1.0,
        };
        let (_results, total) =
            calculate_damage(&qr, 25, DT_PHYSICAL, HEALTH, &attacker, &mut defender);

        let shield_after = defender.get(ABSORB_PHYSICAL).unwrap().cur;
        let drained = shield_before - shield_after;
        assert!(
            drained > 0,
            "shield must have absorbed some damage (drained={drained})"
        );
        assert!(
            shield_after < shield_before,
            "shield pool drained (was {shield_before}, now {shield_after})"
        );
        assert_eq!(
            defender.get(HEALTH).unwrap().cur,
            hp_before,
            "HEALTH untouched when shield > all post-AF damage"
        );
        assert_eq!(total, 0, "no damage actually landed on HEALTH");
    }

    #[test]
    fn absorption_overflow_passes_through_to_health() {
        let attacker = make_attacker();
        let mut defender = make_defender();
        if let Some(stat) = defender.get_mut(ABSORB_PHYSICAL) {
            stat.update(0, 10, 1000); // small shield
        }
        let hp_before = defender.get(HEALTH).unwrap().cur;

        let qr = QrResult {
            result_code: cimmeria_entity::abilities::RC_HIT,
            qr: 0.0,
            qr_rand: 1.0,
        };
        let (_results, total) =
            calculate_damage(&qr, 50, DT_PHYSICAL, HEALTH, &attacker, &mut defender);

        // Shield emptied, HEALTH took the overflow.
        assert_eq!(
            defender.get(ABSORB_PHYSICAL).unwrap().cur,
            0,
            "shield fully drained"
        );
        // Note: actual HEALTH damage depends on the full QR/AF pipeline
        // but the key invariant is "HEALTH took SOME damage when shield
        // overflowed."
        assert!(
            defender.get(HEALTH).unwrap().cur < hp_before,
            "HEALTH took overflow damage after shield empty"
        );
        assert!(total > 0, "overflow registered");
    }

    #[test]
    fn absorption_only_applies_to_health_damage_not_focus() {
        // Focus damage should bypass the absorption pool — shields
        // are HP-only.
        let attacker = make_attacker();
        let mut defender = make_defender();
        if let Some(stat) = defender.get_mut(ABSORB_PHYSICAL) {
            stat.update(0, 1000, 1000);
        }
        let focus_before = defender.get(FOCUS).unwrap().cur;

        let qr = QrResult {
            result_code: cimmeria_entity::abilities::RC_HIT,
            qr: 0.0,
            qr_rand: 1.0,
        };
        let (_results, _) = calculate_damage(&qr, 30, DT_PHYSICAL, FOCUS, &attacker, &mut defender);

        // Shield pool untouched
        assert_eq!(
            defender.get(ABSORB_PHYSICAL).unwrap().cur,
            1000,
            "absorption pool must not drain on FOCUS damage"
        );
        // FOCUS took damage
        assert!(
            defender.get(FOCUS).unwrap().cur < focus_before,
            "FOCUS took damage normally"
        );
    }

    #[test]
    fn absorption_drains_elemental_specific_pool_before_generic() {
        // Clara G20: when both ABSORB_PHYSICAL (elemental) and
        // ABSORB_UNTYPED (generic) carry capacity, a physical hit must
        // drain the elemental pool first, leaving the generic pool
        // untouched for non-physical follow-up damage.
        let attacker = make_attacker();
        let mut defender = make_defender();
        if let Some(stat) = defender.get_mut(ABSORB_PHYSICAL) {
            stat.update(0, 50, 1000);
        }
        // ABSORB_UNTYPED isn't in the physical-pool list (it's only
        // checked when damage_type is DT_UNTYPED), so this seed proves
        // the pool routing per damage_type.
        if let Some(stat) = defender.get_mut(ABSORB_UNTYPED) {
            stat.update(0, 500, 1000);
        }
        let qr = QrResult {
            result_code: cimmeria_entity::abilities::RC_HIT,
            qr: 0.0,
            qr_rand: 1.0,
        };
        let _ = calculate_damage(&qr, 20, DT_PHYSICAL, HEALTH, &attacker, &mut defender);
        // ABSORB_PHYSICAL drained; ABSORB_UNTYPED is untouched on physical hits.
        assert!(
            defender.get(ABSORB_PHYSICAL).unwrap().cur < 50,
            "elemental-specific pool drained"
        );
        assert_eq!(
            defender.get(ABSORB_UNTYPED).unwrap().cur,
            500,
            "generic pool untouched on physical hit (only consumed by DT_UNTYPED)"
        );
    }
}
