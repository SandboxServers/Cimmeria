//! `cover.coverage event=space_summary`: can the NPCs in this space use any
//! cover at all?
//!
//! One line per space, once the space has its NPCs and the cover index has
//! loaded: the startup spaces after `SpaceManager::cover_loaded`, and each
//! instanced space after `spawn_instance_npcs_from_records`. It reads the
//! world-scoped index (NA21): the nodes of the space's world, how many of
//! them stand on the space's navmesh, and how many NPCs there would use
//! cover.
//!
//! "On the mesh" is `NavMesh::get_height_near` searched around the node's
//! own Y (so a node reads the storey it is on, not the one nearest world
//! `Y = 0`), with the node within [`NODE_FLOOR_TOLERANCE`] of that floor.
//!
//! WARN when a meshed space has cover-eligible NPCs (`use_cover`, not
//! stationary) and not one usable node: those NPCs will return
//! `no_cover reason=no_candidate_in_radius` on every fight tick. Before
//! NA21 that was every world (audit C1: the seed was prefab-local); after
//! it, only the worlds with no extracted cover.

use cimmeria_entity::navigation::NavMesh;

use super::Cover;

/// How far a node may sit from the floor under it and still count as on
/// the mesh. Cover markers are authored at or just above the floor.
pub const NODE_FLOOR_TOLERANCE: f32 = 1.0;

/// The per-space coverage numbers.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct SpaceCoverage {
    /// `resources.worlds.world_id` of the space, if stamped.
    pub world_id: Option<i32>,
    /// Nodes in the index for this world.
    pub nodes_in_world: usize,
    /// Of those, nodes on the space's navmesh. `None` with no navmesh.
    pub nodes_on_mesh: Option<usize>,
    /// Distinct cover sets with a node in this world.
    pub sets_in_world: usize,
    /// NPCs in the space that would look for cover (`use_cover`, mobile).
    pub cover_npcs: usize,
}

impl SpaceCoverage {
    /// Nodes an NPC here could actually use.
    pub fn usable_nodes(&self) -> usize {
        self.nodes_on_mesh.unwrap_or(self.nodes_in_world)
    }

    /// A meshed space whose cover-seeking NPCs have nothing to take.
    pub fn warns(&self) -> bool {
        self.nodes_on_mesh.is_some() && self.cover_npcs > 0 && self.usable_nodes() == 0
    }
}

/// Count `cover`'s nodes for one world. `cover_npcs` is supplied by the
/// caller, which owns the entity table.
pub fn space_coverage(
    cover: &Cover,
    world_id: Option<i32>,
    navmesh: Option<&NavMesh>,
    cover_npcs: usize,
) -> SpaceCoverage {
    let mut out = SpaceCoverage {
        world_id,
        nodes_on_mesh: navmesh.map(|_| 0),
        cover_npcs,
        ..SpaceCoverage::default()
    };
    let Some(world_id) = world_id else {
        return out;
    };
    let mut sets = std::collections::HashSet::new();
    for node in cover
        .index
        .all_nodes()
        .iter()
        .filter(|n| n.world_id == world_id)
    {
        out.nodes_in_world += 1;
        sets.insert(node.chunk_id);
        if let (Some(mesh), Some(n)) = (navmesh, out.nodes_on_mesh.as_mut()) {
            let p = node.pos;
            if mesh
                .get_height_near(p.x, p.y, p.z)
                .is_some_and(|h| (p.y - h).abs() <= NODE_FLOOR_TOLERANCE)
            {
                *n += 1;
            }
        }
    }
    out.sets_in_world = sets.len();
    out
}

/// Emit the summary row: INFO, or WARN when [`SpaceCoverage::warns`].
pub fn log_space_coverage(
    space_id: u32,
    world: &str,
    c: &SpaceCoverage,
    navmesh_hash: Option<&str>,
) {
    if c.warns() {
        tracing::warn!(
            target: "cover.coverage",
            event = "space_summary",
            space_id,
            world,
            world_id = c.world_id,
            nodes_in_world = c.nodes_in_world,
            nodes_on_mesh = c.nodes_on_mesh,
            sets_in_world = c.sets_in_world,
            cover_npcs = c.cover_npcs,
            navmesh_hash,
            reason = "no_usable_cover",
            "cover.coverage: NPCs here use cover but no cover node in this world is on its navmesh"
        );
    } else {
        tracing::info!(
            target: "cover.coverage",
            event = "space_summary",
            space_id,
            world,
            world_id = c.world_id,
            nodes_in_world = c.nodes_in_world,
            nodes_on_mesh = c.nodes_on_mesh,
            sets_in_world = c.sets_in_world,
            cover_npcs = c.cover_npcs,
            navmesh_hash,
            "cover.coverage: cover available to this space"
        );
    }
}
