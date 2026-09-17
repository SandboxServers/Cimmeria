//! Validate-before-teardown contract.
//!
//! The primitive's whole reason to exist is that the *cell* half of gate
//! travel is where the destructive step lives (`destroy_entity`), so the cell
//! half is where validation has to happen. Every test here injects one
//! failure and asserts the subject's origin space, position and player flag
//! are byte-identical afterwards, and that nothing reached the base channel.

use super::*;

#[tokio::test]
async fn unknown_world_is_rejected_before_any_teardown() {
    let mut mgr = make_manager();
    spawn_player(&mut mgr, 1, AGNOS, [10.0, 0.0, 20.0]);
    let before = snapshot_origin(&mgr, 1);
    let (tx, mut rx) = mpsc::channel(8);

    let err = transfer_player_to_space(
        1,
        &TransferDestination::in_world("NotAWorld", [1.0, 2.0, 3.0]),
        &tx,
        &mut mgr,
    )
    .await
    .expect_err("an unknown world must be refused");

    assert_eq!(err, TransferRejected::UnknownWorld("NotAWorld".to_string()));
    assert_origin_untouched(&before, &mgr, 1, &mut rx);
}

/// A world that exists in `spaces.xml` but is non-instanced and has no
/// startup space can never be entered. If the transfer let it through, the
/// base's `CreateEntity` would fail *after* teardown and strand the player in
/// no space at all — so it has to be a distinct, pre-teardown rejection
/// rather than collapsing into `UnknownWorld`.
#[tokio::test]
async fn known_but_unloadable_world_is_rejected_before_any_teardown() {
    let mut mgr = make_manager();
    spawn_player(&mut mgr, 1, AGNOS, [10.0, 0.0, 20.0]);
    let before = snapshot_origin(&mgr, 1);
    let (tx, mut rx) = mpsc::channel(8);

    let err = transfer_player_to_space(
        1,
        &TransferDestination::in_world(ORPHAN, [1.0, 2.0, 3.0]),
        &tx,
        &mut mgr,
    )
    .await
    .expect_err("a world with no loadable space must be refused");

    assert_eq!(err, TransferRejected::WorldNotLoadable(ORPHAN.to_string()));
    assert_origin_untouched(&before, &mgr, 1, &mut rx);
}

/// D15: an NPC has no client, so it cannot run the world-entry handshake a
/// cross-world transfer is built on. Transferring one would destroy it and
/// never rebuild it.
#[tokio::test]
async fn npc_subject_is_rejected_before_any_teardown() {
    let mut mgr = make_manager();
    // create_entity without connect_entity → is_player stays false.
    mgr.create_entity(500, AGNOS, [5.0, 0.0, 5.0], [0.0; 3])
        .unwrap();
    assert!(!mgr.get_entity(500).unwrap().is_player);
    let before = snapshot_origin(&mgr, 500);
    let (tx, mut rx) = mpsc::channel(8);

    let err = transfer_player_to_space(
        500,
        &TransferDestination::in_world(CASTLE, [1.0, 2.0, 3.0]),
        &tx,
        &mut mgr,
    )
    .await
    .expect_err("an NPC subject must be refused (D15)");

    assert_eq!(err, TransferRejected::NotAPlayer);
    assert_origin_untouched(&before, &mgr, 500, &mut rx);
}

#[tokio::test]
async fn missing_subject_entity_is_rejected() {
    let mut mgr = make_manager();
    let before = snapshot_origin(&mgr, 9_999);
    let (tx, mut rx) = mpsc::channel(8);

    let err = transfer_player_to_space(
        9_999,
        &TransferDestination::in_world(CASTLE, [1.0, 2.0, 3.0]),
        &tx,
        &mut mgr,
    )
    .await
    .expect_err("a missing subject must be refused");

    assert_eq!(err, TransferRejected::EntityNotFound);
    assert_origin_untouched(&before, &mgr, 9_999, &mut rx);
}

/// NaN/inf coordinates would land in the destination spatial grid and in the
/// `sgw_player` position columns the base persists on gate travel. Rejected
/// at the boundary rather than sanitised — the same rule the native
/// `gmGotoXYZ`/`gmGotoLocation` handlers apply.
#[tokio::test]
async fn non_finite_destination_is_rejected_before_any_teardown() {
    for bad in [
        [f32::NAN, 0.0, 0.0],
        [0.0, f32::INFINITY, 0.0],
        [0.0, 0.0, f32::NEG_INFINITY],
    ] {
        let mut mgr = make_manager();
        spawn_player(&mut mgr, 1, AGNOS, [10.0, 0.0, 20.0]);
        let before = snapshot_origin(&mgr, 1);
        let (tx, mut rx) = mpsc::channel(8);

        let err = transfer_player_to_space(
            1,
            &TransferDestination::in_world(CASTLE, bad),
            &tx,
            &mut mgr,
        )
        .await
        .expect_err("non-finite coordinates must be refused");

        assert_eq!(err, TransferRejected::NonFinitePosition, "for {bad:?}");
        assert_origin_untouched(&before, &mgr, 1, &mut rx);
    }
}

#[tokio::test]
async fn non_finite_rotation_is_rejected_before_any_teardown() {
    let mut mgr = make_manager();
    spawn_player(&mut mgr, 1, AGNOS, [10.0, 0.0, 20.0]);
    let before = snapshot_origin(&mgr, 1);
    let (tx, mut rx) = mpsc::channel(8);

    let mut dest = TransferDestination::in_world(CASTLE, [1.0, 2.0, 3.0]);
    dest.rotation = [0.0, 0.0, f32::NAN];

    let err = transfer_player_to_space(1, &dest, &tx, &mut mgr)
        .await
        .expect_err("non-finite rotation must be refused");

    assert_eq!(err, TransferRejected::NonFinitePosition);
    assert_origin_untouched(&before, &mgr, 1, &mut rx);
}

/// A closed base channel is the one failure that happens *after* validation.
/// The entity must still be left in place: destroying it here would produce a
/// player who is in no space and has no transfer in flight — recoverable only
/// by relogging.
#[tokio::test]
async fn closed_base_channel_leaves_the_entity_in_place() {
    let mut mgr = make_manager();
    spawn_player(&mut mgr, 1, AGNOS, [10.0, 0.0, 20.0]);
    let before = snapshot_origin(&mgr, 1);

    let (tx, rx) = mpsc::channel(8);
    drop(rx); // base side is gone

    let err = transfer_player_to_space(
        1,
        &TransferDestination::in_world(CASTLE, [1.0, 2.0, 3.0]),
        &tx,
        &mut mgr,
    )
    .await
    .expect_err("a closed base channel must be refused");

    assert_eq!(err, TransferRejected::EnqueueFailed);
    assert_eq!(
        before,
        snapshot_origin(&mgr, 1),
        "a failed enqueue must leave the entity in its origin space — \
         destroying it here strands the player with no transfer in flight"
    );
}

/// The happy path's mirror image of the tests above: once validation passes
/// and the send is confirmed, the entity IS torn out. Without this, every
/// "origin untouched" assertion above would pass trivially for a primitive
/// that never tears anything down.
#[tokio::test]
async fn accepted_transfer_tears_the_entity_out_and_emits_gate_travel() {
    let mut mgr = make_manager();
    let origin = spawn_player(&mut mgr, 1, AGNOS, [10.0, 0.0, 20.0]);
    let destination = mgr.space_id_for_world(CASTLE).unwrap();
    assert_ne!(origin, destination, "fixture must cross a space boundary");
    let (tx, mut rx) = mpsc::channel(8);

    let outcome = transfer_player_to_space(
        1,
        &TransferDestination::in_world(CASTLE, [111.0, 222.0, 333.0]),
        &tx,
        &mut mgr,
    )
    .await
    .expect("a valid cross-world transfer must be accepted");

    assert_eq!(
        outcome,
        TransferOutcome::Transferred {
            space_id: Some(destination)
        }
    );
    assert!(
        mgr.get_entity(1).is_none(),
        "an accepted transfer must remove the entity from its origin space"
    );

    match expect_gate_travel(&mut rx) {
        CellToBaseMsg::GateTravel {
            entity_id,
            target_world_name,
            position,
            destination_ring_id,
            destination_space_id,
            ..
        } => {
            assert_eq!(entity_id, 1);
            assert_eq!(target_world_name, CASTLE);
            assert_eq!(position, [111.0, 222.0, 333.0]);
            assert_eq!(
                destination_ring_id, None,
                "GM travel is never a ring transport"
            );
            assert_eq!(destination_space_id, Some(destination));
        }
        other => panic!("expected GateTravel, got {other:?}"),
    }
}

/// The destination resolving to the entity's current space is not a transfer
/// at all. Reporting `SameSpace` (instead of driving a full RESET_ENTITIES
/// reload) lets the command adapters fall back to the cheap authoritative
/// snap, and it must not tear anything down.
#[tokio::test]
async fn destination_equal_to_current_space_reports_same_space_without_teardown() {
    let mut mgr = make_manager();
    let origin = spawn_player(&mut mgr, 1, AGNOS, [10.0, 0.0, 20.0]);
    let before = snapshot_origin(&mgr, 1);
    let (tx, mut rx) = mpsc::channel(8);

    let outcome = transfer_player_to_space(
        1,
        &TransferDestination::in_instance(AGNOS, origin, [1.0, 2.0, 3.0]),
        &tx,
        &mut mgr,
    )
    .await
    .expect("same-space destination is not an error");

    assert_eq!(outcome, TransferOutcome::SameSpace { space_id: origin });
    assert_origin_untouched(&before, &mgr, 1, &mut rx);
}
