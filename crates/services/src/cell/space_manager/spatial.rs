//! NavMesh-backed spatial queries: line-of-sight, pathfinding, position
//! validation, and surface-height sampling.
//!
//! All queries are scoped to the space containing the requested entity. If
//! the space has no navmesh loaded, LoS / validity calls conservatively
//! return `true` (no obstruction) and pathfinding / height return `None`.

use cimmeria_common::Vector3;
use cimmeria_entity::navigation::PointVerdict;

use cimmeria_entity::navigation::LineOfSight;

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
    /// The last case is logged at debug, because on a meshed world it means a
    /// spawn or a player is standing somewhere the mesh does not cover (9 of
    /// the 13 stationary Harset mobs against `harset.nav`), and that used to
    /// read as "blocked" and silence the NPC for good.
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
        let los = navmesh.line_of_sight(&a.position, &b.position);
        if los == LineOfSight::Unknown {
            tracing::debug!(
                target: "movement.navmesh",
                reason = "los_unknown_off_mesh",
                entity_a,
                entity_b,
                a_on_mesh = navmesh.is_point_valid(&a.position),
                b_on_mesh = navmesh.is_point_valid(&b.position),
                "line of sight: an endpoint is outside navmesh coverage -- treated as clear"
            );
        }
        los
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
    pub fn find_path(
        &self,
        entity_id: u32,
        start: &Vector3,
        end: &Vector3,
    ) -> Option<Vec<Vector3>> {
        let space_id = self.entity_space.get(&entity_id)?;
        let space = self.spaces.get(space_id)?;
        let navmesh = space.navmesh.as_ref()?;
        navmesh.find_path(start, end)
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
}
