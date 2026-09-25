//! Cover in the QR roll (NA32).
//!
//! A defender that stands in cover against its attacker gets a defensive QR
//! shift; an attacker in cover against its target gets an offensive one. The
//! units are the designers' own, from the `StatList` comments in the client's
//! `entities/defs/alias.xml` (quoted in
//! `docs/reverse-engineering/findings/combat-formulas-client-evidence.md`):
//!
//! | Stat | `alias.xml` | Term here |
//! |---|---|---|
//! | `coverDefense` (67) | 235: "increases the defense of a player in cover by -0.01 QR" | defender, `-0.01` per point |
//! | `coverAccuracy` (66) | 234: "increases the accuracy of attacks against a target inside cover by +0.01 QR" | attacker, `+0.01` per point, only against cover |
//! | `coverQRModifier` (48) | 216: "increases both attack and defend QR while behind cover by 1 per point" | `1.0` per point, for whichever side is behind cover |
//!
//! Two rules the comments do not spell out, taken from the cooked ability
//! text:
//!
//! - **Cover accuracy only takes cover away.** Every attacker-side cover
//!   stat in the data is written as penetration: ability 1450 "Cover
//!   Penetration" / effect 1741 "+100 CoverAccuracy", ability 1487
//!   "Ignores 2 QR of Cover", effect 4995 "+1 QR Cover Penetration". So the
//!   defender's cover shift is `max(0, cover - penetration)`: a covered
//!   target is never easier to hit than an exposed one.
//! - **Cover counts only where it is.** A defender that is flanked (the
//!   attacker is past the cover's side-on line, [`crate::cell::cover::is_flanked`])
//!   or not at a cover node gets nothing, whatever its `coverDefense` stat
//!   says. Cover Stance keeps the stat raised for as long as an NPC holds its
//!   slot, so the geometry test is what makes a flank pay.
//!
//! The shift is added to the QR that [`super::qr::calculate_qr`] computes,
//! so it goes through the same beta roll and the same `(1 + qr)` damage
//! term as `defense` does. With Cover Stance's +100 that is -1.0 QR, which
//! is the "100 points = 1 QR" scale the data states twice (ability 1729 vs
//! its effect 4299, and 1450 vs 4995).

use super::pipeline::stat_cur;
use cimmeria_entity::stats::{StatList, COVER_ACCURACY, COVER_DEFENSE, COVER_QR_MODIFIER};

/// QR per point of `coverDefense` and `coverAccuracy` (`alias.xml:234-235`).
pub const COVER_STAT_QR_PER_POINT: f64 = 0.01;

/// Where each side of an attack stands relative to cover, as the hit
/// resolution sees it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct CoverSides {
    /// The defender is at a cover node that faces the attacker.
    pub defender_in_cover: bool,
    /// The attacker is at a cover node that faces the defender.
    pub attacker_in_cover: bool,
}

/// The cover terms of one QR roll, kept apart for the telemetry row.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct CoverShift {
    /// The defender's cover QR before penetration:
    /// `coverDefense * 0.01 + coverQRModifier`. Zero when not in cover.
    pub defense: f64,
    /// The attacker's cover penetration: `coverAccuracy * 0.01`. Zero when
    /// the defender is not in cover.
    pub penetration: f64,
    /// What the defender's cover took off the QR:
    /// `max(0, defense - penetration)`.
    pub defense_applied: f64,
    /// The attacker's own `coverQRModifier` while it is behind cover.
    pub attack: f64,
}

impl CoverShift {
    /// The net QR change: positive favours the attacker.
    pub fn total(&self) -> f64 {
        self.attack - self.defense_applied
    }
}

/// The cover shift for `attacker` hitting `defender` with `sides`.
pub fn cover_shift(attacker: &StatList, defender: &StatList, sides: CoverSides) -> CoverShift {
    let mut shift = CoverShift::default();
    if sides.defender_in_cover {
        shift.defense = stat_cur(defender, COVER_DEFENSE) as f64 * COVER_STAT_QR_PER_POINT
            + stat_cur(defender, COVER_QR_MODIFIER) as f64;
        shift.penetration = stat_cur(attacker, COVER_ACCURACY) as f64 * COVER_STAT_QR_PER_POINT;
        shift.defense_applied = (shift.defense - shift.penetration).max(0.0);
    }
    if sides.attacker_in_cover {
        shift.attack = stat_cur(attacker, COVER_QR_MODIFIER) as f64;
    }
    shift
}

#[cfg(test)]
mod tests {
    use super::*;

    fn stats(pairs: &[(i32, i32)]) -> StatList {
        let mut s = StatList::new();
        for &(id, v) in pairs {
            let st = s.get_mut(id).unwrap();
            st.update(-1000, v, 1000);
        }
        s
    }

    const IN_COVER: CoverSides = CoverSides {
        defender_in_cover: true,
        attacker_in_cover: false,
    };

    /// Cover Stance's +100 is -1.0 QR in cover (alias.xml:235).
    #[test]
    fn cover_stance_is_minus_one_qr_in_cover() {
        let shift = cover_shift(&StatList::new(), &stats(&[(COVER_DEFENSE, 100)]), IN_COVER);
        assert!((shift.defense_applied - 1.0).abs() < 1e-9, "{shift:?}");
        assert!((shift.total() + 1.0).abs() < 1e-9);
    }

    /// Out of cover or flanked, the raised stat does nothing.
    #[test]
    fn the_stat_alone_gives_nothing_outside_cover() {
        let shift = cover_shift(
            &StatList::new(),
            &stats(&[(COVER_DEFENSE, 100), (COVER_QR_MODIFIER, 2)]),
            CoverSides::default(),
        );
        assert_eq!(shift, CoverShift::default());
    }

    /// Penetration only removes cover: 150 cover accuracy against 100
    /// cover defense leaves zero, not +0.5.
    #[test]
    fn penetration_is_capped_at_the_cover() {
        let shift = cover_shift(
            &stats(&[(COVER_ACCURACY, 150)]),
            &stats(&[(COVER_DEFENSE, 100)]),
            IN_COVER,
        );
        assert_eq!(shift.defense_applied, 0.0);
        let partial = cover_shift(
            &stats(&[(COVER_ACCURACY, 40)]),
            &stats(&[(COVER_DEFENSE, 100)]),
            IN_COVER,
        );
        assert!((partial.defense_applied - 0.6).abs() < 1e-9, "{partial:?}");
    }

    /// `coverQRModifier` is whole QR per point, on both sides (alias.xml:216).
    #[test]
    fn cover_qr_modifier_counts_for_whichever_side_is_behind_cover() {
        let def = stats(&[(COVER_QR_MODIFIER, 1)]);
        let att = stats(&[(COVER_QR_MODIFIER, 2)]);
        let both = cover_shift(
            &att,
            &def,
            CoverSides {
                defender_in_cover: true,
                attacker_in_cover: true,
            },
        );
        assert!((both.defense_applied - 1.0).abs() < 1e-9);
        assert!((both.attack - 2.0).abs() < 1e-9);
        assert!((both.total() - 1.0).abs() < 1e-9);
    }
}
