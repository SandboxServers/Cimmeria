//! Collision-geometry line of sight (NPC AI restoration NA27, #784,
//! decision D-NA13).
//!
//! A world that ships `data/spaces/<world>.occ` answers line of sight from
//! its collision geometry (a column grid of solid spans plus an exact
//! terrain heightfield, built by `occluder_extract`) instead of the navmesh
//! raycast. The navmesh ray reads every desk, table and cover prop as a
//! wall, because Recast cuts obstacles out as holes with no height: on the
//! Castle_CellBlock sweep 38% of its `Blocked` verdicts were false, and on
//! Castle it saw through walls on 23% of the blocked pairs. The occluder
//! had no false clears and about 1% false blocks on both. So where an
//! occluder exists it replaces the ray:
//!
//! - aggro and assist (`npc_ai::aggro_gates::same_room`): `Unknown` (an
//!   endpoint off the occluder's grid, i.e. outside the explorable area it
//!   was trimmed to) fails closed, D-NA08;
//! - the attack check ([`super::AttackLosPolicy::Occluder`]): the verdict
//!   as is, with no stationary relaxation (D-NA11 is retired here). An
//!   `Unknown` falls back to the navmesh rules, as in a world without one;
//! - the cover sight: an NPC at a cover slot looks from its own eyes, over
//!   the prop. The navmesh peek point (D-NA12) is the rule only where no
//!   occluder exists.
//!
//! Both ends of the segment are raised by the entity's eye height
//! ([`SpaceManager::eye_height_of`], NA31): its body set's
//! `resources.body_sets.eye_height`, measured from the reference skeletal
//! mesh in the cooked client package (a human male 1.81 m, a Jaffa male
//! 2.12 m, an Asgard 1.25 m, a rat 0.15 m), or [`DEFAULT_EYE_HEIGHT`] for an
//! entity with no measured body set. The client's own pawn defaults are no
//! help here: `SGWGamePawn` inherits stock UE3 `BaseEyeHeight` 64 and
//! `CollisionHeight` 78 for every being. See
//! `docs/reverse-engineering/findings/being-eye-heights.md`.
//!
//! **Paging.** The file is a table of 64 m pages, each compressed. It is
//! loaded once per world and shared by every instance of it
//! ([`SpaceManager::occluder_for_world`]); Castle_CellBlock is instanced per
//! player. [`SpaceManager::refresh_occluder_residency`] runs at 1 Hz: every
//! page within [`RESIDENCY_RADIUS`] of a player is unpacked, every other page
//! is dropped. A query that meets a packed page unpacks it on the spot (it
//! takes about a millisecond), and the next refresh drops it again if no
//! player is near.

use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;

use cimmeria_common::Vector3;
use cimmeria_entity::cell_entity::CellEntity;
use cimmeria_entity::navigation::{LineOfSight, LosProbe};
use cimmeria_occluder::{PagedOccluder, Sight};

use super::SpaceManager;

/// Eye height above an entity's position (its feet), metres, for an entity
/// whose body set has no measured eye height (props, terminals, a body set
/// with no reference mesh, and every test entity without a body set).
pub const DEFAULT_EYE_HEIGHT: f32 = 1.5;

/// Pages within this distance (XZ, metres) of a player stay unpacked: an
/// NPC's perception radius (its `CellEntity::aoi_radius`, the 100 m default)
/// plus a margin, so every NPC that can see a player has its pages resident.
/// A player's own, wider view radius (`PLAYER_AOI_RADIUS`) does not change
/// this: being in a player's view is a distance test, not a line-of-sight
/// probe.
pub const RESIDENCY_RADIUS: f32 = 132.0;

/// The eye height for `body_set` in `table` (body set to metres), or
/// [`DEFAULT_EYE_HEIGHT`] when the entity has no body set, the body set has
/// no row, or the value is not a positive finite number.
pub fn eye_height_for(body_set: Option<&str>, table: &HashMap<String, f32>) -> f32 {
    body_set
        .and_then(|b| table.get(b))
        .copied()
        .filter(|h| h.is_finite() && *h > 0.0)
        .unwrap_or(DEFAULT_EYE_HEIGHT)
}

/// The occluder's answer for the segment between two eyes, as the
/// three-state [`LineOfSight`] the rest of the cell speaks: off the grid is
/// `Unknown`. `from` / `to` are the eye points, `hit` where it was blocked.
pub fn occluder_probe(
    occ: &PagedOccluder,
    a: Vector3,
    a_eye: f32,
    b: Vector3,
    b_eye: f32,
) -> LosProbe {
    let from = [a.x, a.y + a_eye, a.z];
    let to = [b.x, b.y + b_eye, b.z];
    let (result, hit) = match occ.sight(from, to) {
        Sight::Clear => (LineOfSight::Clear, None),
        Sight::Blocked { at, .. } => (LineOfSight::Blocked, Some(at)),
        Sight::OffGrid => (LineOfSight::Unknown, None),
    };
    LosProbe {
        result,
        from: Some(from),
        to: Some(to),
        hit,
    }
}

/// What a residency refresh last reported for one world, for the gauges.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ResidencyGauge {
    pub pages: i64,
    pub bytes: i64,
}

impl SpaceManager {
    /// The eye height of `e`, metres above its position: its body set's
    /// measured value, else [`DEFAULT_EYE_HEIGHT`] (see the module docs).
    pub fn eye_height_of(&self, e: &CellEntity) -> f32 {
        eye_height_for(e.body_set.as_deref(), &self.body_set_eye_heights)
    }

    /// The occluder for `world_name`: its own `.occ`, else its client map's
    /// (D-DA5, see `space_files`), loaded on first use and cached, including
    /// a miss, so an instanced world is read once. The cache is keyed by the
    /// **file**, so two worlds on one map (`DebugArea` and
    /// `Ihpet_Crater_Light`) share one occluder and one residency set.
    pub(crate) fn occluder_for_world(&mut self, world_name: &str) -> Option<Arc<PagedOccluder>> {
        let world_key = super::space_files::file_key(world_name);
        let file_key = self.occluder_files.get(&world_key).unwrap_or(&world_key);
        if let Some(cached) = self.occluders.get(file_key) {
            return cached.clone();
        }
        let resolved =
            super::space_files::resolve_space_file(&self.space_data_dir, world_name, "occ");
        let (file_key, loaded) = match resolved {
            Ok(file) => {
                let loaded = match self.occluders.get(&file.key) {
                    Some(cached) => cached.clone(),
                    None => load_occluder(&file.path, world_name, file.source.label()),
                };
                (file.key, loaded)
            }
            Err(own_path) => {
                log_occluder_absent(&own_path, world_name);
                (world_key.clone(), None)
            }
        };
        self.occluders.insert(file_key.clone(), loaded.clone());
        self.occluder_files.insert(world_key, file_key);
        loaded
    }

    /// The occluder-cache key a world's spaces count toward: the file it
    /// resolved to, else its own name key (a world the cache was seeded for
    /// directly, as tests do).
    fn occluder_file_key(&self, world_name: &str) -> String {
        let world_key = super::space_files::file_key(world_name);
        self.occluder_files
            .get(&world_key)
            .cloned()
            .unwrap_or(world_key)
    }

    /// The occluder of the space `entity_id` is in.
    pub fn occluder_of(&self, entity_id: u32) -> Option<&PagedOccluder> {
        let space_id = self.entity_space.get(&entity_id)?;
        self.spaces.get(space_id)?.occluder.as_deref()
    }

    /// Whether the space `entity_id` is in has an occluder.
    pub fn space_has_occluder(&self, entity_id: u32) -> bool {
        self.occluder_of(entity_id).is_some()
    }

    /// Whether the space `entity_id` is in has any line-of-sight source (an
    /// occluder or a navmesh). Without one, `Unknown` is the only answer and
    /// means nothing; with one it means an endpoint the source does not
    /// cover.
    pub fn space_has_line_of_sight_source(&self, entity_id: u32) -> bool {
        self.space_has_occluder(entity_id) || self.space_has_navmesh(entity_id)
    }

    /// Keep each loaded occluder's pages near its players unpacked and drop
    /// the rest (1 Hz, from the cell loop). Every instance of a world shares
    /// one occluder, so the players of all its instances count. Emits the
    /// `npc_ai_occluder_resident_pages` / `_bytes` gauges per world and one
    /// `npc_ai.occluder` DEBUG row per world whose residency changed.
    pub fn refresh_occluder_residency(&mut self) {
        let mut players: HashMap<String, Vec<[f32; 2]>> = HashMap::new();
        let mut spaces: HashMap<String, Vec<u32>> = HashMap::new();
        for space in self.spaces.values() {
            if space.occluder.is_none() {
                continue;
            }
            let key = self.occluder_file_key(&space.world_name);
            spaces.entry(key.clone()).or_default().push(space.space_id);
            let pts = players.entry(key).or_default();
            for pid in &space.players {
                if let Some(e) = space.entities.get(pid) {
                    pts.push([e.position.x, e.position.z]);
                }
            }
        }
        let mut keys: Vec<&String> = self
            .occluders
            .iter()
            .filter(|(_, o)| o.is_some())
            .map(|(k, _)| k)
            .collect();
        keys.sort();
        let mut updates = Vec::new();
        for key in keys {
            let Some(occ) = self.occluders.get(key).cloned().flatten() else {
                continue;
            };
            let before = occ.stats();
            let pts = players.get(key).map(Vec::as_slice).unwrap_or(&[]);
            let r = occ.retain_near(pts, RESIDENCY_RADIUS);
            let after = occ.stats();
            let gauge = ResidencyGauge {
                pages: r.resident_pages as i64,
                bytes: r.resident_bytes as i64,
            };
            let query_unpacks = after.query_unpacks - before.query_unpacks;
            if !r.unpacked.is_empty() || !r.evicted.is_empty() || query_unpacks > 0 {
                tracing::debug!(
                    target: "npc_ai.occluder",
                    event = "residency",
                    world = %key,
                    space_ids = ?spaces.get(key).cloned().unwrap_or_default(),
                    players = pts.len(),
                    unpacked = r.unpacked.len(),
                    evicted = r.evicted.len(),
                    unpacked_pages = ?r.unpacked,
                    evicted_pages = ?r.evicted,
                    query_unpacks,
                    resident_pages = r.resident_pages,
                    resident_bytes = r.resident_bytes,
                    packed_bytes = after.packed_bytes,
                    unpack_us_max = after.unpack_us_max,
                    occluder_hash = occ.short_hash(),
                    "occluder: pages unpacked / evicted"
                );
            }
            updates.push((key.clone(), gauge));
        }
        for (key, gauge) in updates {
            let prev = self
                .occluder_residency
                .insert(key.clone(), gauge)
                .unwrap_or_default();
            if prev != gauge {
                cimmeria_observability::gauge_add!(
                    "npc_ai_occluder_resident_pages",
                    gauge.pages - prev.pages,
                    "world" => key.clone(),
                );
                cimmeria_observability::gauge_add!(
                    "npc_ai_occluder_resident_bytes",
                    gauge.bytes - prev.bytes,
                    "world" => key,
                );
            }
        }
    }
}

/// Absent is normal (the world keeps the navmesh ray).
fn log_occluder_absent(path: &Path, world_name: &str) {
    tracing::info!(target: "npc_ai.occluder", world = %world_name,
        world_id = cimmeria_wire::mercury::world_data::known_world_id(world_name),
        path = %path.display(), event = "occluder_absent",
        "occluder: no .occ for this world or its client map -- line of sight uses the navmesh ray");
}

/// Read one `.occ` file. Unreadable is a WARN, because a shipped file that
/// fails to load silently changes every NPC's line of sight in that world.
/// `file_source` is `world` or `client_map` (D-DA5).
fn load_occluder(path: &Path, world_name: &str, file_source: &str) -> Option<Arc<PagedOccluder>> {
    let world_id = cimmeria_wire::mercury::world_data::known_world_id(world_name);
    let started = std::time::Instant::now();
    match PagedOccluder::load(path) {
        Ok(occ) => {
            let st = occ.stats();
            tracing::info!(target: "npc_ai.occluder", world = %world_name, world_id,
                file_source, path = %path.display(),
                event = "occluder_loaded", occluder_hash = occ.short_hash(),
                pages = st.pages, packed_bytes = st.packed_bytes,
                full_ram_bytes = occ.full_ram_bytes(),
                load_ms = started.elapsed().as_millis() as u64,
                label = occ.label(),
                "occluder: loaded collision-geometry line of sight (pages stay packed until a player is near)");
            Some(Arc::new(occ))
        }
        Err(e) => {
            tracing::warn!(target: "npc_ai.occluder", world = %world_name, world_id,
                file_source, path = %path.display(),
                event = "occluder_load_failed", error = %e,
                "occluder: .occ failed to load -- line of sight falls back to the navmesh ray");
            None
        }
    }
}
