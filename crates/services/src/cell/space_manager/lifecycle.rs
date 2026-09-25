//! Space lifecycle: ID allocation, creation, lookup, destruction.

use std::collections::{HashMap, HashSet};

use cimmeria_common::SpaceId;
use cimmeria_entity::navigation::NavMesh;
use cimmeria_entity::space::Space;

use super::{SpaceInstance, SpaceManager, GRID_CELL_SIZE};

impl SpaceManager {
    /// Allocate the next space ID using the `(cell_id << 16) | local_index` scheme.
    pub(crate) fn allocate_space_id(&mut self) -> u32 {
        let id = ((self.cell_id as u32) << 16) | self.next_local_id;
        self.next_local_id += 1;
        id
    }

    /// Create a `SpaceInstance` and insert it into the spaces map.
    pub(crate) fn create_space_instance(&mut self, space_id: u32, world_name: &str) {
        let space = Space::new(
            SpaceId(space_id as i32),
            world_name.to_string(),
            GRID_CELL_SIZE,
        );
        // Try to load navmesh for this space
        let nav_name = world_name.to_lowercase().replace(' ', "_");
        let nav_path = format!("data/spaces/{nav_name}.nav");
        let navmesh = match NavMesh::load(std::path::Path::new(&nav_path)) {
            Ok(nm) => {
                super::movement_telemetry::log_navmesh_loaded(
                    space_id,
                    world_name,
                    nm.fingerprint(),
                );
                Some(nm)
            }
            Err(e) => {
                // Every navmesh consumer fails OPEN without a mesh
                // (`find_path` -> None -> straight-line fallbacks,
                // `has_line_of_sight` / `is_position_valid` -> true), so
                // NPCs in this space path blind through geometry. That is
                // not an "optional" condition, so surface it.
                tracing::warn!(target: "movement.navmesh", space_id, world = %world_name,
                    path = %nav_path, error = %e, reason = "navmesh_missing",
                    "navmesh: no .nav file for space -- NPCs here path in straight lines through geometry and LoS/position checks fail open");
                None
            }
        };
        let instance = SpaceInstance {
            space_id,
            world_name: world_name.to_string(),
            space,
            entities: HashMap::new(),
            players: HashSet::new(),
            navmesh,
        };
        tracing::debug!(space_id, world = %world_name, "Created space instance");
        self.spaces.insert(space_id, instance);
    }

    /// Emit `cover.coverage event=space_summary` for one space: the cover
    /// nodes of its world, how many stand on its navmesh, and how many of
    /// its NPCs would use them. WARN when cover-seeking NPCs have nothing.
    /// No-op until the cover index has loaded.
    pub(crate) fn log_cover_coverage(&self, space_id: u32) {
        if !self.npc_detectors.cover_load_done {
            return;
        }
        let Some(space) = self.spaces.get(&space_id) else {
            return;
        };
        let cover_npcs = space
            .entities
            .values()
            .filter(|e| !e.is_player && e.use_cover && !e.is_stationary)
            .count();
        let coverage = super::super::cover::space_coverage(
            &self.cover,
            self.world_id_for_world(&space.world_name),
            space.navmesh.as_ref(),
            cover_npcs,
        );
        super::super::cover::log_space_coverage(
            space_id,
            &space.world_name,
            &coverage,
            space.navmesh.as_ref().map(|n| n.short_hash()),
        );
    }

    /// The cover service has loaded: summarise every space that exists now
    /// (their NPCs are already spawned). Instanced spaces are summarised
    /// after their own spawn, in `spawn_instance_npcs_from_records`.
    pub(crate) fn cover_loaded(&mut self) {
        self.npc_detectors.cover_load_done = true;
        let mut ids: Vec<u32> = self.spaces.keys().copied().collect();
        ids.sort_unstable();
        for id in ids {
            self.log_cover_coverage(id);
        }
    }

    /// Check if a non-instanced space already exists for a world name.
    ///
    /// Only checks `world_spaces` (non-instanced startup spaces). For instanced
    /// worlds this always returns false because instances are per-player and not
    /// cached in `world_spaces`.
    pub fn has_space_for_world(&self, world_name: &str) -> bool {
        self.world_spaces.contains_key(world_name)
    }

    /// Stamp the `resources.worlds` per-world settings — the numeric
    /// `world_id` and the `navmesh_mode` — onto every matching
    /// [`WorldDef`](super::WorldDef), from a map produced by
    /// [`spawner::load_world_rows`](crate::cell::spawner::load_world_rows).
    ///
    /// Kept as a stamp on the existing world table rather than a parallel
    /// `HashMap<String, WorldRow>`: `self.worlds` is already the canonical
    /// keyed-by-world-name structure, and a second map keyed identically
    /// would be free to drift from it.
    ///
    /// Matching is exact and case-sensitive, like every other keyed lookup
    /// in this module — both sides originate from the same 2009 content
    /// pipeline, so a mismatch is a data bug worth surfacing, not something
    /// to paper over. Both set differences are logged once at startup:
    /// a `spaces.xml` world with no DB row silently disables every
    /// content-engine `world` condition in that world (the condition fails
    /// closed) **and** keeps that world's navmesh containment enforced,
    /// and a DB world with no `spaces.xml` entry can never be loaded as a
    /// space at all.
    ///
    /// A world this never reaches — because the DB was down, or because it
    /// has no row — keeps [`NavmeshMode::Enforce`](super::NavmeshMode),
    /// which is today's behaviour. The failure mode of a missed stamp is
    /// therefore "stricter than intended", never "a movement gate quietly
    /// disappeared".
    pub fn stamp_world_rows(
        &mut self,
        world_rows: &HashMap<String, super::super::spawner::WorldRow>,
    ) {
        let mut stamped = 0usize;
        let mut advisory: Vec<String> = Vec::new();
        // Owned, not `&str`: the borrow would come out of `iter_mut` and
        // block the immutable reads below.
        let mut missing_in_db: Vec<String> = Vec::new();
        for (name, def) in self.worlds.iter_mut() {
            match world_rows.get(name) {
                Some(row) => {
                    def.world_id = Some(row.world_id);
                    def.navmesh_mode = row.navmesh_mode;
                    if row.navmesh_mode == super::NavmeshMode::Advisory {
                        advisory.push(name.clone());
                    }
                    stamped += 1;
                }
                None => missing_in_db.push(name.clone()),
            }
        }

        let missing_in_xml: Vec<&str> = world_rows
            .keys()
            .filter(|name| !self.worlds.contains_key(*name))
            .map(String::as_str)
            .collect();

        advisory.sort();
        tracing::info!(
            target: "movement.navmesh",
            stamped,
            worlds = self.worlds.len(),
            advisory_worlds = ?advisory,
            "Stamped world ids and navmesh modes onto spaces.xml world definitions"
        );
        if !missing_in_db.is_empty() {
            tracing::warn!(
                worlds = ?missing_in_db,
                "spaces.xml worlds have no resources.worlds row — content-engine \
                 `world` conditions will fail closed in these worlds"
            );
        }
        if !missing_in_xml.is_empty() {
            tracing::debug!(
                worlds = ?missing_in_xml,
                "resources.worlds rows have no spaces.xml entry — no space can be created for these"
            );
        }
    }

    /// Check if a world is marked as instanced in spaces.xml.
    pub fn is_world_instanced(&self, world_name: &str) -> bool {
        self.worlds.get(world_name).is_some_and(|w| w.instanced)
    }

    /// Find or create a space for the given world name.
    ///
    /// - Non-instanced worlds: return the existing startup space from `world_spaces`.
    /// - Instanced worlds: always create a NEW space. Each player gets their own
    ///   private instance (Castle_CellBlock, SGC_W1, etc.). The space is NOT cached
    ///   in `world_spaces` — it lives only in `spaces` and is destroyed when the
    ///   last player leaves.
    pub fn find_or_create_space(&mut self, world_name: &str) -> Result<u32, String> {
        // Non-instanced: return the shared startup space
        if let Some(&space_id) = self.world_spaces.get(world_name) {
            return Ok(space_id);
        }

        // Check if we know about this world at all
        let world_def = self
            .worlds
            .get(world_name)
            .ok_or_else(|| format!("Unknown world: {world_name}"))?;

        if !world_def.instanced {
            return Err(format!(
                "Non-instanced world '{world_name}' has no startup space — \
                 it should be listed in cell_spaces.xml"
            ));
        }

        // Instanced: always create a fresh space — do NOT cache in world_spaces
        let space_id = self.allocate_space_id();
        self.create_space_instance(space_id, world_name);

        tracing::info!(space_id, world = %world_name, "Created new instanced space");
        Ok(space_id)
    }

    /// Destroy an instanced space, removing all entities and freeing resources.
    ///
    /// Only called for instanced spaces when the last player leaves. Removes
    /// all NPC entities, the space instance, and any entity_space entries.
    ///
    /// # This is the *other* teardown path
    ///
    /// [`SpaceManager::destroy_entity`] is the per-entity teardown and
    /// releases that entity's per-id side state. Every NPC still resident
    /// when the last player leaves an instance goes away through **this**
    /// function instead, without `destroy_entity` ever running for it — so
    /// any per-id map released only there leaks one slot per NPC per
    /// instance for the process lifetime, and hands a recycled entity id a
    /// predecessor's state. `npc_path_fail_log` did exactly that (PR #700
    /// review): a reused id inherited an open throttle window, silently
    /// swallowing the first path failure of the new occupant — the one row
    /// an incident timeline most needs.
    ///
    /// Anything added to `destroy_entity`'s release block belongs here too.
    pub(crate) fn destroy_space(&mut self, space_id: u32) {
        if let Some(space) = self.spaces.remove(&space_id) {
            let entity_count = space.entities.len();

            // Remove all entity_space entries for entities in this space
            for &eid in space.entities.keys() {
                self.entity_space.remove(&eid);
                self.movement_telemetry.forget(eid);
                self.movement_validator.forget(eid);
                // Missing here until NA02: `destroy_entity` released it,
                // this path did not.
                self.zero_health_npc_log.forget(eid);
                self.npc_detectors.forget(eid);
            }
            self.npc_detectors.forget_world(&space.world_name);

            tracing::info!(
                space_id,
                world = %space.world_name,
                entities_removed = entity_count,
                "Destroyed instanced space (last player left)"
            );
        }
    }
}
