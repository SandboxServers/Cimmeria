//! Nodes → cover sets.
//!
//! A cover set is the unit the server reserves against and the content
//! engine keys `player_entered_cover` on (`cover_set_id`). Pattern B
//! already groups its nodes explicitly — one `CoverNodeArray` owner, one
//! set. Pattern A markers are standalone actors, so their grouping is
//! implicit: the markers a designer dropped around one obstacle. They are
//! clustered by single linkage — two markers share a set when they are
//! within [`LINK_HORIZONTAL_M`] of each other horizontally and
//! [`LINK_VERTICAL_M`] vertically, transitively.
//!
//! The thresholds are set by the one hand-checked cluster there is: the
//! med-station desk in Castle_CellBlock (NA20 Q3), whose seven markers
//! sit 2.3-3.0 m from their nearest neighbour and must stay one set,
//! because chains 1132/1133 key the take-cover objective on it.

use super::{CoverPattern, ExtractedCoverNode};

/// Horizontal single-linkage distance for Pattern A markers, metres.
pub const LINK_HORIZONTAL_M: f32 = 3.5;
/// Vertical tolerance for linking, metres — keeps a balcony's cover out
/// of the set on the floor below (the detector's own tolerance is 2 m).
pub const LINK_VERTICAL_M: f32 = 1.0;

/// Set ids are `world_id * SET_ID_WORLD_STRIDE + n` (n from 1), so ids
/// never collide across worlds and one world's re-extract never renumbers
/// another's. 100,000 sets per world is ~25x Castle's node count.
pub const SET_ID_WORLD_STRIDE: i32 = 100_000;

/// One output set with its nodes in `node_id` order.
#[derive(Debug, Clone)]
pub struct CoverSetOut {
    pub set_id: i32,
    pub world_id: i32,
    /// Unique, human-readable: `<world>.<chunk>.<pattern>.<first export>`.
    pub name: String,
    /// Chunk file the set's first node came from, e.g. `Castle-00060005.umap`.
    pub src: String,
    pub pattern: CoverPattern,
    pub nodes: Vec<ExtractedCoverNode>,
}

fn find(parent: &mut [usize], mut x: usize) -> usize {
    while parent[x] != x {
        parent[x] = parent[parent[x]];
        x = parent[x];
    }
    x
}

fn linked(a: &ExtractedCoverNode, b: &ExtractedCoverNode) -> bool {
    let dx = a.pos[0] - b.pos[0];
    let dz = a.pos[2] - b.pos[2];
    (a.pos[1] - b.pos[1]).abs() <= LINK_VERTICAL_M
        && dx * dx + dz * dz <= LINK_HORIZONTAL_M * LINK_HORIZONTAL_M
}

/// Group one world's nodes into sets. `nodes` must already be in the
/// deterministic (chunk filename, export index) order
/// [`super::extract_map_cover`] produces; set and node ids follow it, so
/// the same client build always yields the same ids.
pub fn group_into_sets(
    world_id: i32,
    world_name: &str,
    nodes: Vec<ExtractedCoverNode>,
) -> Vec<CoverSetOut> {
    let n = nodes.len();
    let mut parent: Vec<usize> = (0..n).collect();

    // Pattern B: union by owner.
    let mut owner_first: std::collections::HashMap<(&str, usize), usize> =
        std::collections::HashMap::new();
    for (i, node) in nodes.iter().enumerate() {
        if node.pattern != CoverPattern::NodeArray {
            continue;
        }
        let first = *owner_first
            .entry((node.chunk.as_str(), node.owner_export))
            .or_insert(i);
        let (ra, rb) = (find(&mut parent, first), find(&mut parent, i));
        parent[rb.max(ra)] = ra.min(rb);
    }

    // Pattern A: single linkage among markers only. O(n^2) is ~8M pair
    // tests for Castle — well under a second, and simpler than a grid.
    let spec: Vec<usize> = (0..n)
        .filter(|&i| nodes[i].pattern == CoverPattern::SpecNode)
        .collect();
    for (k, &i) in spec.iter().enumerate() {
        for &j in &spec[k + 1..] {
            if linked(&nodes[i], &nodes[j]) {
                let (ra, rb) = (find(&mut parent, i), find(&mut parent, j));
                if ra != rb {
                    // Keep the lowest index as root so a set's identity
                    // is its earliest node.
                    parent[ra.max(rb)] = ra.min(rb);
                }
            }
        }
    }

    // Collect in root order; a root is always its set's lowest index, so
    // iterating 0..n visits sets in first-node order.
    let mut slot_of_root: std::collections::HashMap<usize, usize> =
        std::collections::HashMap::new();
    let mut sets: Vec<CoverSetOut> = Vec::new();
    for (i, node) in nodes.into_iter().enumerate() {
        let root = find(&mut parent, i);
        let slot = *slot_of_root.entry(root).or_insert_with(|| {
            let seq = sets.len() as i32 + 1;
            sets.push(CoverSetOut {
                set_id: world_id * SET_ID_WORLD_STRIDE + seq,
                world_id,
                name: format!(
                    "{world_name}.{}.{}.{}",
                    node.chunk,
                    node.pattern.label(),
                    node.component_export
                ),
                src: format!("{}.umap", node.chunk),
                pattern: node.pattern,
                nodes: Vec::new(),
            });
            sets.len() - 1
        });
        sets[slot].nodes.push(node);
    }
    sets
}
