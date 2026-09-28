//! Black Market on the wire: the server's view of the auction-house
//! contract.
//!
//! Every argument layout comes from `cimmeria-patch-wire`, the std-only
//! codec crate the injected client-patch DLL links too, so the server and
//! the client patch encode and decode with the same code (plan
//! `docs/analysis/black-market/README.md` §3.2, "One codec for both
//! sides"). This module re-exports the pieces the cell and the base use:
//!
//! - [`BMSearchOptions`] and the cell-method argument types, which the cell
//!   decodes (cell methods 61-66) before forwarding to the base;
//! - [`AuctionItem`] and the `onBM*` client-method argument types (client
//!   methods 90-95), which the base and the cell encode;
//! - [`BMError`], the `onBMError` id vocabulary (decision D3);
//! - [`UIAuctionView`] (`clientKey`) and [`UIAuctionTime`]
//!   (`auctionLength`, `endTimeValue`).
//!
//! [`serialize_on_bm_open`] and [`serialize_on_bm_error`] are the two
//! one-argument encoders both halves of the split send. The index constants
//! are in `cell::cell_methods::black_market` (61-66) and
//! `cell::client_methods::black_market` (90-95).
//!
//! Names use STRING (4-byte LE length prefix + N UTF-8 bytes), **not**
//! WSTRING/UTF-16, unlike most other SGW social systems.

pub use cimmeria_patch_wire::black_market::{
    AuctionItem, BMCancelAuction, BMCreateAuction, BMError, BMPlaceBid, BMSearch, BMSearchOptions,
    BMStartWatchingItem, BMStopWatchingItem, CellCall, CellMethod, OnBMAuctionRemove,
    OnBMAuctionUpdate, OnBMAuctions, OnBMError, OnBMOpen, UIAuctionTime, UIAuctionView,
    MAX_AUCTION_ITEMS, MAX_STRING_LEN,
};
pub use cimmeria_patch_wire::{Decode, DecodeError, Encode, EncodeError};

/// Serialize `onBMOpen` args: `INT32 entityId`, the auctioneer the window
/// binds to as the conversation partner.
pub fn serialize_on_bm_open(entity_id: i32) -> Vec<u8> {
    encode_ints(&OnBMOpen { entity_id })
}

/// Serialize `onBMError` args: `INT32 errorId`.
pub fn serialize_on_bm_error(error: BMError) -> Vec<u8> {
    encode_ints(&OnBMError {
        error_id: error.id(),
    })
}

/// Encode an all-`INT32` argument list, which cannot fail: only strings and
/// arrays have caps.
fn encode_ints(args: &impl Encode) -> Vec<u8> {
    let mut out = Vec::with_capacity(4);
    args.write_to(&mut out)
        .expect("an INT32-only argument list has no cap to exceed");
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A `BMSearch` payload built by hand in `.def` order decodes field for
    /// field through the shared codec, including the eleventh INT32.
    #[test]
    fn search_options_decode_through_the_shared_codec() {
        let mut buf = vec![2u8];
        buf.extend_from_slice(&1i32.to_le_bytes()); // clientKey = MyAuctions
        buf.extend_from_slice(&0x3333_4444i32.to_le_bytes()); // sequenceId
        buf.push(1); // bForward
        for s in ["Sheppard", "", "Naq"] {
            buf.extend_from_slice(&(s.len() as u32).to_le_bytes());
            buf.extend_from_slice(s.as_bytes());
        }
        for v in [100i32, 50_000, 2000, 0x0F0F_0F0F] {
            buf.extend_from_slice(&v.to_le_bytes());
        }
        let opts = BMSearchOptions::decode(&buf).expect("decodes");
        assert_eq!(opts.sort_id, 2);
        assert_eq!(opts.client_key, 1);
        assert_eq!(opts.sequence_id, 0x3333_4444);
        assert_eq!(opts.b_forward, 1);
        assert_eq!(opts.seller_name, "Sheppard");
        assert_eq!(opts.item_name, "Naq");
        assert_eq!(
            (opts.min_tc, opts.max_tc, opts.quality),
            (100, 50_000, 2000)
        );
        assert_eq!(opts.filter_flags, 0x0F0F_0F0F);
        // One byte short, and one byte over, are both refused.
        assert!(BMSearchOptions::decode(&buf[..buf.len() - 1]).is_err());
        let mut long = buf.clone();
        long.push(0);
        assert!(BMSearchOptions::decode(&long).is_err());
    }

    #[test]
    fn bm_open_serializes_to_four_le_entity_id_bytes() {
        assert_eq!(
            serialize_on_bm_open(0x0123_4567),
            vec![0x67, 0x45, 0x23, 0x01]
        );
    }

    /// D3: `onBMError` carries the numeric id; the two shipped ids keep 0
    /// and 1.
    #[test]
    fn bm_error_serializes_the_id() {
        assert_eq!(
            serialize_on_bm_error(BMError::InvalidSortType),
            vec![0, 0, 0, 0]
        );
        assert_eq!(
            serialize_on_bm_error(BMError::BMUnavailable),
            vec![1, 0, 0, 0]
        );
        assert_eq!(
            serialize_on_bm_error(BMError::NotAtAuctioneer),
            vec![9, 0, 0, 0]
        );
    }
}
