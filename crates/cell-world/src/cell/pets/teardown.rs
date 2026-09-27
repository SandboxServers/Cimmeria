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
//!   every other path within 100 ms; PT-02 makes the common ones immediate
//!   through [`super::owner_hooks`].
//! - The same sweep expires pet corpses: a pet that died is stamped with
//!   `despawn_at = now + PET_CORPSE_DESPAWN` and despawned once that passes
//!   (D-PT08).
//!
//! Despawn goes through `despawn_npc`, never bare `destroy_entity`: the
//! former sends `LeftAoI` to every witness and scrubs the witness sets.

use std::time::{Duration, Instant};

use cimmeria_common::EntityId;
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
    /// The pet died and its corpse timer ran out (D-PT08).
    CorpseExpired,
    /// A living timed pet reached its `despawn_at`.
    Expired,
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
            Self::CorpseExpired => "corpse_expired",
            Self::Expired => "expired",
        }
    }

    /// Whether the owner's own client view is being torn down on this path,
    /// so the owner must NOT get a `LeftAoI` for the pet.
    ///
    /// A traveller's client gets `RESET_ENTITIES` from the gate-travel
    /// back half, and a closing session gets nothing more at all. A
    /// `LeftAoI` queued behind the `GateTravel` would reach the base after
    /// the reset: the base holds it until the new world's `onClientReady`
    /// and then flushes a leave for an entity the new world never had.
    /// Every other witness still gets its `LeftAoI`.
    pub fn owner_view_torn_down(self) -> bool {
        matches!(
            self,
            Self::OwnerDisconnected | Self::OwnerGone | Self::OwnerLeftSpace
        )
    }
}

/// How long a dead pet's corpse stays before it despawns (D-PT08).
pub const PET_CORPSE_DESPAWN: Duration = Duration::from_secs(10);

/// The owner's identity for a log line about `pet_id`: the one captured
/// when that pet was summoned (the owner may already be destroyed, or its
/// id reused, when its pet is swept), else the live owner's while it is
/// still a player. `UNKNOWN` (both fields omitted), never a zero
/// (instrumentation-discipline Rule 5).
pub(super) fn owner_identity(
    space_mgr: &SpaceManager,
    pet_id: u32,
    owner: Option<u32>,
) -> PlayerIdentity {
    let cached = space_mgr.pets.summoner_identity(pet_id);
    if cached.is_known() {
        return cached;
    }
    let Some(owner) = owner else {
        return PlayerIdentity::UNKNOWN;
    };
    match space_mgr.get_entity(owner) {
        Some(e) if e.is_player => e.identity(),
        _ => PlayerIdentity::UNKNOWN,
    }
}

/// Despawn `pet_id` visibly (`LeftAoI` to every witness) and drop it from
/// the registry. Refuses (`RefusedPlayer`/`NotFound`) exactly as
/// `despawn_npc` does; the registry is scrubbed either way, so a stale entry
/// cannot outlive the call.
///
/// When [`PetDespawnReason::owner_view_torn_down`] holds, the owner is left
/// out of the `LeftAoI` fan-out: the pet is scrubbed from the owner's
/// witness set first, and that set is what `despawn_npc` reads.
pub async fn despawn_pet(
    space_mgr: &mut SpaceManager,
    pet_id: u32,
    reason: PetDespawnReason,
    tx: &mpsc::Sender<CellToBaseMsg>,
) -> DespawnOutcome {
    despawn_pet_via(space_mgr, pet_id, reason, "direct", tx).await
}

/// [`despawn_pet`] with the caller named: `path` is the `path` field of the
/// `despawned` row (`direct`, `sweep`, or an owner path label from
/// `owner_hooks::OwnerPath`).
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
    let id = owner_identity(space_mgr, pet_id, owner_id);
    let template_id = space_mgr.get_entity(pet_id).and_then(|e| e.template_id);
    if reason.owner_view_torn_down() {
        if let Some(owner) = owner_id.and_then(|o| space_mgr.get_entity_mut(o)) {
            owner.witnesses.remove(&EntityId(pet_id as i32));
        }
    }
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
/// many pets were despawned. The disconnect case of
/// [`super::owner_hooks::on_owner_left`].
pub async fn forget_owner(
    owner: u32,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) -> usize {
    let pets = space_mgr.pets.pets_of(owner);
    if pets.is_empty() {
        return 0;
    }
    // Resolved before the despawns, while the owner is still in the space.
    let id = super::owner_hooks::leaving_owner_identity(space_mgr, owner, &pets);
    let despawned = super::owner_hooks::on_owner_left(
        owner,
        PetDespawnReason::OwnerDisconnected,
        super::owner_hooks::OwnerPath::Disconnect,
        tx,
        space_mgr,
    )
    .await;
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

/// What the sweep should do about one registered pet at `now`.
fn sweep_verdict(
    space_mgr: &SpaceManager,
    pet_id: u32,
    owner: u32,
    now: Instant,
) -> Option<SweepAction> {
    let Some(pet_space) = space_mgr.get_entity_space_id(pet_id) else {
        return Some(SweepAction::Scrub);
    };
    let Some((pet_dead, despawn_at)) = space_mgr
        .get_entity(pet_id)
        .and_then(|e| Some((e.state_field & BSF_DEAD != 0, e.pet.as_ref()?.despawn_at)))
    else {
        return Some(SweepAction::Scrub);
    };
    let Some(owner_entity) = space_mgr.get_entity(owner) else {
        return Some(SweepAction::Despawn(PetDespawnReason::OwnerGone));
    };
    // Entity ids are reused: after the owner is destroyed the id can come
    // back as an NPC, or as another player, in the same space. Only the
    // player who summoned THIS pet owns it; a pet the id's new holder
    // summoned since has its own capture and is kept.
    if !owner_entity.is_player
        || !space_mgr
            .pets
            .summoner_matches(pet_id, owner_entity.identity())
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
    // The owner still holds the pet; the pet's own timer is next.
    match despawn_at {
        Some(at) if now >= at => Some(SweepAction::Despawn(if pet_dead {
            PetDespawnReason::CorpseExpired
        } else {
            PetDespawnReason::Expired
        })),
        // A fresh corpse: start its timer. The death itself happens in
        // `resolve_death`, which knows nothing about pets; stamping here
        // keeps every pet rule in this module, at the cost of up to one AoI
        // tick (100 ms) on a 10 s timer.
        None if pet_dead => Some(SweepAction::StampCorpse),
        _ => None,
    }
}

enum SweepAction {
    /// The pet entity is already gone; only the registry entry is left.
    Scrub,
    /// The pet has to go: its owner no longer holds it, or its timer ran out.
    Despawn(PetDespawnReason),
    /// The pet just died: start the corpse timer.
    StampCorpse,
}

/// The self-healing sweep: despawn every pet whose owner is gone, dead or in
/// another space, expire pet corpses, and scrub registry entries whose pet
/// entity is gone. Returns how many pets were despawned. Returns at once when
/// no pet exists, so it is cheap enough for every 100 ms AoI tick.
pub async fn pet_owner_sweep(
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) -> usize {
    pet_owner_sweep_at(Instant::now(), tx, space_mgr).await
}

/// [`pet_owner_sweep`] against an explicit clock, so tests can step past
/// the corpse timer without sleeping.
pub async fn pet_owner_sweep_at(
    now: Instant,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) -> usize {
    if space_mgr.pets.is_empty() {
        return 0;
    }
    let mut despawned = 0;
    for (pet_id, owner) in space_mgr.pets.pairs() {
        match sweep_verdict(space_mgr, pet_id, owner, now) {
            None => {}
            Some(SweepAction::StampCorpse) => {
                let id = owner_identity(space_mgr, pet_id, Some(owner));
                if let Some(pet) = space_mgr
                    .get_entity_mut(pet_id)
                    .and_then(|e| e.pet.as_deref_mut())
                {
                    pet.despawn_at = Some(now + PET_CORPSE_DESPAWN);
                }
                tracing::debug!(
                    target: "pets.lifecycle",
                    decision_outcome = "corpse_timer_started",
                    event = "corpse_timer_started",
                    entity_id = pet_id,
                    pet_id,
                    owner_id = owner,
                    account_id = id.account_id,
                    player_id = id.player_id,
                    corpse_secs = PET_CORPSE_DESPAWN.as_secs(),
                    "pet died; its corpse despawns when the timer runs out"
                );
            }
            Some(SweepAction::Scrub) => {
                let id = owner_identity(space_mgr, pet_id, Some(owner));
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
                if reason == PetDespawnReason::CorpseExpired {
                    let id = owner_identity(space_mgr, pet_id, Some(owner));
                    tracing::debug!(
                        target: "pets.lifecycle",
                        event = "corpse_expired",
                        entity_id = pet_id,
                        pet_id,
                        owner_id = owner,
                        account_id = id.account_id,
                        player_id = id.player_id,
                        corpse_secs = PET_CORPSE_DESPAWN.as_secs(),
                        "pet corpse timer ran out; despawning the corpse"
                    );
                }
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
