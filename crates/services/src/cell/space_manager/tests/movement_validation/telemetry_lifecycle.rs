//! Release of the per-entity movement-telemetry state, on **every**
//! teardown path.
//!
//! Two failure modes, both invisible from the outside:
//!
//! - **Leak.** The maps are keyed by `entity_id` and nothing bounds them
//!   except the release call, so a path that removes entities without
//!   releasing grows them for the process lifetime.
//! - **Stale window.** `entity_id`s are recycled. An id that inherits a
//!   predecessor's open throttle window has its *first* occurrence
//!   silently swallowed — the one row an incident timeline most needs.
//!
//! The reporting-side guards are in [`super::telemetry_reject`] and
//! [`super::telemetry_sampling`].

use std::time::{Duration, Instant};

use crate::cell::{EntityId, SpaceId};
use cimmeria_common::Vector3;
use cimmeria_entity::cell_entity::CellEntity;
use cimmeria_entity::movement_validation::MovementReject;

use super::super::super::SpaceManager;
use super::make_manager;
use super::telemetry_reject::{player_in, report_for, SPAWN};

/// Put an NPC into an existing space the way
/// `spawn_instance_npcs_from_records` does — directly, without
/// `create_entity` picking the space.
fn add_npc(mgr: &mut SpaceManager, entity_id: u32, space_id: u32) {
    let pos = Vector3::new(10.0, 0.0, 10.0);
    let mut npc = CellEntity::new(EntityId(entity_id as i32), SpaceId(space_id as i32), pos);
    npc.is_player = false;
    let space = mgr.spaces.get_mut(&space_id).unwrap();
    space.space.add_entity(EntityId(entity_id as i32), &pos);
    space.entities.insert(entity_id, npc);
    mgr.entity_space.insert(entity_id, space_id);
}

/// Throttle state is released on the per-entity teardown, so it neither
/// leaks nor lets a recycled `entity_id` inherit a predecessor's open
/// window.
#[test]
fn telemetry_state_is_released_when_the_entity_is_destroyed() {
    let mut mgr = make_manager();
    let space_id = player_in(&mut mgr, 7006, "Agnos");
    mgr.report_movement_reject(
        report_for(7006, space_id, MovementReject::OutOfBounds),
        Instant::now(),
    );
    mgr.sample_accepted_position_at(7006, SPAWN, Instant::now());
    assert_eq!(
        mgr.movement_telemetry.tracked(),
        2,
        "precondition: both the reject throttle and the position sample \
         must have a slot, or this test cannot prove they are both freed"
    );

    mgr.destroy_entity(7006);
    assert_eq!(
        mgr.movement_telemetry.tracked(),
        0,
        "every per-entity telemetry slot must be dropped with the entity \
         — dropping only some of them is how the next id reuse inherits \
         a stale throttle window"
    );
}

/// **The `destroy_space` leak guard.** When the last player leaves an
/// instanced space, every remaining NPC is torn down by `destroy_space`
/// directly — `destroy_entity` never runs for them. Releasing telemetry
/// only in `destroy_entity` therefore leaked one `npc_path_fail_log`
/// slot per NPC per instance, unbounded over uptime, and nothing in the
/// suite noticed because the entity is gone either way.
///
/// Reverting the `movement_telemetry.forget(eid)` line in
/// `destroy_space` leaves `tracked() == 1` here.
#[test]
fn destroying_a_space_releases_its_npcs_telemetry_state() {
    let mut mgr = make_manager();
    // Castle_CellBlock is instanced in the test spaces XML, so the last
    // player leaving takes the whole space with it.
    let space_id = player_in(&mut mgr, 7101, "Castle_CellBlock");
    add_npc(&mut mgr, 7102, space_id);

    // Give the NPC a path-failure throttle slot — the map the review
    // named, and the one only `destroy_space` can release for an NPC in
    // an instance.
    mgr.movement_telemetry.npc_path_fail_log.admit(
        7102,
        "path_fail",
        Instant::now(),
        Duration::from_secs(5),
    );
    assert_eq!(
        mgr.movement_telemetry.tracked(),
        1,
        "precondition: the NPC must hold a throttle slot"
    );

    mgr.destroy_entity(7101); // last player → destroy_space
    assert!(
        !mgr.spaces.contains_key(&space_id),
        "precondition: the instance must actually have been destroyed"
    );
    assert_eq!(
        mgr.movement_telemetry.tracked(),
        0,
        "an NPC removed by destroy_space must have its telemetry state \
         released too — otherwise the map is bounded by total NPCs ever \
         spawned across every instance, not by the live population"
    );
}

/// **The stale-window guard.** The leak above is the cheap half; this is
/// the one that corrupts data. A recycled `entity_id` that inherits an
/// open throttle window has its first occurrence suppressed, so the row
/// that says "this started now" never gets written.
///
/// Reverting the `destroy_space` cleanup makes the second `admit`
/// return `None` instead of `Some(0)`.
#[test]
fn a_reused_entity_id_does_not_inherit_a_destroyed_spaces_throttle_window() {
    let mut mgr = make_manager();
    let t0 = Instant::now();
    let window = Duration::from_secs(5);

    let space_id = player_in(&mut mgr, 7103, "Castle_CellBlock");
    add_npc(&mut mgr, 7104, space_id);
    assert_eq!(
        mgr.movement_telemetry
            .npc_path_fail_log
            .admit(7104, "path_fail", t0, window),
        Some(0),
        "precondition: the first occurrence for a fresh id emits"
    );
    mgr.destroy_entity(7103); // last player → destroy_space takes the NPC

    // A new instance, and the id comes back around onto a different NPC
    // well inside the old window.
    let space_id = player_in(&mut mgr, 7105, "Castle_CellBlock");
    add_npc(&mut mgr, 7104, space_id);
    assert_eq!(
        mgr.movement_telemetry.npc_path_fail_log.admit(
            7104,
            "path_fail",
            t0 + Duration::from_millis(10),
            window
        ),
        Some(0),
        "the new occupant of a recycled id must emit its first path \
         failure — inheriting the predecessor's window swallows exactly \
         the row that says when the problem started"
    );
}
