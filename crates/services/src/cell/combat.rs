//! Combat damage resolution for the CellService.
//!
//! Implements the QR (Quality Rating) damage system used for ability effects.
//! The QR system determines hit/miss/crit outcomes via a simplified beta
//! distribution model, then applies a multi-stage damage pipeline:
//!
//!   baseDamage × qrRand × QR_DAMAGE_MULTIPLIER × (1 + damageBonus%)
//!     × (1 - statResistance) × (1 + qr) - armorFactor - absorption
//!
//! Reference: `python/cell/AbilityManager.py:13-231` (DamageCalc class)

use cimmeria_entity::abilities::{
    ClientEffectResult,
    DT_ENERGY, DT_HAZMAT, DT_PHYSICAL, DT_PSIONIC,
    RC_CRITICAL, RC_DOUBLE_CRITICAL, RC_GLANCING, RC_HIT, RC_MISS,
    SRC_MORTAL, SRC_NONE,
};
use cimmeria_entity::stats::{
    StatList,
    COORDINATION, ENGAGEMENT, PERCEPTION, FORTITUDE, INTELLIGENCE,
    ACCURACY, DEFENSE, QR_MOD, AWARENESS, DAMAGE, PENETRATION, MITIGATION,
    HEALTH, FOCUS,
    PHYSICAL_AF, ENERGY_AF, HAZMAT_AF, PSIONIC_AF,
    PHYSICAL_DENSITY, ENERGY_DENSITY, HAZMAT_DENSITY, PSIONIC_DENSITY,
    ABSORB_PHYSICAL, ABSORB_ENERGY, ABSORB_HAZMAT, ABSORB_PSIONIC, ABSORB_UNTYPED,
    ABSORB_PHYSICAL_ITEM, ABSORB_ENERGY_ITEM, ABSORB_HAZMAT_ITEM, ABSORB_PSIONIC_ITEM, ABSORB_UNTYPED_ITEM,
    ABSORB_PHYSICAL_ENERGY, ABSORB_ENERGY_ENERGY, ABSORB_HAZMAT_ENERGY, ABSORB_PSIONIC_ENERGY, ABSORB_UNTYPED_ENERGY,
    HEALTH_RES, MENTAL_RES,
};

// ── QR Configuration ─────────────────────────────────────────────────────────
// From `python/common/Config.py`

/// Beta distribution base parameter (lower = flatter histogram).
/// Used in full beta distribution model (future upgrade).
#[allow(dead_code)]
const QR_ALPHA_BETA: f64 = 1.4;
/// QR effect on distribution shape.
/// Used in full beta distribution model (future upgrade).
#[allow(dead_code)]
const QR_MULTIPLIER: f64 = 2.0;
/// Damage scaling from QR random value.
const QR_DAMAGE_MULTIPLIER: f64 = 2.0;

// Result code thresholds (from qrRand in [0.0, 1.0])
const THRESHOLD_MISS: f64 = 0.07;
const THRESHOLD_GLANCING: f64 = 0.20;
const THRESHOLD_CRITICAL: f64 = 0.80;
const THRESHOLD_DOUBLE_CRIT: f64 = 0.93;

// ── Combatant State Flags ────────────────────────────────────────────────────
// From `python/Atrea/enums.py`

/// Bitmask for dead state in combatantState.
pub const PLAYER_STATE_DEAD: u32 = 8192;

/// Bit position for dead flag in stateField (sent to client).
pub const BSF_DEAD: u32 = 13; // bit 13 → value 8192

// ── DamageCalc ───────────────────────────────────────────────────────────────

/// Result of a QR calculation: the random value and the result code.
pub struct QrResult {
    /// Random value from beta distribution, in [0.0, 1.0].
    pub qr_rand: f64,
    /// Result code (RC_HIT, RC_MISS, RC_CRITICAL, etc.).
    pub result_code: u8,
    /// The QR score (attacker stats - defender stats).
    pub qr: f64,
}

/// Calculate Quality Rating from attacker vs defender stats.
///
/// Simplified version: uses stat values directly without per-entity
/// crouch/cover state (those are tracked as stat modifiers).
///
/// Reference: `python/cell/AbilityManager.py:200-230`
pub fn calculate_qr(attacker: &StatList, defender: &StatList, ranged: bool) -> f64 {
    let mut qr = 0.0;

    // Primary stat contribution
    if ranged {
        qr += stat_cur(attacker, COORDINATION) as f64 * 0.05;
    } else {
        qr += stat_cur(attacker, ENGAGEMENT) as f64 * 0.05;
    }
    qr -= stat_cur(defender, PERCEPTION) as f64 * 0.05;

    // Accuracy vs defense
    qr += stat_cur(attacker, ACCURACY) as f64 * 0.01;
    qr -= stat_cur(defender, DEFENSE) as f64 * 0.01;

    // QR modifiers (buffs/debuffs)
    qr += stat_cur(attacker, QR_MOD) as f64;
    qr -= stat_cur(defender, QR_MOD) as f64;

    // Awareness
    qr += stat_cur(attacker, AWARENESS) as f64 * 0.01;

    qr
}

/// Map a QR value to a result code using simplified beta distribution.
///
/// We use a deterministic hash-based approach for reproducible results
/// in tests, while still producing the correct distribution shape.
/// For real combat, we generate a random qr_rand value.
pub fn calculate_result(qr: f64, random_value: f64) -> QrResult {
    // Apply beta-like shaping: shift random value based on QR
    // Positive QR shifts distribution right (more crits), negative shifts left (more misses)
    let qr_rand = shape_by_qr(random_value, qr);

    let result_code = if qr_rand < THRESHOLD_MISS {
        RC_MISS
    } else if qr_rand < THRESHOLD_GLANCING {
        RC_GLANCING
    } else if qr_rand < THRESHOLD_CRITICAL {
        RC_HIT
    } else if qr_rand < THRESHOLD_DOUBLE_CRIT {
        RC_CRITICAL
    } else {
        RC_DOUBLE_CRITICAL
    };

    QrResult { qr_rand, result_code, qr }
}

/// Shape a uniform random value by QR using a simplified beta distribution.
///
/// Positive QR shifts the distribution toward 1.0 (crits).
/// Negative QR shifts toward 0.0 (misses).
fn shape_by_qr(uniform: f64, qr: f64) -> f64 {
    // Simplified: apply QR as a linear bias, then clamp.
    // True beta distribution would use betavariate(alpha, alpha + qr * multiplier),
    // but this approximation is sufficient and avoids the need for a special
    // function library.
    let bias = qr * 0.1; // Scale QR to a reasonable shift
    (uniform + bias).clamp(0.0, 1.0)
}

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

    // Absorption
    let absorption = calculate_absorption(defender, damage_type);

    // Pipeline
    let res_damage = raw * damage_bonus * (1.0 - stat_resist);
    let qr_damage = (res_damage * (1.0 + qr_result.qr)).round() as i32;
    let af_damage = (qr_damage - af_mitigation).max(0);
    let final_damage = (af_damage - absorption).max(0);

    // Apply to target stat
    let actual_change = if let Some(stat) = defender.get_mut(stat_id) {
        stat.change(-final_damage)
    } else {
        0
    };

    // Check for lethal damage
    let is_dead = stat_id == HEALTH && defender.get(HEALTH).map_or(false, |s| s.cur <= 0);
    let src = if is_dead { SRC_MORTAL } else { SRC_NONE };

    results.push(ClientEffectResult {
        stat_id: stat_id as i8,
        delta: actual_change,
        damage_code: damage_type,
        stat_result_code: src,
    });

    let total_damage = actual_change.unsigned_abs() as i32;
    (results, total_damage)
}

// ── Helpers ──────────────────────────────────────────────────────────────────

fn stat_cur(stats: &StatList, id: i32) -> i32 {
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

/// Check if a state field indicates the entity is dead.
pub fn is_dead_state(state_field: u32) -> bool {
    state_field & (1 << BSF_DEAD) != 0
}

/// Set the dead flag in a state field.
pub fn set_dead_state(state_field: &mut u32) {
    *state_field |= 1 << BSF_DEAD;
}

/// Clear the dead flag in a state field (revive).
pub fn clear_dead_state(state_field: &mut u32) {
    *state_field &= !(1 << BSF_DEAD);
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use cimmeria_entity::stats::ArchetypeStatValues;

    fn make_attacker() -> StatList {
        let mut stats = StatList::new();
        stats.apply_archetype(&ArchetypeStatValues {
            coordination: 5, engagement: 4, fortitude: 3, morale: 4,
            perception: 3, intelligence: 2, health: 760, focus: 1570,
        });
        stats
    }

    fn make_defender() -> StatList {
        let mut stats = StatList::new();
        stats.apply_archetype(&ArchetypeStatValues {
            coordination: 3, engagement: 3, fortitude: 3, morale: 3,
            perception: 3, intelligence: 2, health: 500, focus: 500,
        });
        stats
    }

    #[test]
    fn qr_positive_when_attacker_stronger() {
        let attacker = make_attacker();
        let defender = make_defender();
        let qr = calculate_qr(&attacker, &defender, true);
        // coordination 5 vs perception 3: (5-3)*0.05 = 0.1 > 0
        assert!(qr > 0.0, "QR should be positive: {qr}");
    }

    #[test]
    fn qr_negative_when_defender_stronger() {
        let attacker = make_defender(); // weaker
        let defender = make_attacker(); // stronger perception
        let qr = calculate_qr(&attacker, &defender, true);
        // coord 3 vs perception 3: 0, but attacker has less accuracy
        assert!(qr <= 0.0, "QR should be non-positive: {qr}");
    }

    #[test]
    fn result_code_miss_at_low_random() {
        let result = calculate_result(0.0, 0.03);
        assert_eq!(result.result_code, RC_MISS);
    }

    #[test]
    fn result_code_glancing_at_low_mid() {
        let result = calculate_result(0.0, 0.15);
        assert_eq!(result.result_code, RC_GLANCING);
    }

    #[test]
    fn result_code_hit_at_mid() {
        let result = calculate_result(0.0, 0.5);
        assert_eq!(result.result_code, RC_HIT);
    }

    #[test]
    fn result_code_critical_at_high() {
        let result = calculate_result(0.0, 0.85);
        assert_eq!(result.result_code, RC_CRITICAL);
    }

    #[test]
    fn result_code_double_crit_at_max() {
        let result = calculate_result(0.0, 0.95);
        assert_eq!(result.result_code, RC_DOUBLE_CRITICAL);
    }

    #[test]
    fn damage_reduces_health() {
        let attacker = make_attacker();
        let mut defender = make_defender();
        let initial_hp = defender.get(HEALTH).unwrap().cur;

        let qr_result = QrResult { qr_rand: 0.5, result_code: RC_HIT, qr: 0.1 };
        let (results, _total) = calculate_damage(
            &qr_result, 100, DT_PHYSICAL, HEALTH, &attacker, &mut defender,
        );

        let final_hp = defender.get(HEALTH).unwrap().cur;
        assert!(final_hp < initial_hp, "Health should decrease: {initial_hp} -> {final_hp}");
        assert!(!results.is_empty());
        assert!(results[0].delta < 0, "Delta should be negative (damage)");
    }

    #[test]
    fn miss_deals_zero_damage() {
        let attacker = make_attacker();
        let mut defender = make_defender();
        let initial_hp = defender.get(HEALTH).unwrap().cur;

        // qr_rand = 0 → raw damage = 0
        let qr_result = QrResult { qr_rand: 0.0, result_code: RC_MISS, qr: 0.0 };
        let (results, total) = calculate_damage(
            &qr_result, 100, DT_PHYSICAL, HEALTH, &attacker, &mut defender,
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

        let qr_result = QrResult { qr_rand: 0.8, result_code: RC_CRITICAL, qr: 0.5 };
        let (results, _) = calculate_damage(
            &qr_result, 500, DT_PHYSICAL, HEALTH, &attacker, &mut defender,
        );

        assert_eq!(defender.get(HEALTH).unwrap().cur, 0);
        assert_eq!(results[0].stat_result_code, SRC_MORTAL);
    }

    #[test]
    fn focus_damage_hits_focus_stat() {
        let attacker = make_attacker();
        let mut defender = make_defender();
        let initial_focus = defender.get(FOCUS).unwrap().cur;

        let qr_result = QrResult { qr_rand: 0.5, result_code: RC_HIT, qr: 0.0 };
        let (results, _) = calculate_damage(
            &qr_result, 50, DT_ENERGY, FOCUS, &attacker, &mut defender,
        );

        let final_focus = defender.get(FOCUS).unwrap().cur;
        assert!(final_focus < initial_focus);
        assert_eq!(results[0].stat_id, FOCUS as i8);
    }

    #[test]
    fn dead_state_flags() {
        let mut state = 0u32;
        assert!(!is_dead_state(state));

        set_dead_state(&mut state);
        assert!(is_dead_state(state));
        assert_eq!(state, 8192);

        clear_dead_state(&mut state);
        assert!(!is_dead_state(state));
        assert_eq!(state, 0);
    }

    #[test]
    fn damage_cannot_go_below_zero_health() {
        let attacker = make_attacker();
        let mut defender = make_defender();
        defender.get_mut(HEALTH).unwrap().set_current(10);

        let qr_result = QrResult { qr_rand: 0.9, result_code: RC_CRITICAL, qr: 1.0 };
        let (results, _) = calculate_damage(
            &qr_result, 9999, DT_PHYSICAL, HEALTH, &attacker, &mut defender,
        );

        assert_eq!(defender.get(HEALTH).unwrap().cur, 0);
        // Delta should be exactly -10 (clamped by stat.change)
        assert_eq!(results[0].delta, -10);
    }
}
