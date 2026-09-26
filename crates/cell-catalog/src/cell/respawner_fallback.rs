//! The one "where is it safe to put this player" respawner search.
//!
//! Two paths need the same answer and used to compute it twice:
//!
//! * `cell::arrival::resolve_arrival_with` (`cimmeria-services`) — an authored arrival
//!   (gate pin, ring pad) that the destination world's navmesh rejects.
//! * `SpaceManager::resolve_recovery_position` — a client position the
//!   movement validator rejects and cannot reproject onto the mesh.
//!
//! They drifted, and the drift was a bug: the arrival path filtered out the
//! six seeded `(0, 0, 0)` placeholder respawner rows (Castle CA00 / overlap
//! row U16) and the recovery path did not, so the same unauthored row that
//! could never become a gate arrival could still snap a rubber-banding player
//! to the world origin — `[0,0,0]` demonstrably passes both validity layers
//! on the `castle_cellblock` mesh. PR #662 review, finding 8.
//!
//! Hoisting the search here also pins the *ordering* (nearest-to-`near`) in
//! one place. Two different orderings for the same candidate class would make
//! the two paths disagree about the same world for no reason a reader could
//! reconstruct.

use cimmeria_common::Vector3;
use cimmeria_entity::movement_validation::{position_within_bounds, SpaceBounds};
use cimmeria_entity::navigation::NavMesh;

use super::spawner::RespawnerDef;

/// `true` for a respawner row whose coordinates were never authored.
///
/// Exact equality on purpose: this is a sentinel test, not a proximity test.
/// An epsilon band would start excluding a legitimately-authored near-origin
/// respawner on some future world, and `-0.0 == 0.0` in IEEE-754 so negative
/// zeros are caught without a special case.
///
/// The same rule is spelled `is_unauthored` in
/// `cell_methods::player::combat::respawn`, which searches the *unvalidated*
/// candidate list (first-match, no navmesh) and so cannot share this
/// function's signature.
pub(crate) fn is_unauthored(r: &RespawnerDef) -> bool {
    r.pos == [0.0, 0.0, 0.0]
}

/// The authored respawner for `world_name` nearest to `near` that both
/// validity layers accept, or `None` when the world has no usable one.
///
/// Both layers, not just the navmesh: a point that is on-mesh but outside the
/// space AABB is hard-rejected by the very next client packet, and by then the
/// correction budget has been cleared by the authorised teleport that put the
/// player there — which is how a "recovery" turns into a permanent freeze one
/// position over. `navmesh` is `None` for a space that has no mesh, where
/// `bounds` is the whole test.
///
/// Both callers pass the mesh in its **containment** role — the
/// `SpaceManager::containment_navmesh` flavour — so an advisory world also
/// arrives here as `None`. That is deliberate and belongs at the caller: a
/// respawner row is a coordinate a human chose, and discarding it because a
/// mesh the server has already declared untrustworthy does not cover it would
/// leave the world with no recovery target at all. This function stays pure
/// and takes the mode as an already-applied `Option`.
pub fn nearest_valid_respawner(
    respawners: &[RespawnerDef],
    world_name: &str,
    near: Vector3,
    navmesh: Option<&NavMesh>,
    bounds: &SpaceBounds,
) -> Option<Vector3> {
    respawners
        .iter()
        .filter(|r| r.world_name == world_name)
        .filter(|r| !is_unauthored(r))
        .map(|r| Vector3::new(r.pos[0], r.pos[1], r.pos[2]))
        .filter(|p| {
            position_within_bounds(*p, bounds) && navmesh.is_none_or(|nav| nav.is_point_valid(p))
        })
        .min_by(|a, b| a.distance_to(&near).total_cmp(&b.distance_to(&near)))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn respawner(world: &str, pos: [f32; 3]) -> RespawnerDef {
        RespawnerDef {
            respawner_id: 1,
            world_name: world.to_string(),
            name: "test".to_string(),
            pos,
        }
    }

    fn wide_bounds() -> SpaceBounds {
        SpaceBounds::new([-1000.0, -1000.0, -1000.0], [1000.0, 1000.0, 1000.0])
    }

    /// The finding-8 shape, asserted without a navmesh so it holds on any
    /// checkout: an all-zero placeholder row is not a recovery target.
    /// Deleting the `is_unauthored` filter returns `(0,0,0)` here, because it
    /// is also the nearest candidate to the searched-from point.
    #[test]
    fn an_unauthored_zero_row_is_never_the_answer() {
        let rows = [
            respawner("Harset", [0.0, 0.0, 0.0]),
            respawner("Harset", [50.0, 0.0, 0.0]),
        ];
        let out = nearest_valid_respawner(
            &rows,
            "Harset",
            Vector3::new(1.0, 0.0, 0.0),
            None,
            &wide_bounds(),
        );
        assert_eq!(out, Some(Vector3::new(50.0, 0.0, 0.0)));
    }

    /// Negative zero is the same sentinel — a seed round-trip through a float
    /// column can produce it and it must not slip past the guard.
    #[test]
    fn a_negative_zero_row_is_treated_as_unauthored() {
        let rows = [respawner("Harset", [-0.0, 0.0, -0.0])];
        assert!(is_unauthored(&rows[0]));
        assert_eq!(
            nearest_valid_respawner(
                &rows,
                "Harset",
                Vector3::new(1.0, 0.0, 0.0),
                None,
                &wide_bounds()
            ),
            None
        );
    }

    /// A respawner belonging to another world is never borrowed, however
    /// close it is.
    #[test]
    fn another_worlds_respawner_is_not_a_candidate() {
        let rows = [respawner("Agnos", [1.0, 0.0, 1.0])];
        assert_eq!(
            nearest_valid_respawner(
                &rows,
                "Harset",
                Vector3::new(1.0, 0.0, 1.0),
                None,
                &wide_bounds()
            ),
            None
        );
    }

    /// Nearest-to-`near`, not first-in-table: the ordering is the whole
    /// reason this is shared rather than duplicated.
    #[test]
    fn the_nearest_candidate_wins_regardless_of_row_order() {
        let rows = [
            respawner("Harset", [100.0, 0.0, 0.0]),
            respawner("Harset", [10.0, 0.0, 0.0]),
        ];
        assert_eq!(
            nearest_valid_respawner(
                &rows,
                "Harset",
                Vector3::new(0.0, 0.0, 0.0),
                None,
                &wide_bounds()
            ),
            Some(Vector3::new(10.0, 0.0, 0.0))
        );
    }

    /// The AABB layer applies on its own when the space has no mesh — an
    /// authored row outside the space is not a recovery either.
    #[test]
    fn a_row_outside_the_space_bounds_is_rejected() {
        let rows = [respawner("Harset", [5000.0, 0.0, 0.0])];
        assert_eq!(
            nearest_valid_respawner(
                &rows,
                "Harset",
                Vector3::new(0.0, 0.0, 0.0),
                None,
                &wide_bounds()
            ),
            None
        );
    }
}
