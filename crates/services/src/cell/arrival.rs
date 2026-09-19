//! Safe arrival placement for server-authoritative cross-space moves.
//!
//! Every path that drops a player at an authored coordinate in a space they
//! are not currently in — stargate travel, ring transport — has the
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
//! standable, and if not, where instead" — and, when the answer is "nowhere",
//! it says so: [`ArrivalSource::UnrecoverableOffMesh`] carries no usable
//! position, and every caller gates on [`ResolvedArrival::is_usable`] and
//! refuses the move rather than shipping the rejected coordinate into a
//! transfer. It deliberately does **not** call
//! [`NavMesh::get_nearest_point`]: that returns the input unchanged on a miss
//! (`crates/entity/src/navigation/mod.rs` — `unwrap_or(*pos)`), so its output
//! can never be trusted without re-validating it; and more importantly, an
//! arrival that Detour *can* reproject is an authored pin that is a metre or
//! two wrong and should be re-pinned in-game, not silently papered over at
//! runtime. Recovery is the world's authored respawner instead — the server's
//! existing answer to "where is it safe to put this player".
//!
//! **Two flavours, deliberately.** [`resolve_arrival`] *substitutes*: a gate
//! arrival is a pin on a prop transform, nothing on the client is anchored to
//! it, and putting the traveller on the world's nearest respawner is strictly
//! better than not arriving. [`check_arrival`] only *validates*: ring
//! transport calls it because a pad row is not a pin — the client plays the
//! ring matinee at that pad — so a substitute would desync the fiction from
//! the geometry. A ring pad that fails the check aborts the trip instead
//! (`ring_transport::runtime::tick`). Both share
//! [`check_arrival_with`] so the two can never disagree about what
//! "standable" means.
//!
//! Reference: `deprecated/python/cell/SGWPlayer.py:2129` (`stargatePassed` →
//! `moveTo(addr.xPos, addr.yPos, addr.zPos, addr.yaw, ...)`) — the 2009 server
//! arrived on the gate row verbatim and had no containment validator at all.

use cimmeria_common::Vector3;
use cimmeria_entity::movement_validation::{position_within_bounds, SpaceBounds};
use cimmeria_entity::navigation::NavMesh;

use super::respawner_fallback::nearest_valid_respawner;
use super::space_manager::SpaceManager;
use super::spawner::{RespawnerDef, StargateEntry};

/// Outcome of a **validate-only** arrival check — no substitution, no
/// fallback, just "would the position validator accept this point".
///
/// Ring transport uses this rather than [`resolve_arrival`]: a ring pad row
/// *is* the arrival. The client plays the ring matinee at the pad and the FSM
/// fires `FireTeleportIn` for that region, so quietly landing the passengers
/// on a respawner somewhere else in the world would desync the fiction from
/// the geometry with nothing in the log tying the two together (PR #662
/// review, finding 7).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArrivalCheck {
    /// The destination's navmesh (and its AABB) accept the point.
    Validated,
    /// Nothing could be checked, so nothing is refused. Two causes, treated
    /// identically on purpose: the destination space has no navmesh resident
    /// on this cell (most worlds), **or** the destination world is
    /// [`NavmeshMode::Advisory`](crate::cell::space_manager::NavmeshMode) and
    /// its mesh is not allowed to gate anything. Not a rejection.
    Unvalidated,
    /// The destination has a navmesh and the point is off it (or outside the
    /// mesh's bounds). Putting a player here means every position update they
    /// send is suppressed.
    OffMesh,
}

/// Validate `desired` against `world_name`'s navmesh without substituting
/// anything. See [`ArrivalCheck`].
pub fn check_arrival(
    space_mgr: &SpaceManager,
    world_name: &str,
    desired: [f32; 3],
) -> ArrivalCheck {
    check_arrival_with(containment_navmesh(space_mgr, world_name), desired)
}

/// Pure core of [`check_arrival`], parameterised on the destination's navmesh
/// so it is testable without a live `SpaceManager`.
pub fn check_arrival_with(navmesh: Option<&NavMesh>, desired: [f32; 3]) -> ArrivalCheck {
    let Some(nav) = navmesh else {
        return ArrivalCheck::Unvalidated;
    };
    // Mirror BOTH layers the inbound-position validator applies, not just the
    // navmesh one. `SpaceManager::apply_client_position_update` sources its
    // bounds the same way (navmesh extents when a mesh exists), and a
    // candidate that is on-mesh but outside the AABB is hard-rejected by the
    // very next client packet — with the correction budget already cleared by
    // the authorised teleport, which is how a "recovery" turns into a
    // permanent freeze one position over.
    let bounds = SpaceBounds::new(nav.bmin, nav.bmax);
    let p = Vector3::new(desired[0], desired[1], desired[2]);
    if nav.is_point_valid(&p) && position_within_bounds(p, &bounds) {
        ArrivalCheck::Validated
    } else {
        ArrivalCheck::OffMesh
    }
}

/// The navmesh of `world_name`'s startup space **in its containment role**,
/// if one is resident here and that world enforces containment.
///
/// Non-instanced worlds keep their startup space (and its navmesh) resident
/// for the life of the cell, so a stargate or ring destination is resolvable
/// even though the traveller is not in it yet. Instanced worlds have no entry
/// in `world_spaces` and come back `None`.
///
/// An [`NavmeshMode::Advisory`](crate::cell::space_manager::NavmeshMode)
/// world also comes back `None`, which is what makes an arrival there
/// [`ArrivalCheck::Unvalidated`]. That is the intended reading: refusing to
/// deliver a traveller to a coordinate because a mesh with known holes does
/// not cover it is the same defect as snapping a walking player back, one
/// event earlier. Harset gate 3 is the case — its floor is real, its mesh is
/// not.
fn containment_navmesh<'a>(space_mgr: &'a SpaceManager, world_name: &str) -> Option<&'a NavMesh> {
    space_mgr.containment_navmesh_for_world(world_name)
}

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
    /// The requested point was off-navmesh and no respawner qualified.
    ///
    /// **There is no usable position in this variant.** [`ResolvedArrival`]
    /// still carries the requested point so the caller can name it in a log,
    /// but transferring a player to it recreates the exact silent
    /// `CorrectionSuppressed` freeze this module exists to prevent. Callers
    /// must gate on [`ResolvedArrival::is_usable`] and refuse the move.
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

impl ResolvedArrival {
    /// `true` when [`Self::position`] is safe to put a player on.
    ///
    /// The only failing variant is [`ArrivalSource::UnrecoverableOffMesh`],
    /// where `position` is the *rejected* input rather than an answer.
    /// Every caller that moves a player must gate on this and refuse the
    /// move: handing the rejected point to a transfer is what produced the
    /// original Harset freeze (the traveller arrives off-mesh, every inbound
    /// position update is suppressed, and witnesses see a frozen avatar with
    /// no error anywhere). Refusing is visible and recoverable; arriving is
    /// neither.
    pub fn is_usable(&self) -> bool {
        self.source != ArrivalSource::UnrecoverableOffMesh
    }
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
    let navmesh = containment_navmesh(space_mgr, world_name);
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
    match check_arrival_with(navmesh, desired) {
        ArrivalCheck::Unvalidated => {
            // Not an error — most worlds have no mesh. Logged so the
            // indefinitely-unvalidated destinations stay queryable rather
            // than invisible.
            tracing::debug!(
                world_name = %world_name,
                reason = "no_navmesh",
                "arrival: destination has no navmesh this check may act on (none \
                 resident, or the world is 'advisory') — accepting the authored \
                 arrival unvalidated"
            );
            return ResolvedArrival {
                position: desired,
                yaw,
                source: ArrivalSource::Unvalidated,
            };
        }
        ArrivalCheck::Validated => {
            return ResolvedArrival {
                position: desired,
                yaw,
                source: ArrivalSource::Validated,
            };
        }
        ArrivalCheck::OffMesh => {}
    }

    // `OffMesh` is only ever returned when a mesh was present, so this binds
    // by construction; the `else` is the compiler's price for not unwrapping.
    let Some(nav) = navmesh else {
        return ResolvedArrival {
            position: desired,
            yaw,
            source: ArrivalSource::Unvalidated,
        };
    };
    let desired_v = Vector3::new(desired[0], desired[1], desired[2]);
    let bounds = SpaceBounds::new(nav.bmin, nav.bmax);
    // Nearest-to-desired, through the one helper
    // `SpaceManager::resolve_recovery_position` also calls: two orderings (or
    // two zero-coordinate policies) for the same candidate class in one
    // codebase is how the two paths end up disagreeing about the same world.
    let fallback = nearest_valid_respawner(respawners, world_name, desired_v, Some(nav), &bounds);

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
                 no usable respawner exists for the world — there is no \
                 standable point to arrive on, so the caller must refuse the \
                 transfer (see ResolvedArrival::is_usable); seed a respawner \
                 for this world or re-pin the authored coordinate"
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
    // A startup space implies a world definition, and without one the
    // navmesh-mode lookup has nothing to read: `stamp_world_rows` walks
    // `worlds`, so a grafted space whose world is absent from that table
    // would silently stay `Enforce` however it was stamped, and a test that
    // set it advisory would assert against the mode it was trying to change.
    space_mgr
        .worlds
        .entry(world_name.to_string())
        .or_insert_with(|| super::space_manager::WorldDef {
            world_name: world_name.to_string(),
            world_id: None,
            navmesh_mode: super::space_manager::NavmeshMode::default(),
            instanced: false,
            min_x: -4000,
            max_x: 4000,
            min_y: -4000,
            max_y: 4000,
        });
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
        assert!(
            !out.is_usable(),
            "an unrecoverable arrival must never be reported as usable — the \
             PR #662 review found both callers shipping it into a transfer"
        );
        assert_ne!(
            out.position,
            [0.0, 0.0, 0.0],
            "a placeholder respawner row must not become an arrival"
        );
    }

    /// The three recoverable variants are usable; the fourth is not. Pinned
    /// as a table because `is_usable` is the only thing standing between the
    /// unrecoverable case and a `GateTravel` / ring teleport, and a future
    /// variant added without a matching arm here would silently become
    /// "usable".
    #[test]
    fn only_the_unrecoverable_source_is_refused() {
        let Some(mesh) = fixture_mesh() else { return };
        let on_mesh = [respawner("Castle_CellBlock", ON_MESH)];

        // Validated.
        assert!(
            resolve_arrival_with(Some(&mesh), &[], "Castle_CellBlock", ON_MESH, 0.0).is_usable()
        );
        // Respawner.
        assert!(
            resolve_arrival_with(Some(&mesh), &on_mesh, "Castle_CellBlock", OFF_MESH, 0.0)
                .is_usable()
        );
        // Unvalidated.
        assert!(resolve_arrival_with(None, &[], "Harset_Market", OFF_MESH, 0.0).is_usable());
        // UnrecoverableOffMesh.
        assert!(
            !resolve_arrival_with(Some(&mesh), &[], "Castle_CellBlock", OFF_MESH, 0.0).is_usable()
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
        // `position` still echoes the input — it is the *rejected* point, kept
        // so the caller's warn can name it. It is NOT an arrival: `is_usable`
        // is false and both callers refuse the transfer.
        assert_eq!(out.position, OFF_MESH);
        assert!(!out.is_usable());
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

    /// The advisory half of the arrival contract, on the real
    /// `harset.nav`: the same coordinate is `OffMesh` in an enforcing
    /// world and `Unvalidated` in an advisory one.
    ///
    /// This runs through [`check_arrival`] — the `SpaceManager` entry
    /// point — rather than [`check_arrival_with`], because the mode lookup
    /// is precisely the step the pure core does not have. It is also the
    /// seam both ring-transport consumers reach: `runtime::tick` aborts a
    /// trip on `OffMesh` and `audit_ring_pads` reports one at startup, so
    /// routing this one function fixes all three call sites at once.
    ///
    /// The Command-Center return coordinate is the fixture on purpose: it
    /// is a real authored point (chain 6007) that the mesh does not cover,
    /// and under `enforce` Harset has no seeded respawner to recover to,
    /// which is what turned an off-mesh arrival there into a silent freeze.
    #[test]
    fn an_advisory_destination_is_unvalidated_not_off_mesh() {
        let path = std::path::Path::new("../../data/spaces/harset.nav");
        if !path.exists() {
            return; // fixture-less checkout: skip
        }
        let mesh = NavMesh::load(path).expect("load data/spaces/harset.nav");

        // Controls, before any verdict: the mesh loaded and answers `true`
        // for a coordinate the seed already stands a player on, and `false`
        // for the one under test. Without the first, `OffMesh` below would
        // be produced by an empty mesh rather than by the geometry.
        assert!(
            mesh.is_point_valid(&Vector3::new(-25.641, -67.828, 15.249)),
            "control: ring pad 4 must read on-mesh or the mesh did not load",
        );
        let door = [0.0_f32, -67.6, -231.0];
        assert!(
            !mesh.is_point_valid(&Vector3::new(door[0], door[1], door[2])),
            "control: the Command Center return point must read off-mesh -- \
             it is the fixture the two verdicts below differ on",
        );

        let mut mgr = crate::test_support::make_space_manager();
        test_insert_navmesh_space(&mut mgr, "Harset", mesh);

        // Unstamped: `NavmeshMode::Enforce` by default, which is the
        // pre-H53 behaviour and the reason chain 6007 ships disabled.
        assert_eq!(
            check_arrival(&mgr, "Harset", door),
            ArrivalCheck::OffMesh,
            "an enforcing world must still refuse an off-mesh arrival",
        );

        mgr.stamp_world_rows(&std::collections::HashMap::from([(
            "Harset".to_string(),
            crate::cell::spawner::WorldRow {
                world_id: 57,
                navmesh_mode: crate::cell::space_manager::NavmeshMode::Advisory,
            },
        )]));
        assert_eq!(
            check_arrival(&mgr, "Harset", door),
            ArrivalCheck::Unvalidated,
            "an advisory world's mesh may not refuse an arrival -- the floor \
             is real even where the mesh is not, and refusing here is the \
             same defect as snapping a walking player back, one event earlier",
        );

        // And the substituting flavour agrees: no respawner is consulted,
        // the authored coordinate stands.
        let out = resolve_arrival(&mgr, "Harset", door, 1.25);
        assert_eq!(out.source, ArrivalSource::Unvalidated);
        assert_eq!(out.position, door);
        assert!(out.is_usable());
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
