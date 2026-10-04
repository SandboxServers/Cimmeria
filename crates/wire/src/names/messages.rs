//! Mercury system-message names: the two interface tables of
//! `docs/protocol/message-dispatch-table.md`, by message id.
//!
//! Ids 0x80 and up are entity methods, named by the entity's method tables
//! (`super::inbound_method`), not here; the message name of those ids is
//! the range's (`cellMethod`, `baseMethod`, `entityMethod`). `super::doc_conformance` checks
//! both tables against the doc row by row.

/// Server to client (`ClientInterface`, `ServerMessageList[]` in
/// `messages.cpp`): 57 entries, 0x00-0x38.
const CLIENT_INTERFACE: [&str; 0x39] = [
    "authenticate",                // 0x00
    "bandwidthNotification",       // 0x01
    "updateFrequencyNotification", // 0x02
    "setGameTime",                 // 0x03
    "resetEntities",               // 0x04
    "createBasePlayer",            // 0x05
    "createCellPlayer",            // 0x06
    "spaceData",                   // 0x07
    "spaceViewportInfo",           // 0x08
    "createEntity",                // 0x09
    "updateEntity",                // 0x0A
    "entityInvisible",             // 0x0B
    "leaveAoI",                    // 0x0C
    "tickSync",                    // 0x0D
    "setSpaceViewport",            // 0x0E
    "setVehicle",                  // 0x0F
    "avatarUpdateNoAliasFullPosYawPitchRoll",
    "avatarUpdateNoAliasFullPosYawPitch",
    "avatarUpdateNoAliasFullPosYaw",
    "avatarUpdateNoAliasFullPosNoDir",
    "avatarUpdateNoAliasOnChunkYawPitchRoll",
    "avatarUpdateNoAliasOnChunkYawPitch",
    "avatarUpdateNoAliasOnChunkYaw",
    "avatarUpdateNoAliasOnChunkNoDir",
    "avatarUpdateNoAliasOnGroundYawPitchRoll",
    "avatarUpdateNoAliasOnGroundYawPitch",
    "avatarUpdateNoAliasOnGroundYaw",
    "avatarUpdateNoAliasOnGroundNoDir",
    "avatarUpdateNoAliasNoPosYawPitchRoll",
    "avatarUpdateNoAliasNoPosYawPitch",
    "avatarUpdateNoAliasNoPosYaw",
    "avatarUpdateNoAliasNoPosNoDir",
    "avatarUpdateAliasFullPosYawPitchRoll",
    "avatarUpdateAliasFullPosYawPitch",
    "avatarUpdateAliasFullPosYaw",
    "avatarUpdateAliasFullPosNoDir",
    "avatarUpdateAliasOnChunkYawPitchRoll",
    "avatarUpdateAliasOnChunkYawPitch",
    "avatarUpdateAliasOnChunkYaw",
    "avatarUpdateAliasOnChunkNoDir",
    "avatarUpdateAliasOnGroundYawPitchRoll",
    "avatarUpdateAliasOnGroundYawPitch",
    "avatarUpdateAliasOnGroundYaw",
    "avatarUpdateAliasOnGroundNoDir",
    "avatarUpdateAliasNoPosYawPitchRoll",
    "avatarUpdateAliasNoPosYawPitch",
    "avatarUpdateAliasNoPosYaw",
    "avatarUpdateAliasNoPosNoDir",
    "detailedPosition", // 0x30
    "forcedPosition",   // 0x31
    "controlEntity",    // 0x32
    "voiceData",        // 0x33
    "restoreClient",    // 0x34
    "restoreBaseApp",   // 0x35
    "resourceFragment", // 0x36
    "loggedOff",        // 0x37
    "entityMessage",    // 0x38
];

/// Client to server (`ServerInterface`, `ClientMessageList[]`): 13 entries,
/// 0x00-0x0C.
const SERVER_INTERFACE: [&str; 0x0D] = [
    "baseAppLogin",             // 0x00
    "authenticate",             // 0x01
    "avatarUpdateImplicit",     // 0x02
    "avatarUpdateExplicit",     // 0x03
    "avatarUpdateWardImplicit", // 0x04
    "avatarUpdateWardExplicit", // 0x05
    "switchInterface",          // 0x06
    "requestEntityUpdate",      // 0x07
    "enableEntities",           // 0x08
    "viewportAck",              // 0x09
    "vehicleAck",               // 0x0A
    "restoreClientAck",         // 0x0B
    "disconnect",               // 0x0C
];

/// The Mercury message name of a server-to-client `msg_id`: the
/// `ClientInterface` entry for 0x00-0x38, `entityMethod` for an entity
/// method call (0x80-0xFE; the method itself is `super::client_method`'s)
/// and `replyMessage` for 0xFF. `None` for the unassigned 0x39-0x7F.
pub fn client_msg_name(msg_id: u8) -> Option<&'static str> {
    match msg_id {
        0x80..=0xFE => Some("entityMethod"),
        0xFF => Some("replyMessage"),
        _ => CLIENT_INTERFACE.get(usize::from(msg_id)).copied(),
    }
}

/// The Mercury message name of a client-to-server `msg_id`: the
/// `ServerInterface` entry for 0x00-0x0C, `cellMethod` (0x80-0xBF) or
/// `baseMethod` (0xC0-0xFE) for an entity method call (the method itself is
/// `super::inbound_method`'s), `replyMessage` for 0xFF.
pub fn server_msg_name(msg_id: u8) -> Option<&'static str> {
    match msg_id {
        0x80..=0xBF => Some("cellMethod"),
        0xC0..=0xFE => Some("baseMethod"),
        0xFF => Some("replyMessage"),
        _ => SERVER_INTERFACE.get(usize::from(msg_id)).copied(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The message-id constants the server builds packets with name the
    /// message the table has at their id (`BASEMSG_TICK_SYNC` -> `tickSync`).
    #[test]
    fn server_message_id_constants_match_the_table() {
        use crate::mercury::aoi::{
            BASEMSG_CREATE_ENTITY, BASEMSG_ENTITY_INVISIBLE, BASEMSG_LEAVE_AOI,
            BASEMSG_UPDATE_AVATAR_NO_ALIAS_FULL_POS_YPR,
        };
        use crate::mercury::{
            BASEMSG_CREATE_BASE_PLAYER, BASEMSG_CREATE_CELL_PLAYER, BASEMSG_FORCED_POSITION,
            BASEMSG_LOGGED_OFF, BASEMSG_RESET_ENTITIES, BASEMSG_RESOURCE_FRAGMENT,
            BASEMSG_SET_GAME_TIME, BASEMSG_SPACE_VIEWPORT_INFO, BASEMSG_TICK_SYNC,
            BASEMSG_UPDATE_FREQUENCY_NOTIFICATION,
        };
        for (id, name) in [
            (
                BASEMSG_UPDATE_FREQUENCY_NOTIFICATION,
                "updateFrequencyNotification",
            ),
            (BASEMSG_SET_GAME_TIME, "setGameTime"),
            (BASEMSG_RESET_ENTITIES, "resetEntities"),
            (BASEMSG_CREATE_BASE_PLAYER, "createBasePlayer"),
            (BASEMSG_CREATE_CELL_PLAYER, "createCellPlayer"),
            (BASEMSG_SPACE_VIEWPORT_INFO, "spaceViewportInfo"),
            (BASEMSG_CREATE_ENTITY, "createEntity"),
            (BASEMSG_ENTITY_INVISIBLE, "entityInvisible"),
            (BASEMSG_LEAVE_AOI, "leaveAoI"),
            (BASEMSG_TICK_SYNC, "tickSync"),
            (
                BASEMSG_UPDATE_AVATAR_NO_ALIAS_FULL_POS_YPR,
                "avatarUpdateNoAliasFullPosYawPitchRoll",
            ),
            (BASEMSG_FORCED_POSITION, "forcedPosition"),
            (BASEMSG_RESOURCE_FRAGMENT, "resourceFragment"),
            (BASEMSG_LOGGED_OFF, "loggedOff"),
        ] {
            assert_eq!(client_msg_name(id), Some(name), "{id:#04x}");
        }
        assert_eq!(client_msg_name(0x39), None);
        assert_eq!(server_msg_name(0x0D), None);
        // Entity-method ranges carry a message name too; the method is
        // named separately.
        assert_eq!(client_msg_name(0x9B), Some("entityMethod"));
        assert_eq!(server_msg_name(0xBD), Some("cellMethod"));
        assert_eq!(server_msg_name(0xC5), Some("baseMethod"));
    }
}
