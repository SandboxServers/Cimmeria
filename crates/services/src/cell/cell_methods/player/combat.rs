use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;
use cimmeria_content_engine::chain::ChainEngine;
use cimmeria_entity::stats::HEALTH;
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

/// Hand off to the gate-travel reload path so the client tears down the
/// ragdolled pawn and re-enters the world fresh.
///
/// Why a reload (vs an in-place flow): the in-place path needs the cooked
/// kismet package to wire `SeqEvent_EntitySpawn → APawn::TermRagdoll`, and
/// the shipped client's `KIS-abilities_human.Death` package was authored
/// only for the death animation — the spawn event has no output connected
/// to TermRagdoll. (Python emulator notes "Entity_Spawn was never completed",
/// and two attempts at an in-place burst both left the player ragdolled
/// after teleport.) The cooked `.upk` is not modifiable from the server.
///
/// The reload sidesteps the kismet wiring entirely: `RESET_ENTITIES`
/// destroys the ragdolled pawn, and the follow-up world-entry sequence
/// re-creates a fresh pawn with default state. Same path stargates use,
/// and it properly handshakes via `pending_world_entry` / `ENABLE_ENTITIES`
/// (no `pending_map_loaded` gap — that gap was the bug class in the
/// previously-deleted `RespawnReload` handler).
async fn handle_respawn(
    entity_id: u32,
    respawner_id: i32,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    if space_mgr.get_entity(entity_id).is_none() {
        tracing::warn!(entity_id, "respawn: entity not found");
        return;
    }

    let (target_world, spawn_pos) = resolve_respawn_target(respawner_id, entity_id, space_mgr);

    // Close the Defeat Window before kicking off the reload — otherwise the
    // loading screen renders on top of the still-open "Player Defeated"
    // panel and the panel persists into the post-respawn frame for one tick.
    let _ = tx
        .send(CellToBaseMsg::EntityMethodCall {
            entity_id,
            method_index: crate::mercury::method_idx::ON_END_AID_WAIT,
            args: Vec::new(),
        })
        .await;

    // Flush pending bandolier ammo before we destroy the cell entity —
    // anything still in `bandolier_ammo_dirty` is silently lost across the
    // re-create. Mirrors `handle_dial_gate`'s flush.
    if let Some(entity) = space_mgr.get_entity_mut(entity_id) {
        if let Some(player_id) = entity.player_id {
            super::super::inventory::flush_dirty_bandolier_ammo(entity, player_id, tx).await;
        }
    }

    // Tear down the cell entity. The reload path re-creates it via
    // `BaseToCellMsg::CreateEntity`, and the world-entry sequence that
    // follows sends `InitPlayerState` to repopulate player_id / abilities /
    // bandolier / missions on the fresh entity. Stats (HEALTH/FOCUS) come
    // from archetype defaults baked into mapLoaded — no separate refresh
    // needed.
    space_mgr.destroy_entity(entity_id);

    // Hand off to the BaseApp gate-travel handler. It will:
    //   1. Send `BaseToCellMsg::CreateEntity` → cell creates fresh entity at
    //      (target_world, spawn_pos).
    //   2. Persist destination world+position to `sgw_player`.
    //   3. Send `RESET_ENTITIES` to the client → destroys all entities
    //      including the ragdolled pawn (kismet ragdoll dies with the pawn;
    //      no Entity_Spawn / TermRagdoll dance needed).
    //   4. Set `pending_world_entry` → the client's next `ENABLE_ENTITIES`
    //      drives the create-player + enter-world + mapLoaded sequence.
    let _ = tx
        .send(CellToBaseMsg::GateTravel {
            entity_id,
            target_world_name: target_world,
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

    /// `handle_respawn` is the load-bearing piece of CALL_FOR_AID.
    /// Hands off to the gate-travel reload path: the cell tears down its
    /// own entity (so the BaseApp re-creates it cleanly via the standard
    /// world-entry flow), closes the Defeat Window with `onEndAidWait`,
    /// and emits a `CellToBaseMsg::GateTravel` carrying the resolved
    /// respawn world + position.
    ///
    /// Why no in-place burst: the previous version emitted an
    /// `Entity_Spawn` ON_SEQUENCE expecting the client kismet to call
    /// `APawn::TermRagdoll` on the local pawn. Empirically that didn't
    /// work — the cooked `KIS-abilities_human.Death` package's
    /// `SeqEvent_EntitySpawn` node has no output wired to TermRagdoll
    /// (matching the Python emulator's "Entity_Spawn was never
    /// completed" note from Ghidra inspection). The reload destroys the
    /// pawn outright so kismet wiring is irrelevant.
    ///
    /// What the test pins:
    ///   - `onEndAidWait` fires BEFORE the GateTravel handoff (so the
    ///     loading screen doesn't render on top of the Defeat Window).
    ///   - The cell entity is destroyed (the reload re-creates it).
    ///   - `GateTravel` carries the resolved respawn world + position.
    ///   - No `onSequence` Entity_Spawn / `onStateFieldUpdate` /
    ///     `TeleportPlayer` from the cell — those were the in-place
    ///     primitives, the reload subsumes them.
    #[tokio::test]
    async fn handle_respawn_hands_off_to_gate_travel_reload() {
        use crate::cell::combat::{BSF_DEAD, BSF_MOVEMENT_LOCK};
        let mut mgr = make_mgr_with_player("Castle_CellBlock");

        if let Some(e) = mgr.get_entity_mut(1) {
            // Damaged + flagged dead + an ability on cooldown. None of this
            // server-side state needs to be cleared by handle_respawn
            // anymore — the entity is destroyed, fresh state comes from
            // the post-reload InitPlayerState + mapLoaded sequence.
            if let Some(h) = e.stats.get_mut(HEALTH) {
                h.update(0, 1, 100);
            }
            if let Some(f) = e.stats.get_mut(FOCUS) {
                f.update(0, 0, 50);
            }
            e.set_state_flag(BSF_DEAD);
            e.set_state_flag(BSF_MOVEMENT_LOCK);
            e.abilities
                .start_ability_cooldown(592, std::time::Duration::from_secs(60));
        }
        let (tx, mut rx) = mpsc::channel(16);
        handle_respawn(1, -1, &tx, &mut mgr).await;

        // Cell entity destroyed (the reload re-creates it via
        // `BaseToCellMsg::CreateEntity`).
        assert!(
            mgr.get_entity(1).is_none(),
            "respawn must destroy the cell entity so the reload path re-creates it cleanly — \
             a regression that left the entity around would double up the entity_id and \
             confuse AoI bookkeeping"
        );

        // Track which messages fire and in what order.
        let mut end_aid_wait_at: Option<usize> = None;
        let mut gate_travel_at: Option<usize> = None;
        let mut saw_onsequence = false;
        let mut saw_state_field_update = false;
        let mut saw_teleport_player = false;
        let mut captured_world: Option<String> = None;
        let mut captured_pos: Option<[f32; 3]> = None;

        let mut idx: usize = 0;
        while let Ok(m) = rx.try_recv() {
            match m {
                CellToBaseMsg::EntityMethodCall {
                    entity_id: 1,
                    method_index,
                    ..
                } => {
                    if method_index == crate::mercury::method_idx::ON_END_AID_WAIT
                        && end_aid_wait_at.is_none()
                    {
                        end_aid_wait_at = Some(idx);
                    } else if method_index == crate::mercury::method_idx::ON_SEQUENCE {
                        saw_onsequence = true;
                    } else if method_index == crate::mercury::method_idx::ON_STATE_FIELD_UPDATE {
                        saw_state_field_update = true;
                    }
                }
                CellToBaseMsg::GateTravel {
                    entity_id: 1,
                    target_world_name,
                    position,
                    rotation,
                } => {
                    assert_eq!(rotation, [0.0; 3]);
                    captured_world = Some(target_world_name);
                    captured_pos = Some(position);
                    if gate_travel_at.is_none() {
                        gate_travel_at = Some(idx);
                    }
                }
                CellToBaseMsg::TeleportPlayer { .. } => {
                    saw_teleport_player = true;
                }
                _ => {}
            }
            idx += 1;
        }

        // Negative pins: the in-place primitives must NOT fire — those
        // were the ragdoll-stuck-after-respawn class of bugs. The reload
        // subsumes them.
        assert!(
            !saw_onsequence,
            "respawn must NOT emit onSequence (Entity_Spawn) — the cooked client kismet has no \
             SeqEvent_EntitySpawn → TermRagdoll wiring, so this primitive can't end ragdoll on \
             the local pawn. The reload path destroys the pawn instead."
        );
        assert!(
            !saw_state_field_update,
            "respawn must NOT emit onStateFieldUpdate(0) — the post-reload mapLoaded sequence \
             includes ON_STATE_FIELD_UPDATE(0) by default; emitting it from the cell ahead of \
             the reload duplicates the wire and races the destroy/recreate"
        );
        assert!(
            !saw_teleport_player,
            "respawn must NOT emit TeleportPlayer — the gate-travel reload provides the position \
             snap via the standard create-player + mapLoaded flow"
        );

        // Positive pins.
        let end_aid_idx =
            end_aid_wait_at.expect("respawn must emit onEndAidWait to close the Defeat Window");
        let gate_idx = gate_travel_at.expect(
            "respawn must emit CellToBaseMsg::GateTravel — this is the reload trigger that \
             tears down the ragdolled pawn and replays world entry",
        );

        // Ordering: onEndAidWait BEFORE GateTravel. If GateTravel lands
        // first the loading screen renders on top of the still-open
        // "Player Defeated" panel for one tick.
        assert!(
            end_aid_idx < gate_idx,
            "onEndAidWait (msg #{end_aid_idx}) must precede GateTravel (msg #{gate_idx}) so the \
             Defeat Window closes before the loading screen kicks in"
        );

        // Resolved target: Castle_CellBlock has no respawner registered
        // in the test fixture, so resolve_respawn_target falls back to
        // the Castle default position.
        assert_eq!(
            captured_world.as_deref(),
            Some("Castle_CellBlock"),
            "GateTravel must carry the resolved respawn world"
        );
        assert_eq!(
            captured_pos,
            Some([-334.231, 73.472, -228.026]),
            "GateTravel must carry the resolved respawn position (Castle default)"
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
