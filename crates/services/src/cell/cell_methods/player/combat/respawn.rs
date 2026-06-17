//! Respawn fork — same-world in-place reanchor (preserves the cell entity
//! and the client-side kismet state) vs. cross-world gate-travel (a
//! genuine space change, instance teardown is unavoidable).
//!
//! `resolve_respawn_target` resolves `(world, position)` from the
//! Defeat-Window-supplied respawner id, falling back through the player's
//! current world to the Castle default.

use cimmeria_entity::stats::{FOCUS, HEALTH};
use tokio::sync::mpsc;

use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;

/// In-place respawn: keep the cell entity (and instance) alive, send a
/// targeted client-side burst that re-creates just the pawn actor and
/// repopulates its visible properties.
///
/// Why in-place: `RESET_ENTITIES` (the gate-travel sledgehammer) tears down
/// every client-side entity, which fires kismet `OnInit`/`OnSpawn` events
/// and resets door states / completed encounters / triggered sequences.
/// The user dying in a room with an opened door comes back to find it
/// closed again. The server-side instance survives (cell entity preserved),
/// but the *client-side* kismet state was the visible regression.
///
/// What we send instead (handled in [`reanchor_player.rs`](../../../base/world_entry/reanchor_player.rs)):
/// 1. `BASEMSG_CREATE_BASE_PLAYER` — the load-bearing pawn-recreate
///    primitive. Invokes the client's `createBasePlayer` callback, which
///    destroys the ragdolled pawn actor and instantiates a fresh standing
///    one (same hook used on initial login).
/// 2. `BASEMSG_SPACE_VIEWPORT_INFO` + `BASEMSG_CREATE_CELL_PLAYER` +
///    `BASEMSG_FORCED_POSITION` — the gate-travel "enter-world body",
///    keeps the client's space/viewport tables consistent with the new
///    pawn and snaps it to spawn.
/// 3. `BeingAppearance` + `onEntityTint` (separate bundle) — replays the
///    cached appearance args from initial world entry so the new pawn
///    isn't blank.
///
/// AoI entities, kismet sequence state, and the level itself are
/// untouched because we never send `RESET_ENTITIES`.
///
/// Why not the prior in-place attempts (recorded so future-us doesn't
/// repeat the iteration):
/// - `onSequence Entity_Spawn` (event 5000): the cooked
///   `KIS-abilities_human.Death` package's `SeqEvent_EntitySpawn` node
///   has no output wired to `APawn::TermRagdoll` (confirmed via Ghidra),
///   so the kismet path is a no-op.
/// - `CREATE_CELL_PLAYER`-only burst (no `CREATE_BASE_PLAYER` prefix):
///   instance preserved, but pawn stayed ragdolled. The client's
///   `createCellPlayer` handler treats a re-issue for an existing
///   player id as a space/viewport update, not a pawn recreate.
/// - `CREATE_BASE_PLAYER` prefix without property replay: pawn was
///   re-created cleanly (un-ragdolled) but had no properties, leaving
///   the player invisible. Pulling appearance/tint from the cached
///   world-entry args closed the loop.
/// - Full `RESET_ENTITIES` reload: clears ragdoll cleanly but resets
///   all client kismet (door re-closes, encounters re-fire). Rejected.
///
/// Cross-world respawn (different world from where the player died) still
/// falls through to `GateTravel` because the player is genuinely leaving
/// the space. Instance teardown is unavoidable in that case.
#[tracing::instrument(
    name = "combat.respawn",
    level = "info",
    skip_all,
    fields(
        entity_id,
        respawner_id,
        target_world = tracing::field::Empty,
        same_world = tracing::field::Empty,
    ),
)]
// `pub(crate)` (widened from `pub(super)`) so the native GM `gmRespawn`
// handler (`cell_methods::gm::world`) can reuse the exact same respawn
// sequence as the combat Defeat-Window path — no duplicate respawn logic.
pub(crate) async fn handle_respawn(
    entity_id: u32,
    respawner_id: i32,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    let space_id = match space_mgr.get_entity(entity_id) {
        Some(e) => e.space_id.0 as u32,
        None => {
            tracing::warn!(entity_id, "respawn: entity not found");
            return;
        }
    };

    let (target_world, spawn_pos) = resolve_respawn_target(respawner_id, entity_id, space_mgr);

    let current_world = space_mgr.get_entity_world_name(entity_id);
    let same_world = current_world.as_deref() == Some(target_world.as_str());

    // Backfill the parent span — these resolve via the respawner DB
    // table + current cell, so they only become known after the
    // lookup. Pinning them now means "respawn took the cross-world
    // gate path" is queryable in SigNoz without parsing log text.
    let span = tracing::Span::current();
    span.record("target_world", target_world.as_str());
    span.record("same_world", same_world);

    // Close the Defeat Window first.
    let _ = tx
        .send(CellToBaseMsg::EntityMethodCall {
            entity_id,
            method_index: crate::mercury::method_idx::ON_END_AID_WAIT,
            args: Vec::new(),
        })
        .await;

    if !same_world {
        // Cross-world: full gate-travel reload (player is leaving the space).
        // Entity existence guaranteed by the early-return above.
        let entity = space_mgr
            .get_entity_mut(entity_id)
            .expect("entity existence checked above");
        if let Some(player_id) = entity.player_id {
            super::super::super::inventory::flush_dirty_bandolier_ammo(entity, player_id, tx).await;
        }
        space_mgr.destroy_entity(entity_id);
        tracing::info!(
            entity_id,
            from = ?current_world,
            to = %target_world,
            "Respawn: cross-world via GateTravel"
        );
        let _ = tx
            .send(CellToBaseMsg::GateTravel {
                entity_id,
                target_world_name: target_world,
                position: spawn_pos,
                rotation: [0.0; 3],
                destination_ring_id: None,
            })
            .await;
        return;
    }

    // Same-world: reset cell-entity state in place, then dispatch
    // `ReanchorPlayer` to drive the client-side pawn recreate +
    // appearance replay.
    let entity = space_mgr
        .get_entity_mut(entity_id)
        .expect("entity existence checked above");
    if let Some(h) = entity.stats.get_mut(HEALTH) {
        h.set_current(h.max);
    }
    if let Some(f) = entity.stats.get_mut(FOCUS) {
        f.set_current(f.max);
    }
    let stat_update = entity.stats.serialize_dirty();
    entity.stats.clear_dirty();

    // Hard-reset state flags + their refcounts. A raw `state_field = 0`
    // would clear the bits but leave stale counters, which the next
    // ref-counted unset would interpret as still-positive.
    entity.clear_all_state_flags();
    entity.abilities.clear_all_cooldowns();

    // Re-establish the BSF_IN_COMBAT ↔ threatened_mobs invariant. The
    // state-flag reset above zeroed `state_field` (including
    // BSF_IN_COMBAT), but `threatened_mobs` is a separate set and
    // survives same-world respawn unless explicitly cleared. Leaving
    // it populated means:
    //   - The HUD says OOC (state_field=0) while
    //     `threatened_mobs.is_empty()` is false — `needs_unholster_queue`
    //     skips, `enter_player_combat` no-ops on the next aggro because
    //     the set is still non-empty, and the OOC holster timer never
    //     arms (it gates on the set becoming empty via
    //     `exit_player_combat`).
    //   - The first real `exit_player_combat` after the bogus mobs
    //     "die" (or get evicted) drops the set to empty but with
    //     `state_field & BSF_IN_COMBAT == 0` already, so the
    //     transition broadcast is suppressed — no harm there, but
    //     until that drain happens the player is permanently in the
    //     stuck-drawn-weapon state.
    // Cross-world respawn destroys + recreates the entity, so the
    // set is implicitly reset there; same-world has to do it manually.
    entity.threatened_mobs.clear();

    space_mgr.update_entity_position(entity_id, spawn_pos, [0, 0, 0], [0.0; 3]);

    // Push the refreshed HEALTH/FOCUS to the HUD via onStatUpdate.
    if !stat_update.is_empty() {
        crate::cell::abilities::send_entity_method(
            entity_id,
            crate::mercury::method_idx::ON_STAT_UPDATE,
            stat_update,
            tx,
            space_mgr,
        )
        .await;
    }

    // Clear `state_field` on the owning client (lifts BSF_Dead /
    // BSF_MovementLock / dead-cursor visuals).
    crate::cell::abilities::send_entity_method(
        entity_id,
        crate::mercury::method_idx::ON_STATE_FIELD_UPDATE,
        0u32.to_le_bytes().to_vec(),
        tx,
        space_mgr,
    )
    .await;

    // Re-anchor: CREATE_BASE_PLAYER + VIEWPORT + CREATE_CELL_PLAYER +
    // FORCED_POSITION + cached BeingAppearance/onEntityTint replay.
    // CREATE_BASE_PLAYER is the load-bearing piece — it triggers the
    // client's pawn-recreate hook, dropping the ragdoll state.
    let _ = tx
        .send(CellToBaseMsg::ReanchorPlayer {
            entity_id,
            space_id,
            position: spawn_pos,
            rotation: [0.0; 3],
        })
        .await;

    // Re-push the full inventory snapshot after the reanchor.
    //
    // CREATE_BASE_PLAYER above re-instantiates the client-side pawn
    // actor, and the new pawn's `InventoryComponent` is empty — the
    // ragdolled pawn (and its inventory cache) was just destroyed by
    // the pawn-recreate hook. The bandolier visual survives because
    // it's part of the cached `BeingAppearance.ComponentList`
    // replayed above, but the actual `sgw_inventory` rows
    // (consumables, mission items like Frost's Letter, anything in
    // the main bag) live in a separate `onUpdateItem` snapshot that
    // the server only pushes when the client asks via `listItems`.
    //
    // On initial login the client invokes `listItems` itself after
    // pawn-creation; on respawn it doesn't (it thinks it's the same
    // pawn identity). Without this push, the bag panel renders
    // empty until relog AND subsequent pickups silently no-op
    // because their incremental `onUpdateItem` lands against an
    // empty client-side cache.
    //
    // Cross-world respawn (handled in the early-return branch above
    // via GateTravel) doesn't need this because it re-runs the full
    // world-entry handshake, during which the client invokes
    // `listItems` of its own accord.
    let player_id = space_mgr.get_entity(entity_id).and_then(|e| e.player_id);
    if let Some(player_id) = player_id {
        if let Err(e) = tx
            .send(CellToBaseMsg::ListInventoryItems {
                entity_id,
                player_id,
            })
            .await
        {
            tracing::warn!(
                entity_id,
                player_id,
                error = %e,
                "respawn: ListInventoryItems send failed -- inventory will not repopulate \
                 post-respawn (bag panel will stay empty until relog)"
            );
        }
    } else {
        tracing::warn!(
            entity_id,
            "respawn: entity has no player_id; skipping post-reanchor inventory snapshot \
             (NPC respawn? this branch should be player-only)"
        );
    }
}

/// Resolve `(world, position)` for the respawn target.
///
/// Priority:
///   1. Explicit `respawner_id` from the Defeat Window (must be > 0).
///   2. First respawner registered for the player's current world.
///   3. Castle default for `Castle_CellBlock` / unknown world.
///   4. In-place at the player's current position for any other world
///      (avoids silently teleporting players cross-world).
///
/// Operational note: in-place respawn outside Castle can produce death
/// loops if the player died standing in damaging geometry (lava tile, AoE
/// pool) and no respawner is configured for that world — they'll respawn
/// at full health, take the geometry damage tick, and die again. The
/// clean fix is content-side: every world should ship at least one
/// respawner. The fallback warn log is the operator signal.
pub(super) fn resolve_respawn_target(
    respawner_id: i32,
    entity_id: u32,
    space_mgr: &SpaceManager,
) -> (String, [f32; 3]) {
    const CASTLE_WORLD: &str = "Castle_CellBlock";
    const CASTLE_DEFAULT_POS: [f32; 3] = [-334.231, 73.472, -228.026];

    if respawner_id > 0 {
        if let Some(r) = space_mgr
            .respawners
            .iter()
            .find(|r| r.respawner_id == respawner_id)
        {
            return (r.world_name.clone(), r.pos);
        }
        tracing::warn!(
            entity_id,
            respawner_id,
            "Respawner not found, falling back to world default"
        );
    }

    let world_name = space_mgr.get_entity_world_name(entity_id);
    if let Some(ref wn) = world_name {
        if let Some(r) = space_mgr.respawners.iter().find(|r| r.world_name == *wn) {
            return (r.world_name.clone(), r.pos);
        }
    }

    match world_name.as_deref() {
        Some(CASTLE_WORLD) | None => {
            tracing::debug!(entity_id, world = ?world_name, "No respawner; using Castle default position");
            (CASTLE_WORLD.to_string(), CASTLE_DEFAULT_POS)
        }
        Some(world) => {
            let in_place = space_mgr
                .get_entity(entity_id)
                .map(|e| [e.position.x, e.position.y, e.position.z])
                .unwrap_or(CASTLE_DEFAULT_POS);
            tracing::warn!(
                entity_id,
                world = world,
                "No respawner configured for this world — respawning in place at current position"
            );
            (world.to_string(), in_place)
        }
    }
}
