//! Uniform-grid spatial index over loaded cover nodes.
//!
//! Built once at startup from the `Vec<CoverNode>` returned by the loader.
//! Indexes are integer cell coordinates `(floor(x / GRID_CELL), floor(z /
//! GRID_CELL))`; a query asks for all cells touching a radius around a
//! position and linearly checks each candidate's actual distance.
//!
//! 9,346 nodes across the corpus is small enough that a fancier structure
//! (KD-tree, R-tree) buys little — the grid is simpler, faster to build
//! (single pass), and has predictable memory.
//!
//! The grid cell size is tuned for typical NPC cover-search radii
//! (≈ 30 m), giving an expected 2–9 cell scans per query.

use cimmeria_common::Vector3;
use std::collections::HashMap;

use super::types::{CoverNode, CoverSlotKey};

/// Grid cell edge length in BigWorld meters. Tuned to roughly match the
/// NPC cover-search radius (the scorer prefers nodes within ~30 m).
const GRID_CELL: f32 = 16.0;

/// 2D integer grid cell coordinate.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct CellCoord {
    x: i32,
    z: i32,
}

impl CellCoord {
    fn from_pos(pos: &Vector3) -> Self {
        Self {
            x: (pos.x / GRID_CELL).floor() as i32,
            z: (pos.z / GRID_CELL).floor() as i32,
        }
    }
}

/// Spatial index of loaded cover nodes. Read-only after `build`.
#[derive(Debug)]
pub struct CoverIndex {
    nodes: Vec<CoverNode>,
    grid: HashMap<CellCoord, Vec<usize>>,
}

impl CoverIndex {
    /// Build an empty index. Used in tests and when the DB has no cover data.
    pub fn empty() -> Self {
        Self {
            nodes: Vec::new(),
            grid: HashMap::new(),
        }
    }

    /// Build a fresh index from `nodes`. Single pass over the slice.
    pub fn build(nodes: Vec<CoverNode>) -> Self {
        let mut grid: HashMap<CellCoord, Vec<usize>> = HashMap::new();
        for (i, n) in nodes.iter().enumerate() {
            grid.entry(CellCoord::from_pos(&n.pos)).or_default().push(i);
        }
        Self { nodes, grid }
    }

    /// Total node count.
    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }

    /// Get a node by its index in the underlying `nodes` slice. Useful
    /// when the caller has already resolved a slot via `nearby`.
    pub fn node(&self, idx: usize) -> Option<&CoverNode> {
        self.nodes.get(idx)
    }

    /// All loaded nodes (read-only). Prefer `nearby` for spatial queries.
    pub fn all_nodes(&self) -> &[CoverNode] {
        &self.nodes
    }

    /// Lookup a node by its `(chunk_id, node_id)` key. Linear scan — only
    /// used on slow paths (reservation release, content-trigger payloads).
    /// Returns `None` if the slot key isn't loaded.
    pub fn node_by_key(&self, key: CoverSlotKey) -> Option<&CoverNode> {
        self.nodes
            .iter()
            .find(|n| n.chunk_id == key.chunk_id && n.node_id == key.node_id)
    }

    /// Return every node within `radius` meters of `pos`, sorted by
    /// ascending distance. Returns indices into the underlying nodes
    /// slice; callers can dereference with [`Self::node`].
    ///
    /// Excludes nodes by Y-axis if `max_y_diff` is `Some` — useful for
    /// filtering out cover on different floors of a multi-level chunk.
    pub fn nearby(&self, pos: &Vector3, radius: f32, max_y_diff: Option<f32>) -> Vec<usize> {
        let mut hits: Vec<(usize, f32)> = Vec::new();
        if radius <= 0.0 {
            return Vec::new();
        }
        let radius_sq = radius * radius;
        let cell_radius = (radius / GRID_CELL).ceil() as i32;
        let center = CellCoord::from_pos(pos);

        for dx in -cell_radius..=cell_radius {
            for dz in -cell_radius..=cell_radius {
                let cell = CellCoord {
                    x: center.x + dx,
                    z: center.z + dz,
                };
                let Some(bucket) = self.grid.get(&cell) else {
                    continue;
                };
                for &idx in bucket {
                    let n = &self.nodes[idx];
                    if let Some(max_dy) = max_y_diff {
                        if (n.pos.y - pos.y).abs() > max_dy {
                            continue;
                        }
                    }
                    let dist_sq = n.pos.distance_squared_to(pos);
                    if dist_sq <= radius_sq {
                        hits.push((idx, dist_sq));
                    }
                }
            }
        }

        hits.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));
        hits.into_iter().map(|(idx, _)| idx).collect()
    }

    /// Return every node belonging to a specific `chunk_id`. Used by the
    /// squad-affinity term in the scorer (penalize cover in chunks already
    /// occupied by allied NPCs).
    pub fn nodes_in_chunk(&self, chunk_id: i32) -> impl Iterator<Item = (usize, &CoverNode)> {
        self.nodes
            .iter()
            .enumerate()
            .filter(move |(_, n)| n.chunk_id == chunk_id)
    }
}
