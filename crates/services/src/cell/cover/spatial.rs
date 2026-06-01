//! Uniform-grid spatial index over loaded cover nodes.
//!
//! 9,346 nodes across the corpus is small enough that a fancier structure
//! (KD-tree, R-tree) buys little; the grid is simpler and faster to build.

use cimmeria_common::Vector3;
use std::collections::HashMap;

use super::types::{CoverNode, CoverSlotKey};

/// Tuned to roughly match the NPC cover-search radius (~30 m), giving an
/// expected 2–9 cell scans per query.
const GRID_CELL: f32 = 16.0;

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

#[derive(Debug)]
pub struct CoverIndex {
    nodes: Vec<CoverNode>,
    grid: HashMap<CellCoord, Vec<usize>>,
}

impl CoverIndex {
    pub fn empty() -> Self {
        Self {
            nodes: Vec::new(),
            grid: HashMap::new(),
        }
    }

    pub fn build(nodes: Vec<CoverNode>) -> Self {
        let mut grid: HashMap<CellCoord, Vec<usize>> = HashMap::new();
        for (i, n) in nodes.iter().enumerate() {
            grid.entry(CellCoord::from_pos(&n.pos)).or_default().push(i);
        }
        Self { nodes, grid }
    }

    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }

    pub fn node(&self, idx: usize) -> Option<&CoverNode> {
        self.nodes.get(idx)
    }

    pub fn all_nodes(&self) -> &[CoverNode] {
        &self.nodes
    }

    /// Linear scan — only used on slow paths (reservation release,
    /// content-trigger payloads).
    pub fn node_by_key(&self, key: CoverSlotKey) -> Option<&CoverNode> {
        self.nodes
            .iter()
            .find(|n| n.chunk_id == key.chunk_id && n.node_id == key.node_id)
    }

    /// Returns node indices sorted by ascending distance. `max_y_diff`
    /// filters cover on different floors of a multi-level chunk.
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

    pub fn nodes_in_chunk(&self, chunk_id: i32) -> impl Iterator<Item = (usize, &CoverNode)> {
        self.nodes
            .iter()
            .enumerate()
            .filter(move |(_, n)| n.chunk_id == chunk_id)
    }
}
