//! CellService startup: load XML world defs, hydrate DB caches, spawn the message loop.

use std::collections::BTreeMap;
use std::sync::Arc;

use tokio::sync::Notify;

use cimmeria_cell_world::cell::effects::registry::needs_no_script;

use super::super::content;
use super::super::cover;
use super::super::messages::CellToBaseMsg;
use super::super::space_manager::SpaceManager;
use super::super::{spawner, CellError};
use super::start_profile_audit::audit_start_profiles;
use super::startup_report::{cache_failed, startup_summary};
use super::CellService;

impl CellService {
    /// Start the cell service.
    ///
    /// Loads space definitions from XML, creates startup spaces, and begins
    /// processing messages from BaseApp.
    pub async fn start(&mut self) -> Result<(), CellError> {
        tracing::info!(addr = %self.listener_addr, "Starting cell service");

        // Load space definitions from XML
        let mut space_mgr = SpaceManager::new(1); // cell_id = 1
                                                  // Names of the caches that failed to load, for the boot summary.
        let mut failed_caches: Vec<&'static str> = Vec::new();

        // Feature plugins (#962). The orchestrator refuses to start with an
        // incomplete table; a bare `CellService` (a test harness) starts
        // anyway, and this WARN names what the client can call and nothing
        // will answer.
        if let Err(e) = self.plugins.check_complete() {
            tracing::warn!(
                target: "cell.plugin",
                reason = "plugin_table_incomplete",
                plugins = ?self.plugins.plugin_names(),
                error = %e,
                "cell service starting with an incomplete plugin table -- those cell methods \
                 fall through to 'Unhandled cell method call' and the player's press does nothing"
            );
        } else {
            tracing::info!(
                target: "cell.plugin",
                plugins = ?self.plugins.plugin_names(),
                cell_methods = ?self.plugins.cell_method_indices().collect::<Vec<_>>(),
                "cell plugins installed"
            );
        }
        space_mgr.install_plugins(self.plugins.clone());

        // Effect scripts (#962 step 4). Installed before anything spawns: the
        // spawn-time cover hold runs Cover Stance through the registry. The
        // orchestrator refuses to start with an empty registry; a bare
        // `CellService` (a test harness) starts anyway, and this WARN says
        // every scripted effect falls back to the legacy NVP path.
        if self.effect_scripts.is_empty() {
            tracing::warn!(
                target: "abilities",
                event = "effect_scripts_empty",
                reason = "effect_scripts_not_installed",
                "cell service starting with no effect scripts -- every effect with a \
                 script_name falls back to the legacy NVP path (no heals, shields, stuns, \
                 stat buffs or cover stance)"
            );
        } else {
            tracing::info!(
                target: "abilities",
                event = "effect_scripts_installed",
                count = self.effect_scripts.len(),
                scripts = ?self.effect_scripts.names().collect::<Vec<_>>(),
                "effect scripts installed"
            );
        }
        space_mgr.install_effect_scripts(self.effect_scripts.clone());
        match space_mgr.load_from_xml(&self.entities_dir) {
            Ok(()) => {
                tracing::info!(
                    worlds = space_mgr.world_count(),
                    startup_spaces = space_mgr.space_count(),
                    "Cell service loaded space definitions"
                );
            }
            Err(e) => {
                failed_caches.push(cache_failed(
                    "space_definitions",
                    e,
                    "continuing with an empty space set",
                ));
            }
        }

        // Load spawn records from DB and populate startup spaces
        let spawn_records = if let Some(ref pool) = self.db_pool {
            match spawner::load_spawns_from_db(pool).await {
                Ok(records) => {
                    tracing::info!(count = records.len(), "Loaded spawn records from database");
                    records
                }
                Err(e) => {
                    failed_caches.push(cache_failed(
                        "spawn_records",
                        e,
                        "using the hardcoded fallback",
                    ));
                    vec![]
                }
            }
        } else {
            vec![]
        };
        // Switchable spawn sets (DA-10): their members stay out of the
        // startup spawn (and out of every instance's) until a GM switches the
        // set on.
        let mut spawn_records = spawn_records;
        if let Some(ref pool) = self.db_pool {
            match spawner::load_spawn_sets(pool).await {
                Ok(defs) => {
                    space_mgr.spawn_sets =
                        super::super::space_manager::partition_spawn_sets(&mut spawn_records, defs);
                }
                Err(e) => {
                    tracing::warn!(
                        target: "spawner",
                        event = "spawn_sets_load_failed",
                        error = %e,
                        "Failed to load spawn sets; every spawnlist row spawns at startup"
                    );
                    failed_caches.push("spawn_sets");
                }
            }
        }

        // Ability definitions load before the startup spawn: each NPC's
        // `spawner.npc_behaviour` row reads its abilities' event sets from
        // them (NA44), and without them every startup NPC logged 0.
        if let Some(ref pool) = self.db_pool {
            match spawner::load_ability_defs(pool).await {
                Ok(defs) => {
                    space_mgr.ability_defs = defs;
                }
                Err(e) => {
                    failed_caches.push(cache_failed(
                        "ability_defs",
                        e,
                        "startup NPCs get no ability event sets",
                    ));
                }
            }
        }

        let npc_count =
            super::super::space_manager::spawn_npcs_from_records(&spawn_records, &mut space_mgr);
        tracing::info!(npc_count, "NPC population initialized");

        // Stamp the per-world `resources.worlds` settings onto the
        // spaces.xml world table. The content engine's `world` condition
        // resolves the acting player's space through the numeric id;
        // without them every `world`-gated chain fails closed, so this logs
        // at WARN rather than staying silent. The same stamp carries
        // `navmesh_mode`; a world that misses it keeps containment
        // enforced, which is the pre-H53 behaviour.
        if let Some(ref pool) = self.db_pool {
            match spawner::load_world_rows(pool).await {
                Ok(rows) => {
                    space_mgr.stamp_world_rows(&rows);
                }
                Err(e) => {
                    failed_caches.push(cache_failed(
                        "world_rows",
                        e,
                        "content-engine `world` conditions fail closed everywhere and \
                         every world's navmesh stays a containment gate",
                    ));
                }
            }
        }

        // One line per meshed world, after the modes are stamped: which
        // mesh is a gate, how big it is, and how much of that world's own
        // authored population it does not cover. A mesh that loads cleanly
        // and still describes a different map is otherwise silent until a
        // player walks into it.
        space_mgr.log_navmesh_summary(&spawn_records);

        // Load dialog_set_maps cache for per-player interaction system
        if let Some(ref pool) = self.db_pool {
            match spawner::load_dialog_set_maps(pool).await {
                Ok(maps) => {
                    space_mgr.dialog_set_maps = maps;
                }
                Err(e) => {
                    failed_caches.push(cache_failed(
                        "dialog_set_maps",
                        e,
                        "per-player dialog interaction has no dialog maps",
                    ));
                }
            }
        }

        // Load monologue dialog ids for the DisplayDialog executor.
        // ERROR (not WARN) on failure: an empty cache silently strips
        // ~42% of authored dialog screens (the inner-thought ones)
        // from world entry. The executor falls through to bail-and-
        // warn so behavior stays safe, but operators need to see this.
        if let Some(ref pool) = self.db_pool {
            match spawner::load_monologue_dialog_ids(pool).await {
                Ok(ids) => {
                    space_mgr.monologue_dialog_ids = ids;
                }
                Err(e) => {
                    tracing::error!("Failed to load monologue dialog ids: {e}");
                    failed_caches.push("monologue_dialog_ids");
                }
            }
        }

        // Load tutorial dialog ids so a close for a client-opened tutorial
        // is not logged as a forged dialog choice. WARN on failure: the
        // only cost is the louder log line coming back.
        if let Some(ref pool) = self.db_pool {
            match spawner::load_tutorial_dialog_ids(pool).await {
                Ok(ids) => {
                    space_mgr.tutorial_dialog_ids = ids;
                }
                Err(e) => {
                    failed_caches.push(cache_failed(
                        "tutorial_dialog_ids",
                        e,
                        "a client-opened tutorial close is logged as a forged dialog choice",
                    ));
                }
            }
        }

        // Load dialog screen text for the `npc_bark` content action. An
        // empty cache makes every bark warn and send nothing rather than
        // speak a wrong line, so WARN is the right level: the failure is
        // contained, but no companion line reaches any player until it is
        // fixed.
        if let Some(ref pool) = self.db_pool {
            match spawner::load_dialog_screen_text(pool).await {
                Ok(text) => {
                    space_mgr.dialog_screen_text = text;
                }
                Err(e) => {
                    failed_caches.push(cache_failed(
                        "dialog_screen_text",
                        e,
                        "every npc_bark content action warns and sends nothing",
                    ));
                }
            }
        }

        // Load mission definitions cache for AcceptMission content actions
        if let Some(ref pool) = self.db_pool {
            match spawner::load_mission_defs(pool).await {
                Ok(defs) => {
                    space_mgr.mission_defs = defs;
                }
                Err(e) => {
                    failed_caches.push(cache_failed(
                        "mission_defs",
                        e,
                        "AcceptMission content actions fail",
                    ));
                }
            }
        }

        // Load step objectives cache for AdvanceStep content actions
        if let Some(ref pool) = self.db_pool {
            match spawner::load_step_objectives(pool).await {
                Ok(objs) => {
                    space_mgr.step_objectives = objs;
                }
                Err(e) => {
                    failed_caches.push(cache_failed(
                        "step_objectives",
                        e,
                        "AdvanceStep content actions fail",
                    ));
                }
            }
        }

        // Step owners, so `gmMissionAdvance` can refuse a step from another
        // mission. Without them every GM advance is refused (with feedback).
        if let Some(ref pool) = self.db_pool {
            match spawner::load_step_missions(pool).await {
                Ok(steps) => {
                    space_mgr.step_missions = steps;
                }
                Err(e) => {
                    failed_caches.push(cache_failed(
                        "step_missions",
                        e,
                        "every GM mission advance is refused",
                    ));
                }
            }
        }

        // Load stargate destinations cache for gate travel
        if let Some(ref pool) = self.db_pool {
            match spawner::load_stargates(pool).await {
                Ok(gates) => {
                    space_mgr.stargates = gates;
                }
                Err(e) => {
                    failed_caches.push(cache_failed(
                        "stargates",
                        e,
                        "gate travel has no destinations",
                    ));
                }
            }
        }

        // Load generic regions from DB and register with auto-incrementing runtime IDs.
        // Reference: python/cell/GenericRegion.py — GenericRegionManager.load() + registerRegion()
        if let Some(ref pool) = self.db_pool {
            match spawner::load_regions_from_db(pool).await {
                Ok(region_data) => {
                    for rd in region_data {
                        let runtime_id = space_mgr.next_region_id;
                        space_mgr.next_region_id += 1;
                        space_mgr.regions.insert(
                            runtime_id,
                            super::super::space_manager::RegionData {
                                runtime_id,
                                db_set_id: rd.set_id,
                                tag: rd.name,
                                world_name: rd.world_name,
                                height: rd.height,
                                radius: rd.radius,
                                flags: rd.flags,
                                points: rd.points,
                            },
                        );
                    }
                    tracing::info!(
                        count = space_mgr.regions.len(),
                        "Registered generic regions"
                    );
                }
                Err(e) => {
                    failed_caches.push(cache_failed(
                        "generic_regions",
                        e,
                        "no generic regions are registered",
                    ));
                }
            }
        }

        // Load respawner definitions for death/respawn system
        if let Some(ref pool) = self.db_pool {
            match spawner::load_respawners(pool).await {
                Ok(defs) => {
                    space_mgr.respawners = defs;
                }
                Err(e) => {
                    failed_caches.push(cache_failed(
                        "respawners",
                        e,
                        "death and respawn have no respawner definitions",
                    ));
                }
            }
        }

        // Load effect definitions and the other combat caches from DB
        // (ability definitions load before the startup spawn, above).
        if let Some(ref pool) = self.db_pool {
            // Eye heights for line of sight (NA31). Read at query time, so
            // the NPCs spawned above pick them up too.
            match spawner::load_body_set_eye_heights(pool).await {
                Ok(map) => {
                    space_mgr.body_set_eye_heights = map;
                }
                Err(e) => {
                    failed_caches.push(cache_failed(
                        "body_set_eye_heights",
                        e,
                        "every being uses the 1.5 m default eye height",
                    ));
                }
            }
            match spawner::load_effect_defs(pool).await {
                Ok(defs) => {
                    space_mgr.effect_defs = defs;
                    // #962 step 4: a script name the data carries and no
                    // registered script answers is a missing registration.
                    // Dispatch still falls back per call; this names them
                    // once, in one row. A blank or native name needs no
                    // script, so it is an INFO row, not a WARN (TG-CMB-04).
                    let missing = space_mgr
                        .effect_scripts()
                        .unregistered(space_mgr.effect_defs.values());
                    let mut native: BTreeMap<&str, Vec<i32>> = BTreeMap::new();
                    for def in space_mgr.effect_defs.values() {
                        if let Some(name) =
                            def.script_name.as_deref().filter(|n| needs_no_script(n))
                        {
                            native.entry(name).or_default().push(def.effect_id);
                        }
                    }
                    for ids in native.values_mut() {
                        ids.sort_unstable();
                    }
                    if !native.is_empty() {
                        tracing::info!(
                            target: "abilities",
                            event = "effect_script_native",
                            reason = "native_or_blank",
                            count = native.len(),
                            scripts = ?native,
                            "effect rows name a native or blank script; no registered script is needed"
                        );
                    }
                    if !missing.is_empty() {
                        tracing::warn!(
                            target: "abilities",
                            event = "effect_script_unregistered",
                            reason = "no_registered_script",
                            count = missing.len(),
                            scripts = ?missing,
                            "effect rows name a script no registered script answers -- \
                             they fall back to the legacy NVP path; add the script to \
                             cimmeria-cell-effect-scripts' EFFECT_SCRIPTS or fix the \
                             effects.script_name seed"
                        );
                    }
                }
                Err(e) => {
                    failed_caches.push(cache_failed(
                        "effect_defs",
                        e,
                        "effects have no definitions",
                    ));
                }
            }
            match spawner::load_event_set_sequences(pool).await {
                Ok(map) => {
                    space_mgr.sequence_map = map;
                }
                Err(e) => {
                    failed_caches.push(cache_failed(
                        "event_set_sequences",
                        e,
                        "event-set sequences are not loaded",
                    ));
                }
            }
            // Cover-system data. Builds the per-world spatial index from
            // `resources.cover_sets` + `resources.cover_nodes`. Initialises
            // if either load returns any rows; stays on `Cover::empty()`
            // only when both come back empty (load failure or fresh DB).
            // The rest of the cell service still functions; NPCs just
            // won't use cover until the load is repaired.
            let cover_sets = match cover::load_cover_sets(pool).await {
                Ok(s) => s,
                Err(e) => {
                    tracing::error!("Failed to load cover sets: {e}");
                    failed_caches.push("cover_sets");
                    Vec::new()
                }
            };
            let cover_nodes = match cover::load_cover_nodes(pool).await {
                Ok(n) => n,
                Err(e) => {
                    tracing::error!("Failed to load cover nodes: {e}");
                    failed_caches.push("cover_nodes");
                    Vec::new()
                }
            };
            if !cover_sets.is_empty() || !cover_nodes.is_empty() {
                space_mgr.cover = cover::Cover::from_loaded(cover_sets, cover_nodes);
                tracing::info!(
                    sets = space_mgr.cover.set_count(),
                    nodes = space_mgr.cover.node_count(),
                    nodes_per_world = ?space_mgr.cover.index.world_node_counts(),
                    "Cover service loaded"
                );
            }
            // `cover.coverage event=space_summary` for every space that
            // exists now, and (from here on) every space created later.
            space_mgr.cover_loaded();
            match spawner::load_item_containers(pool).await {
                Ok(map) => {
                    space_mgr.item_containers = map;
                }
                Err(e) => {
                    failed_caches.push(cache_failed(
                        "item_containers",
                        e,
                        "item containers are not loaded",
                    ));
                }
            }
            match spawner::load_item_event_set_abilities(pool).await {
                Ok(map) => {
                    space_mgr.item_event_set_abilities = map;
                }
                Err(e) => {
                    failed_caches.push(cache_failed(
                        "item_event_set_abilities",
                        e,
                        "item event-set abilities are not loaded",
                    ));
                }
            }
            // The shared snapshot: the base's `onAbilityTreeInfo` reads the
            // same one, so the trainer and the client's tree always agree.
            match crate::ability_tree::shared_catalog(pool).await {
                Ok(catalog) => {
                    space_mgr.ability_tree_catalog = (*catalog).clone();
                }
                Err(e) => {
                    failed_caches.push(cache_failed(
                        "archetype_ability_trees",
                        e,
                        "archetype ability trees are not loaded",
                    ));
                }
            }
            // The crafting catalog, loaded once for the whole process: the
            // base validates every crafting verb against this same snapshot
            // (D-CR18). Loading it here surfaces a broken seed at boot.
            if let Err(e) = cimmeria_cell_catalog::crafting::shared_crafting_catalog(pool).await {
                tracing::warn!(
                    target: "crafting",
                    event = "catalog_load_failed",
                    error = %e,
                    "Failed to load the crafting catalog"
                );
                failed_caches.push("crafting_catalog");
            }
            match spawner::load_trainer_abilities(pool).await {
                Ok(map) => {
                    space_mgr.trainer_abilities = map;
                }
                Err(e) => {
                    failed_caches.push(cache_failed(
                        "trainer_abilities",
                        e,
                        "trainers offer no abilities",
                    ));
                }
            }
            match spawner::load_template_trainer_lists(pool).await {
                Ok(map) => {
                    space_mgr.template_trainer_lists = map;
                }
                Err(e) => {
                    failed_caches.push(cache_failed(
                        "template_trainer_lists",
                        e,
                        "template trainers offer no lists",
                    ));
                }
            }
            match spawner::load_item_defs(pool).await {
                Ok(map) => {
                    space_mgr.item_defs = map;
                }
                Err(e) => {
                    failed_caches.push(cache_failed(
                        "item_defs",
                        e,
                        "item lookups find no definitions",
                    ));
                }
            }
            match spawner::load_weapon_ranges(pool).await {
                Ok(map) => {
                    space_mgr.weapon_ranges = map;
                }
                Err(e) => {
                    failed_caches.push(cache_failed(
                        "weapon_ranges",
                        e,
                        "weapon range checks find no table",
                    ));
                }
            }
            // The player weapon requirement (OD-CS11) reads these. Left
            // empty on failure: every weapon-requiring ability is then
            // refused for players, never let through. That breaks every
            // player's weapon attacks at once, so it is an ERROR that says so.
            match spawner::load_weapon_monikers(pool).await {
                Ok(map) => {
                    space_mgr.item_monikers = map;
                }
                Err(e) => {
                    tracing::error!(
                        target: "abilities",
                        event = "item_monikers_load_failed",
                        error = %e,
                        "Failed to load item monikers: every player cast of an ability \
                         with a weapon requirement (item_monikers) will be refused with \
                         WrongWeaponType until the cell restarts with a readable resources.items"
                    );
                    failed_caches.push("item_monikers");
                }
            }
            match spawner::load_loot_tables(pool).await {
                Ok(tables) => {
                    space_mgr.loot_tables = tables;
                }
                Err(e) => {
                    failed_caches.push(cache_failed("loot_tables", e, "no loot tables are loaded"));
                }
            }
            // Prototype records for the content engine's `spawn_entity`
            // action. Distinct from the `spawn_records` above: those are
            // `spawnlist` placements, these are every template whether or
            // not it is placed anywhere.
            match spawner::load_spawn_templates(pool).await {
                Ok(map) => {
                    tracing::info!(
                        count = map.len(),
                        "Loaded entity templates for content spawn_entity"
                    );
                    space_mgr.spawn_templates = map;
                }
                Err(e) => {
                    // Not fatal, but every `spawn_entity` chain action will
                    // refuse until a restart succeeds — worth an error, not
                    // a warn, since the symptom (missions with no NPCs) is
                    // far from the cause.
                    tracing::error!(
                        "Failed to load entity templates: {e} -- every content \
                         spawn_entity action will refuse for this process lifetime"
                    );
                    failed_caches.push("spawn_templates");
                }
            }
            // NPC ability sets for `gmSetMobAbilitySet`. Not fatal: the GM
            // command refuses with `no_ability_set_data` until a restart.
            match spawner::load_ability_sets(pool).await {
                Ok(sets) => {
                    space_mgr.ability_sets = sets;
                }
                Err(e) => {
                    failed_caches.push(cache_failed(
                        "ability_sets",
                        e,
                        "gmSetMobAbilitySet refuses for this process lifetime",
                    ));
                }
            }
            // Summon ability → pet template. Not fatal: with no catalog every
            // summon ability fails as a non-damage ability does today.
            match spawner::load_pet_summons(pool).await {
                Ok(catalog) => {
                    space_mgr.pet_summons = catalog;
                }
                Err(e) => {
                    tracing::error!(
                        "Failed to load pet summons: {e} -- no summon ability \
                         will spawn a pet for this process lifetime"
                    );
                    failed_caches.push("pet_summons");
                }
            }
            // Deployable ability → template and effects. Not fatal: with no
            // catalog every deployable ability is refused at the fire as a
            // non-damage ground cast does today.
            match spawner::load_deployables(pool).await {
                Ok(catalog) => {
                    space_mgr.deployable_specs = catalog;
                }
                Err(e) => {
                    tracing::error!(
                        "Failed to load deployables: {e} -- no deployable ability \
                         will place an object for this process lifetime"
                    );
                    failed_caches.push("deployables");
                }
            }
            // Ambient chatter (Debug Area DA-09): the NPC groups that talk
            // among themselves, spoken by the `cimmeria-cell-chatter` plugin's
            // tick from this resource. Not fatal: with no catalog the plugin
            // finds nothing to say and every NPC stays silent.
            match spawner::load_ambient_chatter(pool).await {
                Ok(catalog) => {
                    space_mgr.resources.insert(catalog);
                }
                Err(e) => {
                    tracing::error!(
                        target: "chatter",
                        event = "catalog_load_failed",
                        error = %e,
                        "Failed to load ambient chatter -- no NPC group will \
                         talk among itself for this process lifetime"
                    );
                    failed_caches.push("ambient_chatter");
                }
            }
            // Special ammo catalog (ammo campaign AM-F). Not fatal: with no
            // catalog every shot fires unmodified and no ammo type has a
            // reserve item, which is today's behaviour.
            match spawner::load_ammo_catalog(pool).await {
                Ok(catalog) => {
                    space_mgr.ammo_catalog = catalog;
                }
                Err(e) => {
                    tracing::error!(
                        target: "ammo",
                        event = "catalog_load_failed",
                        error = %e,
                        "Failed to load the ammo catalog -- special ammo has no \n                         modifiers or reserve items for this process lifetime"
                    );
                    failed_caches.push("ammo_catalog");
                }
            }
            match super::super::ring_transport::load_ring_regions(pool).await {
                Ok(regions) => {
                    space_mgr.ring_transporters.load(&regions);
                    // Multiple rings sharing a point_set_id would silently
                    // route everyone through whichever ring HashMap iteration
                    // landed last. Log the collision so the bad seed data
                    // surfaces at startup rather than as ghost-routing later.
                    let mut point_set_to_region =
                        std::collections::HashMap::with_capacity(regions.len());
                    for (rid, r) in &regions {
                        if let Some(existing) = point_set_to_region.insert(r.point_set_id, *rid) {
                            tracing::error!(
                                point_set_id = r.point_set_id, // nt:id-only ring point sets have no name column
                                first_region = existing, second_region = *rid,
                                "duplicate point_set_id across ring regions — routing will be non-deterministic"
                            );
                        }
                    }
                    space_mgr.ring_point_set_to_region = point_set_to_region;
                    space_mgr.ring_regions = regions;
                    // One pass over the pad coordinates now, while the startup
                    // spaces (and their navmeshes) are already resident. A pad
                    // the navmesh rejects aborts every trip to it at runtime,
                    // and "the rings don't work" is a long way from "this seed
                    // row is off-mesh" without this line.
                    let off_mesh_pads = super::super::ring_transport::audit_ring_pads(
                        &space_mgr.ring_regions,
                        &space_mgr,
                    );
                    tracing::info!(
                        count = space_mgr.ring_regions.len(),
                        off_mesh_pads = ?off_mesh_pads,
                        "Initialized ring transporters"
                    );
                }
                Err(e) => {
                    failed_caches.push(cache_failed(
                        "ring_transport_regions",
                        e,
                        "ring transporters are not loaded",
                    ));
                }
            }
        }

        // One summary row per boot, after every cache load above.
        startup_summary(&failed_caches);

        // Send SpaceData for all startup spaces to BaseApp
        if let Some(ref tx) = self.cell_to_base_tx {
            for (space_id, world_name) in space_mgr.all_spaces() {
                let msg = CellToBaseMsg::SpaceData {
                    space_id,
                    world_name,
                };
                if tx.send(msg).await.is_err() {
                    tracing::warn!("Failed to send SpaceData to BaseApp (channel closed)");
                    break;
                }
            }
            // Instanced worlds have no startup space, so `SpaceData` alone
            // does not tell the base that a player can be delivered to
            // Castle_CellBlock or SGC_W1. Character creation needs to know
            // (Class Start v6 lock L3: a start world with no space refuses).
            let worlds = space_mgr.enterable_worlds();
            if tx
                .send(CellToBaseMsg::EnterableWorlds { worlds })
                .await
                .is_err()
            {
                tracing::warn!("Failed to send EnterableWorlds to BaseApp (channel closed)");
            }
        }

        // The start profiles (Class Start v6 CS-02): the respawn fallback and
        // the console's `.gotolocation <world>` read the process copy. The
        // base loads them too; whichever starts second reloads the same rows.
        if let Some(pool) = self.db_pool.as_deref() {
            cimmeria_resources::base::start_profiles::load_at_boot(pool).await;
            audit_start_profiles(&space_mgr);
        }

        // The process name book (NT-01). The base loads it too; whichever
        // starts first reads the database and the other returns at once.
        if let Some(pool) = self.db_pool.as_deref() {
            cimmeria_names::load_at_boot(pool).await;
        }

        // Build the content engine — load from DB if available, else fallback
        let engine = content::build_engine(self.db_pool.as_deref()).await;
        let db_pool = self.db_pool.clone();

        // Take ownership of channels for the message processing loop
        let rx = self.base_to_cell_rx.take();
        let tx = self.cell_to_base_tx.clone();
        let mut labels_rx = self.entity_labels_rx.take();

        if let (Some(mut rx), Some(tx)) = (rx, tx) {
            // Stash a shutdown signal so `stop()` can ask the loop to exit
            // without dropping the channel out from under it.
            let shutdown = Arc::new(Notify::new());
            self.shutdown_signal = Some(shutdown.clone());
            let handle = tokio::spawn(async move {
                super::message_loop::run_cell_loop(
                    &mut rx,
                    &mut labels_rx,
                    &tx,
                    space_mgr,
                    engine,
                    db_pool,
                    spawn_records,
                    shutdown,
                )
                .await;
            });
            self.cell_loop_handle = Some(handle);
        } else {
            tracing::warn!("Cell service started without channels — operating in stub mode");
        }

        self.is_running = true;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `CellService::new` must produce a non-running service with no
    /// loop handle. This is the minimal surface we can exercise without
    /// real XML files / DB pools — larger startup tests are deferred to
    /// the integration harness.
    #[test]
    fn new_service_is_not_running() {
        let svc = CellService::new(&cimmeria_common::ServerConfig::default());
        assert!(!svc.is_running, "fresh CellService must not be running");
        assert!(
            svc.cell_loop_handle.is_none(),
            "no loop handle before start()"
        );
        assert!(
            svc.shutdown_signal.is_none(),
            "no shutdown signal before start()"
        );
    }
}
