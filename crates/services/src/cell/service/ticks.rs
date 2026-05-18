//! Per-frame tick handlers: AoI propagation, reload-completion promotion,
//! and NPC movement along nav paths.

use tokio::sync::mpsc;

use super::super::messages::CellToBaseMsg;
use super::super::space_manager::SpaceManager;

/// Run one tick of AoI processing across all spaces.
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
        // TODO(#210): inert against the current seed.
        //   Same gap as `handle_reload`: ability 596 has `event_set_id = NULL`
        //   in the seed, so this branch short-circuits in production. The
        //   legacy `AbilityManager.py:671-673` reference is correct *for
        //   abilities that follow the begin/end pattern*, but reload
        //   specifically sources its animation from the player's archetype-
        //   keyed item event set (`Item_Reload`, event id 4002) and is a
        //   single-sequence shape — there is no separate end. #210 will
        //   replace this branch outright once the archetype lookup lands.
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

/// Deferred holster after combat ends.
///
/// `exit_player_combat` stamps `combat_exit_at = Some(now)` instead of
/// flipping `weapon_holstered` immediately so chaining mobs (kill A,
/// aggro B 50ms later) doesn't visibly flicker the model. This tick
/// fires the actual holster once
/// [`crate::cell::combat::OOC_HOLSTER_DELAY`] has elapsed.
///
/// Cancellation: `enter_player_combat` clears `combat_exit_at` whenever
/// it runs (re-aggro inside the grace window). Players the timer wakes
/// up to find back in combat are skipped — their `combat_exit_at` will
/// already be cleared.
///
/// Cadence: every 100ms AoI tick. The cost is a single `Instant::now()`
/// plus a filtered pass over `all_player_entity_ids()`; rebroadcasts
/// only fire on transitioning players, so this is essentially free in
/// steady state.
pub(super) async fn holster_timer_tick(
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    let now = std::time::Instant::now();

    // Snapshot ready entity IDs first — `request_appearance_refresh`
    // takes `&SpaceManager` and we don't want to hold a `&mut` across
    // the `.await`.
    let ready: Vec<u32> = space_mgr
        .all_player_entity_ids()
        .into_iter()
        .filter(|&eid| {
            space_mgr.get_entity(eid).is_some_and(|e| {
                e.combat_exit_at.is_some_and(|t| {
                    now.duration_since(t) >= crate::cell::combat::OOC_HOLSTER_DELAY
                })
            })
        })
        .collect();

    for entity_id in ready {
        // Holster + clear the timer. If `set_weapon_holstered` reports
        // no change (no weapon visual present, or someone else already
        // toggled), skip the rebroadcast — same idempotency rule as
        // the explicit holster button.
        let should_rebroadcast = match space_mgr.get_entity_mut(entity_id) {
            Some(e) => {
                e.combat_exit_at = None;
                e.sync_holster_to_combat(false)
            }
            None => false,
        };
        if !should_rebroadcast {
            continue;
        }

        tracing::debug!(
            entity_id,
            "holster_timer_tick: grace window elapsed, holstering + rebroadcasting"
        );
        super::super::abilities::request_appearance_refresh(entity_id, tx, space_mgr).await;
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

/// NPC movement along nav paths — runs every AoI tick (100ms) for smooth pathing.
///
/// For each NPC with a non-empty `nav_path`, move it toward the next waypoint
/// by `move_speed` units. When it reaches (or overshoots) a waypoint, consume
/// it and continue to the next. Position updates propagate to witnesses via
/// the AoI tick's `EntityMoved` messages.
pub(super) fn npc_movement_tick(space_mgr: &mut SpaceManager) {
    // Collect NPCs that have active paths
    let moving_npcs: Vec<u32> = space_mgr
        .all_npc_entity_ids()
        .iter()
        .filter(|&&eid| {
            space_mgr
                .get_entity(eid)
                .is_some_and(|e| !e.nav_path.is_empty())
        })
        .copied()
        .collect();

    for npc_id in moving_npcs {
        // Read the next waypoint, move_speed, and remaining path length
        let (next_wp, move_speed, cur_pos, path_len) = {
            let npc = match space_mgr.get_entity(npc_id) {
                Some(e) if !e.nav_path.is_empty() => e,
                _ => continue,
            };
            let next_wp = match npc.nav_path.front() {
                Some(wp) => *wp,
                None => continue,
            };
            (next_wp, npc.move_speed, npc.position, npc.nav_path.len())
        };

        let dx = next_wp.x - cur_pos.x;
        let dy = next_wp.y - cur_pos.y;
        let dz = next_wp.z - cur_pos.z;
        let dist = (dx * dx + dy * dy + dz * dz).sqrt();

        // Speed in world units per second (tick is 100ms = 0.1s)
        let speed_per_sec = move_speed * 10.0;

        if dist <= move_speed {
            // Reached (or overshot) the waypoint — snap to it and consume
            // Waypoint Y comes from Detour's findStraightPath (already on navmesh surface)
            let snap_y = next_wp.y;

            // Peek at the NEXT waypoint (index 1) to compute velocity toward it
            let next_next_wp = if path_len > 1 {
                space_mgr
                    .get_entity(npc_id)
                    .and_then(|e| e.nav_path.get(1).copied())
            } else {
                None
            };

            let (velocity, yaw) = if let Some(nn) = next_next_wp {
                // Still more waypoints — compute velocity toward the next one
                let ndx = nn.x - next_wp.x;
                let ndz = nn.z - next_wp.z;
                let ndy = nn.y - next_wp.y;
                let nd = (ndx * ndx + ndy * ndy + ndz * ndz).sqrt();
                if nd > 0.001 {
                    (
                        [
                            ndx / nd * speed_per_sec,
                            ndy / nd * speed_per_sec,
                            ndz / nd * speed_per_sec,
                        ],
                        ndx.atan2(ndz),
                    )
                } else {
                    ([0.0; 3], 0.0)
                }
            } else {
                // Last waypoint — stopping, keep current facing
                ([0.0; 3], dx.atan2(dz))
            };

            space_mgr.update_entity_position(
                npc_id,
                [next_wp.x, snap_y, next_wp.z],
                [0, 0, 0],
                velocity,
            );
            if let Some(npc) = space_mgr.get_entity_mut(npc_id) {
                npc.nav_path.pop_front();
                npc.direction = cimmeria_common::Vector3::new(0.0, yaw, 0.0);
            }
        } else {
            // Move toward waypoint by move_speed units
            let t = move_speed / dist;
            let new_x = cur_pos.x + dx * t;
            let new_z = cur_pos.z + dz * t;

            // Linearly interpolate Y between current position and waypoint.
            // Waypoints from Detour's findStraightPath are on the navmesh surface,
            // so linear interpolation between them stays close to the floor.
            let new_y = cur_pos.y + dy * t;

            // Face the direction of movement (yaw = atan2(dx, dz) in radians)
            // Direction is [pitch, yaw, roll] — only yaw matters for facing
            let yaw = dx.atan2(dz);

            // Velocity = direction * speed_per_sec
            let velocity = [
                dx / dist * speed_per_sec,
                dy / dist * speed_per_sec,
                dz / dist * speed_per_sec,
            ];

            if (npc_id % 10000) < 5 {
                // log a few NPCs
                tracing::debug!(
                    npc_id,
                    cur = format_args!("({:.1},{:.1},{:.1})", cur_pos.x, cur_pos.y, cur_pos.z),
                    new = format_args!("({:.1},{:.1},{:.1})", new_x, new_y, new_z),
                    wp = format_args!("({:.1},{:.1},{:.1})", next_wp.x, next_wp.y, next_wp.z),
                    "NPC movement step"
                );
            }

            space_mgr.update_entity_position(npc_id, [new_x, new_y, new_z], [0, 0, 0], velocity);
            // Set yaw directly as radians (pack_angle reads direction.y)
            if let Some(npc) = space_mgr.get_entity_mut(npc_id) {
                npc.direction = cimmeria_common::Vector3::new(0.0, yaw, 0.0);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cell::space_manager::SpaceManager;

    fn make_holster_test_mgr() -> SpaceManager {
        let mut mgr = SpaceManager::new(1);
        let xml = r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle" Instanced="false" MinX="-800" MaxX="800" MinY="-800" MaxY="800" /></Spaces>"#;
        mgr.parse_spaces_xml(xml).unwrap();
        mgr.create_startup_spaces(
            r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle" /></Spaces>"#,
        )
        .unwrap();
        mgr.create_entity(1, "Castle", [0.0; 3], [0.0; 3]).unwrap();
        let p = mgr.get_entity_mut(1).unwrap();
        p.is_player = true;
        p.player_id = Some(100);
        p.weapon_visual = Some("BS_Gun.Pistol".into());
        p.weapon_holstered = false;
        mgr.connect_entity(1);
        let _ = mgr.compute_aoi_changes();
        mgr
    }

    /// The holster timer is a NO-OP when the grace window hasn't
    /// elapsed yet — drawing weapons must not flicker holstered just
    /// because the next tick runs. Pin it with a stamp that's
    /// effectively "now."
    #[tokio::test]
    async fn holster_timer_tick_skips_players_inside_grace_window() {
        let mut mgr = make_holster_test_mgr();
        if let Some(p) = mgr.get_entity_mut(1) {
            p.combat_exit_at = Some(std::time::Instant::now());
        }

        let (tx, mut rx) = mpsc::channel(8);
        holster_timer_tick(&tx, &mut mgr).await;

        assert!(
            rx.try_recv().is_err(),
            "no RefreshAppearance should fire inside the grace window",
        );
        let player = mgr.get_entity(1).unwrap();
        assert!(
            !player.weapon_holstered,
            "weapon must stay drawn until the grace window elapses",
        );
        assert!(
            player.combat_exit_at.is_some(),
            "timer must remain stamped — only elapsed entries get consumed",
        );
    }

    /// Conversely, once the grace window has elapsed the tick fires
    /// the deferred holster + dispatches `RefreshAppearance`. Stamp a
    /// timestamp from before `OOC_HOLSTER_DELAY` so the elapsed check
    /// trips on first call.
    #[tokio::test]
    async fn holster_timer_tick_holsters_when_grace_elapsed() {
        let mut mgr = make_holster_test_mgr();
        let elapsed = crate::cell::combat::OOC_HOLSTER_DELAY + std::time::Duration::from_millis(1);
        if let Some(p) = mgr.get_entity_mut(1) {
            p.combat_exit_at = std::time::Instant::now().checked_sub(elapsed);
            assert!(
                p.combat_exit_at.is_some(),
                "test invariant: monotonic clock must support sub by OOC_HOLSTER_DELAY+1ms",
            );
        }

        let (tx, mut rx) = mpsc::channel(8);
        holster_timer_tick(&tx, &mut mgr).await;

        let msg = rx
            .try_recv()
            .expect("holster_timer_tick must dispatch RefreshAppearance once grace elapsed");
        match msg {
            CellToBaseMsg::RefreshAppearance {
                entity_id,
                player_id,
                holstered,
            } => {
                assert_eq!(entity_id, 1);
                assert_eq!(player_id, 100);
                assert!(
                    holstered,
                    "deferred holster must send holstered=true so the wire filters the weapon",
                );
            }
            other => panic!("expected RefreshAppearance, got {other:?}"),
        }

        let player = mgr.get_entity(1).unwrap();
        assert!(
            player.weapon_holstered,
            "weapon must flip to holstered after the tick fires",
        );
        assert!(
            player.combat_exit_at.is_none(),
            "timer must clear so subsequent ticks don't re-fire (idempotency)",
        );
    }

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

    #[test]
    fn npc_movement_tick_advances_along_nav_path() {
        let mut mgr = SpaceManager::new(1);
        let xml = r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle" Instanced="false" MinX="-800" MaxX="800" MinY="-800" MaxY="800" /></Spaces>"#;
        mgr.parse_spaces_xml(xml).unwrap();
        mgr.create_startup_spaces(
            r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle" /></Spaces>"#,
        )
        .unwrap();
        mgr.create_entity(200, "Castle", [0.0, 0.0, 0.0], [0.0; 3])
            .unwrap();
        if let Some(npc) = mgr.get_entity_mut(200) {
            npc.is_player = false;
            npc.class_id = 0x04;
            npc.move_speed = 5.0;
            npc.nav_path
                .push_back(cimmeria_common::Vector3::new(10.0, 0.0, 0.0));
        }

        npc_movement_tick(&mut mgr);

        let npc = mgr.get_entity(200).unwrap();
        assert_eq!(npc.position.x, 5.0);
        assert_eq!(npc.position.y, 0.0);
        assert_eq!(npc.position.z, 0.0);
        assert_eq!(npc.nav_path.len(), 1);
    }

    #[test]
    fn npc_movement_tick_does_not_panic_on_empty_path() {
        let mut mgr = SpaceManager::new(1);
        let xml = r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle" Instanced="false" MinX="-800" MaxX="800" MinY="-800" MaxY="800" /></Spaces>"#;
        mgr.parse_spaces_xml(xml).unwrap();
        mgr.create_startup_spaces(
            r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle" /></Spaces>"#,
        )
        .unwrap();
        mgr.create_entity(200, "Castle", [0.0, 0.0, 0.0], [0.0; 3])
            .unwrap();
        if let Some(npc) = mgr.get_entity_mut(200) {
            npc.is_player = false;
            npc.class_id = 0x04;
            npc.nav_path.clear();
        }
        // Must not panic.
        npc_movement_tick(&mut mgr);
        let npc = mgr.get_entity(200).unwrap();
        assert_eq!(npc.position.x, 0.0, "stationary NPC must not move");
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
}
