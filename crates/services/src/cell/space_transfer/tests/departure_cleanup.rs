//! Per-entity state that a transfer has to deal with on the way out.
//!
//! `destroy_entity` does **not** clean either of these up, which is why the
//! two lifecycle arms (`DestroyEntity` / `DisconnectEntity`) call the helpers
//! explicitly. A forced GM transfer is just as much a departure, and both
//! helpers stop working the moment the entity is gone — so ordering is the
//! whole test.
//!
//! The mirror-image property (a *rejected* transfer must touch neither) is
//! asserted by `assert_origin_untouched` in the validation suite, whose
//! `OriginState` snapshot covers the dirty-ammo marker and the trade partner.

use super::*;

/// The accepted path must flush pending bandolier ammo before the entity is
/// torn down — the cross-world respawn rebuilds it from the DB, so anything
/// still marked dirty is lost.
///
/// The ordering assertion is the point: the flush's `BandolierAmmoUpdate` has
/// to reach the channel *before* the `GateTravel` that commits the transfer,
/// because `flush_dirty_bandolier_ammo` only clears a dirty marker after its
/// own send succeeds.
#[tokio::test]
async fn accepted_transfer_flushes_dirty_ammo_before_the_gate_travel() {
    let mut mgr = make_manager();
    spawn_player(&mut mgr, 1, AGNOS, [10.0, 0.0, 20.0]);
    assert!(
        ammo_still_dirty(&mgr, 1),
        "fixture must start with a pending ammo write, or this test proves nothing"
    );
    let (tx, mut rx) = mpsc::channel(16);

    transfer_player_to_space(
        1,
        &TransferDestination::in_world(CASTLE, [1.0, 2.0, 3.0]),
        &tx,
        &mut mgr,
    )
    .await
    .expect("transfer accepted");

    let mut order = Vec::new();
    while let Ok(msg) = rx.try_recv() {
        match msg {
            CellToBaseMsg::BandolierAmmoUpdate {
                player_id,
                current_ammo,
                ..
            } => {
                assert_eq!(
                    player_id,
                    fixture_player_id(1),
                    "the flush must be attributed to the traveller's own player_id"
                );
                assert_eq!(current_ammo, 17, "the seeded ammo value must be persisted");
                order.push("flush");
            }
            CellToBaseMsg::GateTravel { .. } => order.push("gate_travel"),
            _ => {}
        }
    }
    assert_eq!(
        order,
        vec!["flush", "gate_travel"],
        "the ammo flush must be enqueued before the transfer is committed"
    );
}

/// A rejected transfer must NOT flush. The flush is the only statement in the
/// primitive that mutates anything before the teardown phase, so if it ran on
/// a rejection the "origin state is completely unchanged" contract would be
/// false in exactly the place nobody looks.
#[tokio::test]
async fn rejected_transfer_does_not_flush_dirty_ammo() {
    let mut mgr = make_manager();
    spawn_player(&mut mgr, 1, AGNOS, [10.0, 0.0, 20.0]);
    let before = snapshot_origin(&mgr, 1);
    assert_eq!(
        before.ammo_dirty,
        Some(true),
        "fixture must start dirty, or the assertion below is vacuous"
    );
    let (tx, mut rx) = mpsc::channel(16);

    transfer_player_to_space(
        1,
        &TransferDestination::in_world("NotAWorld", [1.0, 2.0, 3.0]),
        &tx,
        &mut mgr,
    )
    .await
    .expect_err("unknown world is rejected");

    // `assert_origin_untouched` covers the dirty marker (via OriginState) and
    // the empty channel (so no BandolierAmmoUpdate escaped) in one shot.
    assert_origin_untouched(&before, &mgr, 1, &mut rx);
}

/// A forced transfer of a player who is mid-trade must cancel the trade first.
///
/// `destroy_entity` doesn't touch trade state, and
/// `cancel_trade_on_disconnect` early-returns as soon as `get_entity` misses —
/// so calling it after the teardown is a silent no-op. Skipping it leaves the
/// surviving partner holding a `trade_partner_entity_id` that points at a
/// freed entity id, with no `onTradeResults(Cancelled)`: a stranded session
/// that only a relog clears.
#[tokio::test]
async fn accepted_transfer_cancels_an_open_trade_before_teardown() {
    let mut mgr = make_manager();
    spawn_player(&mut mgr, 1, AGNOS, [10.0, 0.0, 20.0]);
    spawn_player(&mut mgr, 2, AGNOS, [11.0, 0.0, 20.0]);
    // Open trade between 1 (the traveller) and 2 (the partner left behind).
    mgr.get_entity_mut(1).unwrap().trade_partner_entity_id = Some(2);
    mgr.get_entity_mut(2).unwrap().trade_partner_entity_id = Some(1);

    let (tx, mut rx) = mpsc::channel(32);

    transfer_player_to_space(
        1,
        &TransferDestination::in_world(CASTLE, [1.0, 2.0, 3.0]),
        &tx,
        &mut mgr,
    )
    .await
    .expect("transfer accepted");

    assert_eq!(
        mgr.get_entity(2).unwrap().trade_partner_entity_id,
        None,
        "the partner left behind must have their trade state cleared — a dangling \
         partner id points at an entity that no longer exists"
    );

    // The partner has to be *told*, not just silently cleared, or their trade
    // window stays open client-side. Same assertion shape as the
    // `DisconnectEntity` guard in `base_messages::tests::trade_disconnect`.
    use crate::cell::client_methods::player::ON_TRADE_RESULTS;
    use cimmeria_entity::trade::ETRADERESULTS_CANCELLED;
    let mut survivor_got_cancelled = false;
    while let Ok(msg) = rx.try_recv() {
        if let CellToBaseMsg::EntityMethodCall {
            entity_id,
            method_index,
            args,
        } = msg
        {
            if entity_id == 2 && method_index == ON_TRADE_RESULTS && args.len() >= 8 {
                let result = i32::from_le_bytes([args[4], args[5], args[6], args[7]]);
                if result == ETRADERESULTS_CANCELLED {
                    survivor_got_cancelled = true;
                }
            }
        }
    }
    assert!(
        survivor_got_cancelled,
        "the surviving partner must receive onTradeResults(Cancelled), not just \
         have their server-side state cleared — otherwise their trade window \
         stays open against an entity that no longer exists"
    );
}

/// The rejection mirror: a refused transfer must leave an open trade open.
#[tokio::test]
async fn rejected_transfer_leaves_an_open_trade_alone() {
    let mut mgr = make_manager();
    spawn_player(&mut mgr, 1, AGNOS, [10.0, 0.0, 20.0]);
    spawn_player(&mut mgr, 2, AGNOS, [11.0, 0.0, 20.0]);
    mgr.get_entity_mut(1).unwrap().trade_partner_entity_id = Some(2);
    mgr.get_entity_mut(2).unwrap().trade_partner_entity_id = Some(1);
    let before = snapshot_origin(&mgr, 1);
    assert_eq!(before.trade_partner, Some(Some(2)));
    let (tx, mut rx) = mpsc::channel(32);

    transfer_player_to_space(
        1,
        &TransferDestination::in_world("NotAWorld", [1.0, 2.0, 3.0]),
        &tx,
        &mut mgr,
    )
    .await
    .expect_err("unknown world is rejected");

    assert_origin_untouched(&before, &mgr, 1, &mut rx);
    assert_eq!(
        mgr.get_entity(2).unwrap().trade_partner_entity_id,
        Some(1),
        "a rejected transfer must not cancel the partner's trade either"
    );
}
