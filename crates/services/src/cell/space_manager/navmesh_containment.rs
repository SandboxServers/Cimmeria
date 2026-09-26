//! The navmesh containment predicate: whether a space's mesh may gate
//! where a player stands, given its world's [`NavmeshMode`].
//!
//! The mode itself, and why it exists, is `cell::spawner::navmesh_mode`;
//! these `impl SpaceManager` methods stay with `SpaceManager`.

use cimmeria_entity::navigation::NavMesh;

use super::{NavmeshMode, SpaceManager};

impl SpaceManager {
    /// The containment mode of `world_name`, or [`NavmeshMode::Enforce`]
    /// for a world this cell has no definition for.
    pub fn navmesh_mode(&self, world_name: &str) -> NavmeshMode {
        self.worlds
            .get(world_name)
            .map(|w| w.navmesh_mode)
            .unwrap_or_default()
    }

    /// **The containment predicate.** `true` when `space_id`'s navmesh may
    /// be used as a hard gate on where a player is allowed to be.
    ///
    /// Two conditions, both required:
    ///
    /// 1. a mesh is actually resident for that space, and
    /// 2. the space's world is [`NavmeshMode::Enforce`].
    ///
    /// Every gate that would snap a player back, refuse an arrival or abort
    /// a trip because a point is off-mesh goes through this, so an advisory
    /// world and a meshless world take the same branch everywhere. Callers
    /// that only want to *know* whether a point is on the mesh keep calling
    /// [`SpaceManager::is_position_valid`], which still answers truthfully
    /// in an advisory world.
    ///
    /// An unknown `space_id` is `false`: nothing can be enforced against a
    /// space that is not here.
    pub fn enforces_navmesh_containment(&self, space_id: u32) -> bool {
        let Some(space) = self.spaces.get(&space_id) else {
            return false;
        };
        space.navmesh.is_some() && self.navmesh_mode(&space.world_name) == NavmeshMode::Enforce
    }

    /// The navmesh a containment gate may consult for `space_id` — `None`
    /// both when no mesh is resident and when the world is advisory.
    ///
    /// Lets a call site that already branches on `Option<&NavMesh>` (the
    /// recovery resolver, the respawner fallback) pick up the mode without
    /// growing a second parameter that could be passed inconsistently.
    pub(crate) fn containment_navmesh(&self, space_id: u32) -> Option<&NavMesh> {
        self.enforces_navmesh_containment(space_id)
            .then(|| self.spaces.get(&space_id).and_then(|s| s.navmesh.as_ref()))
            .flatten()
    }

    /// Same, keyed by world name, for the startup space of a world the
    /// caller is not in — the arrival checks, which validate a destination.
    ///
    /// Instanced worlds have no `world_spaces` entry and come back `None`,
    /// exactly as they did before the mode existed.
    pub(crate) fn containment_navmesh_for_world(&self, world_name: &str) -> Option<&NavMesh> {
        let space_id = *self.world_spaces.get(world_name)?;
        self.containment_navmesh(space_id)
    }

    /// One INFO line per resident space that has a mesh, naming the mode,
    /// the poly count and how many of that world's `spawnlist` rows the
    /// mesh does not cover.
    ///
    /// A bad mesh is otherwise silent. `harset.nav` loads cleanly, reports
    /// 19,345 polygons, and still leaves most of the world's own spawn rows
    /// off-mesh — a shape that is obvious in one line at boot and invisible
    /// until a player walks into it otherwise. The off-mesh spawn count is
    /// the cheapest proxy the server has for "does this mesh describe the
    /// map it is named after", because the spawn rows are independently
    /// authored coordinates of things that stand on the floor.
    ///
    /// INFO, not WARN, even at a high off-mesh count: an advisory world is
    /// *expected* to have one, and a WARN that is expected every boot stops
    /// being read. The operator-actionable signal is the number moving.
    pub fn log_navmesh_summary(&self, spawn_records: &[super::super::spawner::SpawnRecord]) {
        for space in self.spaces.values() {
            let Some(nav) = &space.navmesh else { continue };
            let world = space.world_name.as_str();
            let rows = spawn_records.iter().filter(|r| r.world_name == world);
            let (mut total, mut off_mesh) = (0usize, 0usize);
            for r in rows {
                total += 1;
                if !nav.is_point_valid(&cimmeria_common::Vector3::new(r.x, r.y, r.z)) {
                    off_mesh += 1;
                }
            }
            tracing::info!(
                target: "movement.navmesh",
                space_id = space.space_id,
                world_name = %world,
                navmesh_mode = self.navmesh_mode(world).as_db_str(),
                poly_count = nav.poly_count(),
                spawn_rows = total,
                spawn_rows_off_mesh = off_mesh,
                reason = "navmesh_mode_summary",
                "navmesh: mesh resident for this world -- 'advisory' means it is \
                 used for pathing, line of sight and height only and never gates \
                 player movement; a high off-mesh spawn count on an 'enforce' \
                 world is an invisible-wall report waiting to happen"
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cell::spawner::WorldRow;
    use std::collections::HashMap;

    /// `enforces_navmesh_containment` is an AND of two independent facts.
    /// Asserted as a truth table because every containment gate in the cell
    /// now reads this one predicate: a branch that collapsed to "a mesh is
    /// loaded" would restore the invisible wall, and a branch that
    /// collapsed to "the world is enforce" would claim to gate a space that
    /// has nothing to gate with.
    #[test]
    fn containment_needs_both_a_mesh_and_the_enforce_mode() {
        let mut mgr = crate::test_support::make_space_manager(); // Agnos, no mesh
        let space_id = *mgr.world_spaces.get("Agnos").expect("Agnos startup space");

        // No mesh, default mode.
        assert!(!mgr.enforces_navmesh_containment(space_id));
        assert_eq!(mgr.navmesh_mode("Agnos"), NavmeshMode::Enforce);

        // No mesh, advisory: still nothing to enforce.
        mgr.stamp_world_rows(&HashMap::from([(
            "Agnos".to_string(),
            WorldRow {
                world_id: 10,
                navmesh_mode: NavmeshMode::Advisory,
            },
        )]));
        assert!(!mgr.enforces_navmesh_containment(space_id));
        assert!(mgr.containment_navmesh(space_id).is_none());

        // A mesh, still advisory.
        let Some(mesh) = crate::cell::arrival::test_fixture_mesh() else {
            return; // fixture-less checkout: the meshless half above still ran
        };
        mgr.spaces.get_mut(&space_id).unwrap().navmesh = Some(mesh);
        assert!(
            !mgr.enforces_navmesh_containment(space_id),
            "a resident mesh must not re-arm containment in an advisory world",
        );
        assert!(mgr.containment_navmesh(space_id).is_none());
        assert!(
            mgr.containment_navmesh_for_world("Agnos").is_none(),
            "the world-keyed flavour must agree with the space-keyed one",
        );

        // A mesh, back to enforce.
        mgr.stamp_world_rows(&HashMap::from([(
            "Agnos".to_string(),
            WorldRow::enforcing(10),
        )]));
        assert!(mgr.enforces_navmesh_containment(space_id));
        assert!(mgr.containment_navmesh(space_id).is_some());
        assert!(mgr.containment_navmesh_for_world("Agnos").is_some());
    }

    /// A space id nobody knows enforces nothing, and an unknown world name
    /// reads `Enforce` — the strict default, so a typo cannot demote a gate.
    #[test]
    fn unknown_ids_are_safe_in_both_directions() {
        let mgr = crate::test_support::make_space_manager();
        assert!(!mgr.enforces_navmesh_containment(0xDEAD_BEEF));
        assert!(mgr.containment_navmesh(0xDEAD_BEEF).is_none());
        assert_eq!(mgr.navmesh_mode("NoSuchWorld"), NavmeshMode::Enforce);
        assert!(mgr.containment_navmesh_for_world("NoSuchWorld").is_none());
    }

    /// The stamp is what carries the mode from the DB onto the world
    /// table, and a world the stamp never mentions must keep `Enforce`.
    /// This is the DB-down startup path.
    #[test]
    fn an_unstamped_world_keeps_containment_enforced() {
        let mut mgr = crate::test_support::make_space_manager();
        mgr.stamp_world_rows(&HashMap::from([(
            "SomeOtherWorld".to_string(),
            WorldRow {
                world_id: 99,
                navmesh_mode: NavmeshMode::Advisory,
            },
        )]));
        assert_eq!(
            mgr.navmesh_mode("Agnos"),
            NavmeshMode::Enforce,
            "another world's advisory row must not leak across worlds",
        );
    }
}
