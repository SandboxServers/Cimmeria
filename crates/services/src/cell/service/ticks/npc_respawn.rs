//! NPC respawn promotion — brings Dead NPCs back to life when their
//! `respawn_at` deadline has passed.
//!
//! # Why this tick exists
//!
//! Pre-[#48], dead NPCs sat on the ground forever. The death path in
//! `damage_apply` flipped `BSF_DEAD` + `BSF_MOVEMENT_LOCK`, set
//! `ai_state = Dead`, and called `apply_death_transition` to drop the
//! attacker's reticle, fan out the threat clear, OR-merge
//! `INT_NormalLoot` into `interaction_type_flags`, and broadcast the
//! dead-state bit. After that the corpse was just a static object —
//! the spawner had no way to repopulate it.
//!
//! This tick closes the loop: when [`damage_apply`] stamps
//! `respawn_at = now + respawn_secs` (from the resolved
//! spawnlist/template precedence — see [`crate::cell::spawner::npcs`]),
//! the tick promotes the corpse on the next cadence sweep:
//!
//! 1. Restore HP / FOCUS to max, clear dirty stats.
//! 2. Clear `BSF_DEAD | BSF_MOVEMENT_LOCK`. Other state bits stay
//!    untouched — content-driven flags (mission state, etc.) on dead
//!    NPCs are rare but the death path doesn't wipe them and neither
//!    do we.
//! 3. Restore `interaction_type_flags` from the snapshot
//!    `original_interaction_type_flags` taken at spawn time. This
//!    drops the `INT_NormalLoot` bit the death OR-merged in, plus any
//!    other bits the loot or content path might have added.
//! 4. Drop the generated `loot` list and reset `next_loot_index`.
//! 5. Snap position back to `spawn_position` and clear `nav_path`,
//!    `threat_list`, `last_aoe_deaths`, `last_movement_type`,
//!    `ai_retry_at`, `respawn_at`. Transition `ai_state` to `Idle`.
//! 6. Broadcast in load-bearing order: `INTERACTION_TYPE` →
//!    `ON_STATE_FIELD_UPDATE` → `ON_STAT_UPDATE`. Order mirrors the
//!    death path's `INTERACTION_TYPE`-before-state-field invariant
//!    (see [`crate::cell::abilities::death`] module-level doc) — the
//!    client locks in cursor / pose state on the state-field arrival,
//!    so interaction-type must precede it.
//!
//! The position snap goes through `space_mgr.update_entity_position`
//! so the AoI grid sees the new position and the next AoI tick fans
//! out an entity-moved event to witnesses.
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
    use crate::cell::combat::{BSF_DEAD, BSF_MOVEMENT_LOCK};
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
        // Phase 1: collect spawn position before any mutation so the
        // position update at the end has the right target. Snap
        // position via `update_entity_position` (which rewires the AoI
        // grid cell membership) BEFORE mutating other state, so AoI
        // witness recompute on the next tick sees the post-respawn
        // location.
        let spawn_pos = match space_mgr.get_entity(entity_id) {
            Some(e) => e.spawn_position,
            None => continue,
        };

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

            // Clear death/movement-lock state flags via direct
            // bitmask ops. `unset_state_flag` is the refcounted setter
            // for content-driven flags; the death path used raw bit ops
            // (mirrors python `SGWMob.py:292`) so the inverse here does
            // the same — `BSF_DEAD` / `BSF_MOVEMENT_LOCK` aren't
            // refcounted, they're set-once on death, cleared-once on
            // respawn. Mixing in the refcounted helper would hit the
            // zero-counter no-op and leave bits stuck.
            entity.state_field &= !(BSF_DEAD | BSF_MOVEMENT_LOCK);

            // Restore the pre-death interaction-type snapshot. The
            // death path OR-merged `INT_NormalLoot` into the live
            // flags; the loot path may have also flipped bits when
            // loot was taken. Either way, `original_interaction_type_flags`
            // captured at spawn time is the authoritative pre-death
            // state and is what we want the respawned NPC to expose.
            entity.interaction_type_flags = entity.original_interaction_type_flags;

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
            // Don't touch `BSF_IN_COMBAT` — the death path already
            // cleared it (raw bit op, mirrors Python). Anything that
            // re-aggros this NPC post-respawn will re-set it via
            // `enter_player_combat` on the attacker side.

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

        // Phase 3: snap position via the SpaceManager helper so the
        // AoI grid + entity bookkeeping stay in sync. Pre-respawn
        // position was wherever the corpse fell; we want the AoI
        // event for "entity moved to spawn" to fan out to any
        // witnesses on the next AoI tick. Skip if `spawn_position`
        // was somehow `None` (shouldn't happen — all DB-spawned NPCs
        // have one — but defensive).
        if let Some(pos) = spawn_pos {
            space_mgr.update_entity_position(entity_id, [pos.x, pos.y, pos.z], [0, 0, 0], [0.0; 3]);
        }

        // Phase 4: wire broadcasts in load-bearing order. Mirror the
        // death path's INTERACTION_TYPE-before-state-field-update
        // invariant — the client locks in cursor / appearance state
        // on the dead-bit-cleared message, so interaction-type must
        // land first to avoid a one-frame "lootable corpse but
        // alive" flicker.

        // 4a: INTERACTION_TYPE — restore pre-death flags. UINT64 LE,
        // 8 bytes, just like the death-side payload.
        super::super::super::abilities::send_entity_method(
            entity_id,
            crate::mercury::method_idx::INTERACTION_TYPE,
            (interaction_flags as u64).to_le_bytes().to_vec(),
            tx,
            space_mgr,
        )
        .await;

        // 4b: ON_STATE_FIELD_UPDATE — BSF_DEAD / BSF_MOVEMENT_LOCK
        // cleared. UINT32 LE, 4 bytes, matches death-side layout.
        super::super::super::abilities::send_entity_method(
            entity_id,
            crate::mercury::method_idx::ON_STATE_FIELD_UPDATE,
            state_field.to_le_bytes().to_vec(),
            tx,
            space_mgr,
        )
        .await;

        // 4c: ON_STAT_UPDATE — HP / FOCUS restored. `serialize_dirty`
        // always prefixes a UINT32 count; skip the send if no stats
        // actually changed (defensive — should always have changed
        // here since we just set HP=max from HP=0).
        if stat_payload.len() > 4 {
            super::super::super::abilities::send_entity_method(
                entity_id,
                crate::mercury::method_idx::ON_STAT_UPDATE,
                stat_payload,
                tx,
                space_mgr,
            )
            .await;
        }

        tracing::info!(
            entity_id,
            ?spawn_pos,
            state_field,
            interaction_flags,
            "NPC respawn: Dead -> Idle (HP restored, position snapped, witnesses notified)"
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
        // NPC spawned at (10, 0, 0) — corpse will be at the same pos
        // until the respawn tick snaps it back.
        mgr.spawn_npc(50, "Castle", [10.0, 0.0, 0.0], [0.0; 3])
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

    /// Wire-order pin: respawn must emit INTERACTION_TYPE BEFORE
    /// ON_STATE_FIELD_UPDATE, and ON_STAT_UPDATE last. Mirrors the
    /// death path's load-bearing ordering (see
    /// `crate::cell::abilities::death` module doc) — the client locks
    /// in cursor + pose state on the state-field flip, so
    /// interaction-type must land first.
    #[tokio::test]
    async fn respawn_emits_wire_methods_in_load_bearing_order() {
        let past = std::time::Instant::now() - std::time::Duration::from_millis(1);
        let mut mgr = make_mgr_with_dead_npc(Some(30), Some(past));
        let (tx, mut rx) = mpsc::channel(64);

        npc_respawn_tick(&tx, &mut mgr).await;

        // Collect (entity_id, method_index) projections — both
        // EntityMethodCall and WitnessEntityMethod variants are
        // possible for an NPC (witness route).
        let msgs = drain(&mut rx);
        let pairs: Vec<(u32, u16)> = msgs
            .iter()
            .filter_map(|m| match m {
                CellToBaseMsg::EntityMethodCall {
                    entity_id,
                    method_index,
                    ..
                }
                | CellToBaseMsg::WitnessEntityMethod {
                    entity_id,
                    method_index,
                    ..
                } => Some((*entity_id, *method_index)),
                _ => None,
            })
            .collect();
        let ix_int = pairs
            .iter()
            .position(|p| *p == (50, method_idx::INTERACTION_TYPE))
            .expect("respawn must emit INTERACTION_TYPE");
        let ix_state = pairs
            .iter()
            .position(|p| *p == (50, method_idx::ON_STATE_FIELD_UPDATE))
            .expect("respawn must emit ON_STATE_FIELD_UPDATE");
        let ix_stat = pairs
            .iter()
            .position(|p| *p == (50, method_idx::ON_STAT_UPDATE))
            .expect("respawn must emit ON_STAT_UPDATE for HP/FOCUS reset");
        assert!(
            ix_int < ix_state,
            "INTERACTION_TYPE must precede ON_STATE_FIELD_UPDATE (death-path-symmetric ordering); got {pairs:?}"
        );
        assert!(
            ix_state < ix_stat,
            "ON_STATE_FIELD_UPDATE must precede ON_STAT_UPDATE; got {pairs:?}"
        );
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
}
