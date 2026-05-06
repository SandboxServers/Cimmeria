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

    // Push the refreshed health/focus to the client via onStatUpdate
    // (method 20) — without this, the post-respawn HUD would render the
    // pre-death stats.
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

    let _ = tx
        .send(CellToBaseMsg::EntityMethodCall {
            entity_id,
            method_index: crate::mercury::method_idx::ON_END_AID_WAIT,
            args: Vec::new(),
        })
        .await;

    // Fire the Entity_Spawn (5000) kismet sequence so the client ends ragdoll
    // physics on its own pawn (UE3 `APawn::TermRagdoll`) without a full map
    // reload. Mirrors the Entity_Death (5001) emit at
    // `damage_apply/mod.rs:302-322` — same event_set (1025 = Mob), same
    // 26-byte ON_SEQUENCE wire layout (sequence_id / source / target /
    // primary / impact_time / nvp_count / view_type / instance_id).
    //
    // Without this, a previous version triggered a heavy
    // `RESET_ENTITIES + onClientMapLoad` reload to clear the ragdoll, which
    // had a hole in the handshake: the BaseApp handler set
    // `pending_client_ready` but not `pending_map_loaded`, so the client's
    // `mapLoaded` reply was silently dropped and the camera stayed locked.
    // Eliminating the reload eliminates that whole handshake gap.
    {
        const EVENT_ENTITY_SPAWN: i32 = 5000;
        const PLAYER_EVENT_SET_ID: i32 = 1025; // Mob — also drives Entity_Death
        if let Some(&spawn_seq_id) = space_mgr
            .sequence_map
            .get(&(PLAYER_EVENT_SET_ID, EVENT_ENTITY_SPAWN))
        {
            let mut seq_args = Vec::with_capacity(26);
            seq_args.extend_from_slice(&spawn_seq_id.to_le_bytes()); // KismetEventSetSeqID
            seq_args.extend_from_slice(&(entity_id as i32).to_le_bytes()); // SourceID (respawning entity)
            seq_args.extend_from_slice(&(entity_id as i32).to_le_bytes()); // TargetID (also self)
            seq_args.push(1); // PrimaryTarget
            seq_args.extend_from_slice(&0.0f32.to_le_bytes()); // ImpactTime
            seq_args.extend_from_slice(&0u32.to_le_bytes()); // NameValuePairs count
            seq_args.push(0); // ViewType (KISMET_VIEW_Witness — matches Entity_Death)
            seq_args.extend_from_slice(&0i32.to_le_bytes()); // InstanceId
            crate::cell::abilities::send_entity_method(
                entity_id,
                crate::mercury::method_idx::ON_SEQUENCE,
                seq_args,
                tx,
                space_mgr,
            )
            .await;
        } else {
            tracing::error!(
                entity_id,
                event_set_id = PLAYER_EVENT_SET_ID,
                "Entity_Spawn sequence missing from sequence_map — \
                 client will not exit ragdoll. Seed at event_set 1025 / \
                 sequence_id 2753 should cover this; check the resources \
                 dump if respawn appears stuck."
            );
        }
    }

    // Send the cleared state_field to the respawning player's own client
    // so the HUD/movement-lock/dead-cursor state lifts. `send_entity_method`
    // routes player-targeted methods to the owner only — the matching
    // pattern at use_ability:404-410 has the same shape. Witnesses don't
    // get this state-flip directly; they observe the respawn through the
    // AoI broadcast of public stats (HEALTH back to max) and the
    // EntityMoved that follows update_entity_position. The cross-witness
    // broadcast for player death/respawn state-field flips is a known
    // preexisting gap (player Entity_Death + onStateFieldUpdate emit on
    // the same player-only routing) — tracked separately.
    crate::cell::abilities::send_entity_method(
        entity_id,
        crate::mercury::method_idx::ON_STATE_FIELD_UPDATE,
        0u32.to_le_bytes().to_vec(),
        tx,
        space_mgr,
    )
    .await;

    // Server-side spatial state: write the new position into the entity and
    // grid, so AoI ticks broadcast `EntityMoved` for witnesses.
    let spawn_pos: [f32; 3] = resolve_respawn_position(respawner_id, entity_id, space_mgr);
    space_mgr.update_entity_position(entity_id, spawn_pos, [0, 0, 0], [0.0; 3]);

    // Client-side: hand off to BaseApp, which sends `BASEMSG_FORCED_POSITION`
    // (0x31) to authoritatively snap the avatar plus `onPlayerTeleport` (116)
    // for streaming-load coordination. Same path the ring transporters use.
    let space_id = match space_mgr.get_entity(entity_id).map(|e| e.space_id.0 as u32) {
        Some(s) => s,
        None => {
            tracing::error!(
                entity_id,
                "respawn: entity_id missing from entity_space — cannot dispatch TeleportPlayer"
            );
            return;
        }
    };
    if let Err(e) = tx
        .send(CellToBaseMsg::TeleportPlayer {
            entity_id,
            space_id,
            position: spawn_pos,
        })
        .await
    {
        tracing::error!(
            entity_id, space_id, ?spawn_pos, error = %e,
            "TeleportPlayer (respawn) send to base failed -- player will not be snapped to spawn"
        );
        return;
    }
    tracing::info!(
        entity_id,
        ?spawn_pos,
        "Respawn dispatched (in-place; no map reload)"
    );
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
    /// Must restore HEALTH/FOCUS to max, clear all state flags + their
    /// refcounts, clear ability cooldowns, update entity position to
    /// the resolved spawn point (Castle default for Castle_CellBlock),
    /// and fire the in-place respawn burst:
    ///
    ///   1. `onStatUpdate` (refreshed health/focus)
    ///   2. `onEndAidWait` (closes the Defeat Window)
    ///   3. `onSequence` Entity_Spawn (ends client ragdoll)
    ///   4. `onStateFieldUpdate(0)` (lifts dead/movement-lock state)
    ///   5. `TeleportPlayer` (snaps the avatar to spawn)
    ///
    /// Crucially, **no** `RespawnReload`: the prior version triggered a
    /// full map reload to clear ragdoll, which had a handshake gap
    /// (`pending_map_loaded` never set on the BaseApp side) that left
    /// the camera locked. The Entity_Spawn kismet hits
    /// `APawn::TermRagdoll` in-place, no reload needed.
    #[tokio::test]
    async fn handle_respawn_emits_in_place_respawn_burst() {
        use crate::cell::combat::{BSF_DEAD, BSF_MOVEMENT_LOCK};
        let mut mgr = make_mgr_with_player("Castle_CellBlock");

        // Pre-seed the Entity_Spawn sequence the way the production seed
        // does (event_set 1025 / sequence_id 2753 → Entity_Spawn 5000).
        // Without this, the kismet lookup short-circuits and the test
        // can't observe the onSequence emit.
        const ENTITY_SPAWN: i32 = 5000;
        const PLAYER_EVENT_SET_ID: i32 = 1025;
        const SPAWN_SEQ_ID: i32 = 2753;
        mgr.sequence_map
            .insert((PLAYER_EVENT_SET_ID, ENTITY_SPAWN), SPAWN_SEQ_ID);

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

        // Build the byte-exact Entity_Spawn ON_SEQUENCE payload the
        // production code should emit. Reconstructing it here (rather
        // than slicing args field-by-field) means a regression that
        // perturbs ANY byte — view_type, instance_id, primary-target
        // flag, etc. — fails the test instead of silently shipping.
        let mut expected_spawn_payload = Vec::with_capacity(26);
        expected_spawn_payload.extend_from_slice(&SPAWN_SEQ_ID.to_le_bytes()); // KismetEventSetSeqID
        expected_spawn_payload.extend_from_slice(&1i32.to_le_bytes()); // SourceID = entity_id
        expected_spawn_payload.extend_from_slice(&1i32.to_le_bytes()); // TargetID = entity_id (self)
        expected_spawn_payload.push(1); // PrimaryTarget
        expected_spawn_payload.extend_from_slice(&0.0f32.to_le_bytes()); // ImpactTime
        expected_spawn_payload.extend_from_slice(&0u32.to_le_bytes()); // NameValuePairs count
        expected_spawn_payload.push(0); // ViewType (KISMET_VIEW_Witness)
        expected_spawn_payload.extend_from_slice(&0i32.to_le_bytes()); // InstanceId

        let mut saw_stat_update = false;
        let mut saw_end_aid_wait = false;
        let mut saw_entity_spawn = false;
        let mut saw_state_field_clear = false;
        let mut saw_teleport = false;
        while let Ok(m) = rx.try_recv() {
            match m {
                CellToBaseMsg::EntityMethodCall {
                    entity_id: 1,
                    method_index,
                    args,
                } => {
                    if method_index == crate::mercury::method_idx::ON_STAT_UPDATE {
                        // serialize_dirty always emits a 4-byte u32
                        // count prefix; non-empty body means at least
                        // one stat was actually written. The HUD
                        // refresh is the practical reason this exists.
                        assert!(
                            args.len() > 4,
                            "respawn must emit onStatUpdate with the \
                             refreshed HEALTH/FOCUS in the payload, \
                             not an empty count-prefix-only body"
                        );
                        saw_stat_update = true;
                    } else if method_index == crate::mercury::method_idx::ON_END_AID_WAIT {
                        saw_end_aid_wait = true;
                    } else if method_index == crate::mercury::method_idx::ON_SEQUENCE {
                        assert_eq!(
                            args, expected_spawn_payload,
                            "Entity_Spawn ON_SEQUENCE payload must stay \
                             byte-exact — every field is load-bearing \
                             for the client kismet handler"
                        );
                        saw_entity_spawn = true;
                    } else if method_index == crate::mercury::method_idx::ON_STATE_FIELD_UPDATE {
                        // 4 bytes — the cleared u32 state_field.
                        assert_eq!(args.len(), 4);
                        let new_state = u32::from_le_bytes([args[0], args[1], args[2], args[3]]);
                        if new_state == 0 {
                            saw_state_field_clear = true;
                        }
                    }
                }
                CellToBaseMsg::TeleportPlayer {
                    entity_id,
                    position,
                    ..
                } => {
                    assert_eq!(entity_id, 1);
                    assert_eq!(
                        position,
                        [-334.231, 73.472, -228.026],
                        "TeleportPlayer must carry the resolved spawn position"
                    );
                    saw_teleport = true;
                }
                _ => {}
            }
        }
        assert!(
            saw_stat_update,
            "respawn must emit onStatUpdate so the post-respawn HUD \
             reflects the restored HEALTH/FOCUS instead of the \
             pre-death values"
        );
        assert!(saw_end_aid_wait, "respawn must emit onEndAidWait");
        assert!(
            saw_entity_spawn,
            "respawn must emit onSequence Entity_Spawn so the client \
             ends ragdoll without a map reload — load-bearing: a \
             regression that drops this would force a full reload \
             back into the BaseApp respawn handshake (the path with \
             the pending_map_loaded gap)"
        );
        assert!(
            saw_state_field_clear,
            "respawn must broadcast onStateFieldUpdate(0) so dead/\
             movement-lock state lifts on every observer's client"
        );
        assert!(
            saw_teleport,
            "respawn must emit TeleportPlayer (NOT RespawnReload) — \
             this is the snap that drives BASEMSG_FORCED_POSITION on \
             the client; RespawnReload is gone"
        );
    }

    /// Negative-pin: when the Entity_Spawn sequence is missing from
    /// `sequence_map` (e.g., a content gap), respawn must NOT emit an
    /// onSequence with a zero/stale sequence_id — the kismet branch
    /// short-circuits and logs an error. The other respawn-burst
    /// messages still fire so the player at least gets stats /
    /// state-field / teleport (ragdoll just persists until the next
    /// map transition). Pin so a regression that emits a spurious
    /// onSequence on the missing-seed path doesn't ship.
    #[tokio::test]
    async fn handle_respawn_skips_onsequence_when_spawn_seq_missing() {
        let mut mgr = make_mgr_with_player("Castle_CellBlock");
        // Intentionally do NOT seed sequence_map.
        let (tx, mut rx) = mpsc::channel(16);
        handle_respawn(1, -1, &tx, &mut mgr).await;

        let mut saw_onsequence = false;
        let mut saw_teleport = false;
        while let Ok(m) = rx.try_recv() {
            match m {
                CellToBaseMsg::EntityMethodCall { method_index, .. }
                    if method_index == crate::mercury::method_idx::ON_SEQUENCE =>
                {
                    saw_onsequence = true;
                }
                CellToBaseMsg::TeleportPlayer { .. } => saw_teleport = true,
                _ => {}
            }
        }
        assert!(
            !saw_onsequence,
            "missing Entity_Spawn seed must NOT emit a spurious \
             onSequence — the lookup-miss branch logs an error and \
             skips, leaving ragdoll for the next world transition"
        );
        assert!(
            saw_teleport,
            "TeleportPlayer must still fire — the snap and stat \
             refresh are independent of the kismet sequence"
        );
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
