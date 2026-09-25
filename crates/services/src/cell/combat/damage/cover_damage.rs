//! Cover as damage reduction (NA32, D-NA15a).
//!
//! A defender that stands at a cover node facing its attacker takes a
//! percentage off each hit. The owner's rule (2026-09-25): "Cover rating
//! should depend on material. Cement walls are better cover than lunch
//! tables. ... It should probably range from 10-60% damage reduction. We
//! don't want npcs in cover to be complete bullet sponges."
//!
//! # The rating
//!
//! The material signal the server has is what the level designers authored
//! on each node: `CoverQuality` and `CoverHeight` (extracted from the client
//! maps by NA21, `resources.cover_nodes`). The seed carries no prop mesh
//! name (`cover_sets.chunk_name` is the `SpecNode` actor), so there is no
//! cheap material refinement; quality and height are the whole signal.
//!
//! `base = QUALITY_BASE_PCT[quality] + HEIGHT_ADJUST_PCT[height]`, see
//! [`COVER_RATING`]. Tuned on the world-12 and world-8 seeds (4,024 nodes):
//! the 16 spawn-held guard slots are 13 `Mid`/`Better` and 3 `High`/`Best`,
//! and of all nodes 2,368 are `Mid`/`Better` and 1,340 `High`/`Best`. So
//! the typical guard slot is 25% (35% with Cover Stance, mid-band) and the
//! tall best-quality cover is 50% (60% with the stance). `Low`/`Good`
//! (210 nodes, waist-high furniture) is 15%.
//!
//! # Modifiers inside the band
//!
//! The cover stats keep the client's point scale and convert at the one
//! point-to-percent rate the shipped data states: 10 points = 1 percentage
//! point (effect 2004 "+50 (5%) Mental Resist", effect 2005 "Subtlety -100
//! (10% increase to threat)"). So Cover Stance's +100 `coverDefense` is +10
//! points of reduction, and +100 `coverAccuracy` (the designers' "Cover
//! Penetration") is -10. `coverQRModifier` is whole QR per point
//! (`alias.xml:216`), and 1 QR = 100 points (1729 vs 4299), so the
//! defender's is +10 percentage points per point.
//!
//! `final = clamp(base + stance - penetration, 10, 60)` in cover; `0` when
//! flanked or not at a node. Penetration only takes cover away, and the
//! floor keeps a stack of penetration from making cover worse than none.
//! At 60% a covered guard still takes 40% of every hit.

use super::pipeline::stat_cur;
use crate::cell::cover::{CoverHeight, CoverQuality};
use cimmeria_entity::stats::{StatList, COVER_ACCURACY, COVER_DEFENSE, COVER_QR_MODIFIER};

/// Smallest reduction a defender in unflanked cover gets.
pub const COVER_MIN_PCT: f64 = 10.0;
/// Largest reduction any cover gives.
pub const COVER_MAX_PCT: f64 = 60.0;
/// Percentage points per point of `coverDefense` / `coverAccuracy`.
pub const COVER_PCT_PER_STAT_POINT: f64 = 0.1;
/// Percentage points per point of the defender's `coverQRModifier`
/// (1 QR = 100 points).
pub const COVER_PCT_PER_QR: f64 = 10.0;

/// The material table: base reduction by quality, adjustment by height, in
/// percentage points. The one place to tune cover from UAT.
pub struct CoverRatingTable {
    pub none: f64,
    pub good: f64,
    pub better: f64,
    pub best: f64,
    pub low: f64,
    pub mid: f64,
    pub high: f64,
    pub los: f64,
}

/// See the module docs for how these were tuned.
pub const COVER_RATING: CoverRatingTable = CoverRatingTable {
    none: 10.0,
    good: 20.0,
    better: 25.0,
    best: 45.0,
    low: -5.0,
    mid: 0.0,
    high: 5.0,
    los: 10.0,
};

/// A node's authored rating, clamped to the band.
pub fn node_base_pct(quality: CoverQuality, height: CoverHeight) -> f64 {
    let t = &COVER_RATING;
    let q = match quality {
        CoverQuality::None_ => t.none,
        CoverQuality::Good => t.good,
        CoverQuality::Better => t.better,
        CoverQuality::Best => t.best,
    };
    let h = match height {
        CoverHeight::Low => t.low,
        CoverHeight::Mid => t.mid,
        CoverHeight::High => t.high,
        CoverHeight::Los => t.los,
    };
    (q + h).clamp(COVER_MIN_PCT, COVER_MAX_PCT)
}

/// Where the defender stands, as the damage roll sees it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CoverSide {
    /// Not at a cover node.
    Exposed,
    /// At a node that faces the attacker.
    InCover {
        quality: CoverQuality,
        height: CoverHeight,
    },
    /// At a node, but the attacker is past its side-on line.
    Flanked {
        quality: CoverQuality,
        height: CoverHeight,
    },
}

/// The terms of one hit's cover reduction, kept apart for telemetry.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct CoverReduction {
    /// The node's rating ([`node_base_pct`]); 0 when exposed or flanked.
    pub base_pct: f64,
    /// The defender's `coverDefense` and `coverQRModifier`, in points.
    pub stance_pct: f64,
    /// The attacker's `coverAccuracy`, in points (taken off).
    pub penetration_pct: f64,
    /// What comes off the hit: in `[10, 60]` in cover, else 0.
    pub final_pct: f64,
}

impl CoverReduction {
    /// The damage multiplier: `1 - final_pct / 100`.
    pub fn damage_scale(&self) -> f64 {
        1.0 - self.final_pct / 100.0
    }
}

/// The cover reduction for `attacker` hitting `defender` standing at `side`.
pub fn cover_reduction(
    attacker: &StatList,
    defender: &StatList,
    side: CoverSide,
) -> CoverReduction {
    let CoverSide::InCover { quality, height } = side else {
        return CoverReduction::default();
    };
    let base_pct = node_base_pct(quality, height);
    let stance_pct = stat_cur(defender, COVER_DEFENSE) as f64 * COVER_PCT_PER_STAT_POINT
        + stat_cur(defender, COVER_QR_MODIFIER) as f64 * COVER_PCT_PER_QR;
    let penetration_pct =
        (stat_cur(attacker, COVER_ACCURACY) as f64 * COVER_PCT_PER_STAT_POINT).max(0.0);
    CoverReduction {
        base_pct,
        stance_pct,
        penetration_pct,
        final_pct: (base_pct + stance_pct - penetration_pct).clamp(COVER_MIN_PCT, COVER_MAX_PCT),
    }
}

/// The attacker's own `coverQRModifier` while it is behind cover facing its
/// target: whole QR per point (`alias.xml:216`, "increases both attack and
/// defend QR while behind cover"). The defend half is in
/// [`cover_reduction`]; this is the attack half, and stays a QR term.
pub fn attacker_cover_qr(attacker: &StatList, attacker_in_cover: bool) -> f64 {
    if attacker_in_cover {
        stat_cur(attacker, COVER_QR_MODIFIER) as f64
    } else {
        0.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn stats(pairs: &[(i32, i32)]) -> StatList {
        let mut s = StatList::new();
        for &(id, v) in pairs {
            s.get_mut(id).unwrap().update(-1000, v, 1000);
        }
        s
    }

    fn in_cover(quality: CoverQuality, height: CoverHeight) -> CoverSide {
        CoverSide::InCover { quality, height }
    }

    /// Waist-high furniture (a lunch table: `Low` height, `Good` or `None`
    /// quality) is weak cover.
    #[test]
    fn a_lunch_table_node_is_light_cover() {
        for q in [CoverQuality::Good, CoverQuality::None_] {
            let p = node_base_pct(q, CoverHeight::Low);
            assert!((10.0..=20.0).contains(&p), "{q:?}: {p}");
        }
    }

    /// A tall best-quality node (a wall) is heavy cover.
    #[test]
    fn a_wall_node_is_heavy_cover() {
        for h in [CoverHeight::High, CoverHeight::Los] {
            let p = node_base_pct(CoverQuality::Best, h);
            assert!((50.0..=60.0).contains(&p), "{h:?}: {p}");
        }
    }

    /// The typical guard slot in the seeds (`Mid`/`Better`) sits mid-band,
    /// with and without Cover Stance.
    #[test]
    fn the_typical_guard_slot_is_mid_band() {
        let slot = in_cover(CoverQuality::Better, CoverHeight::Mid);
        let bare = cover_reduction(&StatList::new(), &StatList::new(), slot);
        let stance = cover_reduction(&StatList::new(), &stats(&[(COVER_DEFENSE, 100)]), slot);
        assert_eq!(bare.final_pct, 25.0);
        assert_eq!(stance.stance_pct, 10.0, "+100 coverDefense = +10 points");
        assert_eq!(stance.final_pct, 35.0);
    }

    /// Every combination, with any stat stack, stays in [10, 60].
    #[test]
    fn the_band_holds_for_every_node_and_stat_stack() {
        let qualities = [
            CoverQuality::None_,
            CoverQuality::Good,
            CoverQuality::Better,
            CoverQuality::Best,
        ];
        let heights = [
            CoverHeight::Low,
            CoverHeight::Mid,
            CoverHeight::High,
            CoverHeight::Los,
        ];
        let defenders = [
            stats(&[]),
            stats(&[(COVER_DEFENSE, 900), (COVER_QR_MODIFIER, 5)]),
        ];
        let attackers = [stats(&[]), stats(&[(COVER_ACCURACY, 900)])];
        for q in qualities {
            for h in heights {
                for d in &defenders {
                    for a in &attackers {
                        let r = cover_reduction(a, d, in_cover(q, h));
                        assert!(
                            (COVER_MIN_PCT..=COVER_MAX_PCT).contains(&r.final_pct),
                            "{q:?} {h:?}: {r:?}"
                        );
                    }
                }
            }
        }
    }

    /// Penetration takes cover away, down to the floor, never below it.
    #[test]
    fn penetration_only_reduces_the_cover() {
        let slot = in_cover(CoverQuality::Best, CoverHeight::High);
        let r = cover_reduction(&stats(&[(COVER_ACCURACY, 100)]), &StatList::new(), slot);
        assert_eq!(r.penetration_pct, 10.0);
        assert_eq!(r.final_pct, 40.0);
        let r = cover_reduction(&stats(&[(COVER_ACCURACY, 1000)]), &StatList::new(), slot);
        assert_eq!(r.final_pct, COVER_MIN_PCT);
    }

    /// Flanked or exposed: nothing, whatever the stats.
    #[test]
    fn flanked_and_exposed_get_nothing() {
        let d = stats(&[(COVER_DEFENSE, 100)]);
        for side in [
            CoverSide::Exposed,
            CoverSide::Flanked {
                quality: CoverQuality::Best,
                height: CoverHeight::Los,
            },
        ] {
            let r = cover_reduction(&StatList::new(), &d, side);
            assert_eq!(r, CoverReduction::default(), "{side:?}");
            assert_eq!(r.damage_scale(), 1.0);
        }
    }

    #[test]
    fn attacker_cover_qr_counts_only_behind_cover() {
        let a = stats(&[(COVER_QR_MODIFIER, 2)]);
        assert_eq!(attacker_cover_qr(&a, true), 2.0);
        assert_eq!(attacker_cover_qr(&a, false), 0.0);
    }
}
