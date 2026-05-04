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

async fn handle_respawn(
    entity_id: u32,
    respawner_id: i32,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    let entity = match space_mgr.get_entity_mut(entity_id) {
        Some(e) => e,
        None => {
            tracing::warn!(entity_id, "respawn: entity not found");
            return;
        }
    };

    if let Some(health) = entity.stats.get_mut(HEALTH) {
        health.set_current(health.max);
    }
    if let Some(focus) = entity.stats.get_mut(FOCUS) {
        focus.set_current(focus.max);
    }

    let stat_update = entity.stats.serialize_dirty();
    entity.stats.clear_dirty();

    // Respawn is a hard reset — drop both the bit pattern AND the per-flag
    // counters so we don't leak refs from death's BSF_Dead/BSF_MovementLock
    // sets. Raw `state_field = 0` would clear the bits but leave stale
    // counters that the next ref-counted unset would see as still-positive.
    entity.clear_all_state_flags();
    entity.abilities.clear_all_cooldowns();

    tracing::info!(entity_id, "Player respawned, state_field=0");

    // Push the refreshed health/focus to the client via onEntityStat (method 20)
    // — without this, mapLoaded after RespawnReload would query the stale DB
    // values and the player would render with their pre-death stats.
    if !stat_update.is_empty() {
        crate::cell::abilities::send_entity_method(entity_id, 20, stat_update, tx, space_mgr).await;
    }

    let _ = tx
        .send(CellToBaseMsg::EntityMethodCall {
            entity_id,
            method_index: crate::mercury::method_idx::ON_END_AID_WAIT,
            args: Vec::new(),
        })
        .await;

    let spawn_pos: [f32; 3] = resolve_respawn_position(respawner_id, entity_id, space_mgr);
    let world_name = space_mgr
        .get_entity_world_name(entity_id)
        .unwrap_or_else(|| "Castle_CellBlock".to_string());
    space_mgr.update_entity_position(entity_id, spawn_pos, [0, 0, 0], [0.0; 3]);

    if let Err(e) = tx
        .send(CellToBaseMsg::RespawnReload {
            entity_id,
            world_name: world_name.clone(),
            spawn_pos,
        })
        .await
    {
        tracing::error!(
            entity_id, %world_name, ?spawn_pos, error = %e,
            "RespawnReload send to base failed -- player will not be teleported to spawn"
        );
        return;
    }
    tracing::info!(entity_id, ?spawn_pos, "Sent RespawnReload to BaseApp");
}

fn resolve_respawn_position(
    respawner_id: i32,
    entity_id: u32,
    space_mgr: &SpaceManager,
) -> [f32; 3] {
    // Castle_CellBlock starting hub coordinates — only used as a final fallback
    // for that world; in other worlds we respawn in place rather than teleporting
    // players to Castle.
    const CASTLE_DEFAULT_POS: [f32; 3] = [-334.231, 73.472, -228.026];

    if respawner_id > 0 {
        if let Some(resp) = space_mgr
            .respawners
            .iter()
            .find(|r| r.respawner_id == respawner_id)
        {
            return resp.pos;
        }
        tracing::warn!(
            entity_id,
            respawner_id,
            "Respawner not found, falling back to world default"
        );
    }

    let world_name = space_mgr.get_entity_world_name(entity_id);
    if let Some(ref wn) = world_name {
        if let Some(resp) = space_mgr.respawners.iter().find(|r| r.world_name == *wn) {
            return resp.pos;
        }
    }

    // Castle has a known safe default; for other worlds, respawn in place to
    // avoid silently teleporting the player across worlds.
    //
    // Operational note: in-place respawn outside Castle can produce death
    // loops if the player died standing in damaging geometry (e.g., a lava
    // tile or AoE pool) and no respawner is configured for that world —
    // they'll respawn at full health, immediately take damage from the
    // surrounding geometry, and die again. The clean fix is content-side:
    // every world should ship at least one respawner. This warn log is the
    // signal to operators that a world is missing one. A future combat pass
    // can also add a brief invuln window after respawn to absorb the first
    // damage tick if the player happens to respawn inside an active hazard.
    match world_name.as_deref() {
        Some("Castle_CellBlock") | None => {
            tracing::debug!(entity_id, world = ?world_name, "No respawner; using Castle default position");
            CASTLE_DEFAULT_POS
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
            in_place
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
    /// Must restore HEALTH/FOCUS to max, clear all state flags, clear
    /// ability cooldowns, update entity position to the resolved
    /// spawn point (Castle default for Castle_CellBlock), and fire
    /// onEndAidWait + RespawnReload.
    #[tokio::test]
    async fn handle_respawn_restores_stats_clears_flags_and_sends_respawn_reload() {
        use crate::cell::combat::{BSF_DEAD, BSF_MOVEMENT_LOCK};
        let mut mgr = make_mgr_with_player("Castle_CellBlock");
        if let Some(e) = mgr.get_entity_mut(1) {
            // Damaged + flagged dead + an ability on cooldown. Use the
            // refcounting set_state_flag helpers so the
            // state_flag_counts map gets populated; that way the
            // respawn assertion can distinguish `clear_all_state_flags`
            // (which empties both `state_field` AND
            // `state_flag_counts`) from a raw `state_field = 0` (which
            // would leave stale counter entries — the regression shape
            // this test guards against).
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

        let e = mgr.get_entity(1).unwrap();
        assert_eq!(
            e.stats.get(HEALTH).unwrap().cur,
            100,
            "HEALTH must be restored to max"
        );
        assert_eq!(
            e.stats.get(FOCUS).unwrap().cur,
            50,
            "FOCUS must be restored to max"
        );
        assert_eq!(e.state_field, 0, "state_field must be cleared");
        assert!(
            e.state_flag_counts.is_empty(),
            "respawn must clear the per-flag refcount map too — \
             a raw state_field=0 would leave stale counters and the \
             next ref-counted unset would underflow back to a stuck bit"
        );
        assert!(
            !e.abilities.is_on_cooldown(592),
            "respawn must clear ability cooldowns"
        );
        // Player started at [42.0, 1.0, 17.0] in make_mgr_with_player.
        // Castle_CellBlock has no respawner registered → fallback to
        // CASTLE_DEFAULT_POS = [-334.231, 73.472, -228.026]. Pin so a
        // regression that drops the update_entity_position call
        // (leaving the player at their corpse) gets caught.
        assert_eq!(
            [e.position.x, e.position.y, e.position.z],
            [-334.231, 73.472, -228.026],
            "respawn must teleport player to Castle default position"
        );

        // Drain rx and check for RespawnReload.
        let mut saw_respawn_reload = false;
        let mut saw_end_aid_wait = false;
        while let Ok(m) = rx.try_recv() {
            match m {
                CellToBaseMsg::RespawnReload {
                    entity_id,
                    world_name,
                    ..
                } => {
                    assert_eq!(entity_id, 1);
                    assert_eq!(world_name, "Castle_CellBlock");
                    saw_respawn_reload = true;
                }
                CellToBaseMsg::EntityMethodCall {
                    entity_id,
                    method_index,
                    ..
                } if entity_id == 1
                    && method_index == crate::mercury::method_idx::ON_END_AID_WAIT =>
                {
                    saw_end_aid_wait = true;
                }
                _ => {}
            }
        }
        assert!(saw_end_aid_wait, "respawn must emit onEndAidWait");
        assert!(saw_respawn_reload, "respawn must emit RespawnReload");
    }

    /// `resolve_respawn_position` matches a respawner_id to its
    /// stored position. The id-match path is the primary one — pin
    /// it so a refactor that drops the iter().find() doesn't fall
    /// back silently.
    #[test]
    fn resolve_respawn_position_uses_matching_respawner_id() {
        let mut mgr = make_mgr_with_player("Castle_CellBlock");
        mgr.respawners.push(RespawnerDef {
            respawner_id: 42,
            world_name: "Castle_CellBlock".to_string(),
            name: "Hub".to_string(),
            pos: [10.0, 20.0, 30.0],
        });
        let pos = resolve_respawn_position(42, 1, &mgr);
        assert_eq!(pos, [10.0, 20.0, 30.0]);
    }

    /// `resolve_respawn_position` falls back to the world's first
    /// respawner when the requested id isn't found. Pin so the
    /// fallback path can't silently degrade to the Castle default
    /// when the player's world has its own respawner registered.
    #[test]
    fn resolve_respawn_position_falls_back_to_world_respawner_on_id_miss() {
        let mut mgr = make_mgr_with_player("Agnos_test");
        mgr.respawners.push(RespawnerDef {
            respawner_id: 7,
            world_name: "Agnos_test".to_string(),
            name: "Outpost".to_string(),
            pos: [-5.0, 5.0, -5.0],
        });
        // respawner_id 999 doesn't exist.
        let pos = resolve_respawn_position(999, 1, &mgr);
        assert_eq!(pos, [-5.0, 5.0, -5.0]);
    }

    /// `resolve_respawn_position` returns CASTLE_DEFAULT_POS for
    /// Castle_CellBlock when no respawners exist (ship-config
    /// fallback). Pin the canonical fallback so a regression that
    /// uses the in-place path inside Castle can't silently strand
    /// players at their corpse.
    #[test]
    fn resolve_respawn_position_returns_castle_default_when_no_respawners() {
        let mgr = make_mgr_with_player("Castle_CellBlock");
        let pos = resolve_respawn_position(-1, 1, &mgr);
        assert_eq!(pos, [-334.231, 73.472, -228.026]);
    }

    /// `resolve_respawn_position` for non-Castle worlds with no
    /// respawner falls back to in-place (the player's current
    /// position), NOT the Castle default. Pin the cross-world
    /// teleport-prevention shape.
    #[test]
    fn resolve_respawn_position_uses_in_place_for_other_worlds_without_respawners() {
        let mgr = make_mgr_with_player("Agnos_test");
        let pos = resolve_respawn_position(-1, 1, &mgr);
        assert_eq!(
            pos,
            [42.0, 1.0, 17.0],
            "must respawn in place, not at Castle default"
        );
    }
}
