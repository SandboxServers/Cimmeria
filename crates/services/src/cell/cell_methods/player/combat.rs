use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;
use cimmeria_content_engine::chain::ChainEngine;
use cimmeria_entity::stats::{FOCUS, HEALTH};
use tokio::sync::mpsc;

use super::constants::*;

pub async fn dispatch(
    entity_id: u32,
    method_index: u16,
    args: &[u8],
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
    engine: &ChainEngine,
) -> bool {
    match method_index {
        CALL_FOR_AID => {
            if args.len() >= 4 {
                let respawner_id = i32::from_le_bytes([args[0], args[1], args[2], args[3]]);
                tracing::info!(entity_id, respawner_id, "callForAid");
                handle_respawn(entity_id, respawner_id, tx, space_mgr).await;
            }
            true
        }

        USE_ABILITY => {
            if args.len() >= 8 {
                let ability_id = i32::from_le_bytes([args[0], args[1], args[2], args[3]]);
                let target_id = i32::from_le_bytes([args[4], args[5], args[6], args[7]]);
                tracing::debug!(entity_id, ability_id, target_id, "useAbility");

                // Snapshot whether the target was alive *before* the ability
                // resolves. Without this, hitting an already-dead corpse would
                // re-fire fire_entity_death (and stomp the AI cleanup that
                // handle_use_ability already performed on the original kill),
                // double-counting mission progress on every post-death swing.
                let was_alive_before = if target_id > 0 {
                    space_mgr.get_entity(target_id as u32).is_some_and(|t| {
                        !t.is_player && t.stats.get(HEALTH).is_some_and(|s| s.cur > 0)
                    })
                } else {
                    false
                };

                crate::cell::abilities::handle_use_ability(
                    entity_id, ability_id, target_id, tx, space_mgr,
                )
                .await;

                // Only react to alive→dead transitions caused by *this* call.
                // handle_use_ability already handles AI/loot/XP on the kill
                // itself; we only need to fire the content-engine death event
                // here, since that's a separate concern wired off the killing
                // player's player_id.
                if was_alive_before {
                    let target_eid = target_id as u32;
                    let just_died = space_mgr
                        .get_entity(target_eid)
                        .is_some_and(|t| t.stats.get(HEALTH).is_some_and(|s| s.cur <= 0));
                    if just_died {
                        let tag = space_mgr.get_entity(target_eid).and_then(|t| t.tag.clone());
                        if let Some(tag) = tag {
                            match space_mgr.get_entity(entity_id).and_then(|e| e.player_id) {
                                Some(player_id) => {
                                    crate::cell::content::fire_entity_death(
                                        entity_id, player_id, &tag, engine, tx, space_mgr,
                                    )
                                    .await;
                                }
                                None => {
                                    tracing::warn!(
                                        entity_id, npc_tag = %tag,
                                        "Skipping entity_death event: killer entity has no player_id"
                                    );
                                }
                            }
                        }
                    }
                }
            }
            true
        }

        USE_ABILITY_ON_GROUND => {
            if args.len() >= 16 {
                let ability_id = i32::from_le_bytes([args[0], args[1], args[2], args[3]]);
                let x = f32::from_le_bytes([args[4], args[5], args[6], args[7]]);
                let y = f32::from_le_bytes([args[8], args[9], args[10], args[11]]);
                let z = f32::from_le_bytes([args[12], args[13], args[14], args[15]]);
                tracing::debug!(entity_id, ability_id, x, y, z, "useAbilityOnGroundTarget");

                // handle_use_ability_on_ground returns the entity IDs of every
                // NPC that died during this cast (primary + AoE secondaries).
                // We fire the content-engine death event for each, so kill-
                // count missions and other death-triggered chains advance for
                // every AoE kill — not just the primary. Empty Vec means
                // either no targets in radius, primary cast rejected, or
                // nothing died.
                let deaths = crate::cell::abilities::handle_use_ability_on_ground(
                    entity_id,
                    ability_id,
                    [x, y, z],
                    tx,
                    space_mgr,
                )
                .await;

                if !deaths.is_empty() {
                    // Resolve player_id once — it doesn't change across kills.
                    let player_id = space_mgr.get_entity(entity_id).and_then(|e| e.player_id);
                    for dead_eid in deaths {
                        let tag = space_mgr.get_entity(dead_eid).and_then(|t| t.tag.clone());
                        if let Some(tag) = tag {
                            match player_id {
                                Some(pid) => {
                                    crate::cell::content::fire_entity_death(
                                        entity_id, pid, &tag, engine, tx, space_mgr,
                                    )
                                    .await;
                                }
                                None => {
                                    tracing::warn!(
                                        entity_id, npc_tag = %tag, dead_eid,
                                        "Skipping entity_death event (ground target): killer entity has no player_id"
                                    );
                                }
                            }
                        }
                    }
                }
            }
            true
        }

        RESPAWN => {
            tracing::debug!(entity_id, "respawn (auto)");
            handle_respawn(entity_id, -1, tx, space_mgr).await;
            true
        }

        UNSTUCK => {
            tracing::info!(entity_id, "UNIMPLEMENTED: unstuck");
            true
        }

        RESET_MY_ABILITIES => {
            tracing::info!(entity_id, "UNIMPLEMENTED: resetMyAbilities");
            true
        }

        _ => false,
    }
}

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
async fn handle_respawn(
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
            super::super::inventory::flush_dirty_bandolier_ammo(entity, player_id, tx).await;
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
fn resolve_respawn_target(
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cell::spawner::RespawnerDef;
    use cimmeria_entity::stats::{FOCUS, HEALTH};

    /// Build a SpaceManager with one player at id=1 in the
    /// Castle_CellBlock instanced space (every dispatch test sees a
    /// fresh world). Caller can override is_player and stats.
    fn make_mgr_with_player(world: &str) -> SpaceManager {
        let mut mgr = SpaceManager::new(1);
        let xml = format!(
            r#"<?xml version="1.0"?><Spaces><Space WorldName="{world}" Instanced="true" MinX="-800" MaxX="800" MinY="-800" MaxY="800" /></Spaces>"#,
        );
        mgr.parse_spaces_xml(&xml).unwrap();
        mgr.create_startup_spaces(r#"<?xml version="1.0"?><Spaces></Spaces>"#)
            .unwrap();
        mgr.create_entity(1, world, [42.0, 1.0, 17.0], [0.0; 3])
            .unwrap();
        if let Some(p) = mgr.get_entity_mut(1) {
            p.is_player = true;
            p.player_id = Some(100);
        }
        mgr.connect_entity(1);
        mgr
    }

    #[tokio::test]
    async fn dispatch_returns_false_for_unknown_method() {
        let mut mgr = make_mgr_with_player("Castle_CellBlock");
        let engine = ChainEngine::new();
        let (tx, _rx) = mpsc::channel(8);
        let handled = dispatch(1, 9999, &[], &tx, &mut mgr, &engine).await;
        assert!(!handled);
    }

    /// USE_ABILITY with a too-short payload (< 8 bytes) must return
    /// true (handler took the method) but not start any cooldown,
    /// not consume any state, and not emit packets — the args are
    /// silently ignored. Pre-seed an ability + cooldown-free state so
    /// a regression that decodes garbage args and starts a cooldown
    /// gets caught.
    #[tokio::test]
    async fn use_ability_with_short_args_silently_drops() {
        let mut mgr = make_mgr_with_player("Castle_CellBlock");
        if let Some(p) = mgr.get_entity_mut(1) {
            p.abilities.add_ability(7);
        }
        let engine = ChainEngine::new();
        let (tx, mut rx) = mpsc::channel(8);

        let handled = dispatch(1, USE_ABILITY, &[1u8, 2, 3], &tx, &mut mgr, &engine).await;
        assert!(handled);
        assert!(
            rx.try_recv().is_err(),
            "short USE_ABILITY must not emit packets"
        );
        assert!(
            !mgr.get_entity(1).unwrap().abilities.is_on_cooldown(7),
            "short USE_ABILITY must not start a cooldown"
        );
    }

    /// Same-world `handle_respawn` keeps the cell entity (and instance)
    /// alive and sends an in-place burst that re-creates only the local
    /// pawn actor — no `RESET_ENTITIES` (which would reset client-side
    /// kismet state like the stasis-room door).
    ///
    /// Burst contract (in order):
    ///   1. `onEndAidWait` — close Defeat Window
    ///   2. `onStatUpdate` — refreshed HEALTH/FOCUS to the HUD
    ///   3. `onStateFieldUpdate(0)` — clears BSF_Dead / BSF_MovementLock /
    ///      dead-cursor visuals on the owning client
    ///   4. `ReanchorPlayer` — VIEWPORT_INFO + CREATE_CELL_PLAYER +
    ///      FORCED_POSITION (re-creates the cell-side pawn actor; clears
    ///      ragdoll without touching other client entities)
    ///
    /// Cell entity also: HEALTH/FOCUS reset to max, state flags cleared
    /// with refcounts, cooldowns cleared, position snapped.
    ///
    /// Negative pins:
    ///   - No `GateTravel` (would destroy the cell entity and tear down
    ///     the whole instance — kismet reset).
    ///   - No `TeleportPlayer` (FORCED_POSITION-only, doesn't re-create
    ///     the pawn actor — wouldn't clear ragdoll).
    ///   - No `onSequence` Entity_Spawn (cooked kismet has no
    ///     `SeqEvent_EntitySpawn → TermRagdoll` wiring; this is dead weight).
    #[tokio::test]
    async fn handle_respawn_same_world_emits_in_place_burst_with_reanchor() {
        use crate::cell::combat::{BSF_DEAD, BSF_MOVEMENT_LOCK};
        let mut mgr = make_mgr_with_player("Castle_CellBlock");
        let original_space_id = mgr.get_entity(1).unwrap().space_id.0 as u32;

        if let Some(e) = mgr.get_entity_mut(1) {
            if let Some(h) = e.stats.get_mut(HEALTH) {
                h.update(0, 1, 100);
                h.clear_dirty();
            }
            if let Some(f) = e.stats.get_mut(FOCUS) {
                f.update(0, 0, 50);
                f.clear_dirty();
            }
            e.set_state_flag(BSF_DEAD);
            e.set_state_flag(BSF_MOVEMENT_LOCK);
            assert!(
                !e.state_flag_counts.is_empty(),
                "fixture sanity: counters should be populated before respawn"
            );
            e.abilities
                .start_ability_cooldown(592, std::time::Duration::from_secs(60));
        }
        let (tx, mut rx) = mpsc::channel(16);
        handle_respawn(1, -1, &tx, &mut mgr).await;

        // Cell entity SURVIVES.
        let entity = mgr.get_entity(1).expect(
            "same-world respawn must keep the cell entity alive — destroying it tears down the \
             instance and resets client-side kismet (doors, completed encounters)",
        );
        assert_eq!(
            entity.space_id.0 as u32, original_space_id,
            "respawn must keep the entity in its original instanced space"
        );
        assert_eq!(
            entity.stats.get(HEALTH).unwrap().cur,
            100,
            "HEALTH must be restored to max in place"
        );
        assert_eq!(
            entity.stats.get(FOCUS).unwrap().cur,
            50,
            "FOCUS must be restored to max in place"
        );
        assert_eq!(entity.state_field, 0, "state_field must be cleared");
        assert!(
            entity.state_flag_counts.is_empty(),
            "respawn must clear the per-flag refcount map — a raw state_field=0 would leave \
             stale counters that the next ref-counted unset would interpret as still-positive"
        );
        assert!(
            !entity.abilities.is_on_cooldown(592),
            "respawn must clear ability cooldowns"
        );
        assert_eq!(
            [entity.position.x, entity.position.y, entity.position.z],
            [-334.231, 73.472, -228.026],
            "respawn must snap entity to spawn point"
        );

        // Track message order.
        let mut end_aid_wait_at: Option<usize> = None;
        let mut stat_update_at: Option<usize> = None;
        let mut state_field_clear_at: Option<usize> = None;
        let mut reanchor_at: Option<usize> = None;
        let mut saw_gate_travel = false;
        let mut saw_teleport_player = false;
        let mut saw_onsequence = false;
        let mut captured_pos: Option<[f32; 3]> = None;
        let mut captured_space_id: Option<u32> = None;

        let mut idx: usize = 0;
        while let Ok(m) = rx.try_recv() {
            match m {
                CellToBaseMsg::EntityMethodCall {
                    entity_id: 1,
                    method_index,
                    args,
                } => {
                    if method_index == crate::mercury::method_idx::ON_END_AID_WAIT
                        && end_aid_wait_at.is_none()
                    {
                        end_aid_wait_at = Some(idx);
                    } else if method_index == crate::mercury::method_idx::ON_STAT_UPDATE
                        && stat_update_at.is_none()
                    {
                        // serialize_dirty body must contain the refreshed HEALTH/FOCUS,
                        // not a count-prefix-only empty body. The HUD refresh is the
                        // practical reason this exists.
                        assert!(
                            args.len() > 4,
                            "onStatUpdate must carry the refreshed HEALTH/FOCUS payload"
                        );
                        stat_update_at = Some(idx);
                    } else if method_index == crate::mercury::method_idx::ON_STATE_FIELD_UPDATE {
                        assert_eq!(args.len(), 4, "onStateFieldUpdate carries u32");
                        let new_state = u32::from_le_bytes([args[0], args[1], args[2], args[3]]);
                        if new_state == 0 && state_field_clear_at.is_none() {
                            state_field_clear_at = Some(idx);
                        }
                    } else if method_index == crate::mercury::method_idx::ON_SEQUENCE {
                        saw_onsequence = true;
                    }
                }
                CellToBaseMsg::ReanchorPlayer {
                    entity_id: 1,
                    space_id,
                    position,
                    rotation,
                } => {
                    assert_eq!(rotation, [0.0; 3]);
                    captured_pos = Some(position);
                    captured_space_id = Some(space_id);
                    if reanchor_at.is_none() {
                        reanchor_at = Some(idx);
                    }
                }
                CellToBaseMsg::GateTravel { .. } => saw_gate_travel = true,
                CellToBaseMsg::TeleportPlayer { .. } => saw_teleport_player = true,
                _ => {}
            }
            idx += 1;
        }

        // Negative pins: the wrong primitives must not fire.
        assert!(
            !saw_gate_travel,
            "same-world respawn must NOT emit GateTravel — that destroys the cell entity and \
             tears down the instance, resetting kismet state"
        );
        assert!(
            !saw_teleport_player,
            "same-world respawn must NOT emit TeleportPlayer — FORCED_POSITION alone doesn't \
             re-create the pawn actor and so doesn't clear ragdoll"
        );
        assert!(
            !saw_onsequence,
            "respawn must NOT emit onSequence Entity_Spawn — cooked kismet has no \
             SeqEvent_EntitySpawn → TermRagdoll wiring; the message is dead weight"
        );

        // Positive pins.
        let end_aid_idx = end_aid_wait_at.expect("respawn must emit onEndAidWait");
        let stat_idx = stat_update_at.expect("respawn must emit onStatUpdate (HUD refresh)");
        let state_idx = state_field_clear_at.expect("respawn must emit onStateFieldUpdate(0)");
        let reanchor_idx = reanchor_at.expect(
            "respawn must emit CellToBaseMsg::ReanchorPlayer — this is the load-bearing piece \
             that re-creates the pawn actor and clears ragdoll without RESET_ENTITIES",
        );

        // Ordering: onEndAidWait → onStatUpdate → onStateFieldUpdate → ReanchorPlayer.
        // ReanchorPlayer must be last so the pawn re-creation runs after the
        // owning client has cleared dead state.
        assert!(end_aid_idx < stat_idx);
        assert!(stat_idx < state_idx);
        assert!(
            state_idx < reanchor_idx,
            "onStateFieldUpdate(0) (msg #{state_idx}) must precede ReanchorPlayer \
             (msg #{reanchor_idx}) so the dead/movement-lock state lifts before the pawn \
             actor is re-created"
        );

        assert_eq!(
            captured_pos,
            Some([-334.231, 73.472, -228.026]),
            "ReanchorPlayer must carry the resolved spawn position"
        );
        assert_eq!(
            captured_space_id,
            Some(original_space_id),
            "ReanchorPlayer must carry the entity's existing space_id — re-issuing \
             CREATE_CELL_PLAYER for a different space would corrupt the client's space tables"
        );
    }

    /// Cross-world respawn (respawner in a different world from where the
    /// player died) falls through to the gate-travel path because the
    /// player is leaving the space anyway. The instance teardown is
    /// unavoidable in that case — same-world preservation only matters
    /// when staying in the same world.
    #[tokio::test]
    async fn handle_respawn_cross_world_falls_back_to_gate_travel() {
        let mut mgr = make_mgr_with_player("Agnos_test");
        // Register a respawner in a DIFFERENT world so the resolved target
        // doesn't match the entity's current world.
        mgr.respawners.push(RespawnerDef {
            respawner_id: 7,
            world_name: "Castle_CellBlock".to_string(),
            name: "Castle Hub".to_string(),
            pos: [10.0, 20.0, 30.0],
        });

        let (tx, mut rx) = mpsc::channel(16);
        handle_respawn(1, 7, &tx, &mut mgr).await;

        // Cross-world means destroy the cell entity (gate-travel will
        // re-create it on the new world).
        assert!(
            mgr.get_entity(1).is_none(),
            "cross-world respawn must destroy the cell entity (gate-travel pattern)"
        );

        let mut saw_gate_travel = false;
        let mut saw_reanchor = false;
        let mut captured_world: Option<String> = None;
        while let Ok(m) = rx.try_recv() {
            match m {
                CellToBaseMsg::GateTravel {
                    target_world_name, ..
                } => {
                    saw_gate_travel = true;
                    captured_world = Some(target_world_name);
                }
                CellToBaseMsg::ReanchorPlayer { .. } => {
                    saw_reanchor = true;
                }
                _ => {}
            }
        }

        assert!(
            saw_gate_travel,
            "cross-world respawn must emit GateTravel (the player is leaving the space anyway)"
        );
        assert!(
            !saw_reanchor,
            "cross-world respawn must NOT emit ReanchorPlayer — that's the same-world-only path"
        );
        assert_eq!(
            captured_world.as_deref(),
            Some("Castle_CellBlock"),
            "GateTravel must target the respawner's world"
        );
    }

    /// `resolve_respawn_target` matches a `respawner_id` to its stored
    /// (world, pos) tuple. The id-match path is the primary one — pin it
    /// so a refactor that drops the iter().find() doesn't fall back
    /// silently to the world-default branch (which can pick a different
    /// respawner if multiple are registered for the same world).
    #[test]
    fn resolve_respawn_target_uses_matching_respawner_id() {
        let mut mgr = make_mgr_with_player("Castle_CellBlock");
        mgr.respawners.push(RespawnerDef {
            respawner_id: 42,
            world_name: "Castle_CellBlock".to_string(),
            name: "Hub".to_string(),
            pos: [10.0, 20.0, 30.0],
        });
        let (world, pos) = resolve_respawn_target(42, 1, &mgr);
        assert_eq!(world, "Castle_CellBlock");
        assert_eq!(pos, [10.0, 20.0, 30.0]);
    }

    /// `resolve_respawn_target` falls back to the world's first respawner
    /// when the requested id isn't found. Pin so the fallback path can't
    /// silently degrade to the Castle default when the player's world has
    /// its own respawner registered.
    #[test]
    fn resolve_respawn_target_falls_back_to_world_respawner_on_id_miss() {
        let mut mgr = make_mgr_with_player("Agnos_test");
        mgr.respawners.push(RespawnerDef {
            respawner_id: 7,
            world_name: "Agnos_test".to_string(),
            name: "Outpost".to_string(),
            pos: [-5.0, 5.0, -5.0],
        });
        // respawner_id 999 doesn't exist.
        let (world, pos) = resolve_respawn_target(999, 1, &mgr);
        assert_eq!(world, "Agnos_test");
        assert_eq!(pos, [-5.0, 5.0, -5.0]);
    }

    /// `resolve_respawn_target` returns the Castle default world+pos for
    /// `Castle_CellBlock` when no respawners exist (ship-config
    /// fallback). Pin the canonical fallback so a regression that uses
    /// the in-place path inside Castle can't silently strand players at
    /// their corpse.
    #[test]
    fn resolve_respawn_target_returns_castle_default_when_no_respawners() {
        let mgr = make_mgr_with_player("Castle_CellBlock");
        let (world, pos) = resolve_respawn_target(-1, 1, &mgr);
        assert_eq!(world, "Castle_CellBlock");
        assert_eq!(pos, [-334.231, 73.472, -228.026]);
    }

    /// `resolve_respawn_target` for non-Castle worlds with no respawner
    /// falls back to in-place (current world, current position) — NOT a
    /// cross-world snap to Castle. Pin the cross-world teleport-prevention
    /// shape: if a content gap leaves a world without a respawner, the
    /// player should respawn where they died, not get yanked across worlds.
    #[test]
    fn resolve_respawn_target_uses_in_place_for_other_worlds_without_respawners() {
        let mgr = make_mgr_with_player("Agnos_test");
        let (world, pos) = resolve_respawn_target(-1, 1, &mgr);
        assert_eq!(world, "Agnos_test");
        assert_eq!(
            pos,
            [42.0, 1.0, 17.0],
            "must respawn in place, not at Castle default"
        );
    }
}
