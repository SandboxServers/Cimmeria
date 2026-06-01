//! Wire-byte fixtures for the trade flow.
//!
//! Currently a single test guards the info-asymmetry sentinel emitted
//! by [`super::super::wire::stub_inv_items_for`] against accidental
//! regression to plausible-looking placeholders.

use cimmeria_entity::trade::{
    serialize_on_trade_state, TradeItem, TradeProposal, ETRADELOCKSTATE_NONE,
};

use super::super::wire::stub_inv_items_for;

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
