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

use cimmeria_entity::cell_entity::PlayerIdentity;
use cimmeria_wire::state_field::BSF_DEAD;
use tokio::sync::mpsc;

use super::super::messages::CellToBaseMsg;
use super::super::space_manager::{DespawnOutcome, SpaceManager};

/// Why a pet was despawned: the `reason` of the `pets.lifecycle`
/// `event = "despawned"` line.
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

/// The owner's identity for a pet log line: the one captured at summon
/// (the owner may already be destroyed, or its id reused, when its pet is
/// swept), else the live owner's while it is still a player. `UNKNOWN`
/// (both fields omitted), never a zero (instrumentation-discipline Rule 5).
pub(super) fn owner_identity(space_mgr: &SpaceManager, owner: Option<u32>) -> PlayerIdentity {
    let Some(owner) = owner else {
        return PlayerIdentity::UNKNOWN;
    };
    let cached = space_mgr.pets.owner_identity(owner);
    if cached.is_known() {
        return cached;
    }
    match space_mgr.get_entity(owner) {
        Some(e) if e.is_player => e.identity(),
        _ => PlayerIdentity::UNKNOWN,
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
    despawn_pet_via(space_mgr, pet_id, reason, "direct", tx).await
}

/// [`despawn_pet`] with the caller named: `path` is the `path` field of the
/// `despawned` row (`direct`, `sweep` or `disconnect`).
pub(super) async fn despawn_pet_via(
    space_mgr: &mut SpaceManager,
    pet_id: u32,
    reason: PetDespawnReason,
    path: &'static str,
    tx: &mpsc::Sender<CellToBaseMsg>,
) -> DespawnOutcome {
    // Everything the log needs is read before the pet and the registry
    // entry go away.
    let owner_id = space_mgr.pets.owner_of(pet_id);
    let id = owner_identity(space_mgr, owner_id);
    let template_id = space_mgr.get_entity(pet_id).and_then(|e| e.template_id);
    let outcome = space_mgr.despawn_npc(pet_id, tx).await;
    space_mgr.pets.forget_pet(pet_id);
    match outcome {
        DespawnOutcome::Despawned { witnesses_notified } => tracing::info!(
            target: "pets.lifecycle",
            decision_outcome = "despawned",
            event = "despawned",
            entity_id = pet_id,
            pet_id,
            owner_id,
            account_id = id.account_id,
            player_id = id.player_id,
            template_id,
            reason = reason.reason(),
            path,
            witnesses_notified,
            "pet despawned"
        ),
        // A teardown path asked to remove a pet that is not there (or is a
        // player): a server bookkeeping error, never client-triggerable.
        other => tracing::warn!(
            target: "pets.lifecycle",
            decision_outcome = "despawn_failed",
            event = "despawn_failed",
            entity_id = pet_id,
            pet_id,
            owner_id,
            account_id = id.account_id,
            player_id = id.player_id,
            template_id,
            reason = reason.reason(),
            path,
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
    let pets = space_mgr.pets.pets_of(owner);
    if pets.is_empty() {
        return 0;
    }
    let id = owner_identity(space_mgr, Some(owner));
    let mut despawned = 0;
    for &pet_id in &pets {
        let outcome = despawn_pet_via(
            space_mgr,
            pet_id,
            PetDespawnReason::OwnerDisconnected,
            "disconnect",
            tx,
        )
        .await;
        if matches!(outcome, DespawnOutcome::Despawned { .. }) {
            despawned += 1;
        }
    }
    tracing::debug!(
        target: "pets.lifecycle",
        event = "owner_forgotten",
        entity_id = owner,
        owner_id = owner,
        account_id = id.account_id,
        player_id = id.player_id,
        pet_count = pets.len(),
        despawned,
        path = "disconnect",
        "owner disconnected; its pets were despawned"
    );
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
    // Entity ids are reused: after the owner is destroyed the id can come
    // back as an NPC, or as another player, in the same space. Only the
    // player who summoned the pet owns it.
    if !owner_entity.is_player
        || !space_mgr
            .pets
            .owner_identity_matches(owner, owner_entity.identity())
    {
        return Some(SweepAction::Despawn(PetDespawnReason::OwnerGone));
    }
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
                let id = owner_identity(space_mgr, Some(owner));
                space_mgr.pets.forget_pet(pet_id);
                // `destroy_entity` / `destroy_space` scrub every pet they
                // remove, so an orphan entry is a missed teardown path.
                tracing::warn!(
                    target: "pets.lifecycle",
                    decision_outcome = "registry_scrubbed",
                    event = "registry_scrubbed",
                    entity_id = pet_id,
                    pet_id,
                    owner_id = owner,
                    account_id = id.account_id,
                    player_id = id.player_id,
                    reason = "pet_entity_gone",
                    path = "sweep",
                    "pet registry entry without an entity dropped"
                );
            }
            Some(SweepAction::Despawn(reason)) => {
                if matches!(
                    despawn_pet_via(space_mgr, pet_id, reason, "sweep", tx).await,
                    DespawnOutcome::Despawned { .. }
                ) {
                    despawned += 1;
                }
            }
        }
    }
    despawned
}
