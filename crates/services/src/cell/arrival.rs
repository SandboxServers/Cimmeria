//! Safe arrival placement for server-authoritative cross-space moves.
//!
//! Every path that drops a player at an authored coordinate in a space they
//! are not currently in — stargate travel today, ring transport next — has the
//! same failure mode: the authored point is a prop transform lifted out of the
//! cooked map, and a prop transform is not a standable point. If it lands off
//! the destination's navmesh, the player's own client keeps moving them
//! normally while the server rejects every inbound position, so witnesses see
//! a frozen avatar and the only log is a `CorrectionSuppressed` with no
//! obvious cause. (Harset gate 3 is the worked example: the prefab origin sits
//! ~1.5 units above the floor with the nearest walkable vertex ~5 units away
//! in XZ, well outside the ±3.0 `DEST_EXTENTS` search box.)
//!
//! [`resolve_arrival`] is the single place that answers "is this arrival
//! standable, and if not, where instead". It deliberately does **not** call
//! [`NavMesh::get_nearest_point`]: that returns the input unchanged on a miss
//! (`crates/entity/src/navigation/mod.rs` — `unwrap_or(*pos)`), so its output
//! can never be trusted without re-validating it; and more importantly, an
//! arrival that Detour *can* reproject is an authored pin that is a metre or
//! two wrong and should be re-pinned in-game, not silently papered over at
//! runtime. Recovery is the world's authored respawner instead — the server's
//! existing answer to "where is it safe to put this player".
//!
//! Reference: `deprecated/python/cell/SGWPlayer.py:2129` (`stargatePassed` →
//! `moveTo(addr.xPos, addr.yPos, addr.zPos, addr.yaw, ...)`) — the 2009 server
//! arrived on the gate row verbatim and had no containment validator at all.

use cimmeria_common::Vector3;
use cimmeria_entity::movement_validation::{position_within_bounds, SpaceBounds};
use cimmeria_entity::navigation::NavMesh;

use super::space_manager::SpaceManager;
use super::spawner::{RespawnerDef, StargateEntry};

/// Where a resolved arrival position came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArrivalSource {
    /// The requested point was accepted by the destination's navmesh.
    Validated,
    /// The destination space has no navmesh loaded (or is not resident on
    /// this cell), so nothing could be checked and the request stands.
    Unvalidated,
    /// The requested point was off-navmesh and was replaced by one of the
    /// destination world's authored respawners.
    Respawner,
    /// The requested point was off-navmesh and no respawner qualified. The
    /// request stands because there is nothing better — the caller has
    /// already been warned.
    UnrecoverableOffMesh,
}

/// A validated arrival placement.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ResolvedArrival {
    pub position: [f32; 3],
    /// Facing. Carried through unchanged even when [`Self::position`] falls
    /// back to a respawner: respawner rows have no yaw of their own, and the
    /// authored gate/ring facing is the only non-arbitrary answer.
    pub yaw: f32,
    pub source: ArrivalSource,
}

/// Resolve where a traveller through `gate` should be placed.
///
/// One call, one line at the call site, deliberately: this is the whole
/// arrival contract for stargate travel, and the placement is going to move
/// (Castle CA10 relocates it from `onDialGate` to the gate-volume entry path).
/// Keeping it a single call means that move carries the validation with it
/// instead of quietly leaving it behind on the dead path.
///
/// Prefers the gate's authored `arrival_*` pin and falls back to the gate row
/// — including the row's `yaw`, which is already the authored "face this way"
/// value the 2009 `moveTo` passed
/// (`deprecated/python/cell/SGWPlayer.py:2129`).
pub fn validate_gate_arrival(space_mgr: &SpaceManager, gate: &StargateEntry) -> ResolvedArrival {
    let (desired, yaw) = gate.desired_arrival();
    resolve_arrival(space_mgr, &gate.world_name, desired, yaw)
}

/// Resolve a safe arrival in `world_name`, validating `desired` against that
/// world's navmesh when one is resident on this cell.
///
/// Read-only pre-flight: callers run this *before* handing the transfer to the
/// base, so the warn names the gate (or ring) whose authored coordinate needs
/// re-pinning, which is the operator-actionable seam.
///
/// **Scope limit, deliberately not papered over:** this can only validate
/// destinations whose space is already resident in this `SpaceManager` —
/// non-instanced startup worlds. Instanced destinations have no space until
/// one is created, and a future multi-cell split would put the destination on
/// another cell entirely. Both fall out as [`ArrivalSource::Unvalidated`].
/// The authoritative arrival-side re-check belongs in
/// `cell::service::base_messages::lifecycle::handle_create_entity` (which
/// already re-validates `destination_space_id` for the same
/// state-changed-between-decision-and-arrival reason) and is **not** wired by
/// this packet.
pub fn resolve_arrival(
    space_mgr: &SpaceManager,
    world_name: &str,
    desired: [f32; 3],
    yaw: f32,
) -> ResolvedArrival {
    // Non-instanced worlds keep their startup space (and its navmesh) resident
    // for the life of the cell, so a stargate destination is resolvable here
    // even though the traveller is not in it yet. Instanced worlds have no
    // entry in `world_spaces` and fall through to the unvalidated arm.
    let navmesh = space_mgr
        .world_spaces
        .get(world_name)
        .and_then(|space_id| space_mgr.spaces.get(space_id))
        .and_then(|space| space.navmesh.as_ref());
    resolve_arrival_with(navmesh, &space_mgr.respawners, world_name, desired, yaw)
}

/// Pure core of [`resolve_arrival`], parameterised on the destination's
/// navmesh and the respawner table so it is testable without a live
/// `SpaceManager` (whose navmesh loading is relative to the process CWD).
pub fn resolve_arrival_with(
    navmesh: Option<&NavMesh>,
    respawners: &[RespawnerDef],
    world_name: &str,
    desired: [f32; 3],
    yaw: f32,
) -> ResolvedArrival {
    let nav = match navmesh {
        Some(nav) => nav,
        None => {
            // Not an error — most worlds have no mesh. Logged so the
            // indefinitely-unvalidated destinations stay queryable rather
            // than invisible.
            tracing::debug!(
                world_name = %world_name,
                reason = "no_navmesh",
                "arrival: destination has no resident navmesh — accepting the \
                 authored arrival unvalidated"
            );
            return ResolvedArrival {
                position: desired,
                yaw,
                source: ArrivalSource::Unvalidated,
            };
        }
    };

    // Mirror BOTH layers the inbound-position validator applies, not just the
    // navmesh one. `SpaceManager::apply_client_position_update` sources its
    // bounds the same way (navmesh extents when a mesh exists), and a
    // candidate that is on-mesh but outside the AABB is hard-rejected by the
    // very next client packet — with the correction budget already cleared by
    // the authorised teleport, which is how a "recovery" turns into a
    // permanent freeze one position over.
    let bounds = SpaceBounds::new(nav.bmin, nav.bmax);
    let desired_v = Vector3::new(desired[0], desired[1], desired[2]);

    if nav.is_point_valid(&desired_v) && position_within_bounds(desired_v, &bounds) {
        return ResolvedArrival {
            position: desired,
            yaw,
            source: ArrivalSource::Validated,
        };
    }

    // Nearest-to-desired, matching `SpaceManager::resolve_recovery_position`'s
    // ordering for the identical candidate class — two different orderings for
    // the same fallback in one codebase would make the two paths disagree
    // about the same world.
    let fallback = respawners
        .iter()
        .filter(|r| r.world_name == world_name)
        // Zero-coordinate respawner guard (Castle CA00 / overlap row U16).
        //
        // Note the divergence this creates, deliberately and only for now:
        // `SpaceManager::resolve_recovery_position` filters the same
        // candidate class through the same two validity layers but has *no*
        // zero guard, and `[0,0,0]` demonstrably passes both on the
        // `castle_cellblock` mesh — so that path can still snap a player to
        // the world origin there. Castle CA00 owns adding the guard to
        // `resolve_respawn_target`; this one is self-contained until it lands.
        // Six seeded rows are literal (0, 0, 0) placeholders — an unfilled
        // seed cell, not an authored origin-adjacent spawn. Exact equality
        // on purpose: this is a sentinel test, and an epsilon would exclude a
        // legitimately-authored near-origin respawner on some future world.
        .filter(|r| r.pos != [0.0, 0.0, 0.0])
        .map(|r| Vector3::new(r.pos[0], r.pos[1], r.pos[2]))
        .filter(|p| nav.is_point_valid(p) && position_within_bounds(*p, &bounds))
        .min_by(|a, b| {
            a.distance_to(&desired_v)
                .total_cmp(&b.distance_to(&desired_v))
        });

    match fallback {
        Some(p) => {
            tracing::warn!(
                world_name = %world_name,
                desired_x = desired[0],
                desired_y = desired[1],
                desired_z = desired[2],
                fallback_x = p.x,
                fallback_y = p.y,
                fallback_z = p.z,
                fallback_distance = p.distance_to(&desired_v),
                reason = "arrival_off_navmesh",
                "arrival: authored arrival is off the destination navmesh — \
                 placing the traveller on the world's nearest respawner \
                 instead; re-pin the authored coordinate or the arrival will \
                 keep landing in geometry"
            );
            ResolvedArrival {
                position: [p.x, p.y, p.z],
                yaw,
                source: ArrivalSource::Respawner,
            }
        }
        None => {
            tracing::error!(
                world_name = %world_name,
                desired_x = desired[0],
                desired_y = desired[1],
                desired_z = desired[2],
                respawner_candidates = respawners
                    .iter()
                    .filter(|r| r.world_name == world_name)
                    .count(),
                reason = "arrival_unrecoverable",
                "arrival: authored arrival is off the destination navmesh AND \
                 no usable respawner exists for the world — the traveller will \
                 arrive off-mesh and every position update they send will be \
                 suppressed (silent freeze to witnesses); seed a respawner for \
                 this world"
            );
            ResolvedArrival {
                position: desired,
                yaw,
                source: ArrivalSource::UnrecoverableOffMesh,
            }
        }
    }
}

/// Load the committed `castle_cellblock` mesh — the only real navmesh a
/// `cimmeria-services` test can reach.
///
/// `SpaceManager::create_space_instance` resolves `data/spaces/{world}.nav`
/// relative to the process CWD, which under `cargo test -p cimmeria-services`
/// is the crate directory, so no space built by the ordinary test fixtures
/// ever has a mesh. Returns `None` when the fixture file is absent so a
/// checkout without `data/` still runs green.
#[cfg(test)]
pub(crate) fn test_fixture_mesh() -> Option<NavMesh> {
    let path = std::path::Path::new("../../data/spaces/castle_cellblock.nav");
    if !path.exists() {
        return None;
    }
    Some(NavMesh::load(path).expect("castle_cellblock.nav failed to load"))
}

/// Graft a navmesh-backed startup space for `world_name` into `space_mgr`,
/// bypassing the CWD-relative loader above.
///
/// This is what makes the *destination*-world lookup in [`resolve_arrival`]
/// observable at all: without it every `SpaceManager`-level arrival test
/// takes the `Unvalidated` early return, and swapping the destination world
/// for the traveller's current world would pass every test in the packet
/// while reintroducing exactly the off-mesh arrival this module exists to
/// prevent.
#[cfg(test)]
pub(crate) fn test_insert_navmesh_space(
    space_mgr: &mut SpaceManager,
    world_name: &str,
    navmesh: NavMesh,
) {
    use cimmeria_common::SpaceId;
    use cimmeria_entity::space::Space;

    let space_id = 0xFFFF;
    space_mgr.spaces.insert(
        space_id,
        super::space_manager::SpaceInstance {
            space_id,
            world_name: world_name.to_string(),
            space: Space::new(SpaceId(space_id as i32), world_name.to_string(), 50.0),
            entities: std::collections::HashMap::new(),
            players: std::collections::HashSet::new(),
            navmesh: Some(navmesh),
        },
    );
    space_mgr
        .world_spaces
        .insert(world_name.to_string(), space_id);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture_mesh() -> Option<NavMesh> {
        test_fixture_mesh()
    }

    /// A guard spawn coordinate that the fixture mesh accepts — shared with
    /// `crates/entity/src/navigation/tests.rs`.
    const ON_MESH: [f32; 3] = [-289.465, 68.542, -154.276];
    /// Same XZ, 200 units up: far outside both the ±3.0 nearest-poly search
    /// box and the jump-tolerance retry. This is the shape of the Harset
    /// defect (authored point nowhere near walkable ground), exaggerated so
    /// the test does not depend on the fixture's exact local geometry.
    const OFF_MESH: [f32; 3] = [-289.465, 268.542, -154.276];

    fn respawner(world: &str, pos: [f32; 3]) -> RespawnerDef {
        RespawnerDef {
            respawner_id: 1,
            world_name: world.to_string(),
            name: "test".to_string(),
            pos,
        }
    }

    #[test]
    fn a_standable_arrival_is_kept_verbatim() {
        let Some(mesh) = fixture_mesh() else { return };
        let out = resolve_arrival_with(Some(&mesh), &[], "Castle_CellBlock", ON_MESH, 1.25);
        assert_eq!(out.source, ArrivalSource::Validated);
        assert_eq!(out.position, ON_MESH);
        assert_eq!(out.yaw, 1.25);
    }

    /// The H-B1 shape. An off-navmesh arrival must be replaced by the
    /// world's respawner and **must never be handed back as-is** — that is
    /// the silent-freeze bug. Reverting `resolve_arrival_with` to "return
    /// `desired`" fails on both assertions.
    #[test]
    fn an_off_mesh_arrival_is_replaced_by_the_worlds_respawner() {
        let Some(mesh) = fixture_mesh() else { return };
        let respawners = [respawner("Castle_CellBlock", ON_MESH)];
        let out =
            resolve_arrival_with(Some(&mesh), &respawners, "Castle_CellBlock", OFF_MESH, 1.25);
        assert_eq!(out.source, ArrivalSource::Respawner);
        assert_eq!(out.position, ON_MESH);
        assert_ne!(
            out.position, OFF_MESH,
            "the off-mesh input must never be the answer — that is the \
             CorrectionSuppressed freeze this helper exists to prevent"
        );
        assert_eq!(
            out.yaw, 1.25,
            "respawner rows carry no yaw; the authored facing is carried through"
        );
    }

    /// Castle CA00 / overlap row U16: six seeded respawner rows are literal
    /// (0, 0, 0) placeholders. Snapping a traveller to the world origin is
    /// not a recovery. Deleting the zero filter makes this return [0,0,0].
    #[test]
    fn a_zero_coordinate_respawner_is_never_used_as_the_fallback() {
        let Some(mesh) = fixture_mesh() else { return };
        let respawners = [respawner("Castle_CellBlock", [0.0, 0.0, 0.0])];
        let out = resolve_arrival_with(Some(&mesh), &respawners, "Castle_CellBlock", OFF_MESH, 0.0);
        assert_eq!(out.source, ArrivalSource::UnrecoverableOffMesh);
        assert_ne!(
            out.position,
            [0.0, 0.0, 0.0],
            "a placeholder respawner row must not become an arrival"
        );
    }

    /// A respawner for a different world is not a candidate, even when the
    /// coordinate itself would validate. Asserted on the *position* as well
    /// as the source: `UnrecoverableOffMesh` alone cannot distinguish "the
    /// other world's respawner was correctly excluded" from "the fallback
    /// was never consulted at all".
    #[test]
    fn a_respawner_from_another_world_is_not_a_candidate() {
        let Some(mesh) = fixture_mesh() else { return };
        let respawners = [respawner("Harset", ON_MESH)];
        let out = resolve_arrival_with(Some(&mesh), &respawners, "Castle_CellBlock", OFF_MESH, 0.0);
        assert_eq!(out.source, ArrivalSource::UnrecoverableOffMesh);
        assert_eq!(out.position, OFF_MESH);
        assert_ne!(
            out.position, ON_MESH,
            "a respawner belonging to another world must never be borrowed"
        );
    }

    /// The *destination* world is what gets validated — not the traveller's
    /// current world, and not whichever space happens to have a mesh.
    ///
    /// This exercises the `SpaceManager` lookup in [`resolve_arrival`], which
    /// the `resolve_arrival_with` tests above cannot reach. Pointing that
    /// lookup at the origin world would leave every other test in this packet
    /// green while validating the wrong mesh — shipping the Harset freeze
    /// with a full green suite.
    #[test]
    fn resolve_arrival_validates_the_destination_worlds_mesh() {
        let Some(mesh) = fixture_mesh() else { return };
        let mut mgr = crate::test_support::make_space_manager(); // Agnos, no mesh
        test_insert_navmesh_space(&mut mgr, "Castle_CellBlock", mesh);
        mgr.respawners.push(respawner("Castle_CellBlock", ON_MESH));

        // Off-mesh point in the meshed destination → recovered.
        let out = resolve_arrival(&mgr, "Castle_CellBlock", OFF_MESH, 0.5);
        assert_eq!(out.source, ArrivalSource::Respawner);
        assert_eq!(out.position, ON_MESH);

        // The same point in the unmeshed world stands unvalidated — which is
        // what the meshed case would wrongly report if the lookup used the
        // traveller's current world instead of the destination.
        let out = resolve_arrival(&mgr, "Agnos", OFF_MESH, 0.5);
        assert_eq!(out.source, ArrivalSource::Unvalidated);
        assert_eq!(out.position, OFF_MESH);
    }

    /// Nearest-to-desired, matching `resolve_recovery_position`'s ordering
    /// for the same candidate class.
    #[test]
    fn the_nearest_qualifying_respawner_wins() {
        let Some(mesh) = fixture_mesh() else { return };
        // Both are on-mesh; the first is the one the off-mesh point sits
        // directly above.
        let near = ON_MESH;
        let far = [-260.0, 68.542, -120.0];
        if !mesh.is_point_valid(&Vector3::new(far[0], far[1], far[2])) {
            // Geometry-dependent second candidate; skip rather than assert a
            // fixture fact this test doesn't own.
            return;
        }
        let respawners = [
            respawner("Castle_CellBlock", far),
            respawner("Castle_CellBlock", near),
        ];
        let out = resolve_arrival_with(Some(&mesh), &respawners, "Castle_CellBlock", OFF_MESH, 0.0);
        assert_eq!(out.position, near);
    }

    /// Most worlds have no mesh — the arrival stands, and the caller can
    /// tell the difference between "checked and fine" and "not checked".
    /// Note there is deliberately no test for the `position_within_bounds`
    /// layer: bounds are derived from the navmesh extents, so on the
    /// navmesh path the two layers coincide by construction. The check is
    /// kept as defence in depth against a future bounds source and to match
    /// `resolve_recovery_position` exactly.
    #[test]
    fn a_world_with_no_navmesh_is_unvalidated_not_rejected() {
        let out = resolve_arrival_with(None, &[], "Harset_Market", [1.0, 2.0, 3.0], 0.5);
        assert_eq!(out.source, ArrivalSource::Unvalidated);
        assert_eq!(out.position, [1.0, 2.0, 3.0]);
        assert_eq!(out.yaw, 0.5);
    }
}
