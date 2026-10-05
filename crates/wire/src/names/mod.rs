//! Names for the wire's numeric ids, for logs: Mercury message ids and the
//! entity-method indices of every client-visible entity type.
//!
//! Every lookup returns `Option<&'static str>` and `None` for an id with no
//! name. tracing records an `Option` field only when it is `Some`, so a log
//! line carries the name next to its id when the id is known and leaves it
//! out otherwise; it never writes `"unknown"`.
//! Resolve the name inside the logging macro: tracing evaluates field values
//! only when the event is enabled, so a disabled DEBUG line pays nothing.
//!
//! - Method indices are per entity type, keyed by clientIndex (the wire
//!   typeID: `SGWPlayer` 0x02, `SGWGmPlayer` 0x03, `SGWMob` 0x04, `SGWPet`
//!   0x05, `Account` 0x07). The tables in [`defs`] are generated from
//!   `entities/defs/` by `mercury::def_conformance::names_codegen`, the same
//!   flattener that checks every hand-written index constant.
//! - Message ids get a message name of their own ([`client_msg_name`],
//!   [`server_msg_name`]): the Mercury interface entry for a system message
//!   (`authenticate`, `forcedPosition`), the range name for an entity method
//!   call. Rule 6 pairs `msg_id`/`opcode` with `msg_name` and
//!   `method_index`/`method_id` with `method_name`; a `msg_id` in an
//!   entity-method range carries both.
//!
//! Both are checked against `docs/protocol/*-dispatch-table.md` by
//! [`doc_conformance`], which fails naming the doc row that disagrees.

mod defs;
mod messages;

#[cfg(test)]
mod doc_conformance;

pub use crate::mercury::{
    ACCOUNT_CLASS_ID, SGWGMPLAYER_CLASS_ID, SGWMOB_CLASS_ID, SGWPET_CLASS_ID, SGWPLAYER_CLASS_ID,
};
pub use messages::{client_msg_name, server_msg_name};

/// The extended-encoding marker byte for an entity method whose index is at
/// or above the entity type's idbase.
const EXTENDED_MARKER: u8 = 0xBD;

/// The entity type name of clientIndex `class_id` (`"SGWPlayer"`).
pub fn class_name(class_id: u8) -> Option<&'static str> {
    defs::CLASSES
        .iter()
        .find(|(id, _)| *id == class_id)
        .map(|(_, name)| *name)
}

/// The ClientMethod (server to client) `index` of entity type `class_id`.
pub fn client_method(class_id: u8, index: u16) -> Option<&'static str> {
    defs::client_methods(class_id)
        .get(usize::from(index))
        .copied()
}

/// The exposed CellMethod (client to cell) `index` of entity type `class_id`.
pub fn cell_method(class_id: u8, index: u16) -> Option<&'static str> {
    defs::cell_methods(class_id)
        .get(usize::from(index))
        .copied()
}

/// The exposed BaseMethod (client to base) `index` of entity type `class_id`;
/// the client sends it as message id `0xC0 | index`.
pub fn base_method(class_id: u8, index: u16) -> Option<&'static str> {
    defs::base_methods(class_id)
        .get(usize::from(index))
        .copied()
}

/// A ClientMethod sent to the player's own entity. Read from the
/// `SGWGmPlayer` table, which is `SGWPlayer`'s plus the GM-only tail
/// (157-162), so a GM's debug methods are named too.
pub fn player_client_method(index: u16) -> Option<&'static str> {
    client_method(SGWGMPLAYER_CLASS_ID, index)
}

/// A cell method the player's client called. Read from the `SGWGmPlayer`
/// table (`SGWPlayer`'s 0-108 plus the GM tail 109-225): a non-GM sending a
/// GM index is named for what it asked for, and the GM gate refuses it.
pub fn player_cell_method(index: u16) -> Option<&'static str> {
    cell_method(SGWGMPLAYER_CLASS_ID, index)
}

/// A base method the in-world player's client called (`0xC0 | index`).
pub fn player_base_method(index: u16) -> Option<&'static str> {
    base_method(SGWPLAYER_CLASS_ID, index)
}

/// A ClientMethod sent to an entity whose type the caller does not know: the
/// name every in-world entity type that has `index` agrees on, else `None`.
///
/// The shared ancestors fix indices 0-26 for every being, and only SGWPlayer
/// and SGWGmPlayer reach past 31, so this names everything except 27-31,
/// where SGWPlayer's `Communicator` methods collide with SGWMob's and
/// SGWPet's own (`docs/protocol/client-method-dispatch-table.md`).
/// `Account` is not an in-world type and does not vote.
pub fn any_entity_client_method(index: u16) -> Option<&'static str> {
    let mut agreed = None;
    for &(class_id, _) in &defs::CLASSES {
        if class_id == ACCOUNT_CLASS_ID {
            continue;
        }
        match (agreed, client_method(class_id, index)) {
            (_, None) => {}
            (None, Some(name)) => agreed = Some(name),
            (Some(seen), Some(name)) if seen == name => {}
            (Some(_), Some(_)) => return None,
        }
    }
    agreed
}

/// A ClientMethod sent about an entity the caller knows only as a player or
/// not: the player table (GM tail included) for a player, else
/// [`any_entity_client_method`], which leaves a mob's or pet's 27-31 unnamed.
pub fn entity_client_method(entity_is_player: bool, index: u16) -> Option<&'static str> {
    if entity_is_player {
        player_client_method(index)
    } else {
        any_entity_client_method(index)
    }
}

/// The idbase of a method table `len` entries long: indices below it encode
/// directly (`0x80 | index`), the rest as `0xBD` plus `index - idbase`
/// (`EntityDescription_AssignClientMethodIds @ ghidra://SGW.exe@0x01590df0`).
fn idbase(len: usize) -> u16 {
    // Tables are a few hundred entries; the cast cannot truncate.
    (0x3E - (len + 0xC0) / 0xFF) as u16
}

/// The method a client-to-server entity message `msg_id` calls, on a client
/// whose entity is of type `class_id` (`ACCOUNT_CLASS_ID` at character
/// select, the player's class in world; `SGWPlayer` reads the SGWGmPlayer
/// superset, so a probe of a GM index is named). `None` for a system message, whose
/// name is [`server_msg_name`]'s. `payload` is the message body: the `0xBD`
/// sub-slot form reads its index from it (`[entity_id: u32][sub_index: u8]..`).
pub fn inbound_method(class_id: u8, msg_id: u8, payload: &[u8]) -> Option<&'static str> {
    // A non-GM player sending a GM-tail index is named for what it asked
    // for: SGWGmPlayer's tables strictly extend SGWPlayer's.
    let class_id = if class_id == SGWPLAYER_CLASS_ID {
        SGWGMPLAYER_CLASS_ID
    } else {
        class_id
    };
    match msg_id {
        0x80..=0xBF => {
            let base = idbase(defs::cell_methods(class_id).len());
            let direct = u16::from(msg_id - 0x80);
            if msg_id == EXTENDED_MARKER && base <= direct {
                let sub = *payload.get(4)?;
                cell_method(class_id, base + u16::from(sub))
            } else {
                cell_method(class_id, direct)
            }
        }
        0xC0..=0xFE => base_method(class_id, u16::from(msg_id - 0xC0)),
        _ => None,
    }
}

/// [`inbound_method`] from an in-world player's client: the `SGWGmPlayer`
/// tables, whose cell methods extend `SGWPlayer`'s with the GM tail and
/// whose base methods are the same.
pub fn player_inbound_method(msg_id: u8, payload: &[u8]) -> Option<&'static str> {
    inbound_method(SGWGMPLAYER_CLASS_ID, msg_id, payload)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unknown_ids_resolve_to_none_not_a_placeholder() {
        assert_eq!(client_method(SGWPLAYER_CLASS_ID, 157), None);
        assert_eq!(client_method(0xEE, 0), None);
        assert_eq!(cell_method(SGWPLAYER_CLASS_ID, 109), None);
        assert_eq!(base_method(ACCOUNT_CLASS_ID, 8), None);
        assert_eq!(
            class_name(0x08),
            None,
            "0x08 is Account's raw row, not its clientIndex"
        );
        assert_eq!(inbound_method(SGWPLAYER_CLASS_ID, 0xFF, &[]), None);
    }

    #[test]
    fn player_tables_include_the_gm_tail() {
        assert_eq!(player_client_method(156), Some("onCancelMovie"));
        assert_eq!(player_client_method(157), Some("onLOSResult"));
        assert_eq!(player_cell_method(108), Some("cancelMovie"));
        assert_eq!(player_cell_method(109), Some("gmMissionAssign"));
        assert_eq!(player_base_method(29), Some("perfStats"));
    }

    #[test]
    fn classes_are_keyed_by_client_index() {
        assert_eq!(class_name(SGWPLAYER_CLASS_ID), Some("SGWPlayer"));
        assert_eq!(class_name(ACCOUNT_CLASS_ID), Some("Account"));
        assert_eq!(client_method(ACCOUNT_CLASS_ID, 2), Some("onCharacterList"));
        assert_eq!(
            client_method(SGWMOB_CLASS_ID, 27),
            Some("onAggressionOverrideUpdate")
        );
        assert_eq!(
            client_method(SGWPET_CLASS_ID, 31),
            Some("onPetStanceUpdate")
        );
    }

    /// 27-31 mean different methods on a player, a mob and a pet; the shared
    /// prefix and the player-only range are unambiguous.
    #[test]
    fn any_entity_names_only_indices_every_type_agrees_on() {
        assert_eq!(any_entity_client_method(26), Some("BeingAppearance"));
        for index in 27..=31 {
            assert_eq!(any_entity_client_method(index), None, "index {index}");
        }
        assert_eq!(any_entity_client_method(32), Some("onChatLeft"));
        assert_eq!(any_entity_client_method(156), Some("onCancelMovie"));
        assert_eq!(
            entity_client_method(true, 27),
            Some("onSystemCommunication")
        );
        assert_eq!(entity_client_method(true, 157), Some("onLOSResult"));
        assert_eq!(entity_client_method(false, 27), None);
    }

    /// Character select and in-world give 0xC0 different meanings, and the
    /// 0xBD sub-slot reads its index from the payload.
    #[test]
    fn inbound_methods_resolve_by_entity_type() {
        assert_eq!(
            inbound_method(ACCOUNT_CLASS_ID, 0xC0, &[]),
            Some("versionInfoRequest")
        );
        assert_eq!(
            inbound_method(ACCOUNT_CLASS_ID, 0xC4, &[]),
            Some("playCharacter")
        );
        assert_eq!(
            inbound_method(SGWPLAYER_CLASS_ID, 0xC0, &[]),
            Some("chatJoin")
        );
        assert_eq!(
            inbound_method(SGWPLAYER_CLASS_ID, 0x80, &[]),
            Some("setTargetID")
        );
        assert_eq!(inbound_method(SGWPLAYER_CLASS_ID, 0x03, &[]), None);
        // useAbility = 68 = 61 + 7.
        let payload = [1, 0, 0, 0, 7, 0xAA];
        assert_eq!(
            inbound_method(SGWPLAYER_CLASS_ID, 0xBD, &payload),
            Some("useAbility")
        );
        assert_eq!(
            inbound_method(SGWGMPLAYER_CLASS_ID, 0xBD, &[1, 0, 0, 0, 48]),
            Some("gmMissionAssign")
        );
        assert_eq!(inbound_method(SGWPLAYER_CLASS_ID, 0xBD, &[1, 2]), None);
        // A non-GM probing the GM tail (109 = 61 + 48) is still named.
        assert_eq!(
            inbound_method(SGWPLAYER_CLASS_ID, 0xBD, &[1, 0, 0, 0, 48]),
            Some("gmMissionAssign")
        );
    }

    #[test]
    fn idbase_matches_the_client_formula() {
        assert_eq!(idbase(157), 61, "SGWPlayer ClientMethods");
        assert_eq!(idbase(109), 61, "SGWPlayer CellMethods");
        assert_eq!(idbase(226), 61, "SGWGmPlayer CellMethods");
        assert_eq!(idbase(32), 62, "SGWPet ClientMethods");
    }
}
