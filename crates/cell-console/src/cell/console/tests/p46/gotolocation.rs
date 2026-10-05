//! `.gotolocation <worldName> <x> <y> <z>` — explicit coordinates in a named
//! world.
//!
//! Legacy `deprecated/python/cell/commands/Player.py:344-365`.

use cimmeria_common::Vector3;

use super::*;

/// Legacy `Player.py:359-362`: an unknown world reports `"Unable to find
/// world: %s"` verbatim and nothing is torn down.
#[tokio::test]
async fn legacy_p46_gotolocation_unknown_world_reports_legacy_wording() {
    let (mut mgr, gm, _npc) = setup_worlds();
    let before = position_of(&mgr, gm);
    let space = mgr.get_entity_space_id(gm);

    let t = run(
        "gotolocation",
        gm,
        &[UNKNOWN_WORLD, "10", "0", "10"],
        None,
        &mut mgr,
    )
    .await;

    assert!(
        t.has_line(&format!("Unable to find world: {UNKNOWN_WORLD}")),
        "legacy's exact unknown-world wording, unprefixed; got {:?}",
        t.feedback
    );
    assert_no_move(&t);
    assert_eq!(position_of(&mgr, gm), before);
    assert_eq!(
        mgr.get_entity_space_id(gm),
        space,
        "an unknown world must be caught before any teardown"
    );
}

/// Naming the world the subject is already in is a snap, not a loading
/// screen, and the feedback quotes the subject's name (legacy
/// `entity.getName()`).
#[tokio::test]
async fn legacy_p46_gotolocation_same_world_snaps_without_gate_travel() {
    let (mut mgr, gm, _npc) = setup_worlds();
    let before = position_of(&mgr, gm);
    let agnos = mgr.get_entity_space_id(gm).unwrap();

    let t = run(
        "gotolocation",
        gm,
        &[AGNOS, "40", "0", "40"],
        None,
        &mut mgr,
    )
    .await;

    assert!(
        t.gate_travels.is_empty(),
        "moving within the current world must not enqueue a transfer: {:?}",
        t.gate_travels
    );
    assert_eq!(t.teleports, vec![(gm, agnos, [40.0, 0.0, 40.0], before)]);
    assert_eq!(position_of(&mgr, gm), [40.0, 0.0, 40.0]);
    assert!(
        t.has_line("Moving entity Vala to Agnos (40, 0, 40)"),
        "the feedback line must name the subject and the world; got {:?}",
        t.feedback
    );
}

/// Cross-world: D15's first/default loaded instance of the named world.
#[tokio::test]
async fn legacy_p46_gotolocation_cross_world_uses_the_default_instance() {
    let (mut mgr, gm, _npc) = setup_worlds();
    let castle = mgr
        .default_space_for_world(CASTLE)
        .expect("Castle has a startup space");

    let t = run(
        "gotolocation",
        gm,
        &[CASTLE, "70", "1", "80"],
        None,
        &mut mgr,
    )
    .await;

    assert_eq!(
        t.only_gate_travel(),
        &(gm, CASTLE.to_string(), Some(castle), [70.0, 1.0, 80.0]),
        "the transfer must name Castle's default instance and the exact coordinates"
    );
    assert_eq!(
        mgr.get_entity_space_id(gm),
        None,
        "a cross-world move tears the subject out of its origin space"
    );
}

/// Regression guard (found by CodeRabbit review): `TransferDestination::
/// in_world`/`in_instance` default `rotation` to `[0.0; 3]`, and that value
/// flows through `GateTravel` into the destination entity's `direction`
/// unconditionally — so a cross-world `.gotolocation` must carry the
/// subject's *actual* current facing, not the zero default, or the subject
/// arrives facing an arbitrary direction. A distinctive non-zero pitch/roll
/// (not just yaw) so a regression that only preserves `.y` still fails.
#[tokio::test]
async fn legacy_p46_gotolocation_cross_world_preserves_subject_facing() {
    let (mut mgr, gm, _npc) = setup_worlds();
    mgr.get_entity_mut(gm).unwrap().direction = Vector3::new(0.3, 2.1, -0.5);

    let t = run(
        "gotolocation",
        gm,
        &[CASTLE, "70", "1", "80"],
        None,
        &mut mgr,
    )
    .await;

    assert_eq!(
        t.only_gate_travel_rotation(),
        [0.3, 2.1, -0.5],
        "the GateTravel destination must carry the subject's actual facing, not the zero default"
    );
}

/// The caller is in instance B of an instanced world and names *that same
/// world*. They must stay in B.
///
/// Regression shape: handing the world name straight to
/// `TransferDestination::in_world` resolves to the *oldest* loaded instance
/// (A here), so the GM would be yanked out of the instance they are running,
/// through a full loading screen, just to change coordinates.
#[tokio::test]
async fn legacy_p46_gotolocation_own_instanced_world_keeps_the_current_instance() {
    let (mut mgr, _gm, _npc) = setup_worlds();
    let instance_a = spawn_named_player(&mut mgr, 50, INSTANCED, [1.0, 0.0, 1.0], "Ana");
    let instance_b = spawn_named_player(&mut mgr, 51, INSTANCED, [2.0, 0.0, 2.0], "Bob");
    assert_ne!(instance_a, instance_b);
    assert_eq!(
        mgr.default_space_for_world(INSTANCED),
        Some(instance_a.min(instance_b)),
        "the default instance must be the OTHER one for this test to mean anything"
    );

    // Caller = Bob, in instance B.
    let t = run(
        "gotolocation",
        51,
        &[INSTANCED, "55", "0", "66"],
        None,
        &mut mgr,
    )
    .await;

    assert!(
        t.gate_travels.is_empty(),
        "staying in the same instance must not enqueue a transfer: {:?}",
        t.gate_travels
    );
    assert_eq!(
        t.teleports,
        vec![(51, instance_b, [55.0, 0.0, 66.0], [2.0, 0.0, 2.0])],
        "the snap must happen in instance B"
    );
    assert_eq!(
        mgr.get_entity_space_id(51),
        Some(instance_b),
        "the caller must still be in instance B"
    );
}

/// DA-F4 (deliberate departure from legacy `target or player`): with an NPC
/// selected, `.gotolocation` in the caller's own world moves the **caller**
/// and leaves the selection where it stands. Live, the legacy rule moved the
/// Debug Area's friendly dummy, because the server kept a target the client
/// had cleared with Escape. Restoring `target.unwrap_or(caller_id)` moves the
/// NPC here and fails every assertion.
#[tokio::test]
async fn gotolocation_with_an_npc_selected_moves_the_caller_not_the_npc() {
    let (mut mgr, gm, npc) = setup_worlds();
    let npc_before = position_of(&mgr, npc);
    let gm_before = position_of(&mgr, gm);
    let agnos = mgr.get_entity_space_id(gm).unwrap();

    let t = run(
        "gotolocation",
        gm,
        &[AGNOS, "44", "0", "45"],
        Some(npc),
        &mut mgr,
    )
    .await;

    assert_eq!(position_of(&mgr, gm), [44.0, 0.0, 45.0]);
    assert_eq!(t.teleports, vec![(gm, agnos, [44.0, 0.0, 45.0], gm_before)]);
    assert_eq!(
        position_of(&mgr, npc),
        npc_before,
        "the selection must not move"
    );
    assert!(
        t.has_line("Moving entity Vala to Agnos (44, 0, 45)"),
        "the feedback names the caller; got {:?}",
        t.feedback
    );
}

/// The same across worlds: an NPC selection used to be refused for a
/// cross-world move (D15, NPCs have no client to transfer). Now the caller
/// travels and the NPC stays put.
#[tokio::test]
async fn gotolocation_cross_world_with_an_npc_selected_transfers_the_caller() {
    let (mut mgr, gm, npc) = setup_worlds();
    let npc_before = position_of(&mgr, npc);
    let npc_space = mgr.get_entity_space_id(npc);
    let castle = mgr
        .default_space_for_world(CASTLE)
        .expect("Castle has a startup space");

    let t = run(
        "gotolocation",
        gm,
        &[CASTLE, "70", "1", "80"],
        Some(npc),
        &mut mgr,
    )
    .await;

    assert_eq!(
        t.only_gate_travel(),
        &(gm, CASTLE.to_string(), Some(castle), [70.0, 1.0, 80.0])
    );
    assert!(!t.mentions("not a player"), "got {:?}", t.feedback);
    assert_eq!(position_of(&mgr, npc), npc_before);
    assert_eq!(mgr.get_entity_space_id(npc), npc_space);
}

/// Non-finite coordinates are rejected by the shared `parse_f32` filter,
/// before either move mechanism is reached.
#[tokio::test]
async fn legacy_p46_gotolocation_rejects_non_finite_coordinates() {
    for bad in ["NaN", "inf", "-inf"] {
        let (mut mgr, gm, _npc) = setup_worlds();
        let before = position_of(&mgr, gm);

        let t = run("gotolocation", gm, &[CASTLE, bad, "0", "0"], None, &mut mgr).await;

        assert_no_move(&t);
        assert_eq!(
            position_of(&mgr, gm),
            before,
            "{bad} must not move anything"
        );
        assert!(
            t.mentions("finite"),
            "{bad} must feed back a finite-number rejection; got {:?}",
            t.feedback
        );
    }
}

/// Malformed (non-numeric) coordinates take the same path.
#[tokio::test]
async fn legacy_p46_gotolocation_rejects_malformed_coordinates() {
    let (mut mgr, gm, _npc) = setup_worlds();

    let t = run(
        "gotolocation",
        gm,
        &[CASTLE, "1", "abc", "3"],
        None,
        &mut mgr,
    )
    .await;

    assert_no_move(&t);
    assert!(
        t.mentions("y must be"),
        "the offending argument must be named; got {:?}",
        t.feedback
    );
}
