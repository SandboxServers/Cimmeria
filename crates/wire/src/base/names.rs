//! Names of the SGWPlayer exposed base methods, by flattened index.
//!
//! The client sends base method `index` as the message id `0xC0 | index`
//! (`docs/protocol/sgwplayer-base-method-dispatch-table.md`). The index is
//! the method's position among SGWPlayer's exposed base methods in `.def`
//! parse order: the implemented interfaces first (Communicator,
//! OrganizationMember, MinigamePlayer, ClientCache), then SGWPlayer's own.
//! `def_conformance::base_methods` checks this table against
//! `entities/defs/`, so it cannot drift.
//!
//! The base plugin registry (`cimmeria-base-session`'s `base::plugin`)
//! refuses a registration whose index has no name here.

/// Every SGWPlayer exposed base method, indexed by flattened index.
const BASE_METHOD_NAMES: [&str; 30] = [
    // Communicator (0-14)
    "chatJoin",
    "chatLeave",
    "sendPlayerCommunication",
    "chatSetAFKMessage",
    "chatSetDNDMessage",
    "chatIgnore",
    "chatFriend",
    "chatList",
    "chatMute",
    "chatKick",
    "chatOp",
    "chatBan",
    "chatPassword",
    "petition",
    "announcePetition",
    // OrganizationMember (15-18)
    "organizationInvite",
    "organizationInviteByType",
    "organizationKick",
    "organizationRankChange",
    // MinigamePlayer (19)
    "minigameCallRequest",
    // ClientCache (20-21)
    "versionInfoRequest",
    "elementDataRequest",
    // SGWPlayer (22-29)
    "logOff",
    "cancelLogOff",
    "onClientReady",
    "sendDuelChallenge",
    "onSpaceQueueStatus",
    "onSpaceQueueReadyResponse",
    "onSpaceQueuedResponse",
    "perfStats",
];

/// The number of SGWPlayer exposed base methods.
pub const BASE_METHOD_COUNT: u8 = BASE_METHOD_NAMES.len() as u8;

/// The name of SGWPlayer base method `index` (flattened, the message id
/// minus `0xC0`), or `"unknown"` past the last one.
pub fn base_method_name(index: u8) -> &'static str {
    BASE_METHOD_NAMES
        .get(index as usize)
        .copied()
        .unwrap_or("unknown")
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
