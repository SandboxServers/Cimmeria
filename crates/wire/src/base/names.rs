//! Names of the SGWPlayer exposed base methods, by flattened index.
//!
//! The client sends base method `index` as the message id `0xC0 | index`
//! (`docs/protocol/sgwplayer-base-method-dispatch-table.md`). The index is
//! the method's position among SGWPlayer's exposed base methods in `.def`
//! parse order: the implemented interfaces first (Communicator,
//! OrganizationMember, MinigamePlayer, ClientCache), then SGWPlayer's own.
//! The names are generated from `entities/defs/` (`crate::names`), and
//! `def_conformance::base_methods` checks [`BASE_METHOD_COUNT`] against
//! them.
//!
//! The base plugin registry (`cimmeria-base-session`'s `base::plugin`)
//! refuses a registration whose index has no name here.

use crate::names::{base_method, SGWPLAYER_CLASS_ID};

/// The number of SGWPlayer exposed base methods.
pub const BASE_METHOD_COUNT: u8 = 30;

/// The name of SGWPlayer base method `index` (flattened, the message id
/// minus `0xC0`), or `"unknown"` past the last one. Reads the table
/// `crate::names` generates from `entities/defs/`.
pub fn base_method_name(index: u8) -> &'static str {
    base_method(SGWPLAYER_CLASS_ID, u16::from(index)).unwrap_or("unknown")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_follow_the_dispatch_table_at_its_anchors() {
        assert_eq!(base_method_name(0), "chatJoin");
        assert_eq!(base_method_name(0xD5 - 0xC0), "elementDataRequest");
        assert_eq!(base_method_name(0xD9 - 0xC0), "sendDuelChallenge");
        assert_eq!(base_method_name(0xDD - 0xC0), "perfStats");
        assert_eq!(base_method_name(BASE_METHOD_COUNT), "unknown");
        assert_eq!(base_method_name(u8::MAX), "unknown");
    }
}
