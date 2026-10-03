//! Respawn fork — same-world in-place reanchor (preserves the cell entity
//! and the client-side kismet state) vs. cross-world gate-travel (a
//! genuine space change, instance teardown is unavoidable).
//!
//! `resolve_respawn_target` resolves `(world, position)` from the
//! Defeat-Window-supplied respawner id, falling back through the player's
//! current world to the Castle default.
//!
//! Two callers drive [`handle_respawn`]: the player's own Defeat Window and
//! auto-respawn (`callForAid` / `respawn`, dispatched in
//! `cell_methods::player::combat`) and the native GM `gmRespawn` (in
//! `console::gm::world`). The GM console and the cell methods are sibling
//! crates in the services split, so the respawn core they share sits one layer
//! below both (`docs/architecture/services-crate-split.md` §2H), together with
//! the two client-cache replays it queues behind the reanchor:
//!
//! - [`region_registration`] — the world's client-hinted trigger volumes.
//! - [`resync`] — level, state field, the full stat set, archetype, ability
//!   tree, the hotbar, the active bandolier slot and the mission journal.
//!
//! Nothing stat- or state-related is sent before the reanchor: the client
//! destroys that pawn when the reanchor arrives.
//!
//! World entry (`InitPlayerState`) sends the same two, so it reaches them here
//! too.

use cimmeria_cell_catalog::cell::respawner_fallback::nearest_valid_respawner_def;
use cimmeria_cell_world::cell::plugin::EntityHookPoint;
use cimmeria_entity::movement_validation::SpaceBounds;
use cimmeria_entity::stats::{FOCUS, HEALTH};
use tokio::sync::mpsc;

use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;
use crate::cell::spawner::RespawnerDef;

pub mod region_registration;
pub mod resync;

#[cfg(test)]
mod tests;

pub use resync::send_known_abilities_update;

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
/// 4. The cell's own replay behind it ([`resync::resync_after_pawn_recreate`]):
///    level, state field, every stat, archetype, ability tree, hotbar,
///    active slot and missions — the player-state half of the login burst.
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
// `pub` (widened from `pub(super)`) so the native GM `gmRespawn`
// handler (`console::gm::world`) can reuse the exact same respawn sequence
// as the combat Defeat-Window path — no duplicate respawn logic.
pub async fn handle_respawn(
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

    // Discord gameplay-channel (off by default). Fires for both the
    // same-world reanchor and the cross-world gate path; `target_world` is the
    // world the player comes back up in. Cloned because it's moved into the
    // GateTravel message on the cross-world branch below.
    let respawn_character_name = space_mgr
        .get_entity(entity_id)
        .and_then(|e| e.character_name.clone())
        .unwrap_or_else(|| format!("entity:{entity_id}"));
    cimmeria_discord::emit_player_respawn(respawn_character_name, target_world.clone());

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
            cimmeria_cell_combat::cell::cell_methods::inventory::bandolier::flush_dirty_bandolier_ammo(entity, player_id, tx)
                .await;
        }
        // SS-D3, #962: the travel hook. The duel plugin ends the traveller's duel
        // (`EDUEL_DEFEAT_Teleport`) here; `every_travel_site_fires_the_travel_hook`.
        space_mgr
            .fire_entity_hook(EntityHookPoint::BeforeTravelSend, entity_id, tx)
            .await;
        // Enqueue the transfer first and tear down only once it is sent,
        // the order gate travel and `gmGotoLocation` use: a closed base
        // channel must not leave the player (or its pet) removed cell-side
        // with no transfer in flight.
        if let Err(e) = tx
            .send(CellToBaseMsg::GateTravel {
                entity_id,
                target_world_name: target_world.clone(),
                position: spawn_pos,
                rotation: [0.0; 3],
                destination_ring_id: None,
                // Respawn resolves the destination by world name.
                destination_space_id: None,
            })
            .await
        {
            let id = space_mgr.player_identity(entity_id);
            tracing::warn!(
                entity_id,
                account_id = id.account_id,
                player_id = id.player_id,
                to = %target_world,
                reason = "cell_to_base_closed",
                error = %e,
                "Respawn: cross-world GateTravel not sent; player and pets left in place"
            );
            return;
        }
        // Pets stay in this world (D-PT01: re-summon after the trip).
        cimmeria_cell_world::cell::pets::on_owner_left(
            entity_id,
            cimmeria_cell_world::cell::pets::PetDespawnReason::OwnerLeftSpace,
            cimmeria_cell_world::cell::pets::OwnerPath::Respawn,
            tx,
            space_mgr,
        )
        .await;
        space_mgr.destroy_entity(entity_id);
        tracing::info!(
            entity_id,
            from = ?current_world,
            to = %target_world,
            "Respawn: cross-world via GateTravel"
        );
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
    // The post-reanchor resync sends every stat, so the dirty set is spent.
    entity.stats.clear_dirty();

    // Hard-reset state flags + their refcounts. A raw `state_field = 0`
    // would clear the bits but leave stale counters, which the next
    // ref-counted unset would interpret as still-positive. `BSF_AutoCycling`
    // goes with the rest: the death burst already stopped the dying player's
    // own loop, and a respawn, like a relog, starts with it off.
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

    // `offered_dialog_ids` is deliberately NOT cleared here, unlike
    // everything else in this block. It is not combat state: the offers
    // were made to this same player in this same session, each is still
    // single-use, and each choice chain still evaluates its own
    // conditions when answered. Clearing would destroy exactly the case
    // DU-08 exists to protect — a zero-button dialog whose late
    // `(id, -1)` close is interleaved with a death — and buy no
    // authority in exchange. Cross-world respawn destroys the entity, so
    // the set goes with it there.

    space_mgr.update_entity_position(entity_id, spawn_pos, [0, 0, 0], [0.0; 3]);
    // Authorized teleport (death → respawn point): reseed the movement-
    // validator clock so the first post-respawn client packet isn't
    // measured against the pre-death sample (which would log a spurious
    // speed warning).
    space_mgr.note_authorized_teleport(entity_id);

    // No stat or state-field packet goes out here. The reanchor below makes
    // the client destroy this pawn and build a new one, so anything sent now
    // lands on the pawn about to go away (2026-09-28 colo playtest: the
    // HEALTH/FOCUS update and the state-field clear arrived before the
    // reanchor and the new pawn never got either). The full stat set and
    // the state field are replayed after the reanchor, in
    // `resync::resync_after_pawn_recreate`.

    // Re-anchor: CREATE_BASE_PLAYER + VIEWPORT + CREATE_CELL_PLAYER +
    // FORCED_POSITION + cached BeingAppearance/onEntityTint replay.
    // CREATE_BASE_PLAYER is the load-bearing piece — it triggers the
    // client's pawn-recreate hook, dropping the ragdoll state.
    match tx
        .send(CellToBaseMsg::ReanchorPlayer {
            entity_id,
            space_id,
            position: spawn_pos,
            rotation: [0.0; 3],
        })
        .await
    {
        // A pet still out (a GM `gmRespawn` of a living player; a death
        // already despawned it, D-PT08) comes along to the respawn point,
        // queued behind the owner's own snap like every other same-space
        // move (pets PT-02).
        Ok(()) => {
            cimmeria_cell_world::cell::pets::on_owner_teleported(
                entity_id,
                cimmeria_cell_world::cell::pets::OwnerPath::Respawn,
                tx,
                space_mgr,
            )
            .await;
        }
        // The client never gets the reanchor, so it still shows the owner
        // where it stood: its pets stay beside it. The rest of the respawn
        // (region re-registration, inventory re-push) carries on as before.
        Err(e) => {
            let id = space_mgr.player_identity(entity_id);
            tracing::warn!(
                entity_id,
                account_id = id.account_id,
                player_id = id.player_id,
                space_id,
                reason = "cell_to_base_closed",
                error = %e,
                "Respawn: ReanchorPlayer not sent; pets left in place"
            );
        }
    }

    // Re-register the world's trigger volumes after the reanchor, for the
    // same reason the inventory is re-pushed below: CREATE_BASE_PLAYER
    // recreates the client's pawn, and in the 2026-09-18 Castle playtest a
    // respawned client sent no `triggerClientHintedGenericRegion` for the
    // rest of its session (finding H8). Without the hints no `enter_region`
    // chain, ring pad or stargate volume fires. Clear-then-add, so the
    // outcome is the same whether or not the client kept its old list.
    // Queued on the same channel AFTER `ReanchorPlayer`, so the base sends
    // it behind the pawn-recreate burst.
    if let Some(world_name) = space_mgr.get_entity_world_name(entity_id) {
        let regions = region_registration::send_client_hinted_regions(
            entity_id,
            &world_name,
            region_registration::ClearFirst::Yes,
            tx,
            space_mgr,
        )
        .await;
        tracing::info!(
            entity_id,
            world = %world_name,
            regions,
            "respawn: re-registered client-hinted regions after reanchor"
        );
    }

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

    // The same pawn recreate that empties the inventory and the region list
    // also empties the hotbar, the cached active bandolier slot, the mission
    // journal and the client's cached `state_field`. Replay them the way
    // `InitPlayerState` does on login, queued behind the reanchor on the
    // same channel so they land after the client's creation transaction.
    resync::resync_after_pawn_recreate(entity_id, tx, space_mgr).await;
}

/// A respawner row sitting exactly at the world origin is an unauthored
/// placeholder, not a respawn point.
///
/// `load_respawners` copies `resources.respawners` in with no validation,
/// and the recovered data ships rows whose name survived but whose
/// coordinates did not (all four World 8 / Castle rows were `(0,0,0)`
/// until CA00; the two World 23 rows still are — see the KNOWN GAP
/// comment in `db/resources/Worlds/Seed/respawners.sql`). Treating those
/// as real is worse than having no respawner at all: because the row
/// *exists*, every fallback below it becomes unreachable and the player
/// is teleported to the world origin on death — usually out of bounds,
/// under the map, or in an inescapable void, with `unstuck` still
/// unimplemented. Skipping them instead degrades to the in-place /
/// Castle-default fallbacks, which are survivable.
///
/// Exact equality is the right test: (0,0,0) is a sentinel written by the
/// authoring gap, never a coordinate anyone would author deliberately, and
/// a tolerance band would start rejecting legitimately near-origin points
/// in worlds whose geometry straddles it. `-0.0 == 0.0` in IEEE-754, so
/// negative zeros are caught too.
fn is_unauthored(r: &RespawnerDef) -> bool {
    r.pos == [0.0, 0.0, 0.0]
}

/// The `reason` on the nearest-respawner log: why no named row was used.
fn fallback_reason(respawner_id: i32) -> &'static str {
    match respawner_id {
        0 => "respawner_id_zero",
        id if id < 0 => "respawner_id_unset",
        _ => "respawner_unusable",
    }
}

/// Resolve `(world, position)` for the respawn target.
///
/// Priority:
///   1. Explicit `respawner_id` from the Defeat Window (must be > 0).
///   2. The respawner registered for the player's current world that is
///      nearest the death position. Taken for id 0 (the Defeat Window's
///      synthetic "Respawn Point" entry), the auto-respawn `-1`, and an id
///      that matches no usable row. Decision (@Cadacious, 2026-09-28): it
///      used to be the *first* row for the world, which in Castle_CellBlock
///      was the Stasis Chamber start room, so a death in the Mess Hall sent
///      the player back through Hallway01 and an out-of-order guard kill
///      soft-locked the chain.
///   3. Castle default for `Castle_CellBlock` / unknown world.
///   4. In-place at the player's current position for any other world
///      (avoids silently teleporting players cross-world).
///
/// Respawners that fail [`is_unauthored`] are invisible to steps 1 and 2 —
/// an all-zero row is treated as absent at every priority, so the search
/// continues past it instead of returning the world origin.
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
        match space_mgr
            .respawners
            .iter()
            .find(|r| r.respawner_id == respawner_id)
        {
            Some(r) if is_unauthored(r) => {
                // Negative-log seam, `reason` pinned by
                // `origin_respawner_warn_fires_once_on_the_explicit_id_path`.
                // `world` (not `world_name`) matches the field this
                // function's other logs already use, so one ops query
                // catches every respawn-resolution event; see the
                // worknote for the convention-vs-practice note.
                tracing::warn!(
                    entity_id,
                    respawner_id,
                    respawner_name = %r.name,
                    world = %r.world_name,
                    reason = "respawner_at_origin",
                    "Respawn: requested respawner is at the world origin \
                     (unauthored coordinates) — ignoring it and falling back, \
                     so the player lands at a fallback point rather than at \
                     (0,0,0); fix the row in \
                     db/resources/Worlds/Seed/respawners.sql"
                );
            }
            Some(r) => return (r.world_name.clone(), r.pos),
            None => tracing::warn!(
                entity_id,
                respawner_id,
                reason = "respawner_not_found",
                "Respawner not found, falling back to the nearest respawner"
            ),
        }
    }

    let world_name = space_mgr.get_entity_world_name(entity_id);
    if let Some(ref wn) = world_name {
        // The player has not moved since dying (a corpse accepts no
        // movement), so its current position is the death position.
        let nearest = space_mgr.get_entity(entity_id).and_then(|e| {
            // No navmesh and the permissive AABB on purpose: a respawner row is
            // a coordinate a human authored for exactly this use, and this path
            // has never second-guessed one. The shared search still applies the
            // all-zero placeholder guard and the finite-coordinate test.
            nearest_valid_respawner_def(
                &space_mgr.respawners,
                wn,
                e.position,
                None,
                &SpaceBounds::FALLBACK,
            )
            .map(|r| {
                let p = e.position;
                let (dx, dy, dz) = (r.pos[0] - p.x, r.pos[1] - p.y, r.pos[2] - p.z);
                (r, (dx * dx + dy * dy + dz * dz).sqrt())
            })
        });
        if let Some((r, distance_m)) = nearest {
            let id = space_mgr.player_identity(entity_id);
            // Not a refusal: the Defeat Window's synthetic entry and the
            // auto-respawn timer name no respawner, so choosing one is the
            // normal path. INFO so SigNoz shows which row a death resolved to
            // (2026-09-28 playtest: id 0 sent the player to the start room,
            // far from the fight, and the run back broke the Cellblock chain).
            tracing::info!(
                target: "player.respawn",
                entity_id,
                account_id = id.account_id,
                player_id = id.player_id,
                requested_respawner_id = respawner_id,
                respawner_id = r.respawner_id,
                respawner_name = %r.name,
                world = %wn,
                distance_m,
                reason = fallback_reason(respawner_id),
                "Respawn: no usable respawner was named -- using the one nearest \
                 the death position"
            );
            return (r.world_name.clone(), r.pos);
        }
        let skipped = space_mgr
            .respawners
            .iter()
            .filter(|r| r.world_name == *wn && is_unauthored(r))
            .count();
        if skipped > 0 {
            // Negative-log seam, `reason` pinned by
            // `origin_respawner_warn_fires_once_on_the_world_scan_path`.
            tracing::warn!(
                entity_id,
                world = %wn,
                skipped,
                reason = "world_respawners_all_at_origin",
                "Respawn: every respawner registered for this world is at the \
                 origin (unauthored coordinates) — falling back, so the player \
                 respawns in place rather than at (0,0,0); fix the rows in \
                 db/resources/Worlds/Seed/respawners.sql"
            );
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
