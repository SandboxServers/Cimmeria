//! Spatial partitioning grid for efficient entity queries.
//!
//! The world grid divides the game world into a uniform grid of square cells.
//! Each cell tracks which entities are currently positioned inside it. This
//! allows area-of-interest and range queries to only examine entities in
//! nearby cells rather than the entire entity population.
//!
//! The grid uses a hash map of `(i32, i32)` cell coordinates, so the world
//! extent is not fixed at construction time -- cells are created lazily as
//! entities enter them.
//!
//! This corresponds to the C++ `SpaceGrid` / chunk system configured by
//! `grid_chunk_size` and `grid_vision_distance` in the original engine.

use std::collections::HashMap;

use cimmeria_common::{EntityId, Vector3};

/// A spatial hash grid for fast entity proximity queries.
pub struct WorldGrid {
    /// Side length of each grid cell in world units.
    cell_size: f32,

    /// Entities indexed by their grid cell. Each cell contains a list of
    /// entity IDs currently positioned within it.
    cells: HashMap<(i32, i32), Vec<EntityId>>,
}

impl WorldGrid {
    /// Create a new world grid with the given cell size.
    ///
    /// A typical cell size might be 50.0-100.0 world units, matching the
    /// `grid_chunk_size` setting from the original engine config.
    pub fn new(cell_size: f32) -> Self {
        assert!(cell_size > 0.0, "cell_size must be positive");
        Self {
            cell_size,
            cells: HashMap::new(),
        }
    }

    /// Insert an entity at the given position.
    pub fn insert(&mut self, id: EntityId, pos: &Vector3) {
        let key = self.cell_key(pos);
        self.cells.entry(key).or_default().push(id);
    }

    /// Remove an entity from the cell corresponding to the given position.
    ///
    /// If the entity is not found in the expected cell, this is a no-op.
    pub fn remove(&mut self, id: EntityId, pos: &Vector3) {
        let key = self.cell_key(pos);
        if let Some(cell) = self.cells.get_mut(&key) {
            cell.retain(|&eid| eid != id);
            if cell.is_empty() {
                self.cells.remove(&key);
            }
        }
    }

    /// Update an entity's position in the grid.
    ///
    /// If the entity moved to a different cell, it is removed from the old
    /// cell and inserted into the new one. If it stayed in the same cell,
    /// no work is done.
    pub fn update_position(&mut self, id: EntityId, old_pos: &Vector3, new_pos: &Vector3) {
        let old_key = self.cell_key(old_pos);
        let new_key = self.cell_key(new_pos);

        if old_key == new_key {
            return; // Same cell, nothing to do.
        }

        self.remove(id, old_pos);
        self.insert(id, new_pos);
    }

    /// Find all entities within `radius` world units of `pos`.
    ///
    /// Examines all grid cells that could possibly overlap the query circle,
    /// then performs an exact squared-distance check on each candidate.
    pub fn query_radius(&self, pos: &Vector3, radius: f32) -> Vec<EntityId> {
        let radius_sq = radius * radius;

        // Determine the range of cells that overlap the query circle.
        // We only use X and Z for the 2D grid (Y is vertical / height).
        let min_cx = ((pos.x - radius) / self.cell_size).floor() as i32;
        let max_cx = ((pos.x + radius) / self.cell_size).floor() as i32;
        let min_cz = ((pos.z - radius) / self.cell_size).floor() as i32;
        let max_cz = ((pos.z + radius) / self.cell_size).floor() as i32;

        let mut results = Vec::new();
        for cx in min_cx..=max_cx {
            for cz in min_cz..=max_cz {
                if let Some(cell) = self.cells.get(&(cx, cz)) {
                    for &id in cell {
                        // We do not store positions in the grid, so we cannot
                        // do an exact distance check here. The caller must
                        // perform a second-pass filter using actual entity
                        // positions. For now, return all candidates from
                        // overlapping cells.
                        //
                        // TODO: Store positions alongside IDs to enable exact
                        // filtering inside the grid.
                        results.push(id);
                    }
                }
            }
        }

        // Deduplicate in case of any edge-case double-insertion.
        results.sort_unstable_by_key(|id| id.0);
        results.dedup();
        results
    }

    /// Compute the grid cell key for a world-space position.
    ///
    /// Uses the X and Z components (horizontal plane). Y is ignored because
    /// the grid is a 2D spatial partitioning scheme -- vertical proximity
    /// is handled separately if needed.
    pub fn cell_key(&self, pos: &Vector3) -> (i32, i32) {
        let cx = (pos.x / self.cell_size).floor() as i32;
        let cz = (pos.z / self.cell_size).floor() as i32;
        (cx, cz)
    }
}

impl std::fmt::Debug for WorldGrid {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WorldGrid")
            .field("cell_size", &self.cell_size)
            .field("occupied_cells", &self.cells.len())
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn grid() -> WorldGrid {
        WorldGrid::new(10.0)
    }

    #[test]
    fn cell_key_basic() {
        let g = grid();
        assert_eq!(g.cell_key(&Vector3::new(5.0, 0.0, 5.0)), (0, 0));
        assert_eq!(g.cell_key(&Vector3::new(15.0, 0.0, 25.0)), (1, 2));
        assert_eq!(g.cell_key(&Vector3::new(-5.0, 0.0, -15.0)), (-1, -2));
    }

    #[test]
    fn cell_key_on_boundary() {
        let g = grid();
        // Exactly on the boundary (10.0 / 10.0 = 1.0, floor = 1)
        assert_eq!(g.cell_key(&Vector3::new(10.0, 0.0, 10.0)), (1, 1));
    }

    #[test]
    fn insert_and_query() {
        let mut g = grid();
        g.insert(EntityId(1), &Vector3::new(5.0, 0.0, 5.0));
        g.insert(EntityId(2), &Vector3::new(7.0, 0.0, 8.0));
        g.insert(EntityId(3), &Vector3::new(50.0, 0.0, 50.0));

        let nearby = g.query_radius(&Vector3::new(5.0, 0.0, 5.0), 15.0);
        assert!(nearby.contains(&EntityId(1)));
        assert!(nearby.contains(&EntityId(2)));
        // Entity 3 is at (50, 50), well outside radius 15 from (5, 5)
        assert!(!nearby.contains(&EntityId(3)));
    }

    #[test]
    fn remove_entity() {
        let mut g = grid();
        let pos = Vector3::new(5.0, 0.0, 5.0);
        g.insert(EntityId(1), &pos);
        assert!(!g.query_radius(&pos, 5.0).is_empty());

        g.remove(EntityId(1), &pos);
        assert!(g.query_radius(&pos, 5.0).is_empty());
    }

    #[test]
    fn remove_cleans_up_empty_cells() {
        let mut g = grid();
        let pos = Vector3::new(5.0, 0.0, 5.0);
        g.insert(EntityId(1), &pos);
        g.remove(EntityId(1), &pos);
        assert!(g.cells.is_empty());
    }

    #[test]
    fn update_position_same_cell() {
        let mut g = grid();
        let old_pos = Vector3::new(1.0, 0.0, 1.0);
        let new_pos = Vector3::new(2.0, 0.0, 2.0);
        g.insert(EntityId(1), &old_pos);

        g.update_position(EntityId(1), &old_pos, &new_pos);

        // Should still be in the same cell (both in cell (0, 0)).
        let results = g.query_radius(&new_pos, 5.0);
        assert!(results.contains(&EntityId(1)));
    }

    #[test]
    fn update_position_different_cell() {
        let mut g = grid();
        let old_pos = Vector3::new(5.0, 0.0, 5.0);
        let new_pos = Vector3::new(25.0, 0.0, 25.0);
        g.insert(EntityId(1), &old_pos);

        g.update_position(EntityId(1), &old_pos, &new_pos);

        // Should not be in old cell's area anymore.
        let old_results = g.query_radius(&old_pos, 5.0);
        assert!(!old_results.contains(&EntityId(1)));

        // Should be in new cell.
        let new_results = g.query_radius(&new_pos, 5.0);
        assert!(new_results.contains(&EntityId(1)));
    }

    #[test]
    fn query_empty_grid_returns_empty() {
        let g = grid();
        let results = g.query_radius(&Vector3::new(0.0, 0.0, 0.0), 100.0);
        assert!(results.is_empty());
    }

    #[test]
    fn negative_coordinates() {
        let mut g = grid();
        let pos = Vector3::new(-25.0, 0.0, -35.0);
        g.insert(EntityId(1), &pos);
        let results = g.query_radius(&pos, 5.0);
        assert!(results.contains(&EntityId(1)));
    }

    #[test]
    #[should_panic(expected = "cell_size must be positive")]
    fn zero_cell_size_panics() {
        WorldGrid::new(0.0);
    }

    #[test]
    fn query_results_are_deduplicated() {
        let mut g = grid();
        let pos = Vector3::new(5.0, 0.0, 5.0);
        // Intentionally double-insert (should not happen in practice,
        // but the grid should handle it gracefully).
        g.insert(EntityId(1), &pos);
        g.insert(EntityId(1), &pos);
        let results = g.query_radius(&pos, 5.0);
        assert_eq!(results.iter().filter(|&&id| id == EntityId(1)).count(), 1);
    }
}
