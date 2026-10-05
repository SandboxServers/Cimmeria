//! Rule 6 guards on the trade rows (NT-28a): the handoff row names both
//! traders next to their ids.

use tokio::sync::mpsc;

use cimmeria_entity::trade::{TradeProposal, ETRADELOCKSTATE_LOCKED_AND_CONFIRMED};

use crate::cell::messages::CellToBaseMsg;
use crate::test_support::{make_space_manager, LogCapture};

use super::super::handoff::request_execute_trade;
use super::make_two_players;

/// "trade execute requested → base" is the one row a trade dispute starts
/// from. It carries each side's entity id and player id with the character
/// name beside it, so the row reads without a lookup.
///
/// Revert-verifier: dropping `partner_player_name` (or any other name) from
/// the row in `handoff.rs` fails the matching assertion.
#[tokio::test]
async fn execute_requested_row_names_both_traders() {
    let mut mgr = make_space_manager();
    make_two_players(&mut mgr, 1, 2, 2.0);
    for (me, partner, name) in [(1, 2, "Teal'c"), (2, 1, "Vala")] {
        let e = mgr.get_entity_mut(me).unwrap();
        e.stamp_log_names(Some(name), None);
        e.trade_partner_entity_id = Some(partner);
        e.trade_proposal = Some(TradeProposal {
            version: 1,
            items: vec![],
            cash: 0,
            lock_state: ETRADELOCKSTATE_LOCKED_AND_CONFIRMED,
        });
    }

    let capture = LogCapture::install();
    let (tx, mut rx) = mpsc::channel(64);
    request_execute_trade(1, 2, &tx, &mut mgr).await;
    assert!(
        matches!(rx.try_recv(), Ok(CellToBaseMsg::ExecuteTrade { .. })),
        "the handoff queued ExecuteTrade"
    );

    let row = capture
        .find_message(tracing::Level::INFO, "trade execute requested")
        .expect("handoff row");
    for (k, v) in [
        ("entity_id", "1"),
        ("entity_name", "Teal'c"),
        ("player_id", "1000"),
        ("player_name", "Teal'c"),
        ("partner_entity_id", "2"),
        ("partner_entity_name", "Vala"),
        ("partner_player_id", "2000"),
        ("partner_player_name", "Vala"),
    ] {
        assert!(row.has_field(k, v), "field {k}={v}: {:?}", row.fields);
    }
}
