//! NavMesh-backed spatial queries: line-of-sight, pathfinding, position
//! validation, and surface-height sampling.
//!
//! All queries are scoped to the space containing the requested entity. If
//! the space has no navmesh loaded, LoS / validity calls conservatively
//! return `true` (no obstruction) and pathfinding / height return `None`.

use cimmeria_common::Vector3;
use cimmeria_entity::navigation::PointVerdict;

use cimmeria_entity::navigation::{LineOfSight, PathOutcome};

use super::SpaceManager;

impl SpaceManager {
    /// Check line-of-sight between two entities in the same space.
    ///
    /// `true` unless the navmesh positively reports a boundary between them.
    /// No navmesh, an entity that is not in the space, and an endpoint the
    /// mesh does not cover all count as clear: see [`Self::line_of_sight`].
    pub fn has_line_of_sight(&self, entity_a: u32, entity_b: u32) -> bool {
        self.line_of_sight(entity_a, entity_b).is_clear_or_unknown()
    }

    /// Three-state line of sight between two entities in the same space.
    ///
    /// [`LineOfSight::Unknown`] covers every case where the navmesh cannot
    /// answer: no space, no navmesh loaded, an entity missing from the space,
    /// or an endpoint further from the mesh than the projection box reaches.
    /// Every answer that is not clear is reported through the sampled
    /// `npc_ai.los` row (one per pair per 5 s), with the ray endpoints: an
    /// unknown on a meshed world means a spawn or a player is standing
    /// somewhere the mesh does not cover (9 of the 13 stationary Harset mobs
    /// against `harset.nav`), and that used to read as "blocked" and silence
    /// the NPC for good.
    pub fn line_of_sight(&self, entity_a: u32, entity_b: u32) -> LineOfSight {
        let Some(space) = self
            .entity_space
            .get(&entity_a)
            .and_then(|sid| self.spaces.get(sid))
        else {
            return LineOfSight::Unknown;
        };
        let Some(navmesh) = &space.navmesh else {
            return LineOfSight::Unknown;
        };
        let (Some(a), Some(b)) = (space.entities.get(&entity_a), space.entities.get(&entity_b))
        else {
            return LineOfSight::Unknown;
        };
        let probe = navmesh.line_of_sight_probe(&a.position, &b.position);
        crate::cell::service::npc_ai::detectors::los::report(
            self,
            entity_a,
            entity_b,
            a.position,
            b.position,
            &probe,
            Some(navmesh.short_hash()),
            std::time::Instant::now(),
        );
        probe.result
    }

    /// Whether a fighting NPC may fire at `target` this tick, as far as line
    /// of sight goes.
    ///
    /// A mobile attacker gets [`Self::has_line_of_sight`]. A stationary one
    /// gets [`LineOfSight::permits_stationary_attack`]: a navmesh `Blocked`
    /// on the NPC's own storey does not stop it firing, because the mesh
    /// cannot see over furniture and the NPC cannot walk around it (NA16,
    /// audit S11: the Find Ambernol drone and the med-station desk).
    pub fn attack_line_of_sight(&self, npc_id: u32, target_id: u32, is_stationary: bool) -> bool {
        let los = self.line_of_sight(npc_id, target_id);
        if !is_stationary {
            return los.is_clear_or_unknown();
        }
        let dy = match (self.get_entity(npc_id), self.get_entity(target_id)) {
            (Some(npc), Some(target)) => target.position.y - npc.position.y,
            // `line_of_sight` already answered Unknown for a missing entity.
            _ => 0.0,
        };
        los.permits_stationary_attack(dy)
    }

    /// Whether the space containing `entity_id` has a navmesh loaded.
    ///
    /// `has_line_of_sight`, `find_path` and `is_position_valid` all fail open
    /// without one, so callers that report those results need this to tell
    /// "clear" from "unknown".
    pub fn space_has_navmesh(&self, entity_id: u32) -> bool {
        self.entity_space
            .get(&entity_id)
            .and_then(|sid| self.spaces.get(sid))
            .is_some_and(|s| s.navmesh.is_some())
    }

    /// Find a path between two positions within the space containing `entity_id`.
    /// Returns waypoints or `None` if no path exists or no navmesh is loaded.
    /// A partial corridor is returned as a path, as it always was — use
    /// [`Self::find_path_outcome`] to see which it was.
    pub fn find_path(
        &self,
        entity_id: u32,
        start: &Vector3,
        end: &Vector3,
    ) -> Option<Vec<Vector3>> {
        self.find_path_outcome(entity_id, start, end)?
            .into_waypoints()
    }

    /// The typed result of a path query: which Detour stage decided, whether
    /// the corridor was partial, and how far each end snapped. `None` when
    /// the entity is in no space or the space has no navmesh.
    pub fn find_path_outcome(
        &self,
        entity_id: u32,
        start: &Vector3,
        end: &Vector3,
    ) -> Option<PathOutcome> {
        let space_id = self.entity_space.get(&entity_id)?;
        let space = self.spaces.get(space_id)?;
        let navmesh = space.navmesh.as_ref()?;
        Some(navmesh.find_path(start, end))
    }

    /// Check if a position is on walkable navmesh in the space containing `entity_id`.
    pub fn is_position_valid(&self, entity_id: u32, pos: &Vector3) -> bool {
        let space_id = match self.entity_space.get(&entity_id) {
            Some(&sid) => sid,
            None => return true,
        };
        let space = match self.spaces.get(&space_id) {
            Some(s) => s,
            None => return true,
        };
        match &space.navmesh {
            Some(nm) => nm.is_point_valid(pos),
            None => true,
        }
    }

    /// Why [`Self::is_position_valid`] answered the way it did, for the
    /// space containing `entity_id`.
    ///
    /// `None` means there is **no navmesh in this space** — which is not
    /// the same as "the point is off the mesh". `is_position_valid` fails
    /// open there and returns `true`, so a caller logging a diagnosis has
    /// to be able to say "there was nothing to check against" rather than
    /// reporting a gate it never evaluated.
    pub fn diagnose_point(&self, entity_id: u32, pos: &Vector3) -> Option<PointVerdict> {
        let space_id = *self.entity_space.get(&entity_id)?;
        let space = self.spaces.get(&space_id)?;
        Some(space.navmesh.as_ref()?.diagnose_point(pos))
    }

    /// Short content hash of the navmesh loaded for the space containing
    /// `entity_id`, or `None` in a meshless space.
    ///
    /// Every navmesh-decision log line carries this so a session can be
    /// tied to the mesh build it ran on — see
    /// [`cimmeria_entity::navigation::NavMeshFingerprint`].
    pub fn navmesh_short_hash(&self, entity_id: u32) -> Option<&str> {
        let space_id = *self.entity_space.get(&entity_id)?;
        let space = self.spaces.get(&space_id)?;
        Some(space.navmesh.as_ref()?.short_hash())
    }

    /// Sample the navmesh surface height under `(x, z)` on the storey nearest
    /// `y_ref`, in the space containing `entity_id`.
    ///
    /// Returns `None` if no navmesh is loaded, or if no walkable surface lies
    /// within the jump tolerance of `y_ref` — see
    /// [`cimmeria_entity::navigation::NavMesh::get_height_near`]. A `None`
    /// for a loaded mesh can mean the entity is floating, not that it is
    /// off-mesh.
    pub fn get_navmesh_height(&self, entity_id: u32, x: f32, y_ref: f32, z: f32) -> Option<f32> {
        let space_id = self.entity_space.get(&entity_id)?;
        let space = self.spaces.get(space_id)?;
        let navmesh = space.navmesh.as_ref()?;
        navmesh.get_height_near(x, y_ref, z)
    }

    /// `pos` moved onto the nearest navmesh polygon within Detour's
    /// destination box (±3 on every axis), in the space containing
    /// `entity_id`.
    ///
    /// For endpoints that come from content or seeds rather than from the
    /// pathfinder: patrol waypoints, investigate POIs, wander candidates. A
    /// raw endpoint an NPC can never stand on keeps the "arrived?" check
    /// false forever, and a straight-line fallback toward it leaves the NPC
    /// hovering or buried at the end. The ±3 box stays under half the
    /// smallest storey gap on a shipped mesh (~7.9 u on `castle_cellblock`),
    /// so it cannot move a point onto another floor.
    ///
    /// `None` when no navmesh is loaded or no polygon is in the box; callers
    /// keep the raw point then.
    pub fn snap_to_navmesh(&self, entity_id: u32, pos: &Vector3) -> Option<Vector3> {
        let space_id = self.entity_space.get(&entity_id)?;
        let space = self.spaces.get(space_id)?;
        let navmesh = space.navmesh.as_ref()?;
        navmesh.find_nearest_poly(pos).map(|(_, p)| p)
    }

    /// Slide from `from` toward `to` along the walkable surface, stopping at
    /// walls, and return the grounded end point. See
    /// [`cimmeria_entity::navigation::NavMesh::move_along_surface`].
    ///
    /// `None` when no navmesh is loaded or `from` is not on it.
    pub fn move_along_navmesh(
        &self,
        entity_id: u32,
        from: &Vector3,
        to: &Vector3,
    ) -> Option<Vector3> {
        let space_id = self.entity_space.get(&entity_id)?;
        let space = self.spaces.get(space_id)?;
        let navmesh = space.navmesh.as_ref()?;
        navmesh.move_along_surface(from, to)
    }
}
