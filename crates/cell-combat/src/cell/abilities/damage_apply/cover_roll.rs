//! Cover for one hit (NA32, D-NA15a).
//!
//! The defender's cover is a damage reduction rated by the node it stands at
//! ([`combat::cover_reduction`]), when that node faces the attacker
//! ([`SpaceManager::cover_standing_node`]). The attacker's own
//! `coverQRModifier` behind cover stays a QR term
//! ([`combat::attacker_cover_qr`]). Both are reported on the
//! `abilities.qr event=cover_resolved` row, written only when one side
//! stands at a cover node, so an ordinary exchange of fire adds nothing to
//! the log.

use cimmeria_entity::stats::StatList;

use crate::cell::combat::{self, CoverReduction, CoverSide};
use crate::cell::space_manager::{CoverStanding, SpaceManager};

/// What cover does to one hit.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub(super) struct HitCover {
    /// Added to the attacker's QR.
    pub attacker_qr: f64,
    /// Taken off the defender's damage.
    pub reduction: CoverReduction,
}

/// Cover for `attacker_id` hitting `target_id`.
pub(super) fn resolve_cover(
    space_mgr: &SpaceManager,
    attacker_id: u32,
    target_id: u32,
    attacker_stats: &StatList,
    target_stats: &StatList,
) -> HitCover {
    let (Some(attacker), Some(target)) = (
        space_mgr.get_entity(attacker_id),
        space_mgr.get_entity(target_id),
    ) else {
        return HitCover::default();
    };
    let (defender, node) = space_mgr.cover_standing_node(target_id, attacker.position);
    let attacker_side = space_mgr.cover_standing(attacker_id, target.position);
    if defender == CoverStanding::Exposed && attacker_side == CoverStanding::Exposed {
        return HitCover::default();
    }
    let side = match (defender, &node) {
        (CoverStanding::InCover, Some(n)) => CoverSide::InCover {
            quality: n.quality,
            height: n.height,
        },
        (CoverStanding::Flanked, Some(n)) => CoverSide::Flanked {
            quality: n.quality,
            height: n.height,
        },
        _ => CoverSide::Exposed,
    };
    let reduction = combat::cover_reduction(attacker_stats, target_stats, side);
    let attacker_qr = combat::attacker_cover_qr(attacker_stats, attacker_side.in_cover());
    tracing::debug!(
        target: "abilities.qr",
        event = "cover_resolved",
        attacker_id,
        target_id,
        defender_cover = defender.label(),
        attacker_cover = attacker_side.label(),
        flanked = defender == CoverStanding::Flanked,
        cover_quality = node.as_ref().map_or("none", |n| n.quality.sql_name()),
        cover_height = node.as_ref().map_or("none", |n| n.height.sql_name()),
        base_pct = reduction.base_pct,
        stance_pct = reduction.stance_pct,
        penetration_pct = reduction.penetration_pct,
        final_pct = reduction.final_pct,
        attacker_cover_qr = attacker_qr,
        "Cover damage reduction: rated by the node, only when it faces the attacker"
    );
    HitCover {
        attacker_qr,
        reduction,
    }
}
