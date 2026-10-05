//! Method-name lookup for the wire-log capture surface: thin wrappers over
//! `cimmeria_wire::names`, whose tables are generated from `entities/defs/`
//! and checked against `docs/protocol/*-dispatch-table.md`.
//!
//! The `&str` forms keep the `msg_name` field's old contract (`"unknown"` for
//! an id with no name); the `Option` forms feed the `method_name` field,
//! which is left out instead.

use cimmeria_wire::names;

/// Outbound (server → client) method name for the flat SGWPlayer method
/// index, `"unknown"` past the table. Correct for a method sent to the
/// player's own entity; [`outbound_name`] also handles other entities.
pub fn outbound_method_name(method_index: u16) -> &'static str {
    names::client_method(names::SGWPLAYER_CLASS_ID, method_index).unwrap_or("unknown")
}

/// The name of client method `method_index` on an entity the caller knows
/// as a player or not: the player table (GM tail included) for a player,
/// otherwise the name every in-world type agrees on (a mob's or pet's 27-31
/// stay unnamed).
pub fn outbound_name(target_is_player: bool, method_index: u16) -> Option<&'static str> {
    names::entity_client_method(target_is_player, method_index)
}

/// One label for an inbound (client → server) message, for the packet
/// tap: the method an entity message calls, else the system message's name,
/// else `"unknown"`. `class_id` is the clientIndex of the client's entity:
/// `Account` at character select, the player's class in world (base method
/// ids 0xC0+ mean different methods on the two). Log lines carry the two
/// names apart, as `msg_name` and `method_name`.
pub fn inbound_label(class_id: u8, msg_id: u8, payload: &[u8]) -> &'static str {
    names::inbound_method(class_id, msg_id, payload)
        .or_else(|| names::server_msg_name(msg_id))
        .unwrap_or("unknown")
}

#[cfg(test)]
mod tests {
    use super::*;
    use cimmeria_wire::names::{ACCOUNT_CLASS_ID, SGWMOB_CLASS_ID, SGWPLAYER_CLASS_ID};

    #[test]
    fn outbound_method_table_covers_full_range() {
        for idx in 0..157u16 {
            assert_ne!(
                outbound_method_name(idx),
                "unknown",
                "client method index {idx} missing from table"
            );
        }
        assert_eq!(outbound_method_name(157), "unknown");
        assert_eq!(outbound_method_name(999), "unknown");
    }

    #[test]
    fn outbound_method_names_load_bearing_examples() {
        assert_eq!(outbound_method_name(1), "onSequence");
        assert_eq!(outbound_method_name(26), "BeingAppearance");
        assert_eq!(outbound_method_name(105), "onDialogDisplay");
        assert_eq!(outbound_method_name(154), "onThreatenedMobsUpdate");
    }

    /// A mob's 27 is `onAggressionOverrideUpdate`, a player's is
    /// `onSystemCommunication`: a witness method on another entity at 27-31
    /// is left unnamed rather than given the player's name.
    #[test]
    fn outbound_name_does_not_guess_another_entitys_type() {
        assert_eq!(outbound_name(true, 27), Some("onSystemCommunication"));
        assert_eq!(outbound_name(false, 27), None);
        assert_eq!(
            names::client_method(SGWMOB_CLASS_ID, 27),
            Some("onAggressionOverrideUpdate")
        );
        assert_eq!(outbound_name(false, 26), Some("BeingAppearance"));
    }

    /// The client-to-server system ids follow `ClientMessageList[]`
    /// (`messages.cpp`, `docs/protocol/message-dispatch-table.md`). The old
    /// table here had 0x06 `loggedOff`, 0x0A `createCellPlayer` and a 0x0D
    /// `channelSetup` that the interface does not have.
    #[test]
    fn inbound_system_messages_named() {
        let p = SGWPLAYER_CLASS_ID;
        assert_eq!(inbound_label(p, 0x00, &[]), "baseAppLogin");
        assert_eq!(inbound_label(p, 0x01, &[]), "authenticate");
        assert_eq!(inbound_label(p, 0x03, &[]), "avatarUpdateExplicit");
        assert_eq!(inbound_label(p, 0x06, &[]), "switchInterface");
        assert_eq!(inbound_label(p, 0x08, &[]), "enableEntities");
        assert_eq!(inbound_label(p, 0x0A, &[]), "vehicleAck");
        assert_eq!(inbound_label(p, 0x0C, &[]), "disconnect");
        assert_eq!(inbound_label(p, 0x0D, &[]), "unknown");
    }

    /// Regression guard: cell entity methods live at 0x80..=0xBF and are
    /// named by METHOD INDEX; base methods (0xC0+) by the entity type.
    #[test]
    fn inbound_entity_methods_resolve_by_index_and_entity_type() {
        use cimmeria_wire::cell::dispatch::cell_method_name;
        let p = SGWPLAYER_CLASS_ID;
        assert_eq!(inbound_label(p, 0x80, &[]), cell_method_name(0));
        assert_eq!(inbound_label(p, 0x80, &[]), "setTargetID");
        // Sub-slot form: [entity_id: u32][sub_index][args..] -> index 61 + sub.
        let payload = [0x02, 0, 0, 0, 7, 0xAA];
        assert_eq!(inbound_label(p, 0xBD, &payload), cell_method_name(61 + 7));
        // Too short for the sub-index: the message name is all there is.
        assert_eq!(inbound_label(p, 0xBD, &[1, 2]), "cellMethod");
        assert_eq!(inbound_label(p, 0xC5, &[]), "chatIgnore");
        assert_eq!(
            inbound_label(ACCOUNT_CLASS_ID, 0xC5, &[]),
            "deleteCharacter"
        );
        assert_eq!(
            inbound_label(ACCOUNT_CLASS_ID, 0xC0, &[]),
            "versionInfoRequest"
        );
    }
}
