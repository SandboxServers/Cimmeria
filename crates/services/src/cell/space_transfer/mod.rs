//! Cross-space / cross-world player transfer primitive (packet P45).
//!
//! This is the shared mechanism behind the GM travel commands `.goto`,
//! `.summon`, `.gotolocation` and `.gotospace` (packet P46). It does exactly
//! one thing: move a *player* entity out of its current space and into
//! another, by driving the same teardown → `pending_world_entry` → re-enter
//! flow that player-initiated stargate travel already uses.
//!
//! Two entry points, differing only in how the destination is named:
//! [`transfer_player_to_space`] takes a world name (optionally plus an exact
//! instance) and resolves it through the `spaces.xml` table, and
//! [`transfer_player_to_loaded_space`] takes a verified space id and skips
//! that table entirely. Everything after resolution — validation, ordering,
//! teardown — is the same code.
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
    /// Destination world name. Case variants of a declared world are
    /// accepted: [`transfer_player_to_space`] runs this through
    /// [`SpaceManager::canonical_world_name`] and every comparison
    /// downstream — instance resolution, the `GateTravel` message, the
    /// base-side `find_or_create_space` — uses the canonical `spaces.xml`
    /// spelling. A world the table does not declare at all is
    /// [`TransferRejected::UnknownWorld`]; reaching a live instance whose
    /// world is absent from the table is what
    /// [`transfer_player_to_loaded_space`] is for.
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
/// The destination world is resolved **through the world-name table**: see
/// [`TransferDestination::world_name`]. Use
/// [`transfer_player_to_loaded_space`] when the caller already holds a
/// verified space id and the table has nothing to add.
///
/// See the module docs for the ordering contract. On `Err`, the entity's
/// origin space, position and AoI are untouched.
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
    execute_transfer(
        entity_id,
        Destination::Named {
            world_name: &dest.world_name,
            space_id: dest.space_id,
        },
        dest.position,
        dest.rotation,
        tx,
        space_mgr,
    )
    .await
}

/// Move `entity_id` into one **already-loaded space instance**, named by id.
///
/// This is the `.gotospace` shape, and the difference from
/// [`transfer_player_to_space`] is destination *resolution* only — the
/// validate → enqueue → teardown ordering is identical. The world name is
/// read off the instance instead of being looked up, so `spaces.xml` is never
/// consulted and **any live instance is reachable whether or not the world
/// table declares its world**. That is the entire point of the command: every
/// other travel leg can only reach a world the table knows, spelled the way
/// the table spells it.
///
/// The id is re-checked here rather than trusted from the caller. Callers do
/// hold the exclusive `&mut SpaceManager` borrow across their own check and
/// this one, so it cannot have gone stale in between — but the by-id path has
/// no by-world-name fallback waiting for it on arrival. `handle_create_entity`
/// degrades an unusable `destination_space_id` to `find_or_create_space`,
/// which for a world the table does not declare fails *after* teardown and
/// strands the player in no space at all. Refusing a vanished instance here
/// keeps that arm unreachable.
#[tracing::instrument(
    name = "space_transfer.execute_by_id",
    level = "info",
    skip_all,
    fields(entity_id, requested_space_id = space_id)
)]
pub async fn transfer_player_to_loaded_space(
    entity_id: u32,
    space_id: u32,
    position: [f32; 3],
    rotation: [f32; 3],
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) -> Result<TransferOutcome, TransferRejected> {
    execute_transfer(
        entity_id,
        Destination::LoadedSpace(space_id),
        position,
        rotation,
        tx,
        space_mgr,
    )
    .await
}

/// How a transfer's destination was named, which decides how much of the
/// world-name table the resolution below has to consult.
enum Destination<'a> {
    /// A world name that may have been typed, plus optionally the exact
    /// instance the caller resolved for itself. Canonicalised against
    /// `spaces.xml`; an undeclared world is refused.
    Named {
        world_name: &'a str,
        space_id: Option<u32>,
    },
    /// One instance the caller has already confirmed is loaded. The world
    /// name is derived *from* that instance, so the table is skipped.
    LoadedSpace(u32),
}

/// Shared core of both entry points.
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
async fn execute_transfer(
    entity_id: u32,
    dest: Destination<'_>,
    position: [f32; 3],
    rotation: [f32; 3],
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) -> Result<TransferOutcome, TransferRejected> {
    // ── Phase 1: validate. No mutation past this point until phase 2. ──
    if !position.iter().all(|c| c.is_finite()) || !rotation.iter().all(|c| c.is_finite()) {
        tracing::warn!(
            entity_id,
            ?position,
            ?rotation,
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

    // Resolved once `is_player` is settled, so the warns below can name the
    // account rather than the recycled entity slot — and *before* the phase-4
    // teardown, after which `player_identity` can only answer UNKNOWN.
    let id = space_mgr.player_identity(entity_id);

    let (world_name, destination_space_id) = match dest {
        Destination::Named {
            world_name,
            space_id,
        } => {
            // Canonicalise before any name comparison. The world table is
            // keyed on the exact `spaces.xml` spelling, but this name may be
            // typed by a GM — `.gotolocation harset ...` must reach `Harset`,
            // not dead-end on "Unable to find world". Everything downstream
            // (instance resolution, the `GateTravel` message, the base-side
            // `find_or_create_space`) uses the canonical spelling so the
            // exact-match invariant holds.
            let Some(canonical) = space_mgr
                .canonical_world_name(world_name)
                .map(str::to_owned)
            else {
                tracing::warn!(
                    entity_id,
                    account_id = id.account_id,
                    player_id = id.player_id,
                    world = %world_name,
                    "space_transfer: unknown destination world"
                );
                return Err(TransferRejected::UnknownWorld(world_name.to_string()));
            };
            let sid = resolve_destination_space(&canonical, space_id, space_mgr)?;
            (canonical, sid)
        }
        // The world name is derived from the instance, so there is nothing
        // for the table to validate — and consulting it anyway would re-reject
        // a destination the caller already proved reachable, which is the one
        // thing this path exists to avoid.
        Destination::LoadedSpace(space_id) => {
            let Some(world_name) = space_mgr.world_name_for_space(space_id).map(str::to_owned)
            else {
                tracing::warn!(
                    entity_id,
                    account_id = id.account_id,
                    player_id = id.player_id,
                    requested_space_id = space_id,
                    "space_transfer: destination instance is no longer loaded"
                );
                return Err(TransferRejected::InstanceNotLoaded(space_id));
            };
            (world_name, Some(space_id))
        }
    };

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
            target_world_name: world_name.clone(),
            position,
            rotation,
            // GM travel is never a ring transport; that field belongs to
            // `Effect::TeleportCrossWorld`.
            destination_ring_id: None,
            destination_space_id,
        })
        .await
        .is_err()
    {
        tracing::warn!(
            entity_id,
            account_id = id.account_id,
            player_id = id.player_id,
            world = %world_name,
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
        account_id = id.account_id,
        player_id = id.player_id,
        origin_space_id,
        ?destination_space_id,
        world = %world_name,
        "space_transfer: GateTravel enqueued, entity torn out of origin space"
    );

    Ok(TransferOutcome::Transferred {
        space_id: destination_space_id,
    })
}

/// Resolve a destination to an exact loaded instance, or `None` meaning "the
/// create path should allocate a fresh instance of this world".
///
/// `world_name` must already be the canonical `spaces.xml` spelling (see
/// [`SpaceManager::canonical_world_name`]) — every comparison below is exact.
///
/// Split out as a pure function so the resolution rules can be unit-tested
/// without driving a whole transfer.
pub(crate) fn resolve_destination_space(
    world_name: &str,
    requested_space_id: Option<u32>,
    space_mgr: &SpaceManager,
) -> Result<Option<u32>, TransferRejected> {
    match requested_space_id {
        // Exact instance requested (P44 resolved a specific player's space).
        Some(sid) => match space_mgr.world_name_for_space(sid) {
            Some(w) if w == world_name => Ok(Some(sid)),
            Some(other) => Err(TransferRejected::InstanceWorldMismatch {
                space_id: sid,
                requested_world: world_name.to_string(),
                actual_world: other.to_string(),
            }),
            None => Err(TransferRejected::InstanceNotLoaded(sid)),
        },
        // D15 default: first/default loaded instance of the world.
        None => match space_mgr.default_space_for_world(world_name) {
            Some(sid) => Ok(Some(sid)),
            // No instance loaded. For an instanced world that's normal —
            // the create path allocates one. For a non-instanced world it
            // means the world is missing from `cell_spaces.xml`, and
            // `find_or_create_space` would fail base-side *after* teardown,
            // so refuse now.
            None if space_mgr.is_world_instanced(world_name) => Ok(None),
            None => Err(TransferRejected::WorldNotLoadable(world_name.to_string())),
        },
    }
}
