//! QR (Quality Rating) damage resolution.
//!
//! Implements the QR system used for ability effects. The QR system
//! determines hit/miss/crit outcomes via a simplified beta distribution
//! model, then applies a multi-stage damage pipeline:
//!
//!   baseDamage × qrRand × QR_DAMAGE_MULTIPLIER × (1 + damageBonus%)
//!     × (1 - statResistance) × (1 + qr) - armorFactor - absorption
//!
//! Reference: `python/cell/AbilityManager.py:13-231` (DamageCalc class)

use cimmeria_entity::abilities::{
    ClientEffectResult, DT_ENERGY, DT_HAZMAT, DT_PHYSICAL, DT_PSIONIC, RC_CRITICAL,
    RC_DOUBLE_CRITICAL, RC_GLANCING, RC_HIT, RC_MISS, SRC_MORTAL, SRC_NONE,
};
use cimmeria_entity::stats::{
    StatList, ABSORB_ENERGY, ABSORB_ENERGY_ENERGY, ABSORB_ENERGY_ITEM, ABSORB_HAZMAT,
    ABSORB_HAZMAT_ENERGY, ABSORB_HAZMAT_ITEM, ABSORB_PHYSICAL, ABSORB_PHYSICAL_ENERGY,
    ABSORB_PHYSICAL_ITEM, ABSORB_PSIONIC, ABSORB_PSIONIC_ENERGY, ABSORB_PSIONIC_ITEM,
    ABSORB_UNTYPED, ABSORB_UNTYPED_ENERGY, ABSORB_UNTYPED_ITEM, ACCURACY, AWARENESS, COORDINATION,
    DAMAGE, DEFENSE, ENERGY_AF, ENERGY_DENSITY, ENGAGEMENT, FOCUS, FORTITUDE, HAZMAT_AF,
    HAZMAT_DENSITY, HEALTH, HEALTH_RES, INTELLIGENCE, MENTAL_RES, MITIGATION, PENETRATION,
    PERCEPTION, PHYSICAL_AF, PHYSICAL_DENSITY, PSIONIC_AF, PSIONIC_DENSITY, QR_MOD,
};

// ── QR Configuration ─────────────────────────────────────────────────────────
// From `python/common/Config.py` and `python/cell/AbilityManager.py`.

/// Beta distribution base parameter. Used as the floor for both the alpha
/// and beta sides — `qr` then perturbs whichever side is on the "stronger"
/// end of the formula (see [`calculate_result`]).
const QR_ALPHA_BETA: f64 = 1.4;
/// How aggressively QR shifts the perturbed side. Python ref:
/// `python/cell/AbilityManager.py:181-184`.
const QR_MULTIPLIER: f64 = 2.0;
/// Damage scaling from QR random value.
const QR_DAMAGE_MULTIPLIER: f64 = 2.0;

// Result code thresholds (from qrRand in [0.0, 1.0])
const THRESHOLD_MISS: f64 = 0.07;
const THRESHOLD_GLANCING: f64 = 0.20;
const THRESHOLD_CRITICAL: f64 = 0.80;
const THRESHOLD_DOUBLE_CRIT: f64 = 0.93;

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

/// Map a QR value to a result code by sampling from the python reference's
/// two-branch beta distribution. From `AbilityManager.py:181-184`:
///
/// ```text
/// if qr >= 0: betavariate(α, α + qr * mult)        # β grows -> mean ↓
/// else:       betavariate(α - qr * mult, α)        # α grows -> mean ↑
/// ```
///
/// The retail damage curve depends on the beta distribution's tails (the
/// crit/double-crit bands), so a linear-bias approximation is silently
/// off-spec at every QR value other than 0 — the prior Rust port took
/// that shortcut.
///
/// Note the counter-intuitive sign convention preserved from python:
/// positive QR (attacker stronger) yields LOWER `qr_rand`; the `(1 + qr)`
/// post-multiply at [`calculate_damage`] is what compensates the damage
/// scaling. See the `mean_*` tests below for the expected per-QR
/// distribution mean.
///
/// `seed` makes the roll deterministic per (entity, ability, sequence) so
/// unit tests and live combat behave the same on the same input. Uses
/// `ChaCha8Rng` (not `StdRng`) so the deterministic stream is stable
/// across `rand` releases — `StdRng`'s underlying algorithm is allowed to
/// change between minor versions, which would silently shift replay
/// results.
pub fn calculate_result(qr: f64, seed: u64) -> QrResult {
    use rand::SeedableRng;
    use rand_chacha::ChaCha8Rng;
    use rand_distr::{Beta, Distribution};

    let (alpha, beta) = if qr >= 0.0 {
        (QR_ALPHA_BETA, QR_ALPHA_BETA + qr * QR_MULTIPLIER)
    } else {
        // qr is negative, so `-qr * mult` is positive; alpha grows.
        (QR_ALPHA_BETA - qr * QR_MULTIPLIER, QR_ALPHA_BETA)
    };
    // Both params are ≥ QR_ALPHA_BETA in their respective branches (the
    // perturbed side only ever grows), so Beta::new can't fail unless
    // QR is NaN/inf — which we treat as a programming error.
    let dist = Beta::new(alpha, beta).expect("alpha/beta both > 0 by construction");
    let mut rng = ChaCha8Rng::seed_from_u64(seed);
    let qr_rand = dist.sample(&mut rng);

    QrResult {
        qr_rand,
        result_code: qr_rand_to_result_code(qr_rand),
        qr,
    }
}

/// Pure threshold mapping from a sampled qr_rand in [0, 1] to a result code.
/// Factored out of [`calculate_result`] so the threshold table can be
/// unit-tested without coupling to the (random) beta sample.
pub fn qr_rand_to_result_code(qr_rand: f64) -> u8 {
    if qr_rand < THRESHOLD_MISS {
        RC_MISS
    } else if qr_rand < THRESHOLD_GLANCING {
        RC_GLANCING
    } else if qr_rand < THRESHOLD_CRITICAL {
        RC_HIT
    } else if qr_rand < THRESHOLD_DOUBLE_CRIT {
        RC_CRITICAL
    } else {
        RC_DOUBLE_CRITICAL
    }
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

    // ── Result-code thresholds ────────────────────────────────────────
    // Pure threshold checks decoupled from the (random) beta sample.
    // The beta sampler doesn't permit "pass me a specific qr_rand", so
    // the threshold table is exposed as its own helper for direct testing.
    #[test]
    fn result_code_miss_below_miss_threshold() {
        assert_eq!(qr_rand_to_result_code(0.03), RC_MISS);
    }

    #[test]
    fn result_code_glancing_in_glancing_band() {
        assert_eq!(qr_rand_to_result_code(0.15), RC_GLANCING);
    }

    #[test]
    fn result_code_hit_in_hit_band() {
        assert_eq!(qr_rand_to_result_code(0.5), RC_HIT);
    }

    #[test]
    fn result_code_critical_above_crit_threshold() {
        assert_eq!(qr_rand_to_result_code(0.85), RC_CRITICAL);
    }

    #[test]
    fn result_code_double_crit_above_double_crit_threshold() {
        assert_eq!(qr_rand_to_result_code(0.95), RC_DOUBLE_CRITICAL);
    }

    #[test]
    fn calculate_result_is_deterministic_per_seed() {
        // Same seed + same QR must always produce the same QR sample.
        // This is what makes combat reproducible for tests / replay.
        let a = calculate_result(0.5, 0xDEAD_BEEF);
        let b = calculate_result(0.5, 0xDEAD_BEEF);
        assert_eq!(a.qr_rand, b.qr_rand);
        assert_eq!(a.result_code, b.result_code);
    }

    // ── Beta-distribution shape ───────────────────────────────────────
    //
    // Statistical tests pinning the python-reference two-branch shape:
    //
    //   if qr >= 0: Beta(α, α + qr*mult)   → β grows → mean drops below 0.5
    //   else:       Beta(α - qr*mult, α)   → α grows → mean climbs above 0.5
    //
    // Counter-intuitive but preserved on purpose (see `calculate_result`
    // doc): positive QR pulls the mean DOWN — the `(1 + qr)` post-multiply
    // in `calculate_damage` is what makes the damage curve increase with
    // attacker advantage despite the lower sampled value.
    //
    // Tolerances are generous (~3%) because we're sampling 10k draws, not
    // computing the analytic CDF — small variance is expected.

    fn distribution_mean(qr: f64, samples: usize) -> f64 {
        let total: f64 = (0..samples)
            .map(|i| calculate_result(qr, i as u64).qr_rand)
            .sum();
        total / samples as f64
    }

    fn miss_fraction(qr: f64, samples: usize) -> f64 {
        let misses = (0..samples)
            .filter(|&i| calculate_result(qr, i as u64).result_code == RC_MISS)
            .count();
        misses as f64 / samples as f64
    }

    fn crit_or_better_fraction(qr: f64, samples: usize) -> f64 {
        let crits = (0..samples)
            .filter(|&i| {
                let rc = calculate_result(qr, i as u64).result_code;
                rc == RC_CRITICAL || rc == RC_DOUBLE_CRITICAL
            })
            .count();
        crits as f64 / samples as f64
    }

    #[test]
    fn mean_at_qr_zero_matches_beta_1_4_1_4() {
        // Beta(1.4, 1.4) is symmetric: mean α/(α+β) = 1.4/2.8 = 0.5.
        let mean = distribution_mean(0.0, 10_000);
        assert!(
            (mean - 0.5).abs() < 0.02,
            "mean at QR=0 should be ~0.5, got {mean}"
        );
    }

    #[test]
    fn mean_drops_at_positive_qr() {
        // Per python branch `qr >= 0`: Beta(1.4, 1.4 + 1*2) = Beta(1.4, 3.4),
        // mean = 1.4/4.8 ≈ 0.292. Note this is *counter-intuitive* — positive
        // QR (attacker stronger) yields LOWER qr_rand. The (1+qr) post-multiply
        // in `calculate_damage` is what makes the damage scaling work out;
        // the threshold logic is the same on both sides of QR=0.
        let mean = distribution_mean(1.0, 10_000);
        assert!(
            (mean - 0.292).abs() < 0.03,
            "mean at QR=+1 should be ~0.29, got {mean}"
        );
    }

    #[test]
    fn mean_climbs_at_negative_qr() {
        // Per python branch `qr < 0`: Beta(1.4 - (-1)*2, 1.4) = Beta(3.4, 1.4),
        // mean = 3.4/4.8 ≈ 0.708. Negative QR (defender stronger) yields
        // HIGHER qr_rand — but `(1+qr)` at calculate_damage time scales the
        // resulting damage down sharply (and to 0 once qr ≤ -1).
        let mean = distribution_mean(-1.0, 10_000);
        assert!(
            (mean - 0.708).abs() < 0.03,
            "mean at QR=-1 should be ~0.71, got {mean}"
        );
    }

    #[test]
    fn extreme_qr_does_not_panic() {
        // Both python formulas only ever GROW one of the beta parameters
        // away from QR_ALPHA_BETA, so neither side can go non-positive
        // for any finite QR. This test pins that invariant — a future
        // refactor that flipped a sign would surface here.
        let _ = calculate_result(50.0, 0xAAAA);
        let _ = calculate_result(-50.0, 0xBBBB);
    }

    #[test]
    fn crit_band_remains_reachable_at_zero_qr() {
        // Sanity: at QR=0 we should still see some crits (the right tail
        // of Beta(1.4, 1.4) extends past 0.8). Without this, a regression
        // to a degenerate distribution would silently zero out crits.
        let crit_rate = crit_or_better_fraction(0.0, 10_000);
        assert!(
            crit_rate > 0.05,
            "crit rate at QR=0 should be >5%, got {crit_rate}"
        );
    }

    #[test]
    fn negative_qr_increases_crit_band_observance() {
        // Python: negative qr → higher mean → more crits land in the
        // crit/double-crit bands at the high end. Pin this so a future
        // sign-flip in the formula (which would silently invert the
        // distribution) surfaces here.
        let crit_at_zero = crit_or_better_fraction(0.0, 10_000);
        let crit_at_neg = crit_or_better_fraction(-1.0, 10_000);
        assert!(
            crit_at_neg > crit_at_zero,
            "neg QR should crit MORE per python (counter-intuitive); zero={crit_at_zero}, neg={crit_at_neg}"
        );
    }

    #[test]
    fn positive_qr_increases_miss_band_observance() {
        // Python: positive qr → lower mean → more samples land in the
        // miss/glancing bands at the low end. Same regression-pin as
        // `negative_qr_increases_crit_band_observance` for the other side.
        let miss_at_zero = miss_fraction(0.0, 10_000);
        let miss_at_pos = miss_fraction(1.0, 10_000);
        assert!(
            miss_at_pos > miss_at_zero,
            "pos QR should miss MORE per python (counter-intuitive); zero={miss_at_zero}, pos={miss_at_pos}"
        );
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
}
