//! Server→client arguments for the Black Market (`onBM*`, client indices
//! 90–95), built with the shared codec, plus the listing-time rules the
//! wire exposes: durations (S5), the time-left bucket (S6), the next minimum
//! bid (D6) and the search page budget (S7).
//!
//! Every layout is `cimmeria-patch-wire`'s (re-exported as
//! `cimmeria_wire::black_market`): the client-patch DLL decodes with the same
//! code, so an argument-order slip like S1 fails a test on both sides
//! instead of arriving as garbage in the auction window. `onBMOpen` (90) is
//! sent by the cell's `open_black_market` action.

use cimmeria_wire::black_market::{
    AuctionItem, Encode, OnBMAuctionRemove, OnBMAuctionUpdate, OnBMAuctions, UIAuctionTime,
    MAX_AUCTION_ITEMS, MAX_STRING_LEN,
};

use super::types::AuctionRow;

pub use cimmeria_wire::black_market::{serialize_on_bm_error, BMError};

// ── durations (S5) and the time-left bucket (S6) ─────────────────────────

/// Hours a listing of each `UIAuctionTime` tier runs, 1-based as the client
/// sends it. The create form offers Medium, Long and VeryLong. No source
/// gives the shipped durations; this table is design (the branch's guess,
/// moved to the 1-based tiers).
pub const fn tier_hours(tier: UIAuctionTime) -> i64 {
    match tier {
        UIAuctionTime::VeryShort => 12,
        UIAuctionTime::Short => 24,
        UIAuctionTime::Medium => 48,
        UIAuctionTime::Long => 72,
        UIAuctionTime::VeryLong => 96,
    }
}

/// The tier a raw `auctionLength` byte asks for, clamped to 1–5: 0 becomes
/// VeryShort and anything above 5 VeryLong. The second value is `true` when
/// the byte was out of range, so the caller can log the clamp. The clamped
/// tier is what is stored and echoed, so storage and duration agree.
pub fn clamp_auction_length(raw: u8) -> (UIAuctionTime, bool) {
    match UIAuctionTime::try_from(raw) {
        Ok(tier) => (tier, false),
        Err(_) if raw == 0 => (UIAuctionTime::VeryShort, true),
        Err(_) => (UIAuctionTime::VeryLong, true),
    }
}

/// How long a listing of `tier` runs, in seconds.
pub fn auction_length_seconds(tier: UIAuctionTime) -> i64 {
    tier_hours(tier) * 3_600
}

/// `AuctionItem.endTimeValue` (S6): the time left, as the smallest tier
/// whose full duration covers it. A fresh listing shows its own tier; one
/// with under 12 hours left, or already due, shows VeryShort. The client
/// picks the row's timer icon from it.
pub fn time_left_bucket(expires_at: i32, now: i32) -> UIAuctionTime {
    let left = i64::from(expires_at) - i64::from(now);
    UIAuctionTime::ALL
        .into_iter()
        .find(|&tier| left <= auction_length_seconds(tier))
        .unwrap_or(UIAuctionTime::VeryLong)
}

// ── bidding (D6) ──────────────────────────────────────────────────────────

/// The minimum next bid over a standing bid (decision D6): 5% more, and
/// at least 1 more.
pub fn next_min_bid(current: i64) -> i64 {
    current.saturating_add((current / 20).max(1))
}

/// The lowest bid the server accepts next: the starting price until someone
/// bids (at least 1), then [`next_min_bid`] over the standing bid.
pub fn required_min_bid(auction: &AuctionRow) -> i64 {
    if auction.current_bidder.is_some() && auction.current_bid > 0 {
        next_min_bid(i64::from(auction.current_bid))
    } else {
        i64::from(auction.starting_price).max(1)
    }
}

// ── the AuctionItem row ───────────────────────────────────────────────────

/// Clamp a string to the codec's `STRING` cap on a character boundary. A
/// player name is far shorter; this only keeps a bad row from failing the
/// whole reply.
fn capped(s: &str) -> String {
    let max = MAX_STRING_LEN as usize;
    if s.len() <= max {
        return s.to_owned();
    }
    let mut end = max;
    while !s.is_char_boundary(end) {
        end -= 1;
    }
    s[..end].to_owned()
}

/// The `AuctionItem` for `row` as of `now`, with the seller's display name.
pub fn auction_item(row: &AuctionRow, seller_name: &str, now: i32) -> AuctionItem {
    AuctionItem {
        sequence_id: row.sequence_id,
        item_def_id: row.item_def_id,
        stack_size: row.stack_size,
        durability: row.durability,
        charges: row.charges,
        current_bid: row.current_bid,
        buyout_price: row.buyout_price,
        end_time_value: time_left_bucket(row.expires_at, now) as u8,
        next_min_bid_price: required_min_bid(row).min(i64::from(i32::MAX)) as i32,
        seller_name: capped(seller_name),
    }
}

/// Encode an argument list whose strings are already capped and whose
/// array is already within [`MAX_AUCTION_ITEMS`], which cannot fail.
fn encode(args: &impl Encode) -> Vec<u8> {
    args.to_bytes()
        .expect("strings are capped and the page is bounded before encoding")
}

/// `onBMAuctionUpdate(AuctionItem)`.
pub fn serialize_on_bm_auction_update(row: &AuctionRow, seller_name: &str, now: i32) -> Vec<u8> {
    encode(&OnBMAuctionUpdate {
        auction_item: auction_item(row, seller_name, now),
    })
}

/// `onBMAuctionRemove(INT32 sequenceId)`.
pub fn serialize_on_bm_auction_remove(sequence_id: i32) -> Vec<u8> {
    encode(&OnBMAuctionRemove { sequence_id })
}

// ── search paging (S1, S7) ────────────────────────────────────────────────

/// Largest `onBMAuctions` argument payload sent in one message. The reply
/// is one unfragmented entity message: Mercury's body is 1,348 bytes
/// (`cimmeria_mercury::consts::MAX_BODY`), less 8 for the extended
/// entity-method header (id, length, entity id, sub-index), leaving room
/// for 35 piggybacked acks. At the typical 50-byte row that is about 23
/// rows, more than two of the UI's 8-row pages.
pub const AUCTIONS_ARG_BUDGET: usize = 1_200;

/// `onBMAuctions` fixed bytes: the array count, `totalResults`, `clientKey`.
const AUCTIONS_FIXED_LEN: usize = 12;

/// Wire size of one `AuctionItem` with `seller_name`.
fn item_wire_len(item: &AuctionItem) -> usize {
    AuctionItem::MIN_WIRE_LEN + item.seller_name.len()
}

/// How many of `rows`, taken in order, fit one `onBMAuctions` within
/// [`AUCTIONS_ARG_BUDGET`] and [`MAX_AUCTION_ITEMS`].
pub fn rows_that_fit<'a>(
    rows: impl IntoIterator<Item = &'a (AuctionRow, String)>,
    now: i32,
) -> usize {
    let mut size = AUCTIONS_FIXED_LEN;
    let mut n = 0;
    for (row, name) in rows {
        let len = item_wire_len(&auction_item(row, name, now));
        if size + len > AUCTIONS_ARG_BUDGET || n >= MAX_AUCTION_ITEMS as usize {
            break;
        }
        size += len;
        n += 1;
    }
    n
}

/// Build `onBMAuctions(items, totalResults, clientKey)` (S1: the `.def`
/// order), taking rows from `rows` in order while the payload stays within
/// [`AUCTIONS_ARG_BUDGET`] and [`MAX_AUCTION_ITEMS`]. Returns the payload
/// and how many rows it carries. `total_results` is the full match count,
/// never the page size.
pub fn serialize_on_bm_auctions(
    rows: &[(AuctionRow, String)],
    total_results: i32,
    client_key: i32,
    now: i32,
) -> (Vec<u8>, usize) {
    let sent = rows_that_fit(rows, now);
    let auction_items: Vec<AuctionItem> = rows[..sent]
        .iter()
        .map(|(row, name)| auction_item(row, name, now))
        .collect();
    let args = encode(&OnBMAuctions {
        auction_items,
        total_results,
        client_key,
    });
    debug_assert!(args.len() <= AUCTIONS_ARG_BUDGET);
    (args, sent)
}

#[cfg(test)]
mod tests;
