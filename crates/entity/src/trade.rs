//! Player-to-player trade types and wire serializers.
//!
//! Reference: `deprecated/python/cell/Trade.py`, `entities/defs/alias.xml`
//! (`LocalTradeProposal`, `RemoteTradeProposal`, `LocalTradeItem`), and
//! `entities/defs/enumerations.xml` (`ETradeLockState`, `ETradeResults`).
//!
//! # Wire format quirks
//!
//! Two wire-format quirks the Python source proved against the client and
//! that we must preserve byte-for-byte:
//!
//! 1. **`onTradeResults.Result` is `INT32`** even though the underlying
//!    `ETradeResults` enum is declared `INT8` in `enumerations.xml`.
//!    `SGWPlayer.def` declares the field as `INT32`. Serializing the
//!    result as 1 byte produces a silent client parse failure (the trade
//!    UI never closes / never updates). See
//!    [`serialize_on_trade_results`].
//!
//! 2. **`cancel()` sends `Completed (1)`**, not `Cancelled (2)`. The
//!    Python `TradeTransaction.cancel` deliberately uses `Completed` so
//!    both players get a clean shutdown notification regardless of who
//!    aborted; `Cancelled` is reserved for the disconnect / distance-break
//!    / atomic-commit-failure paths. See [`ETradeResults`].

use crate::inventory::InvItem;

// ── ETradeLockState (from enumerations.xml:1784-1791) ──────────────────────

pub const ETRADELOCKSTATE_NONE: i8 = 0;
pub const ETRADELOCKSTATE_LOCKED: i8 = 1;
pub const ETRADELOCKSTATE_LOCKED_AND_CONFIRMED: i8 = 2;

// ── ETradeResults (from enumerations.xml:1793-1803) ────────────────────────
//
// Value 0 is intentionally unused — the client UI never sees that byte.
// The Python `TradeTransaction.cancel` (Trade.py:225-228) sends
// `Completed` even on a cancel; `Cancelled` is reserved for paths where
// the trade was never confirmed (disconnect, distance-break, atomic
// commit failure).

pub const ETRADERESULTS_COMPLETED: i32 = 1;
pub const ETRADERESULTS_CANCELLED: i32 = 2;
pub const ETRADERESULTS_NO_LOCAL_SPACE: i32 = 3;
pub const ETRADERESULTS_NO_REMOTE_SPACE: i32 = 4;
pub const ETRADERESULTS_NO_LOCAL_CASH: i32 = 5;
pub const ETRADERESULTS_NO_REMOTE_CASH: i32 = 6;

// ── LocalTradeItem ─────────────────────────────────────────────────────────

/// One item slot reference in a `LocalTradeProposal`.
///
/// Carries only the runtime instance id + slot id — the partner's full
/// `InvItem` payload is constructed server-side from the sender's
/// inventory when emitting `RemoteTradeProposal`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TradeItem {
    /// Runtime inventory item instance ID (`sgw_inventory.item_id`).
    pub instance_id: i32,
    /// Slot index within the source bag.
    pub slot_id: i32,
}

// ── TradeProposal ──────────────────────────────────────────────────────────

/// One player's side of an active trade session.
///
/// Versioned: every `tradeUpdateProposal` increments [`Self::version`] and
/// resets the lock states on both sides (the partner's lock implicitly
/// goes back to `None` whenever this player's proposal changes — clients
/// must re-acknowledge the new contents before they can lock again).
#[derive(Debug, Clone, Default)]
pub struct TradeProposal {
    pub version: i32,
    pub items: Vec<TradeItem>,
    pub cash: i32,
    /// One of `ETRADELOCKSTATE_*`. Single byte on the wire.
    pub lock_state: i8,
}

impl TradeProposal {
    /// Parse a `LocalTradeProposal` from the client wire format.
    ///
    /// Wire layout:
    /// ```text
    /// version:   i32
    /// items:     u32 count + N × { instanceId: i32, slotId: i32 }
    /// cash:      i32
    /// lockState: i8
    /// ```
    ///
    /// Returns `None` on truncation or implausible item counts (the
    /// client is rate-limited to 100 items by inventory rules; we cap
    /// the parse at the remaining-bytes budget so a hostile `count = u32::MAX`
    /// can't OOM the cell process the way it did pre-fix in
    /// `vendor::read_i32_array`).
    pub fn parse(args: &[u8], offset: &mut usize) -> Option<Self> {
        // version
        if args.len() < *offset + 4 {
            return None;
        }
        let version = i32::from_le_bytes([
            args[*offset],
            args[*offset + 1],
            args[*offset + 2],
            args[*offset + 3],
        ]);
        *offset += 4;

        // items count
        if args.len() < *offset + 4 {
            return None;
        }
        let count = u32::from_le_bytes([
            args[*offset],
            args[*offset + 1],
            args[*offset + 2],
            args[*offset + 3],
        ]) as usize;
        *offset += 4;

        // Bound count against remaining payload BEFORE allocating — each
        // entry is `2 × i32 = 8 bytes` + the trailing `cash:i32 +
        // lockState:i8 = 5 bytes`. A malformed `count = u32::MAX` packet
        // would otherwise reserve ~32 GiB on the heap.
        let remaining = args.len().saturating_sub(*offset);
        if count.saturating_mul(8).saturating_add(5) > remaining {
            return None;
        }

        let mut items = Vec::with_capacity(count);
        for _ in 0..count {
            if args.len() < *offset + 8 {
                return None;
            }
            let instance_id = i32::from_le_bytes([
                args[*offset],
                args[*offset + 1],
                args[*offset + 2],
                args[*offset + 3],
            ]);
            let slot_id = i32::from_le_bytes([
                args[*offset + 4],
                args[*offset + 5],
                args[*offset + 6],
                args[*offset + 7],
            ]);
            *offset += 8;
            items.push(TradeItem {
                instance_id,
                slot_id,
            });
        }

        // cash
        if args.len() < *offset + 4 {
            return None;
        }
        let cash = i32::from_le_bytes([
            args[*offset],
            args[*offset + 1],
            args[*offset + 2],
            args[*offset + 3],
        ]);
        *offset += 4;

        // lockState
        if args.len() < *offset + 1 {
            return None;
        }
        let lock_state = args[*offset] as i8;
        *offset += 1;

        Some(Self {
            version,
            items,
            cash,
            lock_state,
        })
    }

    /// Serialize this proposal as a `LocalTradeProposal` (the local
    /// player's own view of their offer — items carry only instance/slot
    /// IDs, no full `InvItem` payload).
    ///
    /// Wire layout matches [`Self::parse`].
    pub fn serialize_local(&self, buf: &mut Vec<u8>) {
        buf.extend_from_slice(&self.version.to_le_bytes());
        buf.extend_from_slice(&(self.items.len() as u32).to_le_bytes());
        for item in &self.items {
            buf.extend_from_slice(&item.instance_id.to_le_bytes());
            buf.extend_from_slice(&item.slot_id.to_le_bytes());
        }
        buf.extend_from_slice(&self.cash.to_le_bytes());
        buf.push(self.lock_state as u8);
    }

    /// Serialize this proposal as a `RemoteTradeProposal` (the partner's
    /// view of the offer — items carry full `InvItem` FIXED_DICT so the
    /// partner's client can render names, icons, durability, etc.).
    ///
    /// Wire layout:
    /// ```text
    /// version:   i32
    /// items:     u32 count + N × InvItem  (see InvItem::serialize)
    /// cash:      i32
    /// lockState: i8
    /// ```
    ///
    /// The caller resolves each `TradeItem.instance_id` against the
    /// sender's inventory to produce the full `InvItem` for that slot —
    /// callers usually pull from a snapshot taken when the proposal was
    /// accepted server-side.
    pub fn serialize_remote(&self, items: &[InvItem], buf: &mut Vec<u8>) {
        buf.extend_from_slice(&self.version.to_le_bytes());
        buf.extend_from_slice(&(items.len() as u32).to_le_bytes());
        for item in items {
            item.serialize(buf);
        }
        buf.extend_from_slice(&self.cash.to_le_bytes());
        buf.push(self.lock_state as u8);
    }
}

// ── Outbound message serializers ───────────────────────────────────────────

/// Serialize the args body of `onTradeState(entityId, localProposal,
/// remoteProposal)`.
///
/// `partner_entity_id` is the recipient's view of their partner — i.e.
/// the OTHER player. `local_proposal` is the recipient's own offer;
/// `remote_proposal` is the partner's offer + the resolved `InvItem`
/// list for it.
pub fn serialize_on_trade_state(
    partner_entity_id: i32,
    local_proposal: &TradeProposal,
    remote_proposal: &TradeProposal,
    remote_items: &[InvItem],
) -> Vec<u8> {
    // Per-InvItem byte cost ranges from 37 (no ammo) to ~50 (a few ammo
    // types) — 48 is a generous mid-range to avoid most resize copies
    // without overshooting wildly.
    let mut buf = Vec::with_capacity(64 + remote_items.len() * 48);
    buf.extend_from_slice(&partner_entity_id.to_le_bytes());
    local_proposal.serialize_local(&mut buf);
    remote_proposal.serialize_remote(remote_items, &mut buf);
    buf
}

/// Serialize the args body of `onTradeResults(entityId, result)`.
///
/// **TRAP**: `result` is `INT32` on the wire even though `ETradeResults`
/// is `INT8` in `enumerations.xml`. `SGWPlayer.def:1387` declares the
/// field as `INT32`. Emitting one byte → silent client parse failure
/// (the trade window never closes, the inventory update never reconciles).
pub fn serialize_on_trade_results(partner_entity_id: i32, result: i32) -> Vec<u8> {
    let mut buf = Vec::with_capacity(8);
    buf.extend_from_slice(&partner_entity_id.to_le_bytes());
    buf.extend_from_slice(&result.to_le_bytes());
    buf
}

// ── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn local_trade_proposal_serializes_byte_exact() {
        // Empty proposal: version=0, no items, cash=0, lockState=None.
        // Layout: version(4) + count(4) + cash(4) + lockState(1) = 13 bytes.
        let p = TradeProposal::default();
        let mut buf = Vec::new();
        p.serialize_local(&mut buf);
        assert_eq!(
            buf,
            vec![
                0, 0, 0, 0, // version = 0
                0, 0, 0, 0, // item count = 0
                0, 0, 0, 0, // cash = 0
                0, // lockState = None
            ]
        );

        // Proposal with two items + cash + Locked state.
        let p = TradeProposal {
            version: 0x12345678,
            items: vec![
                TradeItem {
                    instance_id: 0x1111_2222,
                    slot_id: 3,
                },
                TradeItem {
                    instance_id: 0x3333_4444,
                    slot_id: 7,
                },
            ],
            cash: 999,
            lock_state: ETRADELOCKSTATE_LOCKED,
        };
        let mut buf = Vec::new();
        p.serialize_local(&mut buf);
        // version(4) + count(4) + 2*(instance:4 + slot:4) + cash(4) + lockState(1) = 29
        assert_eq!(buf.len(), 29);
        assert_eq!(&buf[0..4], &0x12345678i32.to_le_bytes());
        assert_eq!(&buf[4..8], &2u32.to_le_bytes());
        assert_eq!(&buf[8..12], &0x1111_2222i32.to_le_bytes());
        assert_eq!(&buf[12..16], &3i32.to_le_bytes());
        assert_eq!(&buf[16..20], &0x3333_4444i32.to_le_bytes());
        assert_eq!(&buf[20..24], &7i32.to_le_bytes());
        assert_eq!(&buf[24..28], &999i32.to_le_bytes());
        assert_eq!(buf[28], 1); // ETRADELOCKSTATE_Locked
    }

    #[test]
    fn local_trade_proposal_parse_roundtrip() {
        let original = TradeProposal {
            version: 42,
            items: vec![
                TradeItem {
                    instance_id: 100,
                    slot_id: 0,
                },
                TradeItem {
                    instance_id: 200,
                    slot_id: 5,
                },
                TradeItem {
                    instance_id: -1,
                    slot_id: -1,
                },
            ],
            cash: 1_000_000,
            lock_state: ETRADELOCKSTATE_LOCKED_AND_CONFIRMED,
        };
        let mut buf = Vec::new();
        original.serialize_local(&mut buf);
        let mut offset = 0;
        let parsed = TradeProposal::parse(&buf, &mut offset).expect("parse");
        assert_eq!(parsed.version, original.version);
        assert_eq!(parsed.cash, original.cash);
        assert_eq!(parsed.lock_state, original.lock_state);
        assert_eq!(parsed.items, original.items);
        assert_eq!(offset, buf.len());
    }

    #[test]
    fn parse_rejects_truncated_proposal() {
        // Too short for version.
        let mut off = 0;
        assert!(TradeProposal::parse(&[0u8; 2], &mut off).is_none());

        // Too short for cash trailer.
        let mut buf = Vec::new();
        buf.extend_from_slice(&0i32.to_le_bytes()); // version
        buf.extend_from_slice(&0u32.to_le_bytes()); // count = 0
        buf.extend_from_slice(&[0u8, 0]); // truncated cash
        let mut off = 0;
        assert!(TradeProposal::parse(&buf, &mut off).is_none());

        // Truncated mid-item.
        let mut buf = Vec::new();
        buf.extend_from_slice(&0i32.to_le_bytes());
        buf.extend_from_slice(&1u32.to_le_bytes());
        buf.extend_from_slice(&[0u8; 6]); // partial item entry
        let mut off = 0;
        assert!(TradeProposal::parse(&buf, &mut off).is_none());
    }

    #[test]
    fn parse_rejects_oversized_count_without_allocating() {
        // Same OOM guard as vendor::read_i32_array: a malicious count =
        // u32::MAX would reserve ~32 GiB if allocated up front. The
        // remaining-bytes check rejects before the Vec::with_capacity.
        let mut buf = Vec::new();
        buf.extend_from_slice(&0i32.to_le_bytes()); // version
        buf.extend_from_slice(&u32::MAX.to_le_bytes()); // count = MAX
        buf.extend_from_slice(&[0u8; 5]); // far less than MAX * 8 bytes
        let mut off = 0;
        assert!(TradeProposal::parse(&buf, &mut off).is_none());
    }

    /// TRAP #1 regression guard: `onTradeResults.Result` is `INT32`,
    /// not `INT8`. Pinning the byte layout means a regression that
    /// drops it back to 1 byte will fail loudly in CI rather than as
    /// a silent client UI hang in QA.
    #[test]
    fn on_trade_results_result_field_is_int32_not_int8() {
        let buf = serialize_on_trade_results(0x1234, ETRADERESULTS_COMPLETED);
        assert_eq!(buf.len(), 8, "entityId(4) + result(4) must be 8 bytes");
        assert_eq!(&buf[0..4], &0x1234i32.to_le_bytes());
        assert_eq!(
            &buf[4..8],
            &1i32.to_le_bytes(),
            "result must be 4 bytes little-endian — reverting to push(result as u8) breaks the client"
        );

        // Spot-check Cancelled (the disconnect/distance-break code) too.
        let buf = serialize_on_trade_results(99, ETRADERESULTS_CANCELLED);
        assert_eq!(&buf[4..8], &2i32.to_le_bytes());
    }

    /// TRAP #2 documentation guard. The `cancel()` path uses
    /// `ETRADERESULTS_COMPLETED` (value 1), not `ETRADERESULTS_CANCELLED`
    /// (value 2). The Python source comment (Trade.py:225-228) calls this
    /// out as intentional — both players get a clean completion
    /// notification regardless of who hit the cancel button. Cancelled
    /// is reserved for paths where the trade window never opened on the
    /// other client (distance break, disconnect, atomic-commit failure).
    ///
    /// We assert by inspecting the constants directly so a renumber
    /// in `enumerations.xml` would surface.
    #[test]
    fn cancel_serializes_result_as_completed_one_not_cancelled_two() {
        assert_eq!(
            ETRADERESULTS_COMPLETED, 1,
            "ETradeResults::Completed must be 1 — used by user-initiated cancel"
        );
        assert_eq!(
            ETRADERESULTS_CANCELLED, 2,
            "ETradeResults::Cancelled must be 2 — used only for disconnect / distance / commit failure"
        );

        // The wire byte for a user-initiated cancel must be (..., 1, 0, 0, 0).
        let buf = serialize_on_trade_results(42, ETRADERESULTS_COMPLETED);
        assert_eq!(
            &buf[4..8],
            &1i32.to_le_bytes(),
            "user cancel must serialize the Result field as Completed (1), \
             NOT Cancelled (2) — see Trade.py:225-228"
        );
    }

    #[test]
    fn on_trade_state_serializes_local_and_remote() {
        let local = TradeProposal {
            version: 5,
            items: vec![TradeItem {
                instance_id: 11,
                slot_id: 0,
            }],
            cash: 100,
            lock_state: ETRADELOCKSTATE_NONE,
        };
        let remote = TradeProposal {
            version: 3,
            items: vec![TradeItem {
                instance_id: 22,
                slot_id: 0,
            }],
            cash: 50,
            lock_state: ETRADELOCKSTATE_LOCKED,
        };
        let remote_items = vec![InvItem {
            id: 22,
            dbid: 999,
            stack_size: 1,
            slot_id: 0,
            container_id: 1,
            is_bound: false,
            durability: 100,
            ammo_types: vec![],
            cur_ammo_type: 0,
            charges: 0,
        }];
        let buf = serialize_on_trade_state(0xAABB, &local, &remote, &remote_items);

        // Leading entityId
        assert_eq!(&buf[0..4], &0xAABBi32.to_le_bytes());

        // Local proposal (local serialization, 1 item)
        // version(4) + count(4) + 1*(instance:4 + slot:4) + cash(4) + lockState(1) = 21
        let local_start = 4;
        assert_eq!(&buf[local_start..local_start + 4], &5i32.to_le_bytes());
        assert_eq!(
            &buf[local_start + 4..local_start + 8],
            &1u32.to_le_bytes()
        );

        // Remote proposal begins after 4 + 21 = 25 bytes
        let remote_start = 4 + 21;
        assert_eq!(&buf[remote_start..remote_start + 4], &3i32.to_le_bytes());
        // 1 item, full InvItem follows the count — InvItem with 0 ammo
        // entries is 37 bytes: id(4) + dbid(4) + stack(4) + slot(4) +
        // container(4) + bound(1) + durability(4) + ammoCount(4) +
        // curAmmo(4) + charges(4) = 37. With 3 ammo entries it grows
        // to 49 bytes — see `inv_item_serialize_with_ammo` in
        // `inventory.rs`. Then trailing cash(4) + lockState(1).
        const INV_ITEM_NO_AMMO_BYTES: usize = 37;
        assert_eq!(buf.len(), 4 + 21 + 4 + 4 + INV_ITEM_NO_AMMO_BYTES + 4 + 1);
        // Last byte is the remote lock state.
        assert_eq!(buf[buf.len() - 1], ETRADELOCKSTATE_LOCKED as u8);
    }
}
