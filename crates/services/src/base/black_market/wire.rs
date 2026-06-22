//! Server→client wire serializers for the Black Market (`onBM*` methods,
//! client indices 90–95) plus the blocked-on-x64dbg placeholders.
//!
//! Names in Black Market payloads are STRING (4-byte LE length prefix + N
//! UTF-8 bytes), **not** WSTRING/UTF-16 — unlike most other SGW social
//! systems. See `docs/reverse-engineering/findings/black-market-wire-formats.md`.

use super::types::AuctionRow;

// ── x64dbg-blocked placeholders ──────────────────────────────────────────────
//
// Each unknown is isolated as a single named const/fn so swapping the real
// captured value later is a one-line edit. Do NOT spread these guesses across
// call sites.

/// Placeholder error codes for `onBMError` (`errorId: INT32`).
///
/// pending x64dbg D.1 — the real `EBlackMarketError` enum values are unknown
/// (the shipped `resources."EBlackMarketError"` type only defines
/// `InvalidSortType` / `BMUnavailable`, which do not cover the create/bid/cancel
/// validation failures below). These ordinals are a best-guess vocabulary used
/// only by the server→client `onBMError` path; capture the real ids via the
/// `0x00d84940` BMError-register breakpoint and replace the values here.
pub mod error_code {
    /// Bidder/seller does not have enough naquadah for the action.
    pub const NOT_ENOUGH_FUNDS: i32 = 1;
    /// The referenced auction no longer exists or is not active.
    pub const AUCTION_GONE: i32 = 2;
    /// The actor is the seller and may not bid on their own auction.
    pub const IS_SELLER: i32 = 3;
    /// The bid amount is below the next legal minimum bid.
    pub const BID_TOO_LOW: i32 = 4;
    /// The item is missing, not owned, or not auctionable.
    pub const INVALID_ITEM: i32 = 5;
    /// The caller is not the seller (cancel) — not authorised.
    pub const NOT_SELLER: i32 = 6;
    /// Catch-all for unexpected server-side failures.
    pub const INTERNAL: i32 = 7;
}

/// Map an `auctionLength` UINT8 duration enum to a span in seconds.
///
/// pending x64dbg D.5 — the real `EBlackMarketTime` → hours mapping is unknown.
/// The shipped `resources."EBlackMarketTime"` type names five tiers
/// (VeryShort/Short/Medium/Long/VeryLong) but carries no durations. This is a
/// best guess; capture the real per-option byte→hours pairing via the
/// `0x00e59970` BMCreateAuction breakpoint and replace the table here.
pub fn auction_length_seconds(length: u8) -> i64 {
    let hours: i64 = match length {
        0 => 12, // VeryShort
        1 => 24, // Short
        2 => 48, // Medium
        3 => 72, // Long
        _ => 96, // VeryLong (and any out-of-range value)
    };
    hours * 3_600
}

/// Compute the minimum legal next bid given the current bid.
///
/// pending x64dbg D.6 — the real `nextMinBidPrice` formula is unknown. Best
/// guess: a 5% increment with a floor of +1 so a zero/low current bid still
/// advances. Capture several `currentBid → nextMinBidPrice` pairs via the
/// client and replace this formula.
pub fn next_min_bid(current: i64) -> i64 {
    current.saturating_add((current / 20).max(1))
}

// ── onBM* serializers ────────────────────────────────────────────────────────

/// Append a STRING (4-byte LE length prefix + UTF-8 body) to `out`.
fn push_string(out: &mut Vec<u8>, s: &str) {
    out.extend_from_slice(&(s.len() as u32).to_le_bytes());
    out.extend_from_slice(s.as_bytes());
}

/// Serialize an `AuctionItem` FIXED_DICT into `out`.
///
/// Field order (per the wire-formats doc):
/// `INT32 sequenceId, INT32 itemDefId, INT32 stackSize, INT32 durability,
///  INT32 charges, INT32 currentBid, INT32 buyoutPrice, UINT8 endTimeValue,
///  INT32 nextMinBidPrice, STRING sellerName`.
fn push_auction_item(out: &mut Vec<u8>, row: &AuctionRow, seller_name: &str) {
    out.extend_from_slice(&row.sequence_id.to_le_bytes());
    out.extend_from_slice(&row.item_def_id.to_le_bytes());
    out.extend_from_slice(&row.stack_size.to_le_bytes());
    out.extend_from_slice(&row.durability.to_le_bytes());
    out.extend_from_slice(&row.charges.to_le_bytes());
    out.extend_from_slice(&row.current_bid.to_le_bytes());
    out.extend_from_slice(&row.buyout_price.to_le_bytes());
    // endTimeValue is the UINT8 duration enum echoed back to the client.
    out.push(row.auction_length as u8);
    // For a fresh auction (current_bid == 0) the next required bid is the
    // starting_price floor, not next_min_bid(0) == 1. The validate layer
    // enforces `bid >= starting_price`, so a nextMinBidPrice of 1 on a
    // 100-naquadah listing would let the client offer a legal-looking 1-bid
    // that the server then correctly rejects — confusing UX. Use
    // next_min_bid only once there is already a live bid.
    let next = if row.current_bid > 0 {
        next_min_bid(row.current_bid as i64).min(i32::MAX as i64)
    } else {
        (row.starting_price as i64).max(1)
    };
    out.extend_from_slice(&(next as i32).to_le_bytes());
    push_string(out, seller_name);
}

/// Serialize `onBMAuctions` args for a search-result page.
///
/// Wire layout (per the recovered client handler):
/// `UINT32 count` (LE) — number of `AuctionItem` entries that follow,
/// then `count` × `AuctionItem` FIXED_DICT (via [`push_auction_item`]),
/// then `INT32 view` (LE) — opaque client-side view/sort index echoed
/// back from the search request,
/// then `INT32 total` (LE) — total number of listings matching the
/// search filter (may exceed `count` if results are paginated).
pub fn serialize_on_bm_auctions(rows: &[(AuctionRow, String)], view: i32, total: i32) -> Vec<u8> {
    let mut out = Vec::with_capacity(4 + rows.len() * 44 + 8);
    out.extend_from_slice(&(rows.len() as u32).to_le_bytes());
    for (row, name) in rows {
        push_auction_item(&mut out, row, name);
    }
    out.extend_from_slice(&view.to_le_bytes());
    out.extend_from_slice(&total.to_le_bytes());
    out
}

/// Serialize `onBMOpen` args: `INT32 entityId`.
///
/// The single argument is the auctioneer NPC's entity id — the client
/// `BlackMarket` window binds to it as the conversation partner when the
/// player interacts with the in-world auctioneer. Little-endian INT32,
/// 4 bytes, no string fields.
pub fn serialize_on_bm_open(entity_id: i32) -> Vec<u8> {
    entity_id.to_le_bytes().to_vec()
}

/// Serialize `onBMError` args: `INT32 errorId`.
pub fn serialize_on_bm_error(error_id: i32) -> Vec<u8> {
    error_id.to_le_bytes().to_vec()
}

/// Serialize `onBMAuctionRemove` args: `INT32 sequenceId`.
pub fn serialize_on_bm_auction_remove(sequence_id: i32) -> Vec<u8> {
    sequence_id.to_le_bytes().to_vec()
}

/// Serialize `onBMAuctionUpdate` args: a single `AuctionItem` FIXED_DICT.
pub fn serialize_on_bm_auction_update(row: &AuctionRow, seller_name: &str) -> Vec<u8> {
    let mut out = Vec::with_capacity(40 + seller_name.len());
    push_auction_item(&mut out, row, seller_name);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Byte-exact layout guard for `onBMAuctions` with two rows.
    ///
    /// Wire format: `[u32 count] [AuctionItem×N] [i32 view] [i32 total]`.
    /// Each AuctionItem is 33 fixed bytes + STRING (4-byte len + body).
    /// With seller "A" (1 byte): per-item = 33 + 4 + 1 = 38.
    /// Two rows: 4 + 38 + 38 + 4 + 4 = 88 bytes.
    #[test]
    fn on_bm_auctions_layout_is_byte_exact() {
        fn make_row(seq: i32) -> AuctionRow {
            AuctionRow {
                sequence_id: seq,
                seller_id: 1,
                item_id: 10,
                item_def_id: 20,
                stack_size: 1,
                durability: 100,
                charges: 0,
                starting_price: 50,
                buyout_price: 200,
                current_bid: 0,
                current_bidder: None,
                auction_length: 1,
                created_at: 0,
                expires_at: 0,
                status: 0,
            }
        }
        let rows: Vec<(AuctionRow, String)> = vec![
            (make_row(1), "A".to_string()),
            (make_row(2), "A".to_string()),
        ];
        let args = serialize_on_bm_auctions(&rows, 7, 99);

        // Header: 4 (count)
        // Per item: 7*4(fixed i32s) + 1(endTime) + 4(nextMinBid) + 4(name len) + 1(name) = 38
        // Trailer: 4 (view) + 4 (total)
        assert_eq!(args.len(), 4 + 38 + 38 + 4 + 4, "total byte count");

        // count = 2
        assert_eq!(u32::from_le_bytes([args[0], args[1], args[2], args[3]]), 2);

        // First row: sequenceId at offset 4
        assert_eq!(
            i32::from_le_bytes([args[4], args[5], args[6], args[7]]),
            1,
            "first row sequenceId"
        );

        // Second row starts at 4 + 38 = 42
        assert_eq!(
            i32::from_le_bytes([args[42], args[43], args[44], args[45]]),
            2,
            "second row sequenceId"
        );

        // view at 4 + 38 + 38 = 80
        assert_eq!(
            i32::from_le_bytes([args[80], args[81], args[82], args[83]]),
            7,
            "view echoed"
        );

        // total at 84
        assert_eq!(
            i32::from_le_bytes([args[84], args[85], args[86], args[87]]),
            99,
            "total count"
        );
    }

    /// Empty result set serializes to [0u32 count] + [i32 view] + [i32 total] = 12 bytes.
    #[test]
    fn on_bm_auctions_empty_is_twelve_bytes() {
        let args = serialize_on_bm_auctions(&[], 0, 0);
        assert_eq!(args.len(), 12);
        assert_eq!(u32::from_le_bytes([args[0], args[1], args[2], args[3]]), 0);
    }

    #[test]
    fn bm_open_serializes_to_four_le_entity_id_bytes() {
        let args = serialize_on_bm_open(0x0123_4567);
        assert_eq!(args.len(), 4, "onBMOpen carries exactly one INT32");
        assert_eq!(
            i32::from_le_bytes([args[0], args[1], args[2], args[3]]),
            0x0123_4567,
            "onBMOpen arg must be the entity id in little-endian"
        );
        // Explicit byte order pin: low byte first.
        assert_eq!(args, vec![0x67, 0x45, 0x23, 0x01]);
    }

    #[test]
    fn error_serializes_to_four_le_bytes() {
        assert_eq!(serialize_on_bm_error(5), 5i32.to_le_bytes().to_vec());
    }

    #[test]
    fn auction_remove_serializes_sequence_id() {
        let args = serialize_on_bm_auction_remove(777);
        assert_eq!(args.len(), 4);
        assert_eq!(
            i32::from_le_bytes([args[0], args[1], args[2], args[3]]),
            777
        );
    }

    #[test]
    fn auction_update_layout_is_byte_exact() {
        let row = AuctionRow {
            sequence_id: 1,
            seller_id: 2,
            item_id: 3,
            item_def_id: 4,
            stack_size: 5,
            durability: 6,
            charges: 7,
            starting_price: 8,
            buyout_price: 9,
            current_bid: 100,
            current_bidder: None,
            auction_length: 2,
            created_at: 0,
            expires_at: 0,
            status: 0,
        };
        let args = serialize_on_bm_auction_update(&row, "AB");
        // 9 INT32 fields (sequenceId..currentBid..buyout..nextMinBid is 8 ints
        // + endTime byte), plus the seller STRING. Count fixed bytes:
        // seq,defId,stack,dur,charges,curBid,buyout = 7*4 = 28
        // + endTime(1) + nextMinBid(4) = 33; + STRING(4 + 2) = 39.
        assert_eq!(args.len(), 33 + 4 + 2);
        // sequenceId first
        assert_eq!(i32::from_le_bytes([args[0], args[1], args[2], args[3]]), 1);
        // endTimeValue byte at offset 28
        assert_eq!(args[28], 2);
        // nextMinBid at 29..33: current 100 -> 100 + max(1,5) = 105
        assert_eq!(
            i32::from_le_bytes([args[29], args[30], args[31], args[32]]),
            105
        );
    }

    #[test]
    fn next_min_bid_floors_at_plus_one() {
        assert_eq!(next_min_bid(0), 1);
        assert_eq!(next_min_bid(10), 11); // 10/20 = 0 -> max(1)
        assert_eq!(next_min_bid(100), 105); // 100/20 = 5
    }

    #[test]
    fn next_min_bid_clamps_at_i32_max_on_wire() {
        // current_bid near i32::MAX: the raw next_min_bid exceeds i32 range, so
        // the serializer must clamp to i32::MAX, never wrap negative on the wire.
        let row = AuctionRow {
            sequence_id: 1,
            seller_id: 2,
            item_id: 3,
            item_def_id: 4,
            stack_size: 1,
            durability: 0,
            charges: 0,
            starting_price: 0,
            buyout_price: 0,
            current_bid: i32::MAX,
            current_bidder: None,
            auction_length: 0,
            created_at: 0,
            expires_at: 0,
            status: 0,
        };
        let args = serialize_on_bm_auction_update(&row, "");
        let next = i32::from_le_bytes([args[29], args[30], args[31], args[32]]);
        assert_eq!(next, i32::MAX, "must clamp to i32::MAX");
        assert!(next > 0, "must never wrap negative on the wire");
    }

    #[test]
    fn auction_length_maps_each_tier() {
        assert_eq!(auction_length_seconds(0), 12 * 3600);
        assert_eq!(auction_length_seconds(1), 24 * 3600);
        assert_eq!(auction_length_seconds(2), 48 * 3600);
        assert_eq!(auction_length_seconds(3), 72 * 3600);
        assert_eq!(auction_length_seconds(4), 96 * 3600);
        // out-of-range clamps to the longest tier
        assert_eq!(auction_length_seconds(200), 96 * 3600);
    }

    /// Regression guard for the fresh-auction `nextMinBidPrice` fix.
    ///
    /// When `current_bid == 0` (no bids yet), `nextMinBidPrice` must equal
    /// `starting_price` (the validate layer requires `bid >= starting_price`).
    /// Before the fix, `next_min_bid(0) = 1` was used, which would show 1 on
    /// a 100-naquadah listing — a misleading floor the server then rejects.
    ///
    /// Bug shape: reverting to `next_min_bid(row.current_bid as i64)` for all
    /// cases causes a fresh auction with `starting_price = 100` to serialise
    /// `nextMinBidPrice = 1` instead of `100`, failing this assertion.
    #[test]
    fn fresh_auction_next_min_bid_price_equals_starting_price() {
        let row = AuctionRow {
            sequence_id: 1,
            seller_id: 2,
            item_id: 3,
            item_def_id: 4,
            stack_size: 1,
            durability: 100,
            charges: 0,
            starting_price: 100,
            buyout_price: 500,
            current_bid: 0,
            current_bidder: None,
            auction_length: 1,
            created_at: 0,
            expires_at: 0,
            status: 0,
        };
        let args = serialize_on_bm_auction_update(&row, "");
        // nextMinBidPrice is at offset 29..33 (after 7×INT32 + 1 UINT8 endTime).
        let next_min = i32::from_le_bytes([args[29], args[30], args[31], args[32]]);
        assert_eq!(
            next_min, 100,
            "fresh auction nextMinBidPrice must equal starting_price, not next_min_bid(0)=1"
        );
    }
}
