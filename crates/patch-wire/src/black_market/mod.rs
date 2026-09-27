//! The Black Market (`SGWBlackMarketManager`, implemented by `SGWPlayer`).
//!
//! - [`methods`]: the method indices, names and the two UI enums.
//! - [`types`]: the `AuctionItem` and `BMSearchOptions` `FIXED_DICT`s.
//! - [`client`]: the server-to-client methods `onBMOpen` … `onBMWatchedItemsUpdate`
//!   (client indices 90–95), which the stock client drops.
//! - [`cell`]: the client-to-server methods `BMSearch` … `BMStopWatchingItem`
//!   (cell indices 61–66), which the stock client never sends.
//!
//! Every layout follows `entities/defs/interfaces/SGWBlackMarketManager.def`
//! and the `AuctionItem` / `BMSearchOptions` entries in
//! `entities/defs/alias.xml`, field for field. The tests read those files
//! and fail if the encoders drift from them. Evidence for the client side
//! (the dispatcher, the enum values, `clientKey`):
//! `docs/reverse-engineering/findings/black-market-client-io.md`.
//!
//! # Caps
//!
//! Chosen so the largest payload the caps allow still fits one entity
//! message (the server's framing limits a message body to 65,535 bytes,
//! of which 5 are the entity id and the sub-index byte):
//!
//! | Cap | Value | Applies to |
//! |---|---|---|
//! | [`MAX_STRING_LEN`] | 255 bytes | every `STRING`: `sellerName`, `bidderName`, `itemName` |
//! | [`MAX_AUCTION_ITEMS`] | 200 | `onBMAuctions.auctionItems`. The UI shows 8 rows a page |
//! | [`MAX_WATCHED_ITEMS`] | 256 | `onBMWatchedItemsUpdate.itemList` |
//!
//! At the caps, `onBMAuctions` is 4 + 200 × (37 + 255) + 8 = 58,412 bytes.

pub mod cell;
pub mod client;
pub mod methods;
pub mod types;

pub use cell::{
    BMCancelAuction, BMCreateAuction, BMPlaceBid, BMSearch, BMStartWatchingItem,
    BMStopWatchingItem, CellCall,
};
pub use client::{
    ClientCall, OnBMAuctionRemove, OnBMAuctionUpdate, OnBMAuctions, OnBMError, OnBMOpen,
    OnBMWatchedItemsUpdate,
};
pub use methods::{
    CellMethod, ClientMethod, UIAuctionTime, UIAuctionView, EXTENDED_MESSAGE_ID,
    EXTENDED_METHOD_BASE,
};
pub use types::{AuctionItem, BMSearchOptions};

/// Longest `STRING` either side will encode or accept, in bytes.
pub const MAX_STRING_LEN: u32 = 255;

/// Most `AuctionItem`s one `onBMAuctions` may carry.
pub const MAX_AUCTION_ITEMS: u32 = 200;

/// Most item definition ids one `onBMWatchedItemsUpdate` may carry.
pub const MAX_WATCHED_ITEMS: u32 = 256;

/// Declares an argument list made only of `INT32`s: the struct, and an
/// encoder and decoder that read and write the fields in the order listed,
/// which must be the `.def` order. The string after each field is its `.def`
/// name, used in error messages.
macro_rules! int32_args {
    (
        $(#[$meta:meta])*
        $name:ident { $( $(#[$fmeta:meta])* $field:ident = $def_name:literal ),+ $(,)? }
    ) => {
        $(#[$meta])*
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
        pub struct $name {
            $( $(#[$fmeta])* pub $field: i32, )+
        }

        impl $crate::Encode for $name {
            fn write_to(&self, out: &mut Vec<u8>) -> Result<(), $crate::EncodeError> {
                $( $crate::codec::write_i32(out, self.$field); )+
                Ok(())
            }
        }

        impl $crate::Decode for $name {
            fn decode_from<S: $crate::ByteSource + ?Sized>(
                src: &mut S,
            ) -> Result<Self, $crate::DecodeError> {
                // Struct-literal fields evaluate in the order written, which
                // is the order listed above.
                Ok(Self {
                    $( $field: $crate::codec::read_i32(src, $def_name)?, )+
                })
            }
        }
    };
}
use int32_args;

#[cfg(test)]
mod tests;
