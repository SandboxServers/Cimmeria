//! Health-percentage snapshots for the `entity_health_below` content
//! trigger (Harset H04).
//!
//! The trigger's "once per downward crossing" semantics are stateless:
//! the damage path samples the target's health percentage before and
//! after a hit and hands both numbers to the content engine, which fires
//! a chain only when `pct_before > threshold && pct_after <= threshold`.
//! Nothing latches per entity, so a heal back above the threshold and a
//! second crossing fire again — and the firing site never needs to know
//! which thresholds content has seeded.
//!
//! This module owns only the percentage arithmetic, deliberately split
//! from the dispatch site so the edge cases (zero/negative max, overheal,
//! negative current health on a killing blow) are unit-testable without
//! a `SpaceManager`.

use cimmeria_entity::cell_entity::CellEntity;
use cimmeria_entity::stats::HEALTH;

/// A target's health at one instant, as a percentage of its maximum.
///
/// `None` when the entity has no HEALTH stat or a non-positive maximum —
/// a percentage is undefined there and the caller must skip the trigger
/// rather than invent one.
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct HealthPct(pub f64);

/// Compute a health percentage from raw current/maximum values.
///
/// Returns `None` for a non-positive `max` (uninitialised stat block,
/// or a debuff that zeroed the maximum) — dividing there would produce
/// an infinity or a NaN, and NaN compares false against every threshold,
/// which would silently disable the trigger instead of skipping it
/// loudly at the call site.
///
/// `cur` is clamped at zero on the low side: a killing blow can drive
/// `cur` negative (overkill damage is not floored by the stat setter),
/// and a negative percentage would still satisfy every `<= threshold`
/// test. The kill case is routed to `entity_dead_tag` by the caller
/// regardless, but clamping keeps this function honest on its own.
/// Overheal above `max` is left unclamped: a value above 100 only makes
/// the `pct_before > threshold` half of the crossing test more true,
/// which is the correct reading of "the entity was above the threshold".
pub fn health_pct_from(cur: i32, max: i32) -> Option<HealthPct> {
    if max <= 0 {
        return None;
    }
    let cur = cur.max(0);
    Some(HealthPct(f64::from(cur) * 100.0 / f64::from(max)))
}

/// Sample an entity's current health percentage.
pub fn health_pct(entity: &CellEntity) -> Option<HealthPct> {
    let stat = entity.stats.get(HEALTH)?;
    health_pct_from(stat.cur, stat.max)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn full_health_is_one_hundred_percent() {
        assert_eq!(health_pct_from(500, 500), Some(HealthPct(100.0)));
    }

    #[test]
    fn half_health_is_fifty_percent() {
        assert_eq!(health_pct_from(250, 500), Some(HealthPct(50.0)));
    }

    /// Integer division would report 0% for any NPC with more than 100
    /// max health at low absolute health — the arithmetic must be done
    /// in floating point or a 3/500 HP boss reads as dead-but-alive.
    #[test]
    fn small_remainders_are_not_truncated_to_zero() {
        let pct = health_pct_from(3, 500).expect("positive max must yield a percentage");
        assert!(
            pct.0 > 0.0 && pct.0 < 1.0,
            "3/500 must land strictly between 0% and 1%, got {}",
            pct.0
        );
    }

    /// A non-positive maximum has no defined percentage. Returning
    /// `Some(inf)` or `Some(NaN)` here would either fire every threshold
    /// or silently fire none.
    #[test]
    fn non_positive_max_has_no_percentage() {
        assert_eq!(health_pct_from(10, 0), None);
        assert_eq!(health_pct_from(10, -5), None);
    }

    /// Overkill drives `cur` negative; the percentage floors at zero so
    /// it can never read as "below every threshold and then some".
    #[test]
    fn overkill_clamps_to_zero_percent() {
        assert_eq!(health_pct_from(-40, 500), Some(HealthPct(0.0)));
    }

    /// Overheal is intentionally NOT clamped — "above the threshold" is
    /// the thing the crossing test asks about, and 120% is emphatically
    /// above.
    #[test]
    fn overheal_is_not_clamped() {
        let pct = health_pct_from(600, 500).expect("positive max must yield a percentage");
        assert_eq!(pct, HealthPct(120.0));
    }
}
