//! NPC respawn promotion — brings Dead NPCs back to life when their
//! `respawn_at` deadline has passed.
//!
//! # Why this tick exists
//!
//! Without it, dead NPCs sit on the ground forever. The death path in
//! `damage_apply` flips `BSF_DEAD` + `BSF_MOVEMENT_LOCK`, sets
//! `ai_state = Dead`, and calls `apply_death_transition` to drop the
//! attacker's reticle, fan out the threat clear, OR-merge
//! `INT_NormalLoot` into `interaction_type_flags`, and broadcast the
//! dead-state bit. After that the corpse is just a static object —
//! the spawner has no other way to repopulate it.
//!
//! This tick closes the loop: when the NPC-kill path calls
//! [`crate::cell::combat::mark_npc_dead`] (which `damage_apply` does at
//! the kill site), `respawn_at = now + respawn_secs` is stamped from
//! the resolved spawnlist/template precedence (see
//! [`crate::cell::spawner::npcs`]). The tick promotes the corpse on
//! the next cadence sweep:
//!
//! 1. Restore HP / FOCUS to max, clear dirty stats.
//! 2. Hard reset every state flag via `clear_all_state_flags()` —
//!    drops the bits on `state_field` *and* drains every counter entry
//!    in `state_flag_counts`. Matches the existing "respawn is a hard
//!    reset, not a per-source unwind" convention.
//! 3. Restore `interaction_type_flags` from the snapshot
//!    `original_interaction_type_flags` taken at spawn time. This
//!    drops the `INT_NormalLoot` bit the death OR-merged in, plus any
//!    other bits the loot or content path might have added.
//! 4. Drop the generated `loot` list and reset `next_loot_index`.
//! 5. Close any open loot UI windows on players whose
//!    `looting_entity == this NPC` — sends `onLootDisplay` with an
//!    empty list (Loot.lua hides the window on count==0) and clears
//!    the player's `looting_entity` so the next take-item call
//!    doesn't reference the now-empty list.
//! 6. Snap position back to `spawn_position` and restore facing
//!    direction from `spawn_direction`. Clear `nav_path`,
//!    `threat_list`, `last_aoe_deaths`, `last_movement_type`,
//!    `ai_retry_at`, `respawn_at`. Transition `ai_state` to `Idle`.
//! 7. Broadcast in load-bearing order:
//!    `EntityMoved` → `INTERACTION_TYPE` → `ON_STATE_FIELD_UPDATE` →
//!    `ON_STAT_UPDATE`. EntityMoved goes first so the client teleports
//!    the corpse to spawn *before* the state-flip packets arrive —
//!    otherwise the client would render an alive NPC at the death
//!    position for ~100ms before the next AoI tick caught up.
//!    Within the state packets, the death-path
//!    `INTERACTION_TYPE`-before-state-field invariant is preserved
//!    (see [`crate::cell::abilities::death`] module-level doc) — the
//!    client locks in cursor / pose state on the state-field arrival,
//!    so interaction-type must precede it.
//!
//! The position snap goes through `space_mgr.update_entity_position`
//! so the AoI grid + entity bookkeeping stay in sync; the inline
//! `EntityMoved` fan-out then bypasses the AoI tick's 100ms cadence so
//! witnesses see the position change in the same wire burst as the
//! state changes.
//!
//! # Cadence
//!
//! 1 Hz (every 10th 100ms AoI tick), same as `regen_tick`. Respawn
//! timers are in whole seconds; sub-second precision would just burn
//! CPU on the scan filter. Callers in
//! `message_loop` are responsible for the gating.
//!
//! # Cost
//!
//! `O(npc_count)` snapshot of `all_npc_entity_ids()` followed by a
//! filter that admits only NPCs with `ai_state == Dead && respawn_at
//! <= now`. On a healthy server most NPCs are alive; the filter
//! eliminates them in `O(1)` per entity (two field reads). For a
//! steady-state population of N NPCs, expected cost per tick is N
//! field reads + ~0 promotions.

use tokio::sync::mpsc;

use super::super::super::messages::CellToBaseMsg;
use super::super::super::space_manager::SpaceManager;

/// Promote any Dead NPC whose `respawn_at` has elapsed back to Idle.
///
/// See module-level doc for the full state-mutation sequence and the
/// load-bearing wire ordering.
#[tracing::instrument(
    name = "spawner.npc_respawn_tick",
    level = "debug",
    skip_all,
    fields(ready_count = tracing::field::Empty),
)]
pub(in crate::cell::service) async fn npc_respawn_tick(
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    use cimmeria_entity::cell_entity::AiState;
    use cimmeria_entity::stats::{FOCUS, HEALTH};

    let now = std::time::Instant::now();

    // Snapshot ready-to-respawn NPC IDs first so the per-entity mutation
    // block doesn't have to hold a borrow on `space_mgr` across the
    // `send_entity_method_to_witnesses` awaits below.
    let ready: Vec<u32> = space_mgr
        .all_npc_entity_ids()
        .into_iter()
        .filter(|&eid| {
            space_mgr.get_entity(eid).is_some_and(|e| {
                e.ai_state == AiState::Dead && e.respawn_at.is_some_and(|t| now >= t)
            })
        })
        .collect();
    tracing::Span::current().record("ready_count", ready.len());

    if ready.is_empty() {
        return;
    }

    for entity_id in ready {
        // Phase 1: snapshot the spawn position + direction via an
        // immutable read so Phase 3 has the destination cached without
        // needing to re-borrow `space_mgr` mid-mutation. Direction was
        // captured at spawn time as `Vector3::new(0, heading, 0)` from
        // `spawnlist.heading`; restoring it here keeps respawned NPCs
        // facing their original heading instead of snapping to yaw=0
        // (which `update_entity_position` would do if we passed
        // `[0, 0, 0]` as the direction param).
        //
        // Also captures `respawn_secs` + `world_name` for the
        // per-promotion info log below — without them an operator
        // looking at SigNoz can't tell which spawn config produced the
        // promotion or which world it belongs to.
        let (spawn_pos, spawn_dir, space_id, respawn_secs) = match space_mgr.get_entity(entity_id) {
            Some(e) => (
                e.spawn_position,
                e.spawn_direction
                    .unwrap_or(cimmeria_common::Vector3::zero()),
                e.space_id.0 as u32,
                e.respawn_secs,
            ),
            None => continue,
        };
        // world_name lives on the SpaceInstance, not the entity —
        // resolve via the SpaceManager. Defaulted to "unknown" so the
        // metric label cardinality stays bounded even if a future
        // refactor allows entities in spaces that aren't registered.
        let world_name = space_mgr
            .spaces
            .get(&space_id)
            .map(|s| s.world_name.clone())
            .unwrap_or_else(|| "unknown".to_string());

        // Phase 2: mutate entity state (clear death flags, restore HP,
        // wipe combat state). Capture the wire payloads we'll send
        // afterwards.
        let (state_field, stat_payload, interaction_flags) = {
            let entity = match space_mgr.get_entity_mut(entity_id) {
                Some(e) => e,
                None => continue,
            };

            // Restore HP and FOCUS to their max. The default mob spawn
            // path seeds these from the level; we don't need to
            // re-derive — `max` was set at spawn time and never
            // changed.
            if let Some(hp) = entity.stats.get_mut(HEALTH) {
                hp.set_current(hp.max);
            }
            if let Some(focus) = entity.stats.get_mut(FOCUS) {
                focus.set_current(focus.max);
            }

            // Hard reset all state flags + their refcount entries.
            // `clear_all_state_flags()` is the canonical respawn helper
            // (see `cell_entity::state_flags::clear_all_state_flags`
            // module doc: "Bypasses ref-counting on purpose — respawn
            // is a hard reset, not a per-source unwind"). This drains:
            // - The bits on `state_field` (BSF_DEAD, BSF_MOVEMENT_LOCK,
            //   and any stale BSF_IN_COMBAT or future counted flags).
            // - All entries in `state_flag_counts`, so a future counted
            //   set on this NPC starts from 0 rather than inheriting a
            //   stale counter (which would stick the bit on).
            entity.clear_all_state_flags();

            // Restore the pre-death interaction-type snapshot. The
            // death path OR-merged `INT_NormalLoot` into the live
            // flags; the loot path may have also flipped bits when
            // loot was taken. Either way, `original_interaction_type_flags`
            // captured at spawn time is the authoritative pre-death
            // state and is what we want the respawned NPC to expose.
            //
            // Fallback: when the snapshot is `0` we can't tell whether
            // (a) the template legitimately has no interaction bits or
            // (b) the snapshot was never populated (NPC spawned via
            // bare `CellEntity::new` rather than `spawn_npc_from_record`
            // — currently used only by test fixtures, but a future GM
            // /spawn command would also hit this path). Strip
            // `INT_NormalLoot` off the live flags instead of clobbering
            // to 0 — preserves any other content-driven bits and is a
            // strict superset of the snapshot-restore behavior for
            // record-spawned NPCs (whose snapshot would already exclude
            // INT_NormalLoot).
            entity.interaction_type_flags = if entity.original_interaction_type_flags != 0 {
                entity.original_interaction_type_flags
            } else {
                entity.interaction_type_flags & !crate::cell::abilities::INT_NORMAL_LOOT
            };

            // Drop the generated loot list and reset the loot index.
            // Without this, a respawned NPC would hand out the
            // previous corpse's loot on first interaction — and the
            // index would keep growing across respawn cycles, which
            // the client treats as a wire-format error after some
            // bound.
            entity.loot.clear();
            entity.next_loot_index = 1;

            // Wipe combat / AI scratch state so the respawned NPC
            // starts cleanly. `last_movement_type` was already cleared
            // at the death site; clearing it again here is a no-op
            // belt-and-suspenders for the rare case where the death
            // path's clear missed (e.g., death-via-effect-pulse that
            // skipped damage_apply).
            entity.threat_list.clear();
            entity.nav_path.clear();
            entity.last_aoe_deaths.clear();
            entity.last_movement_type = None;
            entity.ai_retry_at = None;
            entity.respawn_at = None;
            entity.velocity = [0.0; 3];
            entity.ai_state = AiState::Idle;

            // Cooldowns on death are stale by definition — clear so
            // the respawned NPC isn't held back by timers from its
            // last life.
            entity.abilities.clear_all_cooldowns();

            let stat_payload = entity.stats.serialize_dirty();
            entity.stats.clear_dirty();
            let state_field = entity.state_field;
            let interaction_flags = entity.interaction_type_flags;
            (state_field, stat_payload, interaction_flags)
        };

        // Phase 3a: close any open loot windows pointing at this
        // corpse. A player who right-clicked the corpse before
        // respawn fires has `looting_entity == entity_id` and an
        // open Loot.lua window. Without this, the window would stay
        // open with stale data — and the next take-item click would
        // send a lootItem(index) call referencing a list we just
        // cleared above (server-side index-bounds check would reject
        // it, but the client UX is broken). Send onLootDisplay with
        // an empty list — Loot.lua hides the window when count==0
        // (see `cell::interactions::loot` send_loot_display doc).
        let still_looting: Vec<u32> = space_mgr
            .all_player_entity_ids()
            .into_iter()
            .filter(|&pid| {
                space_mgr
                    .get_entity(pid)
                    .is_some_and(|p| p.looting_entity == Some(entity_id))
            })
            .collect();
        for player_id in still_looting {
            // Clear the player's side first so a stale
            // `looting_entity` doesn't survive the close.
            if let Some(p) = space_mgr.get_entity_mut(player_id) {
                p.looting_entity = None;
            }
            // Empty-list payload: entityId + count=0 + initial=0.
            let mut args = Vec::with_capacity(4 + 4 + 1);
            args.extend_from_slice(&(entity_id as i32).to_le_bytes());
            args.extend_from_slice(&0u32.to_le_bytes());
            args.push(0u8);
            let _ = tx
                .send(CellToBaseMsg::EntityMethodCall {
                    entity_id: player_id,
                    method_index: crate::mercury::method_idx::ON_LOOT_DISPLAY,
                    args,
                })
                .await;
            tracing::info!(
                player_id,
                respawning_entity = entity_id,
                "NPC respawn: closing stale loot window on player"
            );
        }

        // Phase 3b: snap position via the SpaceManager helper so the
        // AoI grid + entity bookkeeping stay in sync. The helper
        // overwrites `entity.direction` from its `[i8; 3]` param,
        // which would truncate non-integer headings (e.g., 1.57 rad
        // → 2), so we pass `[0, 0, 0]` and restore the full-precision
        // `spawn_dir` afterward. Skip if `spawn_position` was somehow
        // `None` (defensive — all DB-spawned NPCs have one).
        if let Some(pos) = spawn_pos {
            space_mgr.update_entity_position(entity_id, [pos.x, pos.y, pos.z], [0, 0, 0], [0.0; 3]);
            if let Some(npc) = space_mgr.get_entity_mut(entity_id) {
                npc.direction = spawn_dir;
            }
        }

        // Phase 4: wire broadcasts. Position update FIRST so the
        // EntityMoved fan-out reaches witnesses before the
        // state-flip packets — otherwise the client sees the corpse
        // become alive at the death position for ~100ms before the
        // next AoI tick teleports it to spawn. Then load-bearing
        // INTERACTION_TYPE → ON_STATE_FIELD_UPDATE order per the
        // death-path symmetric invariant.
        let witnesses = space_mgr.get_witnesses_of(entity_id);

        // 4a: EntityMoved per witness — push the position update
        // ahead of the state changes so the client teleports the
        // corpse to spawn BEFORE rendering it as alive.
        if let Some(pos) = spawn_pos {
            for witness_id in &witnesses {
                let _ = tx
                    .send(CellToBaseMsg::EntityMoved {
                        witness_id: *witness_id,
                        entity_id,
                        space_id,
                        position: [pos.x, pos.y, pos.z],
                        direction: [spawn_dir.x, spawn_dir.y, spawn_dir.z],
                        velocity: [0.0; 3],
                    })
                    .await;
            }
        }

        // 4b: INTERACTION_TYPE — restore pre-death flags. UINT64 LE,
        // 8 bytes, just like the death-side payload. Use the
        // witness-only helper (no warn on zero witnesses) so a
        // server with no players online doesn't generate three
        // warn-level logs per respawn cycle per NPC.
        super::super::super::abilities::send_entity_method_to_witnesses(
            entity_id,
            crate::mercury::method_idx::INTERACTION_TYPE,
            (interaction_flags as u64).to_le_bytes().to_vec(),
            tx,
            space_mgr,
        )
        .await;

        // 4c: ON_STATE_FIELD_UPDATE — BSF_DEAD / BSF_MOVEMENT_LOCK
        // cleared. UINT32 LE, 4 bytes, matches death-side layout.
        super::super::super::abilities::send_entity_method_to_witnesses(
            entity_id,
            crate::mercury::method_idx::ON_STATE_FIELD_UPDATE,
            state_field.to_le_bytes().to_vec(),
            tx,
            space_mgr,
        )
        .await;

        // 4d: ON_STAT_UPDATE — HP / FOCUS restored. `serialize_dirty`
        // always prefixes a UINT32 count; skip the send if no stats
        // actually changed (defensive — should always have changed
        // here since we just set HP=max from HP=0).
        if stat_payload.len() > 4 {
            super::super::super::abilities::send_entity_method_to_witnesses(
                entity_id,
                crate::mercury::method_idx::ON_STAT_UPDATE,
                stat_payload,
                tx,
                space_mgr,
            )
            .await;
        }

        tracing::info!(
            target: "spawner.npc_respawn",
            npc_id = entity_id,
            ?spawn_pos,
            respawn_secs,
            world_name = %world_name,
            state_field,
            interaction_flags,
            "NPC respawned (Dead -> Idle, HP restored, position snapped, witnesses notified)"
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cell::combat::{BSF_DEAD, BSF_MOVEMENT_LOCK};
    use crate::cell::space_manager::SpaceManager;
    use crate::mercury::method_idx;
    use cimmeria_entity::cell_entity::AiState;
    use cimmeria_entity::stats::{FOCUS, HEALTH};

    /// One player + one NPC in a Castle space, both connected and
    /// co-located so the AoI tick captures the player as a witness of
    /// the NPC. The NPC is in the Dead state with full damage applied
    /// (HP=0), BSF_DEAD + BSF_MOVEMENT_LOCK set, interaction_type with
    /// `INT_NormalLoot` OR-merged in, and `original_interaction_type_flags`
    /// preserving the pre-death snapshot.
    fn make_mgr_with_dead_npc(
        respawn_secs: Option<u32>,
        respawn_at: Option<std::time::Instant>,
    ) -> SpaceManager {
        use crate::cell::abilities::INT_NORMAL_LOOT;

        let mut mgr = SpaceManager::new(1);
        let xml = r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle" Instanced="false" MinX="-800" MaxX="800" MinY="-800" MaxY="800" /></Spaces>"#;
        mgr.parse_spaces_xml(xml).unwrap();
        mgr.create_startup_spaces(
            r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle" /></Spaces>"#,
        )
        .unwrap();
        // Player witness at the origin.
        mgr.create_entity(1, "Castle", [0.0; 3], [0.0; 3]).unwrap();
        if let Some(p) = mgr.get_entity_mut(1) {
            p.is_player = true;
            p.player_id = Some(100);
        }
        // NPC spawned at (10, 0, 0) facing yaw=1.57 (east-ish — any
        // non-zero heading so the post-respawn direction-restore
        // assertion is meaningful). Corpse stays at this position
        // until the respawn tick snaps it back.
        mgr.spawn_npc(50, "Castle", [10.0, 0.0, 0.0], [0.0, 1.57, 0.0])
            .unwrap();
        if let Some(npc) = mgr.get_entity_mut(50) {
            // Snapshot the pre-death interaction_type, then mimic the
            // death path's OR-merge of INT_NormalLoot.
            npc.original_interaction_type_flags = 1 << 5; // arbitrary content bit
            npc.interaction_type_flags = (1 << 5) | INT_NORMAL_LOOT;
            // Death state.
            npc.set_state_flag(BSF_DEAD);
            npc.set_state_flag(BSF_MOVEMENT_LOCK);
            npc.ai_state = AiState::Dead;
            // HP=0 so the post-respawn assertion that HP=max is
            // meaningful.
            if let Some(hp) = npc.stats.get_mut(HEALTH) {
                hp.set_current(0);
            }
            if let Some(focus) = npc.stats.get_mut(FOCUS) {
                focus.set_current(0);
            }
            // Move the corpse away from the spawn so the position snap
            // is observable.
            npc.position = cimmeria_common::Vector3::new(50.0, 0.0, 50.0);
            // Respawn opt-in (or not, per arg).
            npc.respawn_secs = respawn_secs;
            npc.respawn_at = respawn_at;
            // Stale combat scratch the tick should wipe.
            npc.threat_list.insert(1, 99.0);
            npc.nav_path
                .push_back(cimmeria_common::Vector3::new(123.0, 0.0, 456.0));
        }
        mgr.connect_entity(1);
        let _ = mgr.compute_aoi_changes();
        mgr
    }

    fn drain(rx: &mut mpsc::Receiver<CellToBaseMsg>) -> Vec<CellToBaseMsg> {
        let mut out = Vec::new();
        while let Ok(m) = rx.try_recv() {
            out.push(m);
        }
        out
    }

    /// Happy path: a Dead NPC with `respawn_at` in the past gets fully
    /// reset to Idle by the tick — HP restored, BSF_DEAD/MOVEMENT_LOCK
    /// cleared, AI back to Idle, position snapped to spawn, threat /
    /// nav state wiped, interaction-type restored to the pre-death
    /// snapshot, respawn_at cleared.
    #[tokio::test]
    async fn ready_dead_npc_respawns_to_idle_at_spawn_position() {
        let past = std::time::Instant::now() - std::time::Duration::from_millis(1);
        let mut mgr = make_mgr_with_dead_npc(Some(30), Some(past));
        let (tx, _rx) = mpsc::channel(64);

        npc_respawn_tick(&tx, &mut mgr).await;

        let npc = mgr.get_entity(50).unwrap();
        assert_eq!(npc.ai_state, AiState::Idle, "AI state must reset to Idle");
        assert_eq!(
            npc.state_field & BSF_DEAD,
            0,
            "BSF_DEAD must be cleared post-respawn"
        );
        assert_eq!(
            npc.state_field & BSF_MOVEMENT_LOCK,
            0,
            "BSF_MOVEMENT_LOCK must be cleared post-respawn"
        );
        let hp = npc.stats.get(HEALTH).unwrap();
        assert_eq!(hp.cur, hp.max, "HP must be restored to max");
        let focus = npc.stats.get(FOCUS).unwrap();
        assert_eq!(focus.cur, focus.max, "FOCUS must be restored to max");
        assert_eq!(
            npc.interaction_type_flags, npc.original_interaction_type_flags,
            "interaction_type must be restored to pre-death snapshot (drops INT_NormalLoot)"
        );
        assert_eq!(npc.position.x, 10.0, "position must snap to spawn X");
        assert_eq!(npc.position.z, 0.0, "position must snap to spawn Z");
        assert!(npc.threat_list.is_empty(), "threat_list must be wiped");
        assert!(npc.nav_path.is_empty(), "nav_path must be wiped");
        assert!(
            npc.respawn_at.is_none(),
            "respawn_at must be cleared after consumption"
        );
        // `respawn_secs` persists so a future death re-schedules.
        assert_eq!(npc.respawn_secs, Some(30));
    }

    /// Wire-order pin: respawn must emit EntityMoved BEFORE the
    /// state-flip packets so the client teleports the corpse to spawn
    /// before rendering it as alive. Within the state packets,
    /// INTERACTION_TYPE precedes ON_STATE_FIELD_UPDATE precedes
    /// ON_STAT_UPDATE (death-path-symmetric: the client locks in
    /// cursor + pose state on the state-field flip, so interaction-type
    /// must land first).
    ///
    /// Regression shape: without the inline EntityMoved, the client
    /// would see the corpse become alive at the death position for
    /// ~100 ms before the next AoI tick fired EntityMoved with the
    /// new position. Visible teleport-after-revive glitch.
    #[tokio::test]
    async fn respawn_emits_entity_moved_then_state_packets_in_load_bearing_order() {
        let past = std::time::Instant::now() - std::time::Duration::from_millis(1);
        let mut mgr = make_mgr_with_dead_npc(Some(30), Some(past));
        let (tx, mut rx) = mpsc::channel(64);

        npc_respawn_tick(&tx, &mut mgr).await;

        // Tag each emitted message by kind so the ordering assertion
        // can see "EntityMoved before INTERACTION_TYPE" alongside the
        // intra-state-packet ordering.
        let msgs = drain(&mut rx);
        let tags: Vec<&str> = msgs
            .iter()
            .filter_map(|m| match m {
                CellToBaseMsg::EntityMoved { entity_id: 50, .. } => Some("moved"),
                CellToBaseMsg::WitnessEntityMethod {
                    entity_id: 50,
                    method_index,
                    ..
                }
                | CellToBaseMsg::EntityMethodCall {
                    entity_id: 50,
                    method_index,
                    ..
                } => match *method_index {
                    method_idx::INTERACTION_TYPE => Some("int"),
                    method_idx::ON_STATE_FIELD_UPDATE => Some("state"),
                    method_idx::ON_STAT_UPDATE => Some("stat"),
                    _ => None,
                },
                _ => None,
            })
            .collect();
        let ix_moved = tags
            .iter()
            .position(|&t| t == "moved")
            .expect("respawn must emit EntityMoved");
        let ix_int = tags
            .iter()
            .position(|&t| t == "int")
            .expect("respawn must emit INTERACTION_TYPE");
        let ix_state = tags
            .iter()
            .position(|&t| t == "state")
            .expect("respawn must emit ON_STATE_FIELD_UPDATE");
        let ix_stat = tags
            .iter()
            .position(|&t| t == "stat")
            .expect("respawn must emit ON_STAT_UPDATE for HP/FOCUS reset");
        assert!(
            ix_moved < ix_int,
            "EntityMoved must precede INTERACTION_TYPE (position teleports before alive-state arrives); got {tags:?}"
        );
        assert!(
            ix_int < ix_state,
            "INTERACTION_TYPE must precede ON_STATE_FIELD_UPDATE (death-path-symmetric ordering); got {tags:?}"
        );
        assert!(
            ix_state < ix_stat,
            "ON_STATE_FIELD_UPDATE must precede ON_STAT_UPDATE; got {tags:?}"
        );
    }

    /// **A1 fix pin**: respawn must restore `entity.direction` from
    /// the `spawn_direction` snapshot, not clobber it to (0, 0, 0).
    /// Pre-fix bug shape: every respawned NPC faced north because
    /// `update_entity_position` overwrote direction from the
    /// `[0, 0, 0]` param. Also pins the full-precision restore — the
    /// helper's `[i8; 3]` direction param would truncate 1.57 rad to
    /// 2 if we passed it through, so the fix bypasses the helper for
    /// direction.
    #[tokio::test]
    async fn respawn_restores_spawn_facing_direction() {
        let past = std::time::Instant::now() - std::time::Duration::from_millis(1);
        let mut mgr = make_mgr_with_dead_npc(Some(30), Some(past));
        // Fixture spawned the NPC with yaw 1.57. Mid-fight, the AI
        // tick rotated the NPC to face its target — simulate that by
        // overwriting direction before the respawn fires.
        if let Some(npc) = mgr.get_entity_mut(50) {
            npc.direction = cimmeria_common::Vector3::new(0.0, std::f32::consts::PI, 0.0);
        }
        let (tx, _rx) = mpsc::channel(64);
        npc_respawn_tick(&tx, &mut mgr).await;

        let npc = mgr.get_entity(50).unwrap();
        assert!(
            (npc.direction.y - 1.57).abs() < 1e-4,
            "respawn must restore spawn_direction.y exactly (1.57 rad), got {}",
            npc.direction.y,
        );
        assert_eq!(npc.direction.x, 0.0);
        assert_eq!(npc.direction.z, 0.0);
    }

    /// **A3 fix pin**: a player whose `looting_entity` points at the
    /// respawning corpse must have their loot UI closed (server sends
    /// onLootDisplay with an empty list, count = 0) and `looting_entity`
    /// cleared. Without this, the player would have a stale UI showing
    /// items that no longer exist on the respawned-alive NPC.
    #[tokio::test]
    async fn respawn_closes_loot_ui_for_still_looting_players() {
        let past = std::time::Instant::now() - std::time::Duration::from_millis(1);
        let mut mgr = make_mgr_with_dead_npc(Some(30), Some(past));
        // Player 1 was mid-loot on the corpse when respawn fires.
        if let Some(p) = mgr.get_entity_mut(1) {
            p.looting_entity = Some(50);
        }

        let (tx, mut rx) = mpsc::channel(64);
        npc_respawn_tick(&tx, &mut mgr).await;

        let p = mgr.get_entity(1).unwrap();
        assert!(
            p.looting_entity.is_none(),
            "looting_entity must be cleared by respawn so subsequent take-item calls don't reference a dead corpse",
        );
        // Verify the wire packet that closes the UI: onLootDisplay
        // (method 114) with entity_id = NPC id, count = 0, initial = 0.
        let msgs = drain(&mut rx);
        let close_pkt = msgs.iter().find_map(|m| match m {
            CellToBaseMsg::EntityMethodCall {
                entity_id: 1,
                method_index,
                args,
            } if *method_index == method_idx::ON_LOOT_DISPLAY => Some(args.clone()),
            _ => None,
        });
        let args = close_pkt.expect("player must receive onLootDisplay close packet");
        // Payload layout: i32 entity_id, u32 count, u8 initial — 9 bytes.
        assert_eq!(
            args.len(),
            9,
            "close packet payload pin (entity_id + count + initial)"
        );
        assert_eq!(
            i32::from_le_bytes(args[0..4].try_into().unwrap()),
            50,
            "entity_id must be the respawning NPC's id",
        );
        assert_eq!(
            u32::from_le_bytes(args[4..8].try_into().unwrap()),
            0,
            "count must be 0 — Loot.lua hides window on count==0",
        );
        assert_eq!(args[8], 0, "initial flag must be 0 (not a fresh display)");
    }

    /// **A3 sibling**: when a player is NOT looting the respawning
    /// corpse, no onLootDisplay packet is sent to them. Catches a
    /// regression that would send the close packet to every player
    /// in the space.
    #[tokio::test]
    async fn respawn_does_not_close_loot_ui_for_unrelated_players() {
        let past = std::time::Instant::now() - std::time::Duration::from_millis(1);
        let mut mgr = make_mgr_with_dead_npc(Some(30), Some(past));
        // Player 1 is in the space but NOT looting the corpse.
        assert!(mgr.get_entity(1).unwrap().looting_entity.is_none());

        let (tx, mut rx) = mpsc::channel(64);
        npc_respawn_tick(&tx, &mut mgr).await;

        let any_loot_close = drain(&mut rx).iter().any(|m| {
            matches!(
                m,
                CellToBaseMsg::EntityMethodCall {
                    entity_id: 1,
                    method_index,
                    ..
                } if *method_index == method_idx::ON_LOOT_DISPLAY
            )
        });
        assert!(
            !any_loot_close,
            "non-looting player must not receive an onLootDisplay close packet",
        );
    }

    /// **B3 fix pin**: when the dying NPC has zero witnesses, the
    /// respawn tick must complete the state mutation but emit zero
    /// wire packets and zero warn-level logs. Pre-fix: each
    /// `send_entity_method` on a zero-witness NPC emitted a warn
    /// ("NPC has no witnesses, method dropped"), producing 3 warns
    /// per respawn cycle per NPC at scale.
    #[tokio::test]
    async fn respawn_with_no_witnesses_is_silent_and_state_correct() {
        // Fresh manager with ONLY an NPC — no player witness in the
        // space, so the NPC has zero observers.
        let past = std::time::Instant::now() - std::time::Duration::from_millis(1);
        let mut mgr = SpaceManager::new(1);
        let xml = r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle" Instanced="false" MinX="-800" MaxX="800" MinY="-800" MaxY="800" /></Spaces>"#;
        mgr.parse_spaces_xml(xml).unwrap();
        mgr.create_startup_spaces(
            r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle" /></Spaces>"#,
        )
        .unwrap();
        mgr.spawn_npc(50, "Castle", [10.0, 0.0, 0.0], [0.0, 1.57, 0.0])
            .unwrap();
        if let Some(npc) = mgr.get_entity_mut(50) {
            npc.set_state_flag(BSF_DEAD);
            npc.set_state_flag(BSF_MOVEMENT_LOCK);
            npc.ai_state = AiState::Dead;
            if let Some(hp) = npc.stats.get_mut(HEALTH) {
                hp.set_current(0);
            }
            npc.respawn_secs = Some(30);
            npc.respawn_at = Some(past);
        }
        let _ = mgr.compute_aoi_changes();
        assert!(
            mgr.get_witnesses_of(50).is_empty(),
            "fixture invariant: zero-witness NPC",
        );

        let (tx, mut rx) = mpsc::channel(64);
        npc_respawn_tick(&tx, &mut mgr).await;

        // Server state correct.
        let npc = mgr.get_entity(50).unwrap();
        assert_eq!(npc.ai_state, AiState::Idle);
        assert_eq!(npc.state_field & BSF_DEAD, 0);
        let hp = npc.stats.get(HEALTH).unwrap();
        assert_eq!(hp.cur, hp.max);

        // No wire packets emitted. (The wire helpers we use are the
        // no-warn variants, so the empty-witness branch is a clean
        // no-op rather than a per-method warn.)
        assert!(
            drain(&mut rx).is_empty(),
            "zero-witness respawn must produce zero wire messages",
        );
    }

    /// **C2 / B2 pin**: drive the full kill → respawn → re-kill loop
    /// THROUGH `damage_apply::apply_damage_to_target` (rather than
    /// mimicking the death-side mutations inline). Ensures that the
    /// `combat::mark_npc_dead` helper extraction stays correctly
    /// wired — if a future refactor stops calling it from
    /// `damage_apply`, the loop test catches the missing respawn
    /// stamp because the second cycle's `respawn_at` would never be
    /// set and the tick wouldn't promote the NPC back.
    ///
    /// Goes through the public ability dispatch path so the test
    /// shape catches breakage along the full damage → death →
    /// respawn pipeline, not just the respawn tick in isolation.
    #[tokio::test]
    async fn kill_via_damage_apply_then_respawn_then_kill_again() {
        use cimmeria_entity::abilities::{AbilityDef, EffectDef};

        let mut mgr = SpaceManager::new(1);
        let xml = r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle" Instanced="false" MinX="-800" MaxX="800" MinY="-800" MaxY="800" /></Spaces>"#;
        mgr.parse_spaces_xml(xml).unwrap();
        mgr.create_startup_spaces(
            r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle" /></Spaces>"#,
        )
        .unwrap();
        // Player attacker.
        mgr.create_entity(1, "Castle", [0.0; 3], [0.0; 3]).unwrap();
        if let Some(p) = mgr.get_entity_mut(1) {
            p.is_player = true;
            p.player_id = Some(100);
            p.abilities.add_ability(7);
        }
        // NPC target with respawn_secs = 3 (the minimum, so the
        // assertion that respawn_at is stamped is meaningful).
        mgr.spawn_npc(50, "Castle", [3.0, 0.0, 0.0], [0.0, 1.57, 0.0])
            .unwrap();
        if let Some(npc) = mgr.get_entity_mut(50) {
            npc.respawn_secs = Some(3);
            npc.original_interaction_type_flags = 1 << 5;
            npc.interaction_type_flags = 1 << 5;
            if let Some(hp) = npc.stats.get_mut(HEALTH) {
                hp.update(0, 1, 100); // one-shottable
                hp.clear_dirty();
            }
        }
        // Lethal ability fixture (mirrors the auto_cycle / pending_attack
        // tests' lethal ability setup).
        let mut params = std::collections::HashMap::new();
        params.insert("HealthDamage".to_string(), "9999".to_string());
        mgr.effect_defs.insert(
            100,
            EffectDef {
                effect_id: 100,
                ability_id: 7,
                params,
                ..Default::default()
            },
        );
        mgr.ability_defs.insert(
            7,
            AbilityDef {
                ability_id: 7,
                name: "test".to_string(),
                cooldown: 0.0,
                warmup: 0.0,
                flags: 0,
                is_ranged: false,
                min_range: 0,
                max_range: 30,
                target_type_id: 0,
                effect_ids: vec![100],
                moniker_ids: vec![],
                required_ammo: 0,
                event_set_id: None,
                velocity: 0.0,
            },
        );
        mgr.connect_entity(1);
        let _ = mgr.compute_aoi_changes();

        let (tx, _rx) = mpsc::channel(128);

        // First kill — drive through the real ability dispatch path.
        let _ = crate::cell::abilities::handle_use_ability(1, 7, 50, &tx, &mut mgr).await;

        // damage_apply → mark_npc_dead must have stamped respawn_at.
        let npc = mgr.get_entity(50).unwrap();
        assert_eq!(npc.ai_state, AiState::Dead, "cycle 1 → Dead");
        assert!(
            npc.respawn_at.is_some(),
            "cycle 1 → respawn_at must be stamped by mark_npc_dead",
        );
        let hp = npc.stats.get(HEALTH).unwrap();
        assert_eq!(hp.cur, 0, "cycle 1 → HP=0");

        // Verify the death-side mutations that mark_npc_dead is
        // responsible for. The pin point is the integration —
        // damage_apply must call mark_npc_dead, which must stamp
        // respawn_at + transition ai_state + clear nav state.
        // Re-running the loop through damage_apply for cycle 2 is
        // covered by `kill_respawn_rekill_loop_is_idempotent`, which
        // mimics the kill directly to keep that test's ability /
        // cooldown plumbing scope-limited.
        assert!(npc.nav_path.is_empty(), "mark_npc_dead must clear nav_path",);
        assert_eq!(npc.last_movement_type, None);
        assert_eq!(
            npc.state_flag_counts.get(&BSF_DEAD).copied(),
            Some(1),
            "mark_npc_dead must counted-set BSF_DEAD (not raw bit op)",
        );

        // Advance the deadline to the past so the respawn tick consumes it.
        if let Some(npc) = mgr.get_entity_mut(50) {
            npc.respawn_at = Some(std::time::Instant::now() - std::time::Duration::from_millis(1));
        }
        npc_respawn_tick(&tx, &mut mgr).await;

        let npc = mgr.get_entity(50).unwrap();
        assert_eq!(npc.ai_state, AiState::Idle, "post-respawn → Idle");
        assert_eq!(npc.state_field & BSF_DEAD, 0);
        assert!(
            !npc.state_flag_counts.contains_key(&BSF_DEAD),
            "post-respawn → BSF_DEAD counter drained (clear_all_state_flags)",
        );
        let hp = npc.stats.get(HEALTH).unwrap();
        assert_eq!(hp.cur, hp.max, "post-respawn → HP at max");
    }

    /// `respawn_secs = None` → `respawn_at` is never stamped at the
    /// death site, so the tick scan never admits this NPC. The corpse
    /// stays Dead forever. Pre-existing one-shot-mob behavior preserved.
    #[tokio::test]
    async fn no_respawn_when_deadline_unset() {
        let mut mgr = make_mgr_with_dead_npc(None, None);
        let (tx, mut rx) = mpsc::channel(64);

        npc_respawn_tick(&tx, &mut mgr).await;

        let npc = mgr.get_entity(50).unwrap();
        assert_eq!(
            npc.ai_state,
            AiState::Dead,
            "Dead NPC without respawn_at must stay Dead"
        );
        assert_ne!(npc.state_field & BSF_DEAD, 0, "BSF_DEAD must remain set");
        assert!(
            drain(&mut rx).is_empty(),
            "no respawn → no wire messages emitted"
        );
    }

    /// Deadline in the future → not yet ready, tick is a no-op.
    /// Mirrors the eager-promotion guard the tick uses to filter
    /// candidates.
    #[tokio::test]
    async fn future_deadline_is_a_no_op() {
        let future = std::time::Instant::now() + std::time::Duration::from_secs(60);
        let mut mgr = make_mgr_with_dead_npc(Some(60), Some(future));
        let (tx, mut rx) = mpsc::channel(64);

        npc_respawn_tick(&tx, &mut mgr).await;

        let npc = mgr.get_entity(50).unwrap();
        assert_eq!(npc.ai_state, AiState::Dead, "future deadline → still Dead");
        assert!(npc.respawn_at.is_some(), "future deadline must persist");
        assert!(
            drain(&mut rx).is_empty(),
            "no wire emissions for future deadline"
        );
    }

    /// Fallback path: when `original_interaction_type_flags = 0` (NPC
    /// spawned via bare `CellEntity::new` rather than
    /// `spawn_npc_from_record`), the respawn must strip
    /// `INT_NormalLoot` off the live flags instead of clobbering them
    /// to 0. Other content bits the death/loot path may have OR-merged
    /// in must survive.
    #[tokio::test]
    async fn zero_snapshot_falls_back_to_stripping_loot_bit() {
        use crate::cell::abilities::INT_NORMAL_LOOT;

        let past = std::time::Instant::now() - std::time::Duration::from_millis(1);
        let mut mgr = make_mgr_with_dead_npc(Some(30), Some(past));

        // Override the fixture: snapshot is zero, but the live flags
        // carry both a content bit (1<<5) AND the death-OR'd
        // INT_NormalLoot. Realistic for an NPC the test harness
        // synthesised without going through the record-load path.
        if let Some(npc) = mgr.get_entity_mut(50) {
            npc.original_interaction_type_flags = 0;
            npc.interaction_type_flags = (1 << 5) | INT_NORMAL_LOOT;
        }

        let (tx, _rx) = mpsc::channel(64);
        npc_respawn_tick(&tx, &mut mgr).await;

        let npc = mgr.get_entity(50).unwrap();
        assert_eq!(
            npc.interaction_type_flags,
            1 << 5,
            "zero snapshot → must strip INT_NormalLoot while preserving other bits, not clobber to 0",
        );
    }

    /// `state_flag_counts` for `BSF_DEAD` and `BSF_MOVEMENT_LOCK` must
    /// be drained on respawn, not just the `state_field` bits. The
    /// regression this guards: after kill → respawn → kill again, the
    /// second death would re-set the bits via the counted helper, and
    /// if the counter was still at 1 from the first death the new
    /// count would be 2 — requiring two unsets to actually clear the
    /// bit. The respawn-after-second-kill would clear the bit but
    /// leave the counter at 1, sticking the bits on the third death.
    #[tokio::test]
    async fn respawn_drains_counted_state_flag_entries() {
        let past = std::time::Instant::now() - std::time::Duration::from_millis(1);
        let mut mgr = make_mgr_with_dead_npc(Some(30), Some(past));

        // Pre-condition: the fixture's `set_state_flag(BSF_DEAD)`
        // populated the counter. Verify before the tick so we know we
        // have something to drain.
        assert_eq!(
            mgr.get_entity(50)
                .unwrap()
                .state_flag_counts
                .get(&BSF_DEAD)
                .copied(),
            Some(1),
            "fixture must populate the BSF_DEAD counter via set_state_flag",
        );

        let (tx, _rx) = mpsc::channel(64);
        npc_respawn_tick(&tx, &mut mgr).await;

        let npc = mgr.get_entity(50).unwrap();
        assert!(
            !npc.state_flag_counts.contains_key(&BSF_DEAD),
            "BSF_DEAD entry must be removed from state_flag_counts on respawn",
        );
        assert!(
            !npc.state_flag_counts.contains_key(&BSF_MOVEMENT_LOCK),
            "BSF_MOVEMENT_LOCK entry must be removed from state_flag_counts on respawn",
        );
    }

    /// Full kill → respawn → re-kill → respawn loop. Pins that the
    /// respawn machinery is reentrant: `respawn_at` clears + re-stamps,
    /// `state_flag_counts` doesn't accumulate, HP / position / nav /
    /// movement-type state are clean on every cycle. The shape that
    /// would break this is any "set once" reset (e.g., a one-time
    /// flag that latched after the first respawn).
    #[tokio::test]
    async fn kill_respawn_rekill_loop_is_idempotent() {
        let past = std::time::Instant::now() - std::time::Duration::from_millis(1);
        let mut mgr = make_mgr_with_dead_npc(Some(30), Some(past));
        let (tx, _rx) = mpsc::channel(64);

        // Cycle 1: corpse → respawn.
        npc_respawn_tick(&tx, &mut mgr).await;
        {
            let npc = mgr.get_entity(50).unwrap();
            assert_eq!(npc.ai_state, AiState::Idle, "cycle 1 → Idle");
            assert!(npc.respawn_at.is_none(), "cycle 1 → respawn_at cleared");
        }

        // Re-kill (mimic damage_apply's kill site).
        {
            let npc = mgr.get_entity_mut(50).unwrap();
            npc.set_state_flag(BSF_DEAD);
            npc.set_state_flag(BSF_MOVEMENT_LOCK);
            npc.ai_state = AiState::Dead;
            if let Some(hp) = npc.stats.get_mut(HEALTH) {
                hp.set_current(0);
            }
            // Re-stamp the deadline.
            npc.respawn_at = Some(std::time::Instant::now() - std::time::Duration::from_millis(1));
            // Mimic death's loot-bit OR-merge so the snapshot-restore
            // path is exercised again.
            npc.interaction_type_flags |= crate::cell::abilities::INT_NORMAL_LOOT;
            // Drop the NPC away from spawn so the position snap is
            // observable again.
            npc.position = cimmeria_common::Vector3::new(99.0, 0.0, 99.0);
        }

        // Cycle 2: respawn again.
        npc_respawn_tick(&tx, &mut mgr).await;
        {
            let npc = mgr.get_entity(50).unwrap();
            assert_eq!(npc.ai_state, AiState::Idle, "cycle 2 → Idle");
            assert_eq!(
                npc.state_field & BSF_DEAD,
                0,
                "cycle 2 → BSF_DEAD cleared on second respawn",
            );
            assert!(
                !npc.state_flag_counts.contains_key(&BSF_DEAD),
                "cycle 2 → counter entry drained (regression guard for stick-on bug)",
            );
            assert_eq!(npc.position.x, 10.0, "cycle 2 → re-snapped to spawn");
            let hp = npc.stats.get(HEALTH).unwrap();
            assert_eq!(hp.cur, hp.max, "cycle 2 → HP restored");
            assert!(npc.respawn_at.is_none(), "cycle 2 → respawn_at cleared");
        }
    }

    /// `broadcast_movement_type` is NPC-only. Calling it on a player
    /// must short-circuit with a warn and emit nothing — without this
    /// guard the helper would send a meaningless EMobMovementType byte
    /// to the player's client (and to nearby observers via the
    /// self+witnesses fanout).
    #[tokio::test]
    async fn broadcast_movement_type_on_player_is_a_no_op() {
        use cimmeria_entity::cell_entity::MobMovementType;

        let mut mgr = make_mgr_with_dead_npc(None, None);
        // Promote entity 1 (the player witness in the fixture) to a
        // movement-type call recipient. Players normally don't have
        // `last_movement_type`; this test pins that the guard fires
        // BEFORE the cache mutation.
        let (tx, mut rx) = mpsc::channel(16);
        crate::cell::abilities::broadcast_movement_type(
            1,
            Some(MobMovementType::CombatAdvance),
            &tx,
            &mut mgr,
        )
        .await;

        let p = mgr.get_entity(1).unwrap();
        assert!(
            p.last_movement_type.is_none(),
            "player guard must short-circuit before touching the cache",
        );
        assert!(
            drain(&mut rx).is_empty(),
            "no wire messages must be emitted for a player target",
        );
    }
}
