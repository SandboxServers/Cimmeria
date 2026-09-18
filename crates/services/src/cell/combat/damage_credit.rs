//! Seam-level sampling for the `entity_health_below` content trigger
//! (Harset H04).
//!
//! # Why the sample is taken here and not at the ability caller
//!
//! The trigger's predicate is a stateless downward band: it fires only
//! when `pct_before > threshold && pct_after <= threshold`, with nothing
//! latched per entity. That makes *where* `pct_before` is sampled
//! load-bearing rather than incidental — a crossing that happens on a
//! damage path which never samples is lost **forever**, not merely
//! delayed. A `Rinla_Malac:30` mob dragged 35% → 25% by a DoT tick would
//! never fire, and every later direct hit arrives with `pct_before <= 30`,
//! so the `pct_before > 30` half of the band can never be satisfied again
//! short of a heal.
//!
//! H04 shipped the sample inside
//! `abilities::use_ability::kill_credit::handle_use_ability_with_kill_credit`,
//! which is the single-target player path only. Ground/AoE
//! (`handle_use_ability_on_ground`), cone secondaries, DoT/HoT pulses
//! (`effects::pulsing::tick::fire_pulse`) and effect-script damage all
//! bypassed it (PR #662 review, finding 1).
//!
//! # The shape that fixes it
//!
//! Every path that writes a target's HEALTH stat funnels through one of
//! two seams — [`crate::cell::abilities::damage_apply`] (single target,
//! AoE secondary, cone secondary, and the effect scripts it dispatches
//! afterwards) and `effects::pulsing::tick::fire_pulse` (every pulse of
//! every active effect). Each calls [`note_pre_damage_health`] immediately
//! before applying damage; that queues the pre-hit percentage on the
//! `SpaceManager`. A content-layer drain
//! ([`crate::cell::content::fire_pending_health_below`]) pops the queue
//! and hands each sample to `fire_health_below_for_hit`, which samples the
//! post-hit percentage itself.
//!
//! The queue exists because the seams have no `ChainEngine` and must not
//! grow one: `apply_damage_to_target` is reused by NPC AI, and threading a
//! content handle through the whole damage stack would put the content
//! engine in the middle of combat resolution. This mirrors the
//! `CellEntity::last_aoe_deaths` scratchpad the cone path already uses for
//! kill credit.
//!
//! # Drain discipline
//!
//! The drain must run **promptly** after the seam — before anything else
//! can move the same target's health — or `pct_after` is read against a
//! later world state. Today that means: once per ability resolution
//! (kill-credit wrapper + ground handler) and once per pulse
//! (`effect_pulse_tick`), plus a per-tick safety drain in the cell message
//! loop so a queued sample can never outlive one tick.

use super::{health_pct, HealthPct};
use crate::cell::space_manager::SpaceManager;

/// One target's health percentage, sampled immediately before a hit lands.
///
/// Carries the attacker because the acting player for the chain is the
/// **attacker**, matching `fire_entity_death` — and for a DoT pulse the
/// attacker is the effect's invoker (`ActiveEffectInstance::invoker_id`),
/// not whoever happens to be swinging at the target this tick.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HealthBelowSample {
    pub attacker_entity_id: u32,
    pub target_entity_id: u32,
    pub pct_before: HealthPct,
}

/// Sample `target_entity_id`'s health percentage and queue it for the
/// content-layer drain. Call this immediately before applying damage.
///
/// Four gates, all engine-independent so the hot path never touches the
/// `ChainEngine` (a hit on an untagged trash mob costs one `Option` check
/// and nothing else):
///
/// - **Target exists.** Defensive; the seams re-look-up anyway.
/// - **Target is not a player.** PvP damage drives no mission progression
///   today, matching the pre-existing single-target gate.
/// - **Target has a content tag.** An untagged entity cannot be addressed
///   by an `entity_health_below` chain at all, so there is nothing to
///   sample for. This is the gate that keeps the queue empty in practice.
/// - **Attacker is a player.** The trigger credits a player's mission; an
///   NPC-on-NPC DoT has nothing to advance. Gating here (rather than
///   letting the dispatcher's `player_id` warn fire) keeps NPC AI damage
///   from queueing entries that no drain site is guaranteed to reach.
///
/// A sample for an `(attacker, target)` pair that is already queued is
/// dropped rather than replacing or appending. Both drains run per hit, so
/// a collision means a drain was skipped; keeping the *earlier*
/// `pct_before` preserves the widest genuine band and guarantees at most
/// one event per pair per drain window. Keying on the pair rather than on
/// the target alone means a second player's hit on the same mob is never
/// swallowed by the first player's pending sample.
pub fn note_pre_damage_health(
    space_mgr: &mut SpaceManager,
    attacker_entity_id: u32,
    target_entity_id: u32,
) {
    let Some(target) = space_mgr.get_entity(target_entity_id) else {
        return;
    };
    if target.is_player || target.tag.is_none() {
        return;
    }
    let Some(pct_before) = health_pct(target) else {
        return;
    };
    if space_mgr
        .get_entity(attacker_entity_id)
        .and_then(|a| a.player_id)
        .is_none()
    {
        return;
    }
    if space_mgr.pending_health_below.iter().any(|s| {
        s.attacker_entity_id == attacker_entity_id && s.target_entity_id == target_entity_id
    }) {
        return;
    }
    space_mgr.pending_health_below.push(HealthBelowSample {
        attacker_entity_id,
        target_entity_id,
        pct_before,
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    const PLAYER_EID: u32 = 1;
    const NPC_EID: u32 = 50;

    /// One player and one tagged NPC at 100/100 health in a shared space.
    fn make_mgr() -> SpaceManager {
        let mut mgr = SpaceManager::new(1);
        mgr.parse_spaces_xml(
            r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle" Instanced="false" MinX="-800" MaxX="800" MinY="-800" MaxY="800" /></Spaces>"#,
        )
        .unwrap();
        mgr.create_startup_spaces(
            r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle" /></Spaces>"#,
        )
        .unwrap();
        mgr.create_entity(PLAYER_EID, "Castle", [0.0; 3], [0.0; 3])
            .unwrap();
        mgr.spawn_npc(NPC_EID, "Castle", [5.0, 0.0, 0.0], [0.0; 3])
            .unwrap();
        if let Some(p) = mgr.get_entity_mut(PLAYER_EID) {
            p.is_player = true;
            p.player_id = Some(100);
        }
        if let Some(npc) = mgr.get_entity_mut(NPC_EID) {
            npc.tag = Some("Rinla_Malac".to_string());
        }
        mgr
    }

    #[test]
    fn a_player_hit_on_a_tagged_npc_queues_one_sample() {
        let mut mgr = make_mgr();
        note_pre_damage_health(&mut mgr, PLAYER_EID, NPC_EID);

        assert_eq!(mgr.pending_health_below.len(), 1);
        let s = mgr.pending_health_below[0];
        assert_eq!(s.attacker_entity_id, PLAYER_EID);
        assert_eq!(s.target_entity_id, NPC_EID);
        assert_eq!(s.pct_before, HealthPct(100.0));
    }

    /// The gate that keeps the queue empty on a normal server: most mobs
    /// carry no content tag, and an untagged entity can never be named by
    /// an `entity_health_below` chain.
    #[test]
    fn an_untagged_target_is_not_queued() {
        let mut mgr = make_mgr();
        if let Some(npc) = mgr.get_entity_mut(NPC_EID) {
            npc.tag = None;
        }
        note_pre_damage_health(&mut mgr, PLAYER_EID, NPC_EID);
        assert!(mgr.pending_health_below.is_empty());
    }

    /// NPC-on-NPC damage has no mission to credit. Gating here rather than
    /// at the dispatcher is what stops NPC AI damage — which reaches the
    /// same seam but no drain site — from growing the queue.
    #[test]
    fn an_attacker_without_a_player_id_is_not_queued() {
        let mut mgr = make_mgr();
        if let Some(p) = mgr.get_entity_mut(PLAYER_EID) {
            p.player_id = None;
        }
        note_pre_damage_health(&mut mgr, PLAYER_EID, NPC_EID);
        assert!(mgr.pending_health_below.is_empty());
    }

    #[test]
    fn a_player_target_is_not_queued() {
        let mut mgr = make_mgr();
        if let Some(npc) = mgr.get_entity_mut(NPC_EID) {
            npc.is_player = true;
        }
        note_pre_damage_health(&mut mgr, PLAYER_EID, NPC_EID);
        assert!(mgr.pending_health_below.is_empty());
    }

    /// Two samples for the same pair before a drain keep the *earlier*
    /// `pct_before` — the widest genuine band — and stay a single entry,
    /// so one drain can never emit two events for one pair.
    #[test]
    fn a_repeated_pair_keeps_the_earlier_sample() {
        use cimmeria_entity::stats::HEALTH;

        let mut mgr = make_mgr();
        note_pre_damage_health(&mut mgr, PLAYER_EID, NPC_EID);
        if let Some(stat) = mgr
            .get_entity_mut(NPC_EID)
            .and_then(|e| e.stats.get_mut(HEALTH))
        {
            stat.cur = 40;
        }
        note_pre_damage_health(&mut mgr, PLAYER_EID, NPC_EID);

        assert_eq!(mgr.pending_health_below.len(), 1);
        assert_eq!(mgr.pending_health_below[0].pct_before, HealthPct(100.0));
    }

    /// A second attacker on the same target is a separate pair and must
    /// not be swallowed by the first attacker's pending sample.
    #[test]
    fn a_second_attacker_on_the_same_target_queues_separately() {
        let mut mgr = make_mgr();
        mgr.create_entity(2, "Castle", [1.0, 0.0, 0.0], [0.0; 3])
            .unwrap();
        if let Some(p) = mgr.get_entity_mut(2) {
            p.is_player = true;
            p.player_id = Some(200);
        }

        note_pre_damage_health(&mut mgr, PLAYER_EID, NPC_EID);
        note_pre_damage_health(&mut mgr, 2, NPC_EID);

        assert_eq!(mgr.pending_health_below.len(), 2);
    }

    /// No HEALTH stat (or a non-positive maximum) means no defined
    /// percentage, so there is nothing to sample and the dispatcher must
    /// never be handed an invented one.
    #[test]
    fn a_target_without_a_usable_health_stat_is_not_queued() {
        use cimmeria_entity::stats::HEALTH;

        let mut mgr = make_mgr();
        if let Some(stat) = mgr
            .get_entity_mut(NPC_EID)
            .and_then(|e| e.stats.get_mut(HEALTH))
        {
            stat.update(0, 10, 0);
        }
        note_pre_damage_health(&mut mgr, PLAYER_EID, NPC_EID);
        assert!(mgr.pending_health_below.is_empty());
    }
}
