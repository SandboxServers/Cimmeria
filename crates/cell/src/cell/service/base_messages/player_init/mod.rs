//! Handler for `BaseToCellMsg::InitPlayerState` — restores persisted player
//! state (missions, abilities, bandolier) onto the cell entity and fires the
//! content engine's `player_loaded` trigger.

use tokio::sync::mpsc;

use cimmeria_content_engine::chain::ChainEngine;

use crate::cell::content;
use crate::cell::messages::{CellToBaseMsg, SavedMission};
use crate::cell::space_manager::SpaceManager;

// The client-cache resync and the hotbar seed are shared with the respawn
// reanchor, which the GM `gmRespawn` also drives, so they sit with the respawn
// core in `cell::respawn` (services-crate-split.md §2H).
use crate::cell::respawn::resync;
pub(super) use crate::cell::respawn::send_known_abilities_update;

pub(crate) mod mission_restore;

/// Seed each populated bandolier slot's `AmmoSlot{N}` stat from its persisted
/// `current_ammo` / `clip_size`.
///
/// The default stat tuple is `(0,0,0)`, and `set_slot_ammo` clamps via the
/// stat bounds — without this seed, every later refill/decrement would
/// silently pin to 0. Clearing dirty avoids a duplicate stat send (the
/// initial mapLoaded uses `serialize_all()`).
///
/// Extracted so the production `InitPlayerState` path and its regression
/// guard exercise the same code.
pub(in crate::cell::service) fn seed_bandolier_ammo_stats(
    entity: &mut cimmeria_entity::cell_entity::CellEntity,
) {
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
}

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
    access_level: u32,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
    engine: &ChainEngine,
) {
    tracing::debug!(entity_id, player_id, archetype_id, %world_name, saved_count = saved_missions.len(), ability_count = abilities.len(), "InitPlayerState");
    // Reconstruct before the mutable entity borrow: hydration reads the
    // `mission_defs` / `step_objectives` caches off `space_mgr`.
    let restored_missions = mission_restore::build_restored_missions(&saved_missions, space_mgr);
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
            // The archetype values below replace the primary attributes a
            // timed stat buff (a stimpack) would have raised, so a buff
            // still in the ledger would restore its delta below the base
            // when it expired. `InitPlayerState` reaches a fresh entity on
            // every world entry, so the ledger is empty here; clearing it
            // keeps that true should the message ever reach a live one.
            entity.stat_buffs = Default::default();
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
        // from its persisted current_ammo / clip_size.
        seed_bandolier_ammo_stats(entity);

        // Restore saved missions BEFORE content engine fires, so that
        // chain conditions correctly see existing mission state and
        // don't re-trigger already-active or completed missions.
        //
        // Built outside the entity borrow because the reconstruction
        // reads the mission/step definition caches off `space_mgr` — see
        // `mission_restore` for why the roster comes from the definition
        // rather than from the saved array.
        for mission in restored_missions {
            tracing::debug!(
                entity_id,
                mission_id = mission.mission_id,
                status = mission.status,
                objectives = mission.active_objectives.len(),
                "Restored saved mission"
            );
            entity.missions.add_mission(mission);
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
    }

    // Passive abilities (`EF_AlwaysPersist` effects, e.g. 2852 Heed Our
    // Calling's `speedPet`) hold for as long as the ability is known, and
    // the cell's stats start fresh every session (pets PT-08).
    let _passives = super::passive_sync::apply_passives_and_sync(
        entity_id,
        &abilities,
        crate::cell::effects::passives::PassiveChange::Learned,
        tx,
        space_mgr,
    )
    .await;

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
    // Wire format lives in `resync::send_active_slot_resend`, shared with the
    // post-respawn resync (the reanchor's pawn recreate wipes the cached slot
    // the same way an uninitialized bag list does).
    resync::send_active_slot_resend(entity_id, tx, space_mgr).await;

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
        let burst_span = tracing::info_span!(
            "world_entry.region_burst",
            entity_id,
            world = %world_name,
            count = tracing::field::Empty,
        );
        let _burst_guard = burst_span.enter();
        let burst_start = std::time::Instant::now();
        let region_count = crate::cell::cell_methods::player::world::send_client_hinted_regions(
            entity_id,
            &world_name,
            // mapLoaded already sent the clear
            crate::cell::cell_methods::player::world::ClearFirst::No,
            tx,
            space_mgr,
        )
        .await;
        let burst_elapsed = burst_start.elapsed();
        burst_span.record("count", region_count);
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

    // Everything the cell sends on world entry (regions, missions, stats)
    // is queued above this line; the first `region_hint` after this entry is
    // the client proving it received the region list.
    let missions = space_mgr
        .get_entity(entity_id)
        .map_or(0, |e| e.missions.count());
    {
        let id = space_mgr.player_identity(entity_id);
        let (name, archetype, level, access_level) =
            space_mgr
                .get_entity(entity_id)
                .map_or_else(Default::default, |e| {
                    (
                        e.character_name.clone().unwrap_or_default(),
                        e.archetype_id.unwrap_or(0),
                        e.level,
                        e.access_level,
                    )
                });
        // Pair with `session.end`. Client telemetry (`launcher.ingest` /
        // `launcher.bundle`) is ingested by admin-api, which this crate cannot
        // see -- so liveness is a QUERY: a `session.start` for an account with
        // no `launcher.*` rows in the same window means the tester is playing
        // without client logs. See the "sessions vs client telemetry" view.
        tracing::info!(
            target: "session.start",
            entity_id,
            account_id = id.account_id,
            player_id = id.player_id,
            character_name = %name,
            archetype,
            level,
            access_level,
            world = %world_name,
            missions,
            "player entered world"
        );
    }
    crate::cell::player_journal::note(
        entity_id,
        crate::cell::player_journal::kinds::WORLD_ENTER,
        format!("world={world_name} missions={missions}"),
    );
    content::fire_player_loaded(entity_id, player_id, &world_name, engine, tx, space_mgr).await;
}

#[cfg(test)]
mod tests;
