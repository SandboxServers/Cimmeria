//! NavMesh-backed spatial queries: line-of-sight, pathfinding, position
//! validation, and surface-height sampling.
//!
//! All queries are scoped to the space containing the requested entity. If
//! the space has no navmesh loaded, LoS / validity calls conservatively
//! return `true` (no obstruction) and pathfinding / height return `None`.

use cimmeria_common::Vector3;

use super::SpaceManager;

impl SpaceManager {
    /// Line-of-sight check between two entities using the navmesh.
    /// Returns `true` if there is clear LoS (or if no navmesh is loaded).
    pub fn has_line_of_sight(&self, entity_a: u32, entity_b: u32) -> bool {
        let space_id = match self.entity_space.get(&entity_a) {
            Some(&sid) => sid,
            None => return true, // No space info — assume LoS
        };
        let space = match self.spaces.get(&space_id) {
            Some(s) => s,
            None => return true,
        };
        let navmesh = match &space.navmesh {
            Some(nm) => nm,
            None => return true, // No navmesh — can't check, assume LoS
        };
        let pos_a = match space.entities.get(&entity_a) {
            Some(e) => e.position,
            None => return true,
        };
        let pos_b = match space.entities.get(&entity_b) {
            Some(e) => e.position,
            None => return true,
        };
        navmesh.raycast(&pos_a, &pos_b)
    }

    /// Find a path between two positions within the space containing `entity_id`.
    /// Returns waypoints or `None` if no path exists or no navmesh is loaded.
    pub fn find_path(&self, entity_id: u32, start: &Vector3, end: &Vector3) -> Option<Vec<Vector3>> {
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

    /// Sample the navmesh surface height at (x, z) in the space containing `entity_id`.
    /// Returns `None` if no navmesh is loaded or the point is off-mesh.
    pub fn get_navmesh_height(&self, entity_id: u32, x: f32, z: f32) -> Option<f32> {
        let space_id = self.entity_space.get(&entity_id)?;
        let space = self.spaces.get(space_id)?;
        let navmesh = space.navmesh.as_ref()?;
        navmesh.get_height_at(x, z)
    }
}
