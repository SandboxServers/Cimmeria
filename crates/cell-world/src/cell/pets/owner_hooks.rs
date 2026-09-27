//! Owner lifecycle hooks (pets PT-02, audit A-31): the two calls every
//! owner path makes, so the pet rules live here and not at eleven call
//! sites.
//!
//! - [`on_owner_left`]: the owner is leaving its space or dying. Every pet
//!   it has out is despawned now, with `LeftAoI` to the witnesses. Called
//!   before the owner's `destroy_entity` on logout, base destroy,
//!   cross-world respawn, stargate travel, GM/space transfer,
//!   `gmGotoLocation`, the content cross-world teleport and the cross-world
//!   ring, and from `resolve_death` for a dying owner (D-PT08).
//! - [`on_owner_teleported`]: the owner was moved within its space
//!   (same-world respawn, `.goto`/`.summon`/`.location`, native `gmGoto*`,
//!   the content `teleport` action, and the same-world ring at the moment
//!   the owner reappears). Every live pet is moved beside it and the move is
//!   fanned out to the pet's witnesses as an `EntityMoved`, the same relay
//!   the NPC movement tick uses. Callers queue it behind the owner's own
//!   snap (`TeleportPlayer` / `ReanchorPlayer`). A pet is an NPC: it never
//!   gets `TeleportPlayer` / `onPlayerTeleport`.
//!
//! The self-healing `pet_owner_sweep` stays as the backstop for any path
//! that does not call these.
//!
//! Ownership is the pet's summoner identity, not the bare owner id: entity
//! ids are reused, so a player given a destroyed owner's id is not that
//! pet's owner (`PetRegistry::summoner_matches`). A teleport moves only
//! the pets the teleported player summoned; any other pet registered under
//! the id is despawned as `owner_gone`, as the sweep would.
//!
//! Telemetry (target `pets.lifecycle`): every row names the pet, the owner
//! and the owner's `account_id` / `player_id` (resolved before any
//! teardown; on a per-pet row, the pet's summoner), and the owner path that
//! called (`path`). DEBUG `event = owner_left`, `owner_teleported`; the
//! misses `teleport_skipped` and `grounding_missed` carry a `reason`.

use std::time::Instant;

use cimmeria_common::Vector3;
use cimmeria_entity::cell_entity::PlayerIdentity;
use cimmeria_wire::state_field::BSF_DEAD;
use tokio::sync::mpsc;

use super::super::messages::CellToBaseMsg;
use super::super::service::npc_ai::detectors::MoveSource;
use super::super::service::npc_ai::movement_stop::snap_npc_from;
use super::super::space_manager::{DespawnOutcome, SpaceManager};
use super::spawn::beside_owner;
use super::teardown::{despawn_pet_via, owner_identity, PetDespawnReason};

/// Which owner path called a hook: the `path` field of every row the hook
/// logs, so a despawn or move can be traced to the travel that caused it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OwnerPath {
    /// Client disconnect / logout (`disconnect_entity`).
    Disconnect,
    /// Base `DestroyEntity`.
    BaseDestroy,
    /// The owner died (`resolve_death`).
    OwnerDeath,
    /// A respawn, cross-world or same-world.
    Respawn,
    /// Stargate travel.
    GateTravel,
    /// The space-transfer primitive (GM `.goto` / `.summon` / `.gotospace`
    /// / `.gotolocation` to another space).
    SpaceTransfer,
    /// Native GM travel: `gmGotoLocation`, `gmGoto`, `gmGotoXYZ`,
    /// `gmSummon`.
    GmTravel,
    /// The `.`-console travel commands within the space.
    ConsoleTravel,
    /// The `.location` console command.
    ConsoleLocation,
    /// A content-chain teleport, same- or cross-world.
    ContentTeleport,
    /// A ring transport, same- or cross-world.
    Ring,
}

impl OwnerPath {
    /// Stable `path` value for logs.
    pub fn label(self) -> &'static str {
        match self {
            Self::Disconnect => "disconnect",
            Self::BaseDestroy => "base_destroy",
            Self::OwnerDeath => "owner_death",
            Self::Respawn => "respawn",
            Self::GateTravel => "gate_travel",
            Self::SpaceTransfer => "space_transfer",
            Self::GmTravel => "gm_travel",
            Self::ConsoleTravel => "console_travel",
            Self::ConsoleLocation => "console_location",
            Self::ContentTeleport => "content_teleport",
            Self::Ring => "ring",
        }
    }
}

/// The identity to name on an owner-level row about `owner` leaving: its
/// live identity while it is still a player in the space, else the first
/// pet's summoner capture. `UNKNOWN`, never a zero.
pub(super) fn leaving_owner_identity(
    space_mgr: &SpaceManager,
    owner: u32,
    pets: &[u32],
) -> PlayerIdentity {
    let live = space_mgr.player_identity(owner);
    if live.is_known() {
        return live;
    }
    pets.first().map_or(PlayerIdentity::UNKNOWN, |&pet| {
        owner_identity(space_mgr, pet, None)
    })
}

/// Despawn every pet `owner` has out, for `reason`. Returns how many were
/// despawned. Cheap for a player without pets (one map lookup, no log), so
/// callers on player-only paths call it unconditionally.
///
/// Call it BEFORE the owner's `destroy_entity`: an owner that is the last
/// player in an instanced space takes the space down with it, and the pet
/// must be gone through `despawn_npc` first so its witnesses see it leave.
pub async fn on_owner_left(
    owner: u32,
    reason: PetDespawnReason,
    path: OwnerPath,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) -> usize {
    // A same-world ring trip the owner is abandoning must not move a pet
    // (there is none left to move) when a later `ShowPlayer` arrives.
    space_mgr.pets.take_owner_moved(owner);
    let pets = space_mgr.pets.pets_of(owner);
    if pets.is_empty() {
        return 0;
    }
    // Before the despawns, and before the caller's teardown of the owner.
    let id = leaving_owner_identity(space_mgr, owner, &pets);
    tracing::debug!(
        target: "pets.lifecycle",
        event = "owner_left",
        entity_id = owner,
        owner_id = owner,
        account_id = id.account_id,
        player_id = id.player_id,
        reason = reason.reason(),
        path = path.label(),
        pet_count = pets.len(),
        "pet owner leaving; despawning its pets"
    );
    let mut despawned = 0;
    for pet_id in pets {
        // The entity at `owner` is the one leaving: its view goes with it.
        let outcome = despawn_pet_via(space_mgr, pet_id, reason, path.label(), true, tx).await;
        if matches!(outcome, DespawnOutcome::Despawned { .. }) {
            despawned += 1;
        }
    }
    despawned
}

/// The owner reappeared after a same-world ring trip (`Effect::ShowPlayer`,
/// on arrival or on an abort release). Its pets follow only if the trip
/// really moved it ([`super::PetRegistry::note_owner_moved`], set when the
/// ring's `TeleportPlayer` went out): an aborted or failed trip leaves the
/// owner where it stood, and the pets stay too. Returns how many pets moved.
pub async fn on_owner_reappeared(
    owner: u32,
    path: OwnerPath,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) -> usize {
    if space_mgr.pets.take_owner_moved(owner) {
        return on_owner_teleported(owner, path, tx, space_mgr).await;
    }
    let pets = space_mgr.pets.pets_of(owner);
    if !pets.is_empty() {
        let id = leaving_owner_identity(space_mgr, owner, &pets);
        tracing::debug!(
            target: "pets.lifecycle",
            event = "teleport_skipped",
            entity_id = owner,
            owner_id = owner,
            account_id = id.account_id,
            player_id = id.player_id,
            path = path.label(),
            reason = "owner_not_moved",
            pet_count = pets.len(),
            "pet owner reappeared without having been moved; pets stay"
        );
    }
    0
}

/// How [`grounded_spot_behind`] placed the pet: the `grounding` field of the
/// `owner_teleported` row, and the `reason` of a `grounding_missed` row.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Grounding {
    /// Walked along the navmesh: on the floor, short of any wall.
    Navmesh,
    /// The space has no navmesh; the raw spot at the owner's height.
    NoNavmesh,
    /// The space has a navmesh but the owner is not on it; the raw spot.
    OwnerOffMesh,
}

impl Grounding {
    fn label(self) -> &'static str {
        match self {
            Self::Navmesh => "navmesh",
            Self::NoNavmesh => "no_navmesh",
            Self::OwnerOffMesh => "owner_off_mesh",
        }
    }
}

/// Where a pet lands after its owner's teleport: [`beside_owner`], walked
/// there along the navmesh from the owner's feet so it stops at a wall
/// instead of crossing it and stands on the floor instead of at the owner's
/// (client-reported, possibly airborne) height. Without a mesh, or with the
/// owner off it, the raw spot: the same as the summon.
fn grounded_spot_behind(
    space_mgr: &SpaceManager,
    owner: u32,
    owner_space: u32,
    owner_pos: Vector3,
    owner_dir: Vector3,
) -> ([f32; 3], Grounding) {
    let raw = beside_owner(owner_pos, owner_dir);
    match space_mgr.move_along_navmesh(owner, &owner_pos, &Vector3::new(raw[0], raw[1], raw[2])) {
        Some(p) => ([p.x, p.y, p.z], Grounding::Navmesh),
        None if space_mgr
            .spaces
            .get(&owner_space)
            .is_some_and(|s| s.navmesh.is_some()) =>
        {
            (raw, Grounding::OwnerOffMesh)
        }
        None => (raw, Grounding::NoNavmesh),
    }
}

/// Move every live pet of `owner` beside it after the owner was teleported
/// within its space. Call it after the owner's position is written. Returns
/// how many pets were moved.
///
/// - The spot is [`grounded_spot_behind`] the owner.
/// - A pet in another space than the owner is despawned
///   (`OwnerLeftSpace`); a same-space move cannot reach it.
/// - A dead pet stays where it fell; its corpse timer removes it.
/// - The pet stops (path cleared, velocity zero), faces the owner's
///   heading, and its `last_teleport_at` is stamped, which is the clock the
///   follow teleport rate limit (D-PT07) reads.
/// - Witnesses are the pet's current witness set (last AoI tick): each gets
///   an `EntityMoved` now, so the owner and anyone who saw the pet at its
///   old spot see it jump at once. The next AoI tick sends `LeftAoI` to a
///   witness now out of range and `EnteredAoI` to one newly in range.
pub async fn on_owner_teleported(
    owner: u32,
    path: OwnerPath,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) -> usize {
    let pets = space_mgr.pets.pets_of(owner);
    if pets.is_empty() {
        return 0;
    }
    let live = space_mgr.player_identity(owner);
    let id = leaving_owner_identity(space_mgr, owner, &pets);
    let owner_state = space_mgr.get_entity_space_id(owner).zip(
        space_mgr
            .get_entity(owner)
            .map(|e| (e.position, e.direction)),
    );
    let Some((owner_space, (owner_pos, owner_dir))) = owner_state else {
        // Every caller moves a live owner, so this is a caller bug, not a
        // client-triggerable state.
        tracing::warn!(
            target: "pets.lifecycle",
            event = "teleport_skipped",
            entity_id = owner,
            owner_id = owner,
            account_id = id.account_id,
            player_id = id.player_id,
            path = path.label(),
            reason = "owner_not_found",
            pet_count = pets.len(),
            "pet owner teleported but the owner is in no space; pets left to the sweep"
        );
        return 0;
    };
    let (dest, grounding) =
        grounded_spot_behind(space_mgr, owner, owner_space, owner_pos, owner_dir);
    if grounding != Grounding::Navmesh {
        tracing::debug!(
            target: "pets.lifecycle",
            event = "grounding_missed",
            entity_id = owner,
            owner_id = owner,
            account_id = id.account_id,
            player_id = id.player_id,
            path = path.label(),
            reason = grounding.label(),
            space_id = owner_space,
            "pet teleport spot not walked on the navmesh; using the raw spot"
        );
    }
    let now = Instant::now();

    let mut moved = 0;
    for pet_id in pets {
        // The pet's own summoner, which on a matched pet is the owner.
        let id = owner_identity(space_mgr, pet_id, Some(owner));
        if !space_mgr.pets.summoner_matches(pet_id, live) {
            // The teleported player holds the owner id but did not summon
            // this pet: its summoner is gone and the id was reused. Never
            // pull it after the new holder.
            tracing::debug!(
                target: "pets.lifecycle",
                event = "teleport_skipped",
                entity_id = pet_id,
                pet_id,
                owner_id = owner,
                account_id = id.account_id,
                player_id = id.player_id,
                holder_account_id = live.account_id,
                holder_player_id = live.player_id,
                path = path.label(),
                reason = "owner_identity_mismatch",
                "teleported player did not summon this pet; despawning it"
            );
            let _ = despawn_pet_via(
                space_mgr,
                pet_id,
                PetDespawnReason::OwnerGone,
                path.label(),
                // The id's new holder stays: it is a witness like any other.
                false,
                tx,
            )
            .await;
            continue;
        }
        if space_mgr.get_entity_space_id(pet_id) != Some(owner_space) {
            tracing::debug!(
                target: "pets.lifecycle",
                event = "teleport_skipped",
                entity_id = pet_id,
                pet_id,
                owner_id = owner,
                account_id = id.account_id,
                player_id = id.player_id,
                path = path.label(),
                reason = "pet_in_other_space",
                "pet not in its teleported owner's space; despawning it"
            );
            // `despawn_pet_via` logs the outcome itself (INFO or WARN).
            let _ = despawn_pet_via(
                space_mgr,
                pet_id,
                PetDespawnReason::OwnerLeftSpace,
                path.label(),
                false,
                tx,
            )
            .await;
            continue;
        }
        let Some(from) = space_mgr
            .get_entity(pet_id)
            .filter(|e| e.state_field & BSF_DEAD == 0)
            .map(|e| e.position)
        else {
            tracing::debug!(
                target: "pets.lifecycle",
                event = "teleport_skipped",
                entity_id = pet_id,
                pet_id,
                owner_id = owner,
                account_id = id.account_id,
                player_id = id.player_id,
                path = path.label(),
                reason = "pet_dead",
                "dead pet left where it fell; its corpse timer removes it"
            );
            continue;
        };

        snap_npc_from(
            space_mgr,
            pet_id,
            Vector3::new(dest[0], dest[1], dest[2]),
            Some(owner_dir),
            MoveSource::PetTeleport,
        );
        if let Some(pet) = space_mgr
            .get_entity_mut(pet_id)
            .and_then(|e| e.pet.as_deref_mut())
        {
            pet.last_teleport_at = Some(now);
        }

        let witnesses = space_mgr.get_witnesses_of(pet_id);
        let mut witnesses_notified = 0usize;
        for witness_id in witnesses {
            match tx
                .send(CellToBaseMsg::EntityMoved {
                    witness_id,
                    entity_id: pet_id,
                    space_id: owner_space,
                    position: dest,
                    direction: [owner_dir.x, owner_dir.y, owner_dir.z],
                    velocity: [0.0; 3],
                    npc_moved_since_last: None,
                })
                .await
            {
                Ok(()) => witnesses_notified += 1,
                Err(e) => tracing::warn!(
                    target: "pets.lifecycle",
                    decision_outcome = "teleport_relay_failed",
                    event = "teleport_relay_failed",
                    entity_id = pet_id,
                    pet_id,
                    owner_id = owner,
                    account_id = id.account_id,
                    player_id = id.player_id,
                    witness_id,
                    path = path.label(),
                    reason = "entity_moved_send_failed",
                    error = %e,
                    "pet teleport relay failed; the witness sees the old spot \
                     until the next AoI tick"
                ),
            }
        }
        let (dx, dy, dz) = (dest[0] - from.x, dest[1] - from.y, dest[2] - from.z);
        tracing::debug!(
            target: "pets.lifecycle",
            decision_outcome = "teleported_with_owner",
            event = "owner_teleported",
            entity_id = pet_id,
            pet_id,
            owner_id = owner,
            account_id = id.account_id,
            player_id = id.player_id,
            path = path.label(),
            space_id = owner_space,
            from_x = from.x,
            from_y = from.y,
            from_z = from.z,
            to_x = dest[0],
            to_y = dest[1],
            to_z = dest[2],
            distance = (dx * dx + dy * dy + dz * dz).sqrt(),
            grounding = grounding.label(),
            witnesses_notified,
            "pet moved beside its teleported owner"
        );
        moved += 1;
    }
    moved
}
