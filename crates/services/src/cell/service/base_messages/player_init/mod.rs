//! Handler for `BaseToCellMsg::InitPlayerState` — restores persisted player
//! state (missions, abilities, bandolier) onto the cell entity and fires the
//! content engine's `player_loaded` trigger.

use tokio::sync::mpsc;

use cimmeria_content_engine::chain::ChainEngine;

use crate::cell::content;
use crate::cell::messages::{CellToBaseMsg, SavedMission};
use crate::cell::space_manager::SpaceManager;

/// Handles the `InitPlayerState` message: restores player missions, abilities,
/// bandolier items, and fires the content-engine `player_loaded` trigger.
///
/// Wrapped in a `world_entry.init_player_state` span with all the per-burst
/// counts (saved_missions, abilities, bandolier_items, regions) as fields
/// so SigNoz can correlate freeze symptoms against initial-load burst size.
/// See the freeze investigation in the 15:50:49Z session — bursts > N
/// regions or > M missions are the suspected client-stall trigger.
#[tracing::instrument(
    name = "world_entry.init_player_state",
    level = "info",
    skip(saved_missions, abilities, bandolier_items, system_options, tx, space_mgr, engine),
    fields(
        entity_id,
        player_id,
        archetype_id,
        world = %world_name,
        saved_missions = saved_missions.len(),
        abilities = abilities.len(),
        bandolier_items = bandolier_items.len(),
        active_bandolier_slot,
        regions = tracing::field::Empty,
    ),
)]
pub(in crate::cell::service) async fn handle_init_player_state(
    entity_id: u32,
    player_id: i32,
    world_name: String,
    archetype_id: i32,
    saved_missions: Vec<SavedMission>,
    abilities: Vec<i32>,
    active_bandolier_slot: i32,
    bandolier_items: Vec<(i32, cimmeria_entity::cell_entity::BandolierItem)>,
    system_options: cimmeria_entity::cell_entity::SystemOptions,
    state_field: u32,
    access_level: u32,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
    engine: &ChainEngine,
) {
    tracing::debug!(entity_id, player_id, archetype_id, %world_name, saved_count = saved_missions.len(), ability_count = abilities.len(), "InitPlayerState");
    if let Some(entity) = space_mgr.get_entity_mut(entity_id) {
        entity.player_id = Some(player_id);
        entity.archetype_id = Some(archetype_id);
        // Authoritative GM/admin level for this session, sourced from the
        // login row. The cell-method GM gate reads this; storing it here
        // (rather than trusting any per-call client byte) is the
        // server-authority fix for CAT-N-03 (#475).
        entity.access_level = access_level;

        // Seed core stats from the archetype's base values. Without
        // this seed, the cell-side `CellEntity::stats` stays at the
        // default `Stat { min: 0, cur: 0, max: 0 }` tuple it gets
        // from `StatList::new()`, which means:
        //   - FOCUS is at 0/0/0 → `RangedPhysicalDamage` reads
        //     `target.stats.get(FOCUS).cur` as 0 and absorbs 0 — the
        //     shield mechanism never engages and the player takes
        //     full overflow damage every shot.
        //   - HEALTH is at 0/0/0 → `set_current(max)` on respawn
        //     restores 0, leaving the player with 0 HP at spawn.
        //
        // The base service's `world_data::map_loaded` builds a LOCAL
        // `StatList`, applies the archetype to it, and serializes
        // that for `onStatUpdate` — so the CLIENT sees the right
        // numbers from world entry while the SERVER's cell entity
        // has zeroes. Symptom on lomiada's 2026-06-04 18:09 session:
        // every NPC shot landed `focus_overflow = focus_damage` (no
        // absorb), spillover full strength, 86 HP per hit.
        //
        // The archetype lookup mirrors the map_loaded path so both
        // sides use the same source-of-truth (the values hardcoded
        // from `db/resources/Archetypes/Seed/archetypes.sql` in
        // `mercury::world_data::stats::archetype_stats`).
        {
            let arch = crate::mercury::archetype_stats(archetype_id);
            entity
                .stats
                .apply_archetype(&cimmeria_entity::stats::ArchetypeStatValues {
                    coordination: arch.coordination,
                    engagement: arch.engagement,
                    fortitude: arch.fortitude,
                    morale: arch.morale,
                    perception: arch.perception,
                    intelligence: arch.intelligence,
                    health: arch.health,
                    focus: arch.focus,
                    health_per_level: arch.health_per_level,
                    focus_per_level: arch.focus_per_level,
                });
            tracing::info!(
                target: "stats",
                event = "archetype_stats_seeded",
                entity_id,
                archetype_id,
                health = arch.health,
                focus = arch.focus,
                "Seeded cell-entity stats from archetype base values"
            );
        }

        // Register player's known abilities on the server-side entity
        for &ability_id in &abilities {
            entity.abilities.add_ability(ability_id);
        }
        tracing::debug!(
            entity_id,
            count = abilities.len(),
            "Registered player abilities on cell entity"
        );

        // Apply bandolier state to entity — restore persisted bandolier slot and items
        entity.active_bandolier_slot = active_bandolier_slot;
        entity.bandolier_items = bandolier_items.into_iter().collect();
        tracing::debug!(
            entity_id,
            active_bandolier_slot,
            bandolier_item_count = entity.bandolier_items.len(),
            "Applied bandolier state to cell entity"
        );

        // Apply server-synced client options. Without this assignment
        // the entity would silently fall back to `SystemOptions::default()`
        // on every login — the user could toggle the checkbox in-game,
        // see it appear to save (we persist to DB), then find it back
        // on default after a relog. The hydrate path closes that loop.
        entity.system_options = system_options;
        tracing::debug!(
            entity_id,
            auto_reload = entity.system_options.auto_reload,
            reload_on_activate = entity.system_options.reload_on_activate,
            "Applied system options to cell entity"
        );

        // Stage B: Seed each populated bandolier slot's AmmoSlot{N} stat
        // from its persisted current_ammo / clip_size. The default stat
        // tuple is (0,0,0), and `set_slot_ammo` clamps via the stat
        // bounds — without this seed, every later refill/decrement
        // would silently pin to 0. Clearing dirty avoids a duplicate
        // stat send (the initial mapLoaded uses serialize_all()).
        let slot_seed: Vec<(i32, i32, i32)> = entity
            .bandolier_items
            .iter()
            .map(|(&slot, item)| (slot, item.current_ammo, item.clip_size))
            .collect();
        for (slot_id, current, clip) in slot_seed {
            let stat_id = cimmeria_entity::stats::AMMO_SLOT_1 + slot_id;
            if let Some(stat) = entity.stats.get_mut(stat_id) {
                stat.update(0, current, clip);
                stat.clear_dirty();
            }
        }

        // Restore saved missions BEFORE content engine fires, so that
        // chain conditions correctly see existing mission state and
        // don't re-trigger already-active or completed missions.
        for saved in &saved_missions {
            use cimmeria_entity::missions::{
                MissionInstance, MissionObjective, STATUS_ACTIVE, STATUS_COMPLETED,
            };
            let objectives: Vec<MissionObjective> = saved
                .active_objective_ids
                .iter()
                .map(|&oid| {
                    let status = if saved.completed_objective_ids.contains(&oid) {
                        STATUS_COMPLETED
                    } else {
                        STATUS_ACTIVE
                    };
                    MissionObjective {
                        objective_id: oid,
                        status,
                        hidden: false,
                        optional: false,
                    }
                })
                .collect();

            let mut mission = MissionInstance::new(
                saved.mission_id,
                saved.current_step_id.unwrap_or(0),
                objectives,
            );
            mission.status = saved.status;
            mission.completed_steps = saved.completed_step_ids.clone();
            mission.completed_objectives = saved.completed_objective_ids.clone();
            // Without this, `complete()` on a re-accepted repeatable
            // mission post-relog would jump from 0 -> 1 instead of
            // N -> N+1, defeating the numRepeats cap. (#118)
            mission.repeats = saved.repeats;

            entity.missions.add_mission(mission);
            tracing::debug!(
                entity_id,
                mission_id = saved.mission_id,
                status = saved.status,
                "Restored saved mission"
            );
        }
        entity.saved_missions_loaded = true;

        // Reset all ability cooldowns on world entry. Original server
        // behavior was that cooldowns wiped on relog; we match that here
        // explicitly so the client doesn't sit waiting on a stale cooldown
        // timer that the server has no record of (or vice versa). Also
        // avoids shipping per-ability `onTimerUpdate` packets for stale
        // cooldown state during the initial-load burst — one less thing
        // the client has to chew on. (PR #410)
        entity.abilities.clear_all_cooldowns();

        // Restore the persisted user-preference state bits (#412). The
        // mask strips anything a corrupt / hand-edited row might carry —
        // restoring a saved BSF_Dead or BSF_MovementLock would spawn the
        // player frozen, the exact "relog is a fresh combat slate"
        // violation the cooldown wipe above exists to prevent. Raw `|=`
        // because BSF_AutoCycling is a single-source flag that bypasses
        // the ref-counted helpers (see cell::combat::auto_cycle's module
        // doc). When the bit is set we also re-arm `abilities.auto_cycle`
        // so the player's first attack of the new session enters
        // `arm_auto_cycle` and the server-driven re-fire loop starts —
        // the loop's ability stash (`auto_cycle_ability_id`) stays None
        // until that first commit, by design.
        let restored_state = state_field & crate::cell::combat::PERSISTED_STATE_FIELD_MASK;
        if restored_state != 0 {
            entity.state_field |= restored_state;
            if restored_state & crate::cell::combat::BSF_AUTO_CYCLING != 0 {
                entity.abilities.auto_cycle = true;
            }
            tracing::info!(
                entity_id,
                state_field = restored_state,
                "Restored persisted state_field preference bits"
            );
        }
    }

    // Re-broadcast the restored state field so the client's UI reflects
    // the preference immediately (the auto-cycle gun-icon button
    // highlight listens on the BSF_AutoCycling transition — see the
    // Ghidra note on `BSF_AUTO_CYCLING`). The client initialises its
    // cached state_field to 0, so without this send the bit is armed
    // server-side but the button looks off until the first in-session
    // toggle. Sent post-onClientReady for the same ordering reason as
    // the onActiveSlotUpdate resync below.
    {
        // Broadcast the FULL current state_field (the client's XOR-delta
        // dispatcher diffs against its cached value), but gate the send
        // on a preference bit actually having been restored — a fresh
        // character with state_field 0 needs no packet.
        let (full_state, restored) = space_mgr
            .get_entity(entity_id)
            .map(|e| {
                (
                    e.state_field,
                    e.state_field & crate::cell::combat::PERSISTED_STATE_FIELD_MASK,
                )
            })
            .unwrap_or((0, 0));
        if restored != 0 {
            crate::cell::abilities::send_entity_method(
                entity_id,
                crate::mercury::method_idx::ON_STATE_FIELD_UPDATE,
                full_state.to_le_bytes().to_vec(),
                tx,
                space_mgr,
            )
            .await;
        }
    }

    // Resend active mission state to the client so the journal UI is
    // populated with the player's in-progress missions immediately on
    // world entry. `serialize_resend` filters to active+visible only,
    // so completed missions don't ride this path. Critical for relog
    // usability — without it, the client's journal is empty until the
    // next mission state change fires onMissionUpdate. (PR #410)
    crate::cell::missions::resend_missions(entity_id, tx, space_mgr).await;

    // Push the player's known-abilities list to the client so the hotbar
    // populates immediately at world entry. Without this, the client's
    // hotbar stays empty until the next `addAbility` call (which doesn't
    // happen unless the player visits a trainer), so even players with
    // 3+ starter abilities couldn't see or click any of them.
    send_known_abilities_update(entity_id, tx, space_mgr).await;

    // Re-send `onActiveSlotUpdate` for the bandolier — defensive resync
    // against a client-side initialization race documented in
    // [docs/reverse-engineering/findings/client-wire-emit-suppression.md].
    //
    // The login burst at `mercury::world_data::map_loaded` already sends
    // this packet (see `map_loaded.rs:354`), but Ghidra analysis of the
    // client's NetIn handler `FUN_00da9ce0` shows it walks
    // `SGWPlayer.bagList` at `+0x8c → +0x24` and silently no-ops when
    // the bag-list map is uninitialized. If the burst's
    // `onActiveSlotUpdate` arrives before the bag-init packets in the
    // same bundle are processed, the cached active-slot value at
    // `slot+0xc` never gets written and stays at the default (0). The
    // Lua gate inside `BandolierMod.ActivateBandolierSlotN` then reads
    // `getActiveSlotForContainer(3) ~= N` as false for any keypress
    // matching the stale-cached slot, and `requestActiveSlotChange` is
    // never emitted — symptom: "F2 doesn't swap to the P90."
    //
    // `InitPlayerState` runs AFTER the client sends `onClientReady`,
    // which is itself sent only AFTER the client has processed the
    // entire `mapLoaded` bundle (per the handler's docstring above).
    // So the bag-list map is guaranteed initialized by the time this
    // resend lands — the cached value gets the correct write and the
    // Lua gate stops misfiring.
    //
    // Wire format mirrors `bandolier.rs:449-451` and `map_loaded.rs:354`:
    // bag_id (i32 LE) + (slot_id + 1) (i32 LE, 1-indexed wire) = 8 bytes.
    {
        const CONTAINER_BANDOLIER: i32 = 3;
        let active_slot = space_mgr
            .get_entity(entity_id)
            .map(|e| e.active_bandolier_slot)
            .unwrap_or(0);
        let mut args = Vec::with_capacity(8);
        args.extend_from_slice(&CONTAINER_BANDOLIER.to_le_bytes());
        args.extend_from_slice(&(active_slot + 1).to_le_bytes());
        crate::cell::abilities::send_entity_method(
            entity_id,
            crate::cell::client_methods::inventory::ON_ACTIVE_SLOT_UPDATE,
            args,
            tx,
            space_mgr,
        )
        .await;
        tracing::info!(
            target: "bandolier.resend",
            entity_id,
            active_slot,
            "Re-sent onActiveSlotUpdate post-onClientReady (defensive resync \
             against client bag-list init race — see \
             docs/reverse-engineering/findings/client-wire-emit-suppression.md)"
        );
    }

    // Send addClientHintedGenericRegion for each client-hinted region in
    // this world. Matches Python Space.playerEntered() → queryRegions():
    // clearClientHintedGenericRegions was already sent in mapLoaded body,
    // now register all regions so the client can fire triggerRegion events.
    //
    // PR #410 fix: previously this loop fired 20+ separate EntityMethodCall
    // messages, each becoming an individual reliable Mercury packet. On
    // existing-character login the combined ACK pressure stalled some
    // clients past their render-thread budget (freeze investigation
    // 2026-05-26 — see issue #408 / `world_entry.region_burst` span). Now
    // we collect all hints into a single `EntityMethodCallBatch` so the
    // base side packs them into ONE Mercury packet body. Client sees the
    // same method-call sequence; transport collapses 22 datagrams into 1.
    {
        use crate::cell::space_manager::REGION_FLAG_CLIENT_HINTED;
        let world_regions: Vec<_> = space_mgr
            .regions_for_world(&world_name)
            .iter()
            .filter(|r| r.flags & REGION_FLAG_CLIENT_HINTED != 0)
            .map(|r| (r.runtime_id, r.height, r.radius, r.flags, r.points.clone()))
            .collect();

        let region_count = world_regions.len();
        let burst_span = tracing::info_span!(
            "world_entry.region_burst",
            entity_id,
            world = %world_name,
            count = region_count,
        );
        let _burst_guard = burst_span.enter();
        let burst_start = std::time::Instant::now();
        let mut batch: Vec<(u16, Vec<u8>)> = Vec::with_capacity(region_count);
        for (rid, height, radius, flags, points) in world_regions {
            let mut args = Vec::with_capacity(16 + points.len() * 12);
            args.extend_from_slice(&(rid as i32).to_le_bytes());
            args.extend_from_slice(&height.to_le_bytes());
            args.extend_from_slice(&radius.to_le_bytes());
            args.extend_from_slice(&flags.to_le_bytes());
            args.extend_from_slice(&(points.len() as u32).to_le_bytes()); // ARRAY count
            for p in &points {
                args.extend_from_slice(&p[0].to_le_bytes()); // x
                args.extend_from_slice(&p[1].to_le_bytes()); // y
                args.extend_from_slice(&p[2].to_le_bytes()); // z
            }
            batch.push((
                crate::mercury::method_idx::ADD_CLIENT_HINTED_GENERIC_REGION,
                args,
            ));
        }
        if !batch.is_empty() {
            let _ = tx
                .send(CellToBaseMsg::EntityMethodCallBatch {
                    entity_id,
                    calls: batch,
                })
                .await;
        }
        let burst_elapsed = burst_start.elapsed();
        tracing::Span::current().record("regions", region_count);
        if region_count > 0 {
            tracing::info!(
                entity_id, player_id, world = %world_name,
                count = region_count,
                burst_micros = burst_elapsed.as_micros() as u64,
                packed_into_one_packet = true,
                "Sent region registrations"
            );
        }
    }

    // `reloadOnActivate` activation site for the world-entry path
    // (initial login, gate travel, cross-world ring). Same-world
    // respawn is deliberately NOT a trigger site — `ReanchorPlayer`
    // keeps the cell entity intact, so the weapon is never
    // "activated" in the sense `SystemOptions.xml` defines. Helper
    // self-gates on option-off, non-player, melee, full clip, and
    // in-flight reload, so the unconditional call is safe.
    crate::cell::cell_methods::player::world::maybe_trigger_reload_on_activate(
        entity_id, tx, space_mgr,
    )
    .await;

    content::fire_player_loaded(entity_id, player_id, &world_name, engine, tx, space_mgr).await;
}

/// Send `onKnownAbilitiesUpdate(ARRAY<INT32> abilities)` to the player so the
/// hotbar UI populates at world entry. Without this, characters with starter
/// abilities (every char_def_id after Phase 2) still showed an empty
/// hotbar because the client only gets ability state via this RPC and the
/// previous code never sent it on login.
///
/// Mirrors Python `SGWPlayer.addAbility` which calls `onKnownAbilitiesUpdate`
/// on every add (`deprecated/python/cell/SGWPlayer.py:861`). We send the full
/// list once at world entry so the in-game add path can stay event-driven.
///
/// Wire format: `ARRAY<INT32> AbilityData` → `u32 count` + N × `i32 ability_id`.
/// Method index 101 (`ON_KNOWN_ABILITIES_UPDATE`).
pub(super) async fn send_known_abilities_update(
    entity_id: u32,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &SpaceManager,
) {
    let ability_ids: Vec<i32> = match space_mgr.get_entity(entity_id) {
        Some(e) => e.abilities.known_ability_ids(),
        None => return,
    };

    let mut args = Vec::with_capacity(4 + ability_ids.len() * 4);
    args.extend_from_slice(&(ability_ids.len() as u32).to_le_bytes());
    for id in &ability_ids {
        args.extend_from_slice(&id.to_le_bytes());
    }

    let send_result = tx
        .send(CellToBaseMsg::EntityMethodCall {
            entity_id,
            method_index: crate::cell::client_methods::player::ON_KNOWN_ABILITIES_UPDATE,
            args,
        })
        .await;
    match send_result {
        Ok(()) => {
            tracing::info!(
                entity_id,
                count = ability_ids.len(),
                "Sent onKnownAbilitiesUpdate (hotbar seed)"
            );
        }
        Err(e) => {
            // Channel closed — the player's hotbar will be empty until
            // they reconnect. Log at error so the symptom ("no abilities
            // on the bar") has a corresponding server-side log entry.
            tracing::error!(
                entity_id,
                count = ability_ids.len(),
                error = %e,
                "Failed to send onKnownAbilitiesUpdate (hotbar seed) — cell→base channel closed"
            );
        }
    }
}

#[cfg(test)]
mod tests;
