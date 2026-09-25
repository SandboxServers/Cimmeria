//! `cover.coverage event=space_summary`: does this space have any cover an
//! NPC could stand on?
//!
//! The cover index is one global, unscoped set of nodes. Today's seed is
//! per-prefab template data in prefab-local coordinates, not level
//! placements (audit C1), so the nodes land wherever their local offsets
//! happen to point. One line per space at load says whether an NPC there
//! can use any of them.
//!
//! "On mesh" is the pathfinder's own test: a node counts only if
//! `find_path` could start a route from it (the `±0.5` start box).
//!
//! # Why "zero on mesh" is not enough
//!
//! The telemetry plan asked for a WARN when no node is on the mesh. Against
//! today's seed that never fires on Castle_CellBlock: the seed's Y column
//! is a *horizontal* UE3 axis, and the rebuilt mesh has a ground plane at
//! `y ≈ 0.2` under the whole ±400 square, so about 3,000 of the ~7,000
//! nodes in bounds land on it by coincidence. None is cover anyone can use.
//!
//! What does identify the C1 seed shape is where each set sits. A set is
//! one prefab's nodes. In prefab-local coordinates every set straddles the
//! origin — its centroid is within its own extent of `(0, 0, 0)` — because
//! the prefab is authored around its pivot. Placed in a level, a prefab's
//! nodes are wherever the prefab was placed, and only one standing on the
//! map's centre would do that. When most of a space's sets straddle the
//! origin the index is not in world space, and the row says so
//! (`reason = prefab_local_coordinates`).

use std::collections::HashMap;

use cimmeria_common::Vector3;

use cimmeria_entity::navigation::NavMesh;

use super::Cover;

/// The per-space coverage numbers.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct SpaceCoverage {
    /// Every node in the (global) index.
    pub nodes_total: usize,
    /// Nodes whose X/Z fall inside the space's X/Z bounds.
    pub nodes_in_bounds: usize,
    /// Of those, nodes `find_path` could start from. `None` with no navmesh.
    pub nodes_on_mesh: Option<usize>,
    /// Distinct cover sets (chunks) with at least one node in bounds.
    pub sets_in_bounds: usize,
    /// Of those, sets whose node centroid lies within the set's own extent
    /// of the world origin: the prefab-local signature (see module docs).
    pub sets_origin_centred: usize,
}

/// Why a space has no usable cover.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CoverageGap {
    /// No node sits where `find_path` could start (or none is in bounds, on
    /// a meshless space).
    NoCoverOnMesh,
    /// Most sets in bounds straddle the world origin: the index holds
    /// prefab-local offsets, not level placements (audit C1).
    PrefabLocalCoordinates,
}

impl CoverageGap {
    pub fn label(self) -> &'static str {
        match self {
            Self::NoCoverOnMesh => "no_cover_on_mesh",
            Self::PrefabLocalCoordinates => "prefab_local_coordinates",
        }
    }
}

impl SpaceCoverage {
    /// `None` when an NPC in this space can use cover; otherwise why not.
    pub fn gap(&self) -> Option<CoverageGap> {
        if self.nodes_on_mesh.unwrap_or(self.nodes_in_bounds) == 0 {
            return Some(CoverageGap::NoCoverOnMesh);
        }
        // More than half: a real level can have a prefab or two on its
        // centre, not most of them.
        if self.sets_origin_centred * 2 > self.sets_in_bounds {
            return Some(CoverageGap::PrefabLocalCoordinates);
        }
        None
    }
}

/// Count `cover`'s nodes against one space. `bounds` is
/// `(min_x, max_x, min_z, max_z)` — `spaces.xml`'s `MinY`/`MaxY` are the
/// horizontal Z axis.
pub fn space_coverage(
    cover: &Cover,
    bounds: (f32, f32, f32, f32),
    navmesh: Option<&NavMesh>,
) -> SpaceCoverage {
    let (min_x, max_x, min_z, max_z) = bounds;
    let mut out = SpaceCoverage {
        nodes_total: cover.node_count(),
        nodes_on_mesh: navmesh.map(|_| 0),
        ..SpaceCoverage::default()
    };
    let mut sets = std::collections::HashSet::new();
    for node in cover.index.all_nodes() {
        let p = node.pos;
        if !(min_x..=max_x).contains(&p.x) || !(min_z..=max_z).contains(&p.z) {
            continue;
        }
        out.nodes_in_bounds += 1;
        sets.insert(node.chunk_id);
        if let (Some(mesh), Some(n)) = (navmesh, out.nodes_on_mesh.as_mut()) {
            if mesh.start_poly_snap(&p).is_some() {
                *n += 1;
            }
        }
    }
    out.sets_in_bounds = sets.len();
    let origin_centred = origin_centred_sets(cover);
    out.sets_origin_centred = sets.iter().filter(|c| origin_centred.contains(c)).count();
    out
}

/// Chunk ids whose node centroid is within the set's own extent (the
/// furthest node from the centroid, at least 1 unit) of the world origin.
fn origin_centred_sets(cover: &Cover) -> std::collections::HashSet<i32> {
    let mut by_set: HashMap<i32, Vec<Vector3>> = HashMap::new();
    for n in cover.index.all_nodes() {
        by_set.entry(n.chunk_id).or_default().push(n.pos);
    }
    by_set
        .into_iter()
        .filter(|(_, pts)| {
            let k = pts.len() as f32;
            let c = pts.iter().fold(Vector3::new(0.0, 0.0, 0.0), |a, p| {
                Vector3::new(a.x + p.x / k, a.y + p.y / k, a.z + p.z / k)
            });
            let extent = pts.iter().map(|p| p.distance_to(&c)).fold(1.0f32, f32::max);
            c.distance_to(&Vector3::new(0.0, 0.0, 0.0)) <= extent
        })
        .map(|(id, _)| id)
        .collect()
}

/// Emit the summary row: INFO, or WARN when no NPC in the space can use
/// cover.
pub fn log_space_coverage(
    space_id: u32,
    world: &str,
    c: &SpaceCoverage,
    navmesh_hash: Option<&str>,
) {
    let Some(gap) = c.gap() else {
        tracing::info!(
            target: "cover.coverage",
            event = "space_summary",
            space_id,
            world,
            nodes_total = c.nodes_total,
            nodes_in_bounds = c.nodes_in_bounds,
            nodes_on_mesh = c.nodes_on_mesh,
            sets_in_bounds = c.sets_in_bounds,
            sets_origin_centred = c.sets_origin_centred,
            navmesh_hash,
            "cover.coverage: cover nodes available in this space"
        );
        return;
    };
    tracing::warn!(
        target: "cover.coverage",
        event = "space_summary",
        space_id,
        world,
        nodes_total = c.nodes_total,
        nodes_in_bounds = c.nodes_in_bounds,
        nodes_on_mesh = c.nodes_on_mesh,
        sets_in_bounds = c.sets_in_bounds,
        sets_origin_centred = c.sets_origin_centred,
        navmesh_hash,
        reason = gap.label(),
        "cover.coverage: NPCs in this space have no usable cover ({})",
        gap.label()
    );
}
