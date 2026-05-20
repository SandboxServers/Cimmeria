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
/// Phase 2 delay between firing the `Item_Unequip` animation and
/// broadcasting the mesh-removal `BeingAppearance`. The constant
/// is *intentionally shorter than the visible animation length* —
/// it sets when the cell sends `RefreshAppearance(holstered=true)`,
/// not when the client renders the mesh removal.
///
/// Timing target: the mesh-removal `BeingAppearance` should *arrive
/// on the client* at the instant the holster animation visually
/// completes. Without an early send the client renders the
/// animation's final frame (pistol at thigh), then the blend tree
/// returns to the idle-armed pose (weapon snaps back to the hand
/// socket) for a few frames before the mesh-removal packet lands —
/// a visible flicker.
///
/// 850ms was the first attempt; user playtest confirmed the flicker
/// remained, which means the underlying `Item_Unequip` animation is
/// shorter than ~850ms. 600ms is the next data point — the visible
/// pistol-to-thigh motion seems to be in the ~500-700ms range based
/// on the "few frames" of flicker description.
///
/// Trade-off at lower values: the mesh removes earlier in the
/// animation. At 600ms with a ~700ms animation, the pistol vanishes
/// during the last ~100ms of the holster motion — barely
/// perceptible because the pistol is already settled at the thigh
/// socket at that point. Strictly less visible than the flicker.
///
/// Empirically tuned against the `KIS-abilities_human.KIS-handling`
/// Unequip branch — adjust upward if the flicker returns, downward
/// if the pistol vanishes too early.
pub(crate) const HOLSTER_ANIMATION_DURATION: std::time::Duration =
    std::time::Duration::from_millis(600);

pub(super) async fn holster_timer_tick(
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    let now = std::time::Instant::now();

    // ── Phase 1: OOC grace elapsed → fire holster animation ──────────
    //
    // Players whose `combat_exit_at` has aged past `OOC_HOLSTER_DELAY`
    // get the `Item_Unequip` animation fired and a Phase 2 stamp set
    // for `HOLSTER_ANIMATION_DURATION` later. The weapon mesh stays
    // attached during the animation; Phase 2 removes it.
    let phase1: Vec<u32> = space_mgr
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
    for entity_id in phase1 {
        // Transition stamps: clear combat_exit_at (Phase 1 fired) and
        // schedule Phase 2. Do this BEFORE the animation dispatch so
        // any racing tick doesn't re-fire Phase 1.
        if let Some(e) = space_mgr.get_entity_mut(entity_id) {
            e.combat_exit_at = None;
            e.holster_animation_complete_at = Some(now + HOLSTER_ANIMATION_DURATION);
        }
        tracing::info!(
            entity_id,
            "holster_timer_tick: phase 1 — playing Item_Unequip; appearance deferred"
        );
        // Fire `Item_Unequip` (event 4001) — the bandolier-take-off
        // animation. Same `KIS-abilities_human.KIS-handling` kismet
        // script as `Item_Equip` / `Item_Reload`, but its Unequip
        // branch has the hand-authored "put weapon away" motion.
        // Used in python's `onItemUnequipped` for bandolier removal;
        // we reuse it as the OOC re-holster animation. (UE3 Matinee
        // supports `bReversePlayback` for true reverse playback at
        // `ghidra://SGW.exe@0x01893ef0`, but that flag is set at
        // design time in the kismet editor and isn't reachable
        // through the `playSequence` runtime API.)
        super::super::cell_methods::player::world::fire_item_sequence(
            entity_id,
            super::super::spawner::EVENT_ITEM_UNEQUIP,
            tx,
            space_mgr,
        )
        .await;
    }

    // ── Phase 2: animation done → remove the mesh ────────────────────
    //
    // Players whose `holster_animation_complete_at` has elapsed get
    // `weapon_holstered = true` flipped + a `RefreshAppearance`
    // dispatched so the wire `ComponentList` finally drops the weapon
    // visual. The split exists so the animation has time to play with
    // the weapon mesh attached — without it, the mesh snaps away
    // while/before the animation runs and the visible result is
    // "weapon vanishes mid-motion."
    let phase2: Vec<u32> = space_mgr
        .all_player_entity_ids()
        .into_iter()
        .filter(|&eid| {
            space_mgr
                .get_entity(eid)
                .is_some_and(|e| e.holster_animation_complete_at.is_some_and(|t| now >= t))
        })
        .collect();
    for entity_id in phase2 {
        let should_rebroadcast = match space_mgr.get_entity_mut(entity_id) {
            Some(e) => {
                e.holster_animation_complete_at = None;
                e.sync_holster_to_combat(false)
            }
            None => false,
        };
        if !should_rebroadcast {
            continue;
        }
        tracing::info!(
            entity_id,
            "holster_timer_tick: phase 2 — animation done, removing weapon mesh"
        );
        super::super::abilities::request_appearance_refresh(entity_id, tx, space_mgr).await;
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
        let _ = super::super::abilities::handle_use_ability(
            entity_id, ability_id, target_id, tx, space_mgr,
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

    /// Phase 1 (OOC grace elapsed): fires `Item_Unequip` animation
    /// and schedules Phase 2. The weapon mesh STAYS attached —
    /// `weapon_holstered` does NOT flip yet and no `RefreshAppearance`
    /// is dispatched. The hand-authored animation plays with the mesh
    /// visible; Phase 2 removes the mesh after the animation finishes.
    ///
    /// Bug shape this catches: a refactor collapses Phase 1 and Phase
    /// 2 back into a single tick, the mesh snaps away while the
    /// animation is still playing, and the visible result regresses
    /// to "weapon vanishes mid-motion."
    #[tokio::test]
    async fn holster_timer_tick_phase1_plays_animation_without_removing_mesh() {
        let mut mgr = make_holster_test_mgr();
        if let Some(p) = mgr.get_entity_mut(1) {
            p.archetype_id = Some(1);
        }
        mgr.sequence_map
            .insert((804, crate::cell::spawner::EVENT_ITEM_UNEQUIP), 1873);
        let elapsed = crate::cell::combat::OOC_HOLSTER_DELAY + std::time::Duration::from_millis(1);
        if let Some(p) = mgr.get_entity_mut(1) {
            p.combat_exit_at = std::time::Instant::now().checked_sub(elapsed);
        }

        let (tx, mut rx) = mpsc::channel(8);
        holster_timer_tick(&tx, &mut mgr).await;

        let mut saw_unequip_sequence = false;
        let mut saw_refresh = false;
        while let Ok(msg) = rx.try_recv() {
            match msg {
                CellToBaseMsg::RefreshAppearance { .. } => saw_refresh = true,
                CellToBaseMsg::EntityMethodCall {
                    method_index, args, ..
                } if method_index == crate::cell::client_methods::spawnable_entity::ON_SEQUENCE => {
                    let seq_id = i32::from_le_bytes([args[0], args[1], args[2], args[3]]);
                    if seq_id == 1873 {
                        saw_unequip_sequence = true;
                    }
                }
                _ => {}
            }
        }
        assert!(
            saw_unequip_sequence,
            "Phase 1 must fire Item_Unequip sequence so the client plays the \
             hand-authored holster animation",
        );
        assert!(
            !saw_refresh,
            "Phase 1 must NOT dispatch RefreshAppearance — the weapon mesh \
             must stay attached during the animation. Removing it here is \
             the bug shape that drove the two-phase split.",
        );

        let player = mgr.get_entity(1).unwrap();
        assert!(
            !player.weapon_holstered,
            "Phase 1 must NOT flip weapon_holstered yet — the mesh stays \
             attached until Phase 2 fires after HOLSTER_ANIMATION_DURATION",
        );
        assert!(
            player.combat_exit_at.is_none(),
            "Phase 1 stamp (combat_exit_at) must clear so it doesn't re-fire",
        );
        assert!(
            player.holster_animation_complete_at.is_some(),
            "Phase 2 must be scheduled via holster_animation_complete_at",
        );
    }

    /// Phase 2 (animation duration elapsed): flips `weapon_holstered`
    /// and dispatches `RefreshAppearance(holstered=true)` so the wire
    /// `ComponentList` drops the weapon visual and the client removes
    /// the mesh.
    #[tokio::test]
    async fn holster_timer_tick_phase2_removes_mesh_when_animation_done() {
        let mut mgr = make_holster_test_mgr();
        let elapsed = HOLSTER_ANIMATION_DURATION + std::time::Duration::from_millis(1);
        if let Some(p) = mgr.get_entity_mut(1) {
            p.archetype_id = Some(1);
            p.combat_exit_at = None;
            p.holster_animation_complete_at = std::time::Instant::now().checked_sub(elapsed);
        }

        let (tx, mut rx) = mpsc::channel(8);
        holster_timer_tick(&tx, &mut mgr).await;

        let mut saw_refresh = false;
        while let Ok(msg) = rx.try_recv() {
            if let CellToBaseMsg::RefreshAppearance {
                holstered: true, ..
            } = msg
            {
                saw_refresh = true;
            }
        }
        assert!(
            saw_refresh,
            "Phase 2 must dispatch RefreshAppearance(holstered=true) so the \
             weapon mesh is removed from the wire ComponentList",
        );

        let player = mgr.get_entity(1).unwrap();
        assert!(
            player.weapon_holstered,
            "Phase 2 must flip weapon_holstered=true"
        );
        assert!(
            player.holster_animation_complete_at.is_none(),
            "Phase 2 stamp must clear so subsequent ticks don't re-fire",
        );
    }

    /// Phase 2 stamp scheduled in the future is a no-op (animation
    /// still playing). Pins the boundary.
    #[tokio::test]
    async fn holster_timer_tick_phase2_skips_while_animation_in_flight() {
        let mut mgr = make_holster_test_mgr();
        if let Some(p) = mgr.get_entity_mut(1) {
            p.archetype_id = Some(1);
            p.combat_exit_at = None;
            p.holster_animation_complete_at =
                Some(std::time::Instant::now() + std::time::Duration::from_secs(60));
        }

        let (tx, mut rx) = mpsc::channel(8);
        holster_timer_tick(&tx, &mut mgr).await;

        assert!(
            rx.try_recv().is_err(),
            "no messages should fire while Phase 2 stamp is in the future",
        );
        let player = mgr.get_entity(1).unwrap();
        assert!(
            !player.weapon_holstered,
            "weapon must stay drawn until the animation finishes",
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
