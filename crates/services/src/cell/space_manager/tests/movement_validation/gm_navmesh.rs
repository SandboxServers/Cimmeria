//! The GM off-navmesh allowance.
//!
//! Split out of `recovery.rs` at its own seam once that file crossed the
//! 700-line hard cap, the same way `onphysics` came out of the parent
//! module. The sibling file proves a bad client position stops being
//! *corrected*; this one proves a **GM** is allowed to stand somewhere an
//! ordinary player would be snapped back from, and that the allowance
//! survives both the accept and the reject path.
//!
//! The allowance keys on `CellEntity::access_level`, which is read from
//! `account.accesslevel` at login and is never a client-supplied byte — the
//! same trust model as `cell::dispatch::gm_gate`. It is navmesh-only:
//! bounds and teleport stay hard-rejecting for GMs.
//!
//! Distinct from `movement_unrestricted` (`onPhysics` / `/gmsetfly`), which
//! bypasses every layer and needs an explicit in-game command — that is
//! `onphysics`'s subject.

use std::time::{Duration, Instant};

use cimmeria_common::Vector3;
use cimmeria_entity::movement_validation::MovementReject;
use tracing::Level;

use crate::cell::space_manager::ClientMoveOutcome;
use crate::test_support::LogCapture;

// The navmesh fixture lives with the recovery suite, which is its heaviest
// user; these tests are its second consumer rather than its owner.
use super::recovery::{navmesh_manager, nearby_off_mesh_point, ON_MESH};

/// A GM is allowed off the walkable mesh: inspecting a broken spawn or an
/// unreachable region means standing where a player cannot. The allowance
/// keys on `CellEntity::access_level`, which comes from
/// `account.accesslevel` at login and is never a client-supplied byte.
///
/// Pairs with `off_navmesh_position_is_rejected_and_not_observed` in the
/// parent module, which proves the same position **is** rejected for an
/// ordinary player — reverting the gate makes these two disagree.
#[test]
fn gm_off_navmesh_position_is_accepted_not_snapped() {
    let Some((mut mgr, space_id, bmin, bmax)) = navmesh_manager() else {
        return; // fixture-less CI — skip
    };
    // GameMaster — the same level `cell::dispatch::gm_gate` requires.
    mgr.get_entity_mut(100).unwrap().access_level = 2;

    let Some(off_mesh) = nearby_off_mesh_point(&mgr, bmin, bmax) else {
        return; // no unwalkable point nearby — nothing to assert against
    };
    assert!(
        !mgr.is_position_valid(100, &Vector3::new(off_mesh[0], off_mesh[1], off_mesh[2])),
        "scanned point must read as off-navmesh — precondition for the allowance"
    );

    let outcome =
        mgr.apply_client_position_update_at(Instant::now(), 100, off_mesh, [0, 0, 0], [0.0; 3]);
    assert!(
        matches!(outcome, ClientMoveOutcome::Accepted { position } if position == off_mesh),
        "an off-navmesh position from a GM caller must be accepted, got {outcome:?}"
    );
    let entity = &mgr.spaces[&space_id].entities[&100];
    assert_eq!(
        entity.position,
        Vector3::new(off_mesh[0], off_mesh[1], off_mesh[2]),
        "the GM's position must be written through so witnesses follow them off-mesh"
    );
}

/// The allowance has to survive the *reject* path too, not just the accept
/// path — otherwise it lasts only until the GM trips something unrelated.
///
/// Bounds and teleport stay enforced for GMs, by design. When one of them
/// fires while the GM is standing legitimately off-mesh, `reject_outcome`
/// asks whether the GM's own position is a sound snap target. Ask that
/// question without the GM allowance and the answer is "no" — so the GM is
/// routed into recovery and force-relocated onto the nearest walkable point,
/// undoing by the back door exactly what the accept path just granted. A GM
/// inspecting an unreachable region would be yanked back to the floor by any
/// stray out-of-bounds packet.
///
/// The correct outcome is the ordinary `Rejected`: a harmless no-op snap back
/// to where the GM already legitimately is.
#[test]
fn gm_off_navmesh_is_not_relocated_by_an_unrelated_reject() {
    let Some((mut mgr, space_id, bmin, bmax)) = navmesh_manager() else {
        return; // fixture-less CI — skip
    };
    mgr.get_entity_mut(100).unwrap().access_level = 2;
    let Some(off_mesh) = nearby_off_mesh_point(&mgr, bmin, bmax) else {
        return; // no unwalkable point nearby — nothing to assert against
    };

    let t0 = Instant::now();
    let moved = mgr.apply_client_position_update_at(t0, 100, off_mesh, [0, 0, 0], [0.0; 3]);
    assert!(
        matches!(moved, ClientMoveOutcome::Accepted { .. }),
        "precondition: the GM must first get off-mesh through the allowance, got {moved:?}"
    );

    // Now trip a hard reject that has nothing to do with the navmesh: a
    // position outside the space AABB, still enforced for GMs.
    let oob = [bmin[0] - 5_000.0, ON_MESH[1], ON_MESH[2]];
    let outcome = mgr.apply_client_position_update_at(
        t0 + Duration::from_millis(100),
        100,
        oob,
        [0, 0, 0],
        [0.0; 3],
    );
    assert!(
        matches!(
            outcome,
            ClientMoveOutcome::Rejected {
                reason: MovementReject::OutOfBounds,
                last_valid,
                ..
            } if last_valid == off_mesh
        ),
        "a GM who trips an unrelated reject while off-mesh must get the plain \
         correction back to where they already are — being `Recovered` onto the \
         mesh instead defeats the off-navmesh allowance; got {outcome:?}"
    );
    let entity = &mgr.spaces[&space_id].entities[&100];
    assert_eq!(
        entity.position,
        Vector3::new(off_mesh[0], off_mesh[1], off_mesh[2]),
        "the reject must leave the GM exactly where they were standing"
    );
}

/// The GM off-navmesh allowance is the one place the validator accepts a
/// position it would snap an ordinary player back from, so its warn is an
/// audit line: it has to name who the GM is, not which slot they occupy.
#[test]
fn gm_off_navmesh_bypass_warning_names_the_account() {
    let Some((mut mgr, _space_id, bmin, bmax)) = navmesh_manager() else {
        return; // fixture-less CI — skip
    };
    mgr.get_entity_mut(100).unwrap().access_level = 2;
    mgr.get_entity_mut(100).unwrap().account_id = Some(6);
    mgr.get_entity_mut(100).unwrap().player_id = Some(12);
    let Some(off_mesh) = nearby_off_mesh_point(&mgr, bmin, bmax) else {
        return;
    };

    let capture = LogCapture::install();
    let outcome =
        mgr.apply_client_position_update_at(Instant::now(), 100, off_mesh, [0, 0, 0], [0.0; 3]);
    assert!(
        matches!(outcome, ClientMoveOutcome::Accepted { .. }),
        "precondition: the allowance must accept the move, got {outcome:?}"
    );

    let event = capture
        .find_event(
            Level::WARN,
            "movement.navmesh_gm_bypass",
            "navmesh_gm_bypass",
        )
        .expect("the GM bypass warn must still fire");
    assert!(
        event.has_field("account_id", "6") && event.has_field("player_id", "12"),
        "the bypass is a privileged-action audit line and must name the \
         account; entity_id alone is a recycled per-space slot; got {event:#?}"
    );
    assert!(
        event.has_field("world", "Castle_CellBlock"),
        "every `movement.validation` row whose space id resolves carries \
         `world` — a dashboard filtered by world must not silently drop \
         the GM allowance rows; got {event:#?}"
    );
}
