//! The QR for one hit, with cover (NA32).
//!
//! [`combat::calculate_qr`] scores the two stat lists; this adds the cover
//! shift ([`combat::cover_shift`]) when either side stands in cover against
//! the other ([`SpaceManager::cover_standing`]), and reports it on the
//! `abilities.qr` row. The row is written only when cover was in play (one
//! side at a cover node, in cover or flanked), so an ordinary exchange of
//! fire adds nothing to the log.

use cimmeria_entity::stats::{StatList, COVER_DEFENSE};

use crate::cell::combat::{self, CoverSides};
use crate::cell::space_manager::{CoverStanding, SpaceManager};

/// The QR for `attacker_id` hitting `target_id`, cover included.
pub(super) fn qr_with_cover(
    space_mgr: &SpaceManager,
    attacker_id: u32,
    target_id: u32,
    attacker_stats: &StatList,
    target_stats: &StatList,
    ranged: bool,
) -> f64 {
    let base = combat::calculate_qr(attacker_stats, target_stats, ranged);
    let (Some(attacker), Some(target)) = (
        space_mgr.get_entity(attacker_id),
        space_mgr.get_entity(target_id),
    ) else {
        return base;
    };
    let defender = space_mgr.cover_standing(target_id, attacker.position);
    let attacker_side = space_mgr.cover_standing(attacker_id, target.position);
    if defender == CoverStanding::Exposed && attacker_side == CoverStanding::Exposed {
        return base;
    }
    let shift = combat::cover_shift(
        attacker_stats,
        target_stats,
        CoverSides {
            defender_in_cover: defender.in_cover(),
            attacker_in_cover: attacker_side.in_cover(),
        },
    );
    let qr = base + shift.total();
    tracing::debug!(
        target: "abilities.qr",
        event = "cover_resolved",
        attacker_id,
        target_id,
        defender_cover = defender.label(),
        attacker_cover = attacker_side.label(),
        flanked = defender == CoverStanding::Flanked,
        cover_defense_stat = target_stats.get(COVER_DEFENSE).map_or(0, |s| s.cur),
        cover_defense_applied = shift.defense_applied,
        cover_penetration = shift.penetration,
        cover_attack = shift.attack,
        qr_base = base,
        qr,
        "QR with cover: the defender's cover counts only when it faces the attacker"
    );
    qr
}
