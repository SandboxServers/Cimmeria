//! Pet teardown: the observer-visible despawn, the owner-disconnect hook,
//! and the per-tick self-healing sweep (A-31).
//!
//! An owner can leave through eleven paths (disconnect, destroy, death,
//! respawn, gate travel, space transfer, GM travel, content transport, ring,
//! instanced-space teardown). PT-01 covers them in two layers:
//!
//! - `destroy_entity` / `destroy_space` scrub the registry for any pet they
//!   remove, whatever removed it, so the maps can never name a dead entity
//!   for longer than the call.
//! - [`forget_owner`] runs from `disconnect_entity`, where `tx` is in hand,
//!   and despawns the pets immediately.
//! - [`pet_owner_sweep`] runs every AoI tick and despawns any pet whose
//!   owner is gone, dead (D-PT08) or in another space. It alone catches
//!   every other path within 100 ms; PT-02 makes the common ones immediate.
//!
//! Despawn goes through `despawn_npc`, never bare `destroy_entity`: the
//! former sends `LeftAoI` to every witness and scrubs the witness sets.

use cimmeria_wire::state_field::BSF_DEAD;
use tokio::sync::mpsc;

use super::super::messages::CellToBaseMsg;
use super::super::space_manager::{DespawnOutcome, SpaceManager};

/// Why a pet was despawned: the `reason` of the `pets.lifecycle` INFO line.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PetDespawnReason {
    /// The owner's client disconnected.
    OwnerDisconnected,
    /// The owner's entity no longer exists (destroyed on any other path).
    OwnerGone,
    /// The owner died (D-PT08).
    OwnerDead,
    /// The owner is in a different space (transfer, gate travel, ...).
    OwnerLeftSpace,
    /// Dismissed by command or replaced by a new summon.
    Dismissed,
}

impl PetDespawnReason {
    /// Stable `reason` value for logs.
    pub fn reason(self) -> &'static str {
        match self {
            Self::OwnerDisconnected => "owner_disconnected",
            Self::OwnerGone => "owner_gone",
            Self::OwnerDead => "owner_dead",
            Self::OwnerLeftSpace => "owner_left_space",
            Self::Dismissed => "dismissed",
        }
    }
}

/// Despawn `pet_id` visibly (`LeftAoI` to every witness) and drop it from
/// the registry. Refuses (`RefusedPlayer`/`NotFound`) exactly as
/// `despawn_npc` does; the registry is scrubbed either way, so a stale entry
/// cannot outlive the call.
pub async fn despawn_pet(
    space_mgr: &mut SpaceManager,
    pet_id: u32,
    reason: PetDespawnReason,
    tx: &mpsc::Sender<CellToBaseMsg>,
) -> DespawnOutcome {
    let owner_id = space_mgr.pets.owner_of(pet_id);
    let outcome = space_mgr.despawn_npc(pet_id, tx).await;
    space_mgr.pets.forget_pet(pet_id);
    match outcome {
        DespawnOutcome::Despawned { witnesses_notified } => tracing::info!(
            target: "pets.lifecycle",
            decision_outcome = "despawned",
            pet_id,
            owner_id = owner_id.unwrap_or(0),
            reason = reason.reason(),
            witnesses_notified,
            "pet despawned"
        ),
        other => tracing::warn!(
            target: "pets.lifecycle",
            decision_outcome = "despawn_failed",
            pet_id,
            owner_id = owner_id.unwrap_or(0),
            reason = reason.reason(),
            outcome = ?other,
            "pet despawn did not remove an entity"
        ),
    }
    outcome
}

/// Despawn every pet `owner` has out. Called from `disconnect_entity`
/// before the owner's own AoI teardown, while `tx` is in hand. Returns how
/// many pets were despawned.
pub async fn forget_owner(
    owner: u32,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) -> usize {
    let mut despawned = 0;
    for pet_id in space_mgr.pets.pets_of(owner) {
        let outcome = despawn_pet(space_mgr, pet_id, PetDespawnReason::OwnerDisconnected, tx).await;
        if matches!(outcome, DespawnOutcome::Despawned { .. }) {
            despawned += 1;
        }
    }
    despawned
}

/// What the sweep should do about one registered pet.
fn sweep_verdict(space_mgr: &SpaceManager, pet_id: u32, owner: u32) -> Option<SweepAction> {
    let Some(pet_space) = space_mgr.get_entity_space_id(pet_id) else {
        return Some(SweepAction::Scrub);
    };
    if space_mgr.get_entity(pet_id).is_none_or(|e| e.pet.is_none()) {
        return Some(SweepAction::Scrub);
    }
    let Some(owner_entity) = space_mgr.get_entity(owner) else {
        return Some(SweepAction::Despawn(PetDespawnReason::OwnerGone));
    };
    if space_mgr.get_entity_space_id(owner) != Some(pet_space) {
        return Some(SweepAction::Despawn(PetDespawnReason::OwnerLeftSpace));
    }
    // The state bit, not HEALTH: a player corpse can be healed during the
    // Defeat Window, and HEALTH alone is not a dead test.
    if owner_entity.state_field & BSF_DEAD != 0 {
        return Some(SweepAction::Despawn(PetDespawnReason::OwnerDead));
    }
    None
}

enum SweepAction {
    /// The pet entity is already gone; only the registry entry is left.
    Scrub,
    /// The pet is alive but its owner no longer holds it.
    Despawn(PetDespawnReason),
}

/// The self-healing sweep: despawn every pet whose owner is gone, dead or in
/// another space, and scrub registry entries whose pet entity is gone.
/// Returns how many pets were despawned. Returns at once when no pet exists,
/// so it is cheap enough for every 100 ms AoI tick.
pub async fn pet_owner_sweep(
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) -> usize {
    if space_mgr.pets.is_empty() {
        return 0;
    }
    let mut despawned = 0;
    for (pet_id, owner) in space_mgr.pets.pairs() {
        match sweep_verdict(space_mgr, pet_id, owner) {
            None => {}
            Some(SweepAction::Scrub) => {
                space_mgr.pets.forget_pet(pet_id);
                tracing::debug!(
                    target: "pets.lifecycle",
                    decision_outcome = "registry_scrubbed",
                    pet_id,
                    owner_id = owner,
                    reason = "pet_entity_gone",
                    "pet registry entry without an entity dropped"
                );
            }
            Some(SweepAction::Despawn(reason)) => {
                if matches!(
                    despawn_pet(space_mgr, pet_id, reason, tx).await,
                    DespawnOutcome::Despawned { .. }
                ) {
                    despawned += 1;
                }
            }
        }
    }
    despawned
}
