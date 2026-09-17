//! Cross-space / cross-world player transfer primitive (packet P45).
//!
//! This is the shared mechanism behind the GM travel commands `.goto`,
//! `.summon` and `.gotolocation` (packet P46). It does exactly one thing:
//! move a *player* entity out of its current space and into a named world —
//! optionally a specific already-loaded instance of that world — by driving
//! the same teardown → `pending_world_entry` → re-enter flow that
//! player-initiated stargate travel already uses.
//!
//! # Why this can't just call `handle_gate_travel` blindly
//!
//! `crate::base::world_entry::gate_travel::handle_gate_travel` is the *base*
//! half of the flow, and it runs only after the *cell* half has already
//! removed the entity from its old space (see
//! [`crate::cell::gate_travel::handle_dial_gate`] and
//! [`crate::cell::cell_methods::gm::travel`]). The cell half is where the
//! destructive step lives, so the cell half is where validation has to
//! happen. The order enforced here is:
//!
//! 1. Validate everything (entity, player-ness, finite coordinates, world,
//!    destination instance) while the entity is still untouched.
//! 2. Flush per-entity state that would be lost on teardown.
//! 3. Enqueue `CellToBaseMsg::GateTravel` and **confirm the send**.
//! 4. Only then `destroy_entity`.
//!
//! A rejection at step 1 or a closed channel at step 3 leaves the entity in
//! its origin space with its AoI intact — there is no partial teardown to
//! roll back. (Step 2's bandolier-ammo flush is a persistence write, not a
//! state change: if step 3 then fails, what was persisted still matches the
//! live entity.)
//!
//! # Instance targeting
//!
//! `SpaceManager::find_or_create_space` — which the base's `CreateEntity`
//! round-trip normally resolves through — always allocates a **brand new**
//! space for an instanced world. It therefore cannot join an existing
//! instance, which is precisely what `.goto <player>` needs. The transfer
//! carries an explicit `destination_space_id` through `GateTravel` →
//! `CreateEntity` so the destination is exact. The id is re-validated on
//! arrival cell-side, because an instanced space is destroyed as soon as its
//! last player leaves and that can happen while the message is in flight.
//!
//! # Players only (decision D15)
//!
//! Cross-world transfer is the player-client world-entry handshake
//! (`RESET_ENTITIES` → `pending_world_entry` → re-enter). An NPC has no
//! client to hand that to, so an NPC subject is rejected rather than
//! silently teleported or half-transferred. This matches P26's precedent of
//! keeping NPC travel cell-side.

use tokio::sync::mpsc;

use super::messages::CellToBaseMsg;
use super::space_manager::SpaceManager;

#[cfg(test)]
mod tests;

/// Where a transfer is going.
#[derive(Debug, Clone, PartialEq)]
pub struct TransferDestination {
    /// Destination world name, as it appears in `spaces.xml`.
    pub world_name: String,
    /// Exact loaded instance to join. `None` selects the D15 default —
    /// the first/default loaded instance of `world_name`, or a freshly
    /// allocated one when the world has no instance loaded.
    pub space_id: Option<u32>,
    pub position: [f32; 3],
    pub rotation: [f32; 3],
}

impl TransferDestination {
    /// Destination in the default instance of a world (the `.gotolocation`
    /// shape).
    pub fn in_world(world_name: impl Into<String>, position: [f32; 3]) -> Self {
        Self {
            world_name: world_name.into(),
            space_id: None,
            position,
            rotation: [0.0; 3],
        }
    }

    /// Destination in one exact loaded instance (the `.goto`/`.summon`
    /// shape, where P44 has resolved a specific player's actual instance).
    pub fn in_instance(world_name: impl Into<String>, space_id: u32, position: [f32; 3]) -> Self {
        Self {
            world_name: world_name.into(),
            space_id: Some(space_id),
            position,
            rotation: [0.0; 3],
        }
    }
}

/// Why a transfer was refused. Every variant means **nothing was torn
/// down** — the subject entity is still in its origin space.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TransferRejected {
    /// No cell entity with that id (already gone, or never existed).
    EntityNotFound,
    /// Subject is an NPC. D15: cross-world transfer is players-only.
    NotAPlayer,
    /// Destination coordinates contain NaN/inf. A non-finite position would
    /// corrupt the destination spatial grid and the persisted `sgw_player`
    /// row, so it is rejected at the boundary rather than sanitised.
    NonFinitePosition,
    /// World is not in `spaces.xml`. Caught here, before teardown — the
    /// base side has no way to refuse it once the entity is already gone.
    UnknownWorld(String),
    /// World exists but has no loaded instance and can't get one (a
    /// non-instanced world missing its `cell_spaces.xml` startup space).
    WorldNotLoadable(String),
    /// An exact instance was requested but isn't loaded.
    InstanceNotLoaded(u32),
    /// An exact instance was requested but belongs to a different world.
    InstanceWorldMismatch {
        space_id: u32,
        requested_world: String,
        actual_world: String,
    },
    /// The base channel is closed. The entity is deliberately left in place
    /// rather than removed with no transfer in flight.
    EnqueueFailed,
}

impl TransferRejected {
    /// One-line operator/GM-facing reason. Command adapters (P46) prepend
    /// their own command name and may substitute legacy wording.
    pub fn describe(&self) -> String {
        match self {
            Self::EntityNotFound => "entity not found".to_string(),
            Self::NotAPlayer => {
                "target is not a player — cross-world transfer needs a client".to_string()
            }
            Self::NonFinitePosition => "non-finite coordinate rejected".to_string(),
            Self::UnknownWorld(w) => format!("Unable to find world: {w}"),
            Self::WorldNotLoadable(w) => format!("world {w} has no loaded space"),
            Self::InstanceNotLoaded(sid) => format!("instance {sid} is no longer loaded"),
            Self::InstanceWorldMismatch {
                space_id,
                requested_world,
                actual_world,
            } => format!("instance {space_id} is in world {actual_world}, not {requested_world}"),
            Self::EnqueueFailed => "transfer enqueue failed".to_string(),
        }
    }
}

/// What a successful call did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransferOutcome {
    /// The destination resolved to the space the entity is already in.
    /// **Nothing was torn down.** A full `GateTravel` reload here would be a
    /// gratuitous loading screen, so the caller should finish the move with
    /// the same-space authoritative snap instead (`CellToBaseMsg::
    /// TeleportPlayer`, the P26 `.gotoxyz` path).
    SameSpace { space_id: u32 },
    /// `GateTravel` was enqueued and the entity was removed from its origin
    /// space. `space_id` is the exact destination instance, or `None` when
    /// the destination world had no loaded instance and one will be
    /// allocated by the create path.
    Transferred { space_id: Option<u32> },
}

/// Move `entity_id` to `dest`, validating fully before any teardown.
///
/// See the module docs for the ordering contract. On `Err`, the entity's
/// origin space, position and AoI are untouched.
///
/// # Load-bearing invariant
///
/// This function holds `&mut SpaceManager` across **both** of its `.await`
/// points, and that exclusive borrow is the only thing closing the
/// validate → enqueue → destroy window. Nothing else can mutate the space
/// tables in between, so the destination instance validated in phase 1 is
/// still the one named on the message sent in phase 3, and the entity torn
/// down in phase 4 is still the one inspected in phase 1.
///
/// A refactor that takes `&SpaceManager` and re-acquires the mutable borrow
/// later, or that splits the phases across a `select!` / spawned task, silently
/// reopens that window — and no existing test would fail. Keep the borrow.
#[tracing::instrument(
    name = "space_transfer.execute",
    level = "info",
    skip_all,
    fields(entity_id, world = %dest.world_name, requested_space_id = ?dest.space_id)
)]
pub async fn transfer_player_to_space(
    entity_id: u32,
    dest: &TransferDestination,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) -> Result<TransferOutcome, TransferRejected> {
    // ── Phase 1: validate. No mutation past this point until phase 2. ──
    if !dest.position.iter().all(|c| c.is_finite()) || !dest.rotation.iter().all(|c| c.is_finite())
    {
        tracing::warn!(
            entity_id, position = ?dest.position, rotation = ?dest.rotation,
            "space_transfer: non-finite destination rejected"
        );
        return Err(TransferRejected::NonFinitePosition);
    }

    // `entity_space` is the index `create_entity`/`destroy_entity` maintain and
    // the one `get_entity` itself resolves through, so it — not the entity's
    // own cached `space_id` field — is what the SameSpace comparison below has
    // to be made against.
    // Both reads come from the same borrow and `get_entity` resolves through
    // that same index, so they cannot disagree — one rejection arm covers them.
    let subject = space_mgr.get_entity_space_id(entity_id).zip(
        space_mgr
            .get_entity(entity_id)
            .map(|e| (e.is_player, e.player_id)),
    );
    let Some((origin_space_id, (is_player, player_id))) = subject else {
        tracing::warn!(entity_id, "space_transfer: subject entity not found");
        return Err(TransferRejected::EntityNotFound);
    };

    // D15: players only. An NPC has no client to run the world-entry
    // handshake, so a "transfer" would destroy it and never rebuild it.
    if !is_player {
        tracing::warn!(
            entity_id,
            "space_transfer: subject is not a player — cross-world transfer refused (D15)"
        );
        return Err(TransferRejected::NotAPlayer);
    }

    if !space_mgr.world_is_known(&dest.world_name) {
        tracing::warn!(
            entity_id, world = %dest.world_name,
            "space_transfer: unknown destination world"
        );
        return Err(TransferRejected::UnknownWorld(dest.world_name.clone()));
    }

    let destination_space_id = resolve_destination_space(dest, space_mgr)?;

    // Already there: report it and let the caller take the cheap path.
    if destination_space_id == Some(origin_space_id) {
        tracing::debug!(
            entity_id,
            origin_space_id,
            "space_transfer: destination is the entity's current space — no transfer needed"
        );
        return Ok(TransferOutcome::SameSpace {
            space_id: origin_space_id,
        });
    }

    // ── Phase 2: flush state that teardown would drop. ──
    // Same reason `handle_dial_gate` does it: the cross-world respawn
    // rebuilds the entity from the DB, so anything still sitting in
    // `bandolier_ammo_dirty` is lost. This is a persistence write, not a
    // state change — if phase 3 fails, the live entity still matches it.
    if let Some(pid) = player_id {
        if let Some(entity) = space_mgr.get_entity_mut(entity_id) {
            super::cell_methods::inventory::flush_dirty_bandolier_ammo(entity, pid, tx).await;
        }
    }

    // ── Phase 3: enqueue, and only tear down once the send is confirmed. ──
    // A closed base channel must not leave the player removed cell-side with
    // no transfer in flight — that is the "un-spaced" state.
    if tx
        .send(CellToBaseMsg::GateTravel {
            entity_id,
            target_world_name: dest.world_name.clone(),
            position: dest.position,
            rotation: dest.rotation,
            // GM travel is never a ring transport; that field belongs to
            // `Effect::TeleportCrossWorld`.
            destination_ring_id: None,
            destination_space_id,
        })
        .await
        .is_err()
    {
        tracing::warn!(
            entity_id, world = %dest.world_name,
            "space_transfer: base channel closed — entity left in place"
        );
        return Err(TransferRejected::EnqueueFailed);
    }

    // ── Phase 4: teardown. ──
    // Cancel any open trade FIRST. `destroy_entity` does not clean trade
    // state, which is why both lifecycle arms (`DestroyEntity` and
    // `DisconnectEntity`) call this helper explicitly — and it has to run
    // *before* the entity goes, because it early-returns as soon as
    // `get_entity` misses. Without it, a `.summon` of a player mid-trade
    // leaves their partner holding a `trade_partner_entity_id` pointing at a
    // freed id, with no `onTradeResults(Cancelled)` — a stranded session that
    // only a relog clears.
    //
    // It runs after the confirmed enqueue, so the "a rejection changes
    // nothing" contract above still holds: by this point the transfer is
    // committed.
    super::cell_methods::player::trade::cancel_trade_on_disconnect(entity_id, tx, space_mgr).await;
    space_mgr.destroy_entity(entity_id);

    tracing::info!(
        entity_id,
        origin_space_id,
        ?destination_space_id,
        world = %dest.world_name,
        "space_transfer: GateTravel enqueued, entity torn out of origin space"
    );

    Ok(TransferOutcome::Transferred {
        space_id: destination_space_id,
    })
}

/// Resolve `dest` to an exact loaded instance, or `None` meaning "the create
/// path should allocate a fresh instance of this world".
///
/// Split out as a pure function so the resolution rules can be unit-tested
/// without driving a whole transfer.
pub(crate) fn resolve_destination_space(
    dest: &TransferDestination,
    space_mgr: &SpaceManager,
) -> Result<Option<u32>, TransferRejected> {
    match dest.space_id {
        // Exact instance requested (P44 resolved a specific player's space).
        Some(sid) => match space_mgr.world_name_for_space(sid) {
            Some(w) if w == dest.world_name => Ok(Some(sid)),
            Some(other) => Err(TransferRejected::InstanceWorldMismatch {
                space_id: sid,
                requested_world: dest.world_name.clone(),
                actual_world: other.to_string(),
            }),
            None => Err(TransferRejected::InstanceNotLoaded(sid)),
        },
        // D15 default: first/default loaded instance of the world.
        None => match space_mgr.default_space_for_world(&dest.world_name) {
            Some(sid) => Ok(Some(sid)),
            // No instance loaded. For an instanced world that's normal —
            // the create path allocates one. For a non-instanced world it
            // means the world is missing from `cell_spaces.xml`, and
            // `find_or_create_space` would fail base-side *after* teardown,
            // so refuse now.
            None if space_mgr.is_world_instanced(&dest.world_name) => Ok(None),
            None => Err(TransferRejected::WorldNotLoadable(dest.world_name.clone())),
        },
    }
}
