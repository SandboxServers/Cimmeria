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
//! Both ends of the segment are raised by [`eye_height`]. No per-being-type
//! eye height has been recovered from the client data, so every entity uses
//! [`DEFAULT_EYE_HEIGHT`].
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

/// Eye height above an entity's position (its feet), metres.
pub const DEFAULT_EYE_HEIGHT: f32 = 1.5;

/// Pages within this distance (XZ, metres) of a player stay unpacked: the
/// AoI radius (100 m, `CellEntity::aoi_radius`) plus a margin, so every NPC
/// that can see a player has its pages resident.
pub const RESIDENCY_RADIUS: f32 = 132.0;

/// The eye height of `e`. One value for every being today; see the module
/// docs.
pub fn eye_height(_e: &CellEntity) -> f32 {
    DEFAULT_EYE_HEIGHT
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
    /// The occluder for `world_name` (by the `.nav` file-name rule: lower
    /// case, spaces as underscores), loaded on first use and cached,
    /// including a miss, so an instanced world is read once.
    pub(crate) fn occluder_for_world(&mut self, world_name: &str) -> Option<Arc<PagedOccluder>> {
        let key = world_name.to_lowercase().replace(' ', "_");
        if let Some(cached) = self.occluders.get(&key) {
            return cached.clone();
        }
        let path = format!("data/spaces/{key}.occ");
        let loaded = load_occluder(Path::new(&path), world_name);
        self.occluders.insert(key, loaded.clone());
        loaded
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
            let key = space.world_name.to_lowercase().replace(' ', "_");
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

/// Read one `.occ` file. Absent is normal (the world keeps the navmesh
/// ray); unreadable is a WARN, because a shipped file that fails to load
/// silently changes every NPC's line of sight in that world.
fn load_occluder(path: &Path, world_name: &str) -> Option<Arc<PagedOccluder>> {
    if !path.exists() {
        tracing::info!(target: "npc_ai.occluder", world = %world_name, path = %path.display(),
            event = "occluder_absent",
            "occluder: no .occ for this world -- line of sight uses the navmesh ray");
        return None;
    }
    let started = std::time::Instant::now();
    match PagedOccluder::load(path) {
        Ok(occ) => {
            let st = occ.stats();
            tracing::info!(target: "npc_ai.occluder", world = %world_name, path = %path.display(),
                event = "occluder_loaded", occluder_hash = occ.short_hash(),
                pages = st.pages, packed_bytes = st.packed_bytes,
                full_ram_bytes = occ.full_ram_bytes(),
                load_ms = started.elapsed().as_millis() as u64,
                label = occ.label(),
                "occluder: loaded collision-geometry line of sight (pages stay packed until a player is near)");
            Some(Arc::new(occ))
        }
        Err(e) => {
            tracing::warn!(target: "npc_ai.occluder", world = %world_name, path = %path.display(),
                event = "occluder_load_failed", error = %e,
                "occluder: .occ failed to load -- line of sight falls back to the navmesh ray");
            None
        }
    }
}
