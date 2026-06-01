//! Wire-byte fixtures for the trade flow.
//!
//! Currently a single test guards the info-asymmetry sentinel emitted
//! by [`super::super::wire::stub_inv_items_for`] against accidental
//! regression to plausible-looking placeholders.

use tokio::sync::mpsc;

use cimmeria_entity::trade::{
    serialize_on_trade_state, TradeItem, TradeProposal, ETRADELOCKSTATE_NONE,
    ETRADERESULTS_CANCELLED,
};

use crate::test_support::{make_space_manager, LogCapture};

use super::super::wire::{send_on_trade_results, send_on_trade_state_to_both, stub_inv_items_for};
use super::make_two_players;

/// Wire-level regression guard for the info-asymmetry fix on the
/// trading PR.
///
/// `stub_inv_items_for` previously emitted plausible-looking
/// placeholders for fields the cell cannot resolve from its own
/// state — `durability=100, charges=0, stack_size=1`. A partner
/// who hadn't received `onUpdateItem` for the offered item could
/// trust those values and agree to a trade believing they were
/// getting a durable, charged item.
///
/// Post-fix, the unresolved fields are sentinel `-1`
/// (`0xFFFFFFFF` as LE i32) so a defensive client can detect
/// "unresolved" and a non-defensive client at least renders an
/// obviously-bogus value rather than a plausible lie.
///
/// This test exercises the cell→client InvItem byte stream
/// directly: build a `TradeProposal`, stub items, and parse the
/// serialized InvItem out of the resulting `onTradeState` body to
/// confirm the sentinel bytes are at the expected offsets.
///
/// Revert-verifier: reverting `stub_inv_items_for` back to
/// `durability: 100, stack_size: 1, charges: 0` causes this test
/// to fail with the exact deceptive value at offset
/// (durability/charges/stack_size).
#[test]
fn on_trade_state_stub_invitem_carries_sentinel_not_lying_values() {
    // One offered item — instance id 0x1234, slot 7. These two
    // are known to the server (client just sent them); they must
    // pass through unchanged.
    let local = TradeProposal {
        version: 1,
        items: vec![],
        cash: 0,
        lock_state: ETRADELOCKSTATE_NONE,
    };
    let remote = TradeProposal {
        version: 1,
        items: vec![TradeItem {
            instance_id: 0x1234,
            slot_id: 7,
        }],
        cash: 0,
        lock_state: ETRADELOCKSTATE_NONE,
    };
    let stub = stub_inv_items_for(&remote);
    let buf = serialize_on_trade_state(2, &local, &remote, &stub);

    // Where the InvItem sits in `buf`:
    //   [0..4]   partner entity id (i32) = 2
    //   [4..]    local proposal (empty proposal = 13 bytes)
    //              version(4) + count=0(4) + cash(4) + lockState(1) = 13
    //   [17..]   remote proposal:
    //              version(4) + invItemCount(4) + InvItem(37) + cash(4) + lockState(1)
    //              InvItem starts at 17 + 4 + 4 = 25
    let inv_item_start = 4 + 13 + 4 + 4;

    // InvItem layout (per InvItem::serialize):
    //   id(4) dbid(4) stack(4) slot(4) container(4) bound(1) dur(4)
    //   ammoCount(4) curAmmo(4) charges(4)
    // We assert each unresolved field carries the -1 sentinel
    // (0xFFFFFFFF in LE bytes) rather than the pre-fix lies.

    let id_bytes = &buf[inv_item_start..inv_item_start + 4];
    assert_eq!(
        id_bytes,
        &0x1234_i32.to_le_bytes(),
        "instance_id must pass through from the client's proposal"
    );

    let dbid_bytes = &buf[inv_item_start + 4..inv_item_start + 8];
    assert_eq!(
        dbid_bytes,
        &0_i32.to_le_bytes(),
        "dbid is the pre-existing sentinel — no real item has dbid 0"
    );

    let stack_bytes = &buf[inv_item_start + 8..inv_item_start + 12];
    assert_eq!(
        stack_bytes,
        &(-1_i32).to_le_bytes(),
        "stack_size MUST be -1 sentinel, NOT the pre-fix lie `1`. \
         A real stack of 1 is indistinguishable from the lie; -1 is not."
    );

    let slot_bytes = &buf[inv_item_start + 12..inv_item_start + 16];
    assert_eq!(
        slot_bytes,
        &7_i32.to_le_bytes(),
        "slot_id must pass through from the client's proposal"
    );

    let container_bytes = &buf[inv_item_start + 16..inv_item_start + 20];
    assert_eq!(
        container_bytes,
        &0_i32.to_le_bytes(),
        "container_id stays 0 — no informative meaning, since the \
         item is being offered from INV_MAIN-only via the whitelist"
    );

    let bound_byte = buf[inv_item_start + 20];
    assert_eq!(
        bound_byte, 0,
        "is_bound stays false: emitting true would be a worse lie \
         (would imply the item is soul-bound, contradicting the proposal). \
         Receiver should consult their own cache."
    );

    let dur_bytes = &buf[inv_item_start + 21..inv_item_start + 25];
    assert_eq!(
        dur_bytes,
        &(-1_i32).to_le_bytes(),
        "durability MUST be -1 sentinel, NOT the pre-fix lie `100`. \
         100 looks like a fully-repaired item; -1 obviously isn't."
    );

    let ammo_count_bytes = &buf[inv_item_start + 25..inv_item_start + 29];
    assert_eq!(
        ammo_count_bytes,
        &0_u32.to_le_bytes(),
        "ammo_types stays empty — no sentinel exists for an array, \
         empty is least-deceptive"
    );

    let cur_ammo_bytes = &buf[inv_item_start + 29..inv_item_start + 33];
    assert_eq!(
        cur_ammo_bytes,
        &(-1_i32).to_le_bytes(),
        "cur_ammo_type MUST be -1 sentinel, NOT the pre-fix lie `0`"
    );

    let charges_bytes = &buf[inv_item_start + 33..inv_item_start + 37];
    assert_eq!(
        charges_bytes,
        &(-1_i32).to_le_bytes(),
        "charges MUST be -1 sentinel, NOT the pre-fix lie `0`. \
         0 looks like an exhausted consumable; -1 obviously isn't."
    );

    // Total InvItem footprint = 37 bytes (no ammo types).
    // After InvItem: trailing cash(4) + lockState(1) = 5 bytes.
    // Sanity-check the whole packet length matches.
    const INV_ITEM_NO_AMMO_BYTES: usize = 37;
    assert_eq!(
        buf.len(),
        4 + 13 + 4 + 4 + INV_ITEM_NO_AMMO_BYTES + 5,
        "onTradeState packet length must match the documented layout"
    );
}

/// Regression guard for the `send_on_trade_results` channel-closed
/// negative log. If the cell→base channel has shut down (base task
/// exited / panicked) while a trade is mid-flight, every outbound
/// trade packet would otherwise be silently dropped — the only signal
/// is the warn log. Pinning the level prevents a quiet downgrade to
/// `debug!` (which the production filter usually drops); pinning the
/// "channel closed" substring prevents an accidental rename that
/// breaks the SigNoz alert this log feeds.
///
/// Revert-verifier: replacing the `if let Err(e) = ...` block with
/// `let _ = tx.send(...).await` causes the warn event to never fire
/// and this assertion to fail with "expected event missing".
#[tokio::test]
async fn send_on_trade_results_logs_warn_when_cell_to_base_channel_closed() {
    let capture = LogCapture::install();

    // Close the channel by dropping the receiver before the send.
    let (tx, rx) = mpsc::channel(8);
    drop(rx);

    send_on_trade_results(
        /*entity_id*/ 42,
        /*partner*/ 99,
        ETRADERESULTS_CANCELLED,
        &tx,
    )
    .await;

    let event = capture
        .find_message(
            tracing::Level::WARN,
            "send onTradeResults: cell→base channel closed",
        )
        .expect(
            "send_on_trade_results MUST log WARN when the cell→base channel \
             has closed. A regression that silently swallows the SendError \
             (let _ = tx.send().await) makes mid-flight trade-end \
             notifications invisible to operators.",
        );
    assert!(
        event.has_field("entity_id", "42"),
        "channel-closed warn must record entity_id=42: {event:#?}"
    );
    assert!(
        event.has_field("partner_entity_id", "99"),
        "channel-closed warn must record partner_entity_id=99: {event:#?}"
    );
    assert!(
        event.has_field("result", &ETRADERESULTS_CANCELLED.to_string()),
        "channel-closed warn must record result={ETRADERESULTS_CANCELLED}: {event:#?}"
    );
}

/// `send_on_trade_state_to_both` early-returns with a WARN if EITHER
/// entity is missing a `trade_proposal` (one side never opened the
/// session, or the proposal was cleared mid-flight). The post-fix
/// shape must surface that case as a structured log; pre-fix the
/// function would have happily called `unwrap()` and panicked the
/// cell task.
///
/// Revert-verifier: replacing the `(None, _) | (_, None)` arm with
/// `unwrap()` causes the test to panic instead of finishing — that
/// regression would crash the cell task in production.
#[tokio::test]
async fn send_on_trade_state_to_both_warns_when_proposal_missing() {
    let capture = LogCapture::install();

    let mut mgr = make_space_manager();
    make_two_players(&mut mgr, 1, 2, 2.0);
    // Both entities exist, but neither has a `trade_proposal` (default
    // state for a player that never opened a trade).
    assert!(mgr.get_entity(1).unwrap().trade_proposal.is_none());
    assert!(mgr.get_entity(2).unwrap().trade_proposal.is_none());

    let (tx, _rx) = mpsc::channel(8);
    send_on_trade_state_to_both(1, 2, &tx, &mgr).await;

    let event = capture
        .find_message(
            tracing::Level::WARN,
            "send_on_trade_state_to_both: missing proposal state",
        )
        .expect(
            "send_on_trade_state_to_both must WARN when either side has \
             no trade_proposal. A regression to .unwrap() would panic the \
             cell task.",
        );
    assert!(event.has_field("entity_id", "1"));
    assert!(event.has_field("partner_entity_id", "2"));
}

/// Symmetric guard: if the partner entity is missing entirely (e.g. a
/// race against entity destruction), the function logs a different
/// warn (the entity-lookup branch, not the proposal-lookup branch)
/// and early-returns. Different log substring distinguishes "entity
/// gone" from "entity here but proposal absent" — useful for the
/// post-mortem when a stuck-trade report comes in.
#[tokio::test]
async fn send_on_trade_state_to_both_warns_when_entity_missing() {
    let capture = LogCapture::install();

    let mut mgr = make_space_manager();
    make_two_players(&mut mgr, 1, 2, 2.0);
    // Destroy the partner entity — entity lookup fails before
    // proposal lookup.
    mgr.destroy_entity(2);

    let (tx, _rx) = mpsc::channel(8);
    send_on_trade_state_to_both(1, 2, &tx, &mgr).await;

    let event = capture
        .find_message(
            tracing::Level::WARN,
            "send_on_trade_state_to_both: missing entity",
        )
        .expect(
            "missing-entity branch must log a different message than \
             missing-proposal so operators can tell them apart in the \
             post-mortem.",
        );
    assert!(event.has_field("entity_id", "1"));
    assert!(event.has_field("partner_entity_id", "2"));
}
