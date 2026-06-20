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
        // `world_name` is bounded by the worlds.xml registry (~30
        // entries) — low-cardinality. Useful for "is the respawn
        // timer working as configured per world" / "are we leaking
        // dead NPCs in Castle but not in Agnos" queries.
        cimmeria_observability::counter!(
            "npc_respawns_total",
            "world_name" => world_name,
        );
    }
}

#[cfg(test)]
mod tests;
