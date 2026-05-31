//! Per-frame tick handlers: AoI propagation, reload-completion promotion,
//! NPC movement along nav paths, and NPC respawn promotion.

use cimmeria_content_engine::chain::ChainEngine;
use tokio::sync::mpsc;

use super::super::messages::CellToBaseMsg;
use super::super::space_manager::SpaceManager;

/// Run one tick of AoI processing across all spaces.
mod auto_cycle;
mod cover;
pub(crate) mod holster;
mod npc_movement;
mod npc_respawn;

pub(super) use auto_cycle::auto_cycle_tick;
pub(super) use cover::cover_detection_tick;
pub(crate) use holster::HOLSTER_ANIMATION_DURATION;
pub(super) use holster::{holster_timer_tick, pending_slot_swap_tick};
pub(super) use npc_movement::npc_movement_tick;
pub(super) use npc_respawn::npc_respawn_tick;

pub(super) async fn run_aoi_tick(tx: &mpsc::Sender<CellToBaseMsg>, space_mgr: &mut SpaceManager) {
    let events = space_mgr.compute_aoi_changes();
    for event in events {
        if tx.send(event).await.is_err() {
            tracing::warn!("Failed to send AoI event to BaseApp (channel closed)");
            return;
        }
    }
}

/// Promote any reload whose warmup deadline has elapsed: refill the active
/// bandolier slot's magazine, clear `reload_complete_at`, send `onStatUpdate`
/// for the AmmoSlot{N} stat to the player, and queue a `BandolierAmmoUpdate`
/// to base for persistence.
///
/// Stage C: this is the sole refill path. The fire-path eager-promotion has
/// been removed; `handle_use_ability` reads ammo through `entity.active_ammo()`
/// and the bandolier UI updates on every fire via the AmmoSlot{N} stat.
#[tracing::instrument(
    name = "combat.reload_completion_tick",
    level = "debug",
    skip_all,
    fields(ready_count = tracing::field::Empty),
)]
pub(super) async fn reload_completion_tick(
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    let now = std::time::Instant::now();

    // Snapshot ready-to-promote player IDs first to avoid holding a borrow on
    // `space_mgr` across the `send_entity_method` await below.
    let ready: Vec<u32> = space_mgr
        .all_player_entity_ids()
        .into_iter()
        .filter(|&eid| {
            space_mgr
                .get_entity(eid)
                .and_then(|e| e.reload_complete_at)
                .is_some_and(|t| now >= t)
        })
        .collect();
    tracing::Span::current().record("ready_count", ready.len());

    for entity_id in ready {
        // Phase 1: mutate entity state, capture stat-update payload + the
        // BandolierAmmoUpdate fields we need to send afterwards. Drop the
        // mutable borrow before any `.await`.
        let (stat_payload, persist) = {
            let entity = match space_mgr.get_entity_mut(entity_id) {
                Some(e) => e,
                None => continue,
            };

            // Refill the slot that *started* the reload, not whatever slot is
            // currently active. Without pinning, a mid-reload weapon swap
            // would mis-attribute the refill to the new weapon.
            let slot_id = match entity.reload_slot_id {
                Some(s) => s,
                None => {
                    // Defensive: shouldn't happen — reload_complete_at is only
                    // set together with reload_slot_id. Clear and move on.
                    entity.reload_complete_at = None;
                    tracing::warn!(
                        entity_id,
                        "reload tick: deadline set without slot_id, clearing"
                    );
                    continue;
                }
            };

            // Look up the clip size for the pinned slot. If the slot is
            // empty (item removed mid-reload), clear the deadline and skip
            // the wire send rather than refilling nothing.
            let clip_size = entity.bandolier_items.get(&slot_id).map(|i| i.clip_size);
            let new_ammo = match clip_size {
                Some(cs) => entity.set_slot_ammo(slot_id, cs),
                None => None,
            };
            entity.reload_complete_at = None;
            entity.reload_slot_id = None;

            if new_ammo.is_none() {
                tracing::debug!(
                    entity_id,
                    slot_id,
                    "reload tick: pinned slot empty, no refill"
                );
                continue;
            }

            // The slot was marked dirty by `set_slot_ammo`; persistence drains
            // it via the BandolierAmmoUpdate below.
            entity.bandolier_ammo_dirty.remove(&slot_id);

            let payload = entity.stats.serialize_dirty();
            entity.stats.clear_dirty();

            let (item_id, cur_ammo, cur_ammo_type) = entity
                .bandolier_items
                .get(&slot_id)
                .map_or((0, 0, 0), |i| (i.item_id, i.current_ammo, i.cur_ammo_type));
            let persist = entity
                .player_id
                .map(|pid| (pid, slot_id, item_id, cur_ammo, cur_ammo_type));

            (payload, persist)
        };

        // Phase 2: send onStatUpdate. Skip when no stats actually changed.
        // `serialize_dirty` always emits a 4-byte u32 count prefix, so an
        // `is_empty()` check would never fire and we'd send a zero-entry
        // payload on no-op refills. Gate on the encoded count instead.
        if stat_payload.len() > 4 {
            super::super::abilities::send_entity_method(
                entity_id,
                crate::mercury::method_idx::ON_STAT_UPDATE,
                stat_payload,
                tx,
                space_mgr,
            )
            .await;
        }

        // Phase 3: persistence. CellToBaseMsg::BandolierAmmoUpdate is consumed
        // by base's existing handler that writes `sgw_inventory.ammo`.
        if let Some((player_id, slot_id, expected_item_id, current_ammo, cur_ammo_type)) = persist {
            let _ = tx
                .send(CellToBaseMsg::BandolierAmmoUpdate {
                    player_id,
                    slot_id,
                    expected_item_id,
                    current_ammo,
                    cur_ammo_type,
                })
                .await;
        }

        // Phase 4: fire the `Ability_End` sequence to signal "weapon ready
        // again" to the client. Pairs with the `Ability_Begin` sent at
        // reload-start in `handle_reload`.
        //
        // Currently inert against the production seed: ability 596 has
        // `event_set_id = NULL`, so this branch short-circuits. The legacy
        // `AbilityManager.py:671-673` reference is correct for abilities
        // that follow the begin/end pattern, but reload specifically sources
        // its animation from the player's archetype-keyed item event set
        // (`Item_Reload`, event id 4002) and is a single-sequence shape —
        // there is no separate end. The archetype lookup will replace this
        // branch outright once it lands.
        const ABILITY_RELOAD_WEAPON: i32 = 596;
        let event_set_id = space_mgr
            .ability_defs
            .get(&ABILITY_RELOAD_WEAPON)
            .and_then(|d| d.event_set_id);
        if let Some(esid) = event_set_id {
            use super::super::spawner::EVENT_ABILITY_END;
            if let Some(&seq_id) = space_mgr.sequence_map.get(&(esid, EVENT_ABILITY_END)) {
                let mut seq_args = Vec::with_capacity(28);
                seq_args.extend_from_slice(&seq_id.to_le_bytes());
                seq_args.extend_from_slice(&(entity_id as i32).to_le_bytes());
                seq_args.extend_from_slice(&(entity_id as i32).to_le_bytes());
                seq_args.push(1);
                seq_args.extend_from_slice(&0.0f32.to_le_bytes());
                seq_args.extend_from_slice(&0u32.to_le_bytes());
                seq_args.push(0);
                seq_args.extend_from_slice(&0i32.to_le_bytes());
                super::super::abilities::send_entity_method(
                    entity_id,
                    super::super::client_methods::spawnable_entity::ON_SEQUENCE,
                    seq_args,
                    tx,
                    space_mgr,
                )
                .await;
            }
        }
    }
}

/// Promote queued attack-while-holstered: dispatch the deferred
/// ability after the draw animation has had time to play.
///
/// `handle_use_ability` detects "player is holstered + OOC + attempting
/// to fire," draws the weapon + fires `Item_Equip`, stashes the
/// ability/target on the entity, and returns false WITHOUT committing
/// cooldown or ammo. This tick re-invokes `handle_use_ability` once
/// `UNHOLSTER_DRAW_DURATION` has elapsed — Phase B runs the normal
/// fire path against an already-drawn weapon.
///
/// Cadence: every 100ms AoI tick. Cost is one filter pass; the inner
/// re-invocation only fires on transition.
pub(super) async fn pending_attack_tick(
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
    engine: &ChainEngine,
) {
    let now = std::time::Instant::now();
    // Snapshot the queue entries first — `handle_use_ability` takes
    // `&mut space_mgr` and we don't want to hold a `&` across the
    // re-invocation.
    let ready: Vec<(u32, i32, i32)> = space_mgr
        .all_player_entity_ids()
        .into_iter()
        .filter_map(|eid| {
            let e = space_mgr.get_entity(eid)?;
            let at = e.pending_attack_at?;
            if now < at {
                return None;
            }
            let ability = e.pending_attack_ability_id?;
            let target = e.pending_attack_target_id?;
            Some((eid, ability, target))
        })
        .collect();

    for (entity_id, ability_id, target_id) in ready {
        // Clear the queue BEFORE re-invoking so the early-return
        // guard in handle_use_ability (which rejects on
        // `pending_attack_at.is_some()`) lets Phase B through.
        if let Some(e) = space_mgr.get_entity_mut(entity_id) {
            e.pending_attack_at = None;
            e.pending_attack_ability_id = None;
            e.pending_attack_target_id = None;
        }
        tracing::info!(
            entity_id,
            ability_id,
            target_id,
            "pending_attack_tick: draw window elapsed, firing queued attack"
        );
        // Route Phase-B queued attacks through the kill-credit
        // wrapper. Without this, a player who fires a weapon while
        // holstered (the attack is deferred to Phase B until the draw
        // window elapses) doesn't credit a quest objective if that
        // queued shot makes the kill — same divergence the auto-cycle
        // tick had until both paths joined the canonical helper.
        let _ = super::super::abilities::handle_use_ability_with_kill_credit(
            entity_id, ability_id, target_id, engine, tx, space_mgr,
        )
        .await;
    }
}

/// Promote pending reload-while-holstered phase A → phase B.
///
/// `handle_reload` detects "player is holstered + OOC + no reload in
/// flight," dispatches an `Item_Equip` draw animation + appearance
/// refresh, and stamps `pending_reload_at = now + UNHOLSTER_DRAW_DURATION`.
/// This tick scans for elapsed stamps and re-invokes `handle_reload`,
/// which then finds the weapon already drawn and runs the normal reload
/// start (cooldown timer + `Item_Reload` sequence + deferred ammo refill
/// via [`reload_completion_tick`]).
///
/// Why two phases: firing the reload animation on a model that's still
/// in the middle of the draw motion produces "weapon teleports into
/// hand and the reload anim plays on empty space" — the symptom that
/// drove this fix. Giving the draw `UNHOLSTER_DRAW_DURATION` to play
/// out lets the hand reach the hold position before the reload
/// sequence triggers.
///
/// Cadence: every 100ms AoI tick. Cost is one filter pass; the inner
/// `handle_reload` re-invocation only fires on transition.
#[tracing::instrument(
    name = "combat.pending_reload_tick",
    level = "debug",
    skip_all,
    fields(ready_count = tracing::field::Empty),
)]
pub(super) async fn pending_reload_tick(
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    let now = std::time::Instant::now();

    let ready: Vec<u32> = space_mgr
        .all_player_entity_ids()
        .into_iter()
        .filter(|&eid| {
            space_mgr
                .get_entity(eid)
                .and_then(|e| e.pending_reload_at)
                .is_some_and(|t| now >= t)
        })
        .collect();
    tracing::Span::current().record("ready_count", ready.len());

    for entity_id in ready {
        tracing::info!(
            entity_id,
            "pending_reload_tick: draw window elapsed, starting deferred reload"
        );
        // `handle_reload` clears `pending_reload_at` at the top of its
        // Phase B branch, so this won't re-fire next tick.
        super::super::cell_methods::player::world::handle_reload(entity_id, tx, space_mgr).await;
    }
}

/// Out-of-combat health and focus regeneration.
///
/// For each connected, alive player whose `threatened_mobs` set is empty,
/// advance any pool whose `cur < max` toward `max` by its regen-stat
/// value (with a floor of 1 per pool so a freshly-rolled archetype —
/// every player class seeds `healthRegen` and `focusRegen` to 0 in
/// `SGWBeing.statsTemplate` — still recovers between fights). All
/// pool changes for a given player are bundled into a single
/// `onStatUpdate` via `serialize_dirty`.
///
/// **Pool coverage**:
/// - `HEALTH` — primary HP pool, damaged by `HealthDamage` effects.
/// - `FOCUS` — "mental HP", damaged by `FocusDamage` effects (every
///   human archetype rolls in with `focus = 1570`, see
///   `db/resources/Archetypes/Seed/archetypes.sql`).
/// - `ENERGY_POOL` — *not* regenerated here. The archetypes table has no
///   `energy` column and `SGWBeing.statsTemplate` defaults it to
///   `Stat(0, 0, 0)`; for player classes the pool is dead and the
///   `cur < max` gate would skip it anyway. Adding it would just be
///   noise in the wire payload.
///
/// Why `threatened_mobs.is_empty()` instead of `BSF_IN_COMBAT == 0`: the
/// flag has three setters (`use_ability.rs`, reload in
/// `cell_methods/player/world.rs`, `enter_player_combat`) but only one
/// clear path (death-driven `clear_dead_npc_from_all_player_threat`), so
/// the bit gets stuck after one-shot kills, reload in isolation, and
/// no-target self-casts. `threatened_mobs` is the actual source of
/// aggro truth — empty set means no NPC currently has the player on its
/// threat list.
///
/// Cadence is 1 Hz — the caller must drive this on every 10th 100ms AoI
/// tick. The per-call delta is therefore "points per second"; if cadence
/// ever changes, the floors and the regen values need to be scaled
/// together.
pub(super) async fn regen_tick(tx: &mpsc::Sender<CellToBaseMsg>, space_mgr: &mut SpaceManager) {
    use crate::cell::combat::state::BSF_DEAD;
    use cimmeria_entity::stats::{FOCUS, FOCUS_REGEN, HEALTH, HEALTH_REGEN};

    /// (pool_id, regen_stat_id) pairs the tick advances. Add new pools
    /// here; the inner loop handles `cur < max` skip and floor-of-1 the
    /// same way for every entry.
    const POOLS: &[(i32, i32)] = &[(HEALTH, HEALTH_REGEN), (FOCUS, FOCUS_REGEN)];

    // Snapshot eligible player IDs first — we mutate stats inside the loop
    // and `send_entity_method` awaits, so we cannot hold a borrow on
    // `space_mgr` across iterations.
    let eligible: Vec<u32> = space_mgr
        .all_player_entity_ids()
        .into_iter()
        .filter(|&eid| {
            space_mgr.get_entity(eid).is_some_and(|e| {
                e.threatened_mobs.is_empty()
                    && e.state_field & BSF_DEAD == 0
                    && POOLS.iter().any(|&(pool_id, _)| {
                        e.stats
                            .get(pool_id)
                            .is_some_and(|s| s.cur < s.max && s.max > 0)
                    })
            })
        })
        .collect();

    for entity_id in eligible {
        let stat_payload = {
            let entity = match space_mgr.get_entity_mut(entity_id) {
                Some(e) => e,
                None => continue,
            };

            for &(pool_id, regen_id) in POOLS {
                let regen = entity.stats.get(regen_id).map_or(0, |s| s.cur).max(1);
                if let Some(pool) = entity.stats.get_mut(pool_id) {
                    if pool.cur < pool.max && pool.max > 0 {
                        pool.change(regen);
                    }
                }
            }

            let payload = entity.stats.serialize_dirty();
            entity.stats.clear_dirty();
            payload
        };

        // `serialize_dirty` always emits a 4-byte u32 count prefix, so
        // `is_empty()` would never fire. Gate on the encoded count to
        // suppress empty payloads when the eligibility filter raced with
        // a concurrent state change and nothing actually got dirtied.
        if stat_payload.len() > 4 {
            super::super::abilities::send_entity_method(
                entity_id,
                crate::mercury::method_idx::ON_STAT_UPDATE,
                stat_payload,
                tx,
                space_mgr,
            )
            .await;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cell::space_manager::SpaceManager;
    use tokio::sync::mpsc;

    #[tokio::test]
    async fn aoi_tick_on_empty_space_manager_produces_no_messages() {
        let mut mgr = SpaceManager::new(1);
        let (tx, mut rx) = mpsc::channel(8);
        run_aoi_tick(&tx, &mut mgr).await;
        assert!(
            rx.try_recv().is_err(),
            "empty space manager must produce zero AoI events"
        );
    }

    #[tokio::test]
    async fn reload_completion_tick_skips_entity_with_empty_slot() {
        let mut mgr = SpaceManager::new(1);
        let xml = r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle_CellBlock" Instanced="true" MinX="-800" MaxX="800" MinY="-800" MaxY="800" /></Spaces>"#;
        mgr.parse_spaces_xml(xml).unwrap();
        mgr.create_startup_spaces(r#"<?xml version="1.0"?><Spaces></Spaces>"#)
            .unwrap();
        mgr.create_entity(1, "Castle_CellBlock", [0.0; 3], [0.0; 3])
            .unwrap();
        if let Some(e) = mgr.get_entity_mut(1) {
            e.is_player = true;
            e.player_id = Some(100);
            // Slot 0 was removed mid-reload: reload_slot_id points to missing item.
            e.reload_slot_id = Some(0);
            e.reload_complete_at =
                Some(std::time::Instant::now() - std::time::Duration::from_millis(1));
        }
        mgr.connect_entity(1);

        let (tx, mut rx) = mpsc::channel(8);
        reload_completion_tick(&tx, &mut mgr).await;

        // No messages should be sent because the slot is empty.
        assert!(
            rx.try_recv().is_err(),
            "empty slot must produce zero wire messages"
        );
        let entity = mgr.get_entity(1).unwrap();
        assert!(
            entity.reload_complete_at.is_none(),
            "deadline must be cleared even when slot is empty"
        );
    }

    #[tokio::test]
    async fn regen_tick_advances_health_when_ooc_and_damaged() {
        use cimmeria_entity::stats::HEALTH;

        let mut mgr = SpaceManager::new(1);
        let xml = r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle" Instanced="false" MinX="-800" MaxX="800" MinY="-800" MaxY="800" /></Spaces>"#;
        mgr.parse_spaces_xml(xml).unwrap();
        mgr.create_startup_spaces(
            r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle" /></Spaces>"#,
        )
        .unwrap();
        mgr.create_entity(1, "Castle", [0.0; 3], [0.0; 3]).unwrap();
        if let Some(e) = mgr.get_entity_mut(1) {
            e.is_player = true;
            e.player_id = Some(100);
            // Damage: set health to 50/100
            if let Some(hp) = e.stats.get_mut(HEALTH) {
                hp.update(0, 50, 100);
                hp.dirty = false; // clear the dirty bit from setup
            }
            // HEALTH_REGEN defaults to 0/0/0 — the floor-of-1 logic applies
        }
        mgr.connect_entity(1);

        let (tx, mut rx) = mpsc::channel(8);
        regen_tick(&tx, &mut mgr).await;

        let entity = mgr.get_entity(1).unwrap();
        let hp = entity.stats.get(HEALTH).unwrap();
        assert_eq!(
            hp.cur, 51,
            "OOC regen must advance HP by floor-of-1 (regen stat is 0)"
        );

        // Must have sent an onStatUpdate
        let mut got_stat_update = false;
        while let Ok(msg) = rx.try_recv() {
            if let CellToBaseMsg::EntityMethodCall { method_index, .. } = msg {
                if method_index == crate::mercury::method_idx::ON_STAT_UPDATE {
                    got_stat_update = true;
                }
            }
        }
        assert!(
            got_stat_update,
            "regen_tick must send onStatUpdate when HP changed"
        );
    }

    #[tokio::test]
    async fn regen_tick_skips_player_in_combat() {
        use cimmeria_entity::stats::HEALTH;

        let mut mgr = SpaceManager::new(1);
        let xml = r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle" Instanced="false" MinX="-800" MaxX="800" MinY="-800" MaxY="800" /></Spaces>"#;
        mgr.parse_spaces_xml(xml).unwrap();
        mgr.create_startup_spaces(
            r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle" /></Spaces>"#,
        )
        .unwrap();
        mgr.create_entity(1, "Castle", [0.0; 3], [0.0; 3]).unwrap();
        if let Some(e) = mgr.get_entity_mut(1) {
            e.is_player = true;
            e.player_id = Some(100);
            if let Some(hp) = e.stats.get_mut(HEALTH) {
                hp.update(0, 50, 100);
                hp.dirty = false;
            }
            // In combat: threatened_mobs is non-empty
            e.threatened_mobs.insert(200);
        }
        mgr.connect_entity(1);

        let (tx, mut rx) = mpsc::channel(8);
        regen_tick(&tx, &mut mgr).await;

        let entity = mgr.get_entity(1).unwrap();
        let hp = entity.stats.get(HEALTH).unwrap();
        assert_eq!(
            hp.cur, 50,
            "in-combat player must NOT regen (threatened_mobs non-empty)"
        );
        assert!(
            rx.try_recv().is_err(),
            "no onStatUpdate when nothing changed"
        );
    }

    #[tokio::test]
    async fn regen_tick_skips_player_at_full_health() {
        let mut mgr = SpaceManager::new(1);
        let xml = r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle" Instanced="false" MinX="-800" MaxX="800" MinY="-800" MaxY="800" /></Spaces>"#;
        mgr.parse_spaces_xml(xml).unwrap();
        mgr.create_startup_spaces(
            r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle" /></Spaces>"#,
        )
        .unwrap();
        mgr.create_entity(1, "Castle", [0.0; 3], [0.0; 3]).unwrap();
        if let Some(e) = mgr.get_entity_mut(1) {
            e.is_player = true;
            e.player_id = Some(100);
            // Health at max — default is 100/100
        }
        mgr.connect_entity(1);

        let (tx, mut rx) = mpsc::channel(8);
        regen_tick(&tx, &mut mgr).await;

        assert!(
            rx.try_recv().is_err(),
            "full health player must not trigger onStatUpdate"
        );
    }

    #[tokio::test]
    async fn pending_attack_tick_clears_queue_when_elapsed() {
        let mut mgr = SpaceManager::new(1);
        let xml = r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle" Instanced="false" MinX="-800" MaxX="800" MinY="-800" MaxY="800" /></Spaces>"#;
        mgr.parse_spaces_xml(xml).unwrap();
        mgr.create_startup_spaces(
            r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle" /></Spaces>"#,
        )
        .unwrap();
        mgr.create_entity(1, "Castle", [0.0; 3], [0.0; 3]).unwrap();
        if let Some(e) = mgr.get_entity_mut(1) {
            e.is_player = true;
            e.player_id = Some(100);
            e.pending_attack_at =
                Some(std::time::Instant::now() - std::time::Duration::from_millis(1));
            e.pending_attack_ability_id = Some(42);
            e.pending_attack_target_id = Some(200);
        }
        mgr.connect_entity(1);

        let (tx, _rx) = mpsc::channel(8);
        pending_attack_tick(&tx, &mut mgr, &ChainEngine::new()).await;

        let entity = mgr.get_entity(1).unwrap();
        assert!(
            entity.pending_attack_at.is_none(),
            "pending_attack_at must be cleared after tick fires"
        );
        assert!(
            entity.pending_attack_ability_id.is_none(),
            "pending_attack_ability_id must be cleared"
        );
        assert!(
            entity.pending_attack_target_id.is_none(),
            "pending_attack_target_id must be cleared"
        );
    }

    #[tokio::test]
    async fn pending_attack_tick_no_op_when_stamp_in_future() {
        let mut mgr = SpaceManager::new(1);
        let xml = r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle" Instanced="false" MinX="-800" MaxX="800" MinY="-800" MaxY="800" /></Spaces>"#;
        mgr.parse_spaces_xml(xml).unwrap();
        mgr.create_startup_spaces(
            r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle" /></Spaces>"#,
        )
        .unwrap();
        mgr.create_entity(1, "Castle", [0.0; 3], [0.0; 3]).unwrap();
        if let Some(e) = mgr.get_entity_mut(1) {
            e.is_player = true;
            e.player_id = Some(100);
            e.pending_attack_at =
                Some(std::time::Instant::now() + std::time::Duration::from_secs(60));
            e.pending_attack_ability_id = Some(42);
            e.pending_attack_target_id = Some(200);
        }
        mgr.connect_entity(1);

        let (tx, _rx) = mpsc::channel(8);
        pending_attack_tick(&tx, &mut mgr, &ChainEngine::new()).await;

        let entity = mgr.get_entity(1).unwrap();
        assert!(
            entity.pending_attack_at.is_some(),
            "future stamp must not be consumed"
        );
        assert_eq!(entity.pending_attack_ability_id, Some(42));
        assert_eq!(entity.pending_attack_target_id, Some(200));
    }

    /// **Regression guard: `pending_attack_tick` Phase B fires must
    /// credit quest kills.** A player who fires while holstered has
    /// the attack deferred to Phase B (after the draw window
    /// elapses). When Phase B kills a quest-tagged NPC, the
    /// EntityDeath content event must reach the chain engine.
    ///
    /// Parallel coverage to
    /// `auto_cycle_tick_credits_quest_kill_on_tagged_npc_death` (in
    /// `service/ticks/auto_cycle.rs`) and
    /// `set_auto_cycle_immediate_fire_credits_quest_kill_on_tagged_npc_death`
    /// (in `cell_methods/player/world/tests.rs`). All three player-
    /// driven re-fire paths route through
    /// `handle_use_ability_with_kill_credit`; all three tests fail
    /// when the caller is reverted to bare `handle_use_ability`.
    #[tokio::test]
    async fn pending_attack_tick_credits_quest_kill_on_tagged_npc_death() {
        use cimmeria_content_engine::actions::Action;
        use cimmeria_content_engine::chain::Chain;
        use cimmeria_content_engine::triggers::Trigger;
        use cimmeria_entity::abilities::{AbilityDef, EffectDef};
        use cimmeria_entity::stats::HEALTH;

        const QUEST_TAG: &str = "QuestTargetDrone";
        const COUNTER_NAME: &str = "drone_kills";

        let mut mgr = SpaceManager::new(1);
        let xml = r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle" Instanced="false" MinX="-800" MaxX="800" MinY="-800" MaxY="800" /></Spaces>"#;
        mgr.parse_spaces_xml(xml).unwrap();
        mgr.create_startup_spaces(
            r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle" /></Spaces>"#,
        )
        .unwrap();
        mgr.create_entity(1, "Castle", [0.0; 3], [0.0; 3]).unwrap();
        mgr.spawn_npc(50, "Castle", [3.0, 0.0, 0.0], [0.0; 3])
            .unwrap();
        // Phase-B queue pre-conditions: stamp is in the past so the
        // tick promotes it, and the queue carries the target+ability
        // the deferred fire should pick up.
        if let Some(p) = mgr.get_entity_mut(1) {
            p.is_player = true;
            p.player_id = Some(100);
            p.abilities.add_ability(7);
            p.weapon_holstered = false;
            p.pending_attack_at =
                Some(std::time::Instant::now() - std::time::Duration::from_millis(1));
            p.pending_attack_ability_id = Some(7);
            p.pending_attack_target_id = Some(50);
        }
        mgr.connect_entity(1);
        let _ = mgr.compute_aoi_changes();

        // Lethal effect + ability def so the deferred fire kills the
        // NPC outright. Same fixture shape as the auto_cycle test.
        let mut params = std::collections::HashMap::new();
        params.insert("HealthDamage".to_string(), "9999".to_string());
        mgr.effect_defs.insert(
            100,
            EffectDef {
                effect_id: 100,
                ability_id: 7,
                delay: 0,
                effect_sequence: 0,
                event_set_id: None,
                script_name: None,
                params,
                ..Default::default()
            },
        );
        mgr.ability_defs.insert(
            7,
            AbilityDef {
                ability_id: 7,
                name: "test".to_string(),
                cooldown: 0.5,
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
        if let Some(npc) = mgr.get_entity_mut(50) {
            npc.tag = Some(QUEST_TAG.to_string());
            if let Some(stat) = npc.stats.get_mut(HEALTH) {
                stat.update(0, 1, 100);
                stat.clear_dirty();
            }
        }

        let mut engine = ChainEngine::new();
        engine.register_chain(Chain {
            id: 999_997,
            name: "test: pending_attack drone kill counter".to_string(),
            enabled: true,
            trigger: Trigger::OnEntityDeath {
                entity_type: None,
                entity_tag: Some(QUEST_TAG.to_string()),
            },
            conditions: vec![],
            actions: vec![Action::IncrementCounter {
                counter_name: COUNTER_NAME.to_string(),
                amount: 1,
            }],
            priority: 0,
        });

        let (tx, _rx) = mpsc::channel(64);
        pending_attack_tick(&tx, &mut mgr, &engine).await;

        assert_eq!(
            mgr.get_entity(1).unwrap().counters.get(COUNTER_NAME),
            Some(&1),
            "pending_attack_tick's Phase-B fire must credit a tagged-NPC kill \
             via fire_entity_death — reverting the tick to bare \
             handle_use_ability leaves this counter at None",
        );
        assert!(
            crate::cell::combat::is_dead_state(mgr.get_entity(50).unwrap().state_field),
            "test fixture: target must be dead after the lethal deferred fire",
        );
    }

    /// Defensive path: `reload_complete_at` set but `reload_slot_id` is None
    /// (should be impossible, but guard against state drift). The tick must
    /// clear `reload_complete_at` and produce zero wire messages.
    #[tokio::test]
    async fn reload_completion_tick_clears_deadline_when_slot_id_missing() {
        let mut mgr = SpaceManager::new(1);
        let xml = r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle" Instanced="false" MinX="-800" MaxX="800" MinY="-800" MaxY="800" /></Spaces>"#;
        mgr.parse_spaces_xml(xml).unwrap();
        mgr.create_startup_spaces(
            r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle" /></Spaces>"#,
        )
        .unwrap();
        mgr.create_entity(1, "Castle", [0.0; 3], [0.0; 3]).unwrap();
        if let Some(e) = mgr.get_entity_mut(1) {
            e.is_player = true;
            e.player_id = Some(100);
            // Deadline set but slot_id is None — the defensive guard fires.
            e.reload_complete_at =
                Some(std::time::Instant::now() - std::time::Duration::from_millis(1));
            e.reload_slot_id = None;
        }
        mgr.connect_entity(1);

        let (tx, mut rx) = mpsc::channel(8);
        reload_completion_tick(&tx, &mut mgr).await;

        assert!(
            rx.try_recv().is_err(),
            "missing slot_id must produce zero wire messages"
        );
        let entity = mgr.get_entity(1).unwrap();
        assert!(
            entity.reload_complete_at.is_none(),
            "deadline must be cleared by the defensive guard"
        );
    }

    /// AoI tick propagates entity changes to base as individual messages.
    /// When the channel is full (receiver dropped), the tick exits early
    /// rather than spinning on closed sends.
    #[tokio::test]
    async fn aoi_tick_stops_sending_when_channel_closed() {
        let mut mgr = SpaceManager::new(1);
        let xml = r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle" Instanced="false" MinX="-800" MaxX="800" MinY="-800" MaxY="800" /></Spaces>"#;
        mgr.parse_spaces_xml(xml).unwrap();
        mgr.create_startup_spaces(
            r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle" /></Spaces>"#,
        )
        .unwrap();
        mgr.create_entity(1, "Castle", [0.0; 3], [0.0; 3]).unwrap();
        if let Some(e) = mgr.get_entity_mut(1) {
            e.is_player = true;
            e.player_id = Some(100);
        }
        mgr.connect_entity(1);
        // Force AoI changes by computing once then adding a new entity
        let _ = mgr.compute_aoi_changes();
        mgr.create_entity(2, "Castle", [1.0, 0.0, 0.0], [0.0; 3])
            .unwrap();
        mgr.connect_entity(2);

        // Create a channel and immediately drop the receiver
        let (tx, rx) = mpsc::channel(1);
        drop(rx);

        // Should not panic — just returns early on closed channel
        run_aoi_tick(&tx, &mut mgr).await;
    }
}
