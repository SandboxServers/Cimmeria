//! Stance rules (D-PT09): which target, if any, a pet out of a fight should
//! engage this turn. Pure reads; [`super::engage::engage_stance_pick`] acts on the pick.
//!
//! Engaging on being hit is not here: `combat::generate_threat` preempts a
//! Defensive or Aggressive pet into Fighting the moment damage lands, as it
//! does any NPC, and refuses the threat for a Passive pet.

use cimmeria_entity::cell_entity::{AiState, CellEntity, PetStance};

use crate::cell::combat;
use crate::cell::space_manager::SpaceManager;

use super::super::aggro_gates::same_room;
use super::super::leash::policy::horizontal_distance;

/// Radius of the Aggressive stance's scan around the pet, horizontal, in
/// world units (D-PT09).
pub(super) const PET_AGGRESSIVE_RADIUS: f32 = 15.0;

/// A mob fighting the owner, or the owner's target, is engaged only within
/// this horizontal distance of the owner: the pet's own teleport distance, so
/// it never runs off after something its owner is shooting from across the
/// map.
pub(super) const PET_DEFEND_RADIUS: f32 = super::owner_follow::PET_TELEPORT_DISTANCE;

/// Why a pet engaged. The label is the `why` field on the `pet_engaged` row.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum EngageWhy {
    /// A mob is fighting the owner (Defensive, Aggressive).
    DefendOwner,
    /// A mob is fighting the pet itself, which is not in a fight (it left
    /// one through the leash, or was Passive when the mob engaged it).
    DefendSelf,
    /// The owner is in combat and targeting this mob (Aggressive).
    OwnerTarget,
    /// A hostile NPC within [`PET_AGGRESSIVE_RADIUS`] of the pet
    /// (Aggressive).
    AggressiveScan,
}

impl EngageWhy {
    /// Stable snake_case label. Treat as API.
    pub(super) fn label(self) -> &'static str {
        match self {
            Self::DefendOwner => "defend_owner",
            Self::DefendSelf => "defend_self",
            Self::OwnerTarget => "owner_target",
            Self::AggressiveScan => "aggressive_scan",
        }
    }
}

/// Whether the pet can fight `mob` at all: alive, and not walking home or
/// leaving the world. A leashing mob evades, so engaging it is pointless.
fn fightable(mob: &CellEntity) -> bool {
    !combat::is_dead_state(mob.state_field)
        && mob
            .stats
            .get(cimmeria_entity::stats::HEALTH)
            .is_some_and(|h| h.cur > 0)
        && !matches!(
            mob.ai_state(),
            AiState::Leashing | AiState::Dead | AiState::Despawning
        )
}

/// The target the pet's stance engages now, nearest first within each rule,
/// rules in the order of [`EngageWhy`]. `None` for a Passive pet, and when
/// nothing qualifies.
///
/// Candidates are the mobs (class `SGWMob`) in the pet's space: never a
/// player, another pet, or a being. Every rule also applies
/// [`super::fight_refusal`]: a combatant mob its owner could attack itself
/// (the #444 rule), so a pet never fights a vendor, a quest giver or a
/// neutral NPC, even one a content chain set fighting its owner. The Aggressive scan uses the
/// NPC acquisition gate (`aggro_gates::same_room`: floor band, radius, line
/// of sight failing closed where a navmesh exists).
pub(super) fn pick_engagement(
    space_mgr: &SpaceManager,
    pet_id: u32,
    owner_id: u32,
    stance: PetStance,
) -> Option<(u32, EngageWhy)> {
    if stance == PetStance::Passive {
        return None;
    }
    let pet = space_mgr.get_entity(pet_id)?;
    let owner = space_mgr.get_entity(owner_id)?;
    // A pet left behind (past the teleport distance, or off the owner's
    // floor) goes back to its owner before it picks any fight. Without this a
    // pet that just gave up a fight at the owner-anchored leash re-engaged the
    // same mob on its next turn while its teleport was rate-limited, and
    // flapped between Fighting and Follow for up to 5 s.
    if super::owner_follow::left_behind(&pet.position, &owner.position).is_some() {
        return None;
    }
    // Every rule is bounded: the defend rules and the owner's target within
    // PET_DEFEND_RADIUS of the owner, the Aggressive scan within
    // PET_AGGRESSIVE_RADIUS of the pet. Dropping the rest before the sort
    // keeps the per-turn cost to the mobs near the pair, not the space.
    let in_reach = |m: &CellEntity| {
        horizontal_distance(&m.position, &owner.position) <= PET_DEFEND_RADIUS
            || horizontal_distance(&m.position, &pet.position) <= PET_AGGRESSIVE_RADIUS
    };
    let mut mobs: Vec<&CellEntity> = space_mgr
        .npc_ids_in_space_of(pet_id)
        .into_iter()
        .filter_map(|id| space_mgr.get_entity(id))
        .filter(|m| in_reach(m) && fightable(m) && super::fight_refusal(owner, m).is_none())
        .collect();
    // Nearest first; the id breaks ties so the pick is deterministic.
    mobs.sort_by(|a, b| {
        horizontal_distance(&a.position, &pet.position)
            .total_cmp(&horizontal_distance(&b.position, &pet.position))
            .then(a.entity_id.0.cmp(&b.entity_id.0))
    });
    let id = |m: &CellEntity| m.entity_id.0 as u32;
    let near_owner =
        |m: &CellEntity| horizontal_distance(&m.position, &owner.position) <= PET_DEFEND_RADIUS;

    // Defensive: whatever is fighting the owner, then whatever is fighting
    // the pet.
    let fighting = |m: &&&CellEntity, who: u32| {
        m.ai_state() == AiState::Fighting && m.threat_list.contains_key(&who)
    };
    if let Some(m) = mobs.iter().find(|m| fighting(m, owner_id) && near_owner(m)) {
        return Some((id(m), EngageWhy::DefendOwner));
    }
    if let Some(m) = mobs.iter().find(|m| fighting(m, pet_id) && near_owner(m)) {
        return Some((id(m), EngageWhy::DefendSelf));
    }
    if stance != PetStance::Aggressive {
        return None;
    }

    // Aggressive: the owner's target once the owner is in combat, then any
    // hostile NPC close to the pet.
    //
    // The owner's target needs nothing beyond `fight_refusal` (applied to
    // every candidate above) and the owner radius: it is a fight the owner
    // chose, so a hostile-faction mob content set to Neutral is as valid as
    // it is for the owner's own attack. Only the scan, which picks a fight
    // nobody chose, also asks `is_hostile_to_players`: an Aggressive pet
    // attacks what would attack its owner on sight, not every attackable
    // NPC in range.
    let owner_in_combat = owner.state_field & combat::BSF_IN_COMBAT != 0;
    let owner_target = owner.current_target_id.and_then(|t| u32::try_from(t).ok());
    if let Some(t) = owner_target.filter(|_| owner_in_combat) {
        if let Some(m) = mobs.iter().find(|m| id(m) == t && near_owner(m)) {
            return Some((id(m), EngageWhy::OwnerTarget));
        }
    }
    mobs.iter()
        .find(|m| {
            combat::is_hostile_to_players(m)
                && same_room(space_mgr, pet, m, PET_AGGRESSIVE_RADIUS).is_ok()
        })
        .map(|m| (id(m), EngageWhy::AggressiveScan))
}
