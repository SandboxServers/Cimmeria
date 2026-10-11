//! Typed decoders for server-to-client entity methods.
//!
//! Each decoder is written from `docs/protocol/client-method-dispatch-table.md`,
//! not borrowed from the server's own serializers or the `cimmeria-wire-log`
//! JSON decoders: an oracle that shares code with the thing it checks cannot
//! catch a bug in that code. Decoding is strict: a known method index whose
//! bytes do not fit its layout is an error, while an index with no decoder is
//! `Ok(None)`, because most methods have none.
//!
//! The method table depends on the entity. Index 27 is `onSystemCommunication`
//! on a player but `onAggressionOverrideUpdate` on an NPC, so the caller says
//! which kind it is with [`TargetKind`].

mod dialog;
mod entity;
mod inventory;
mod mission;
pub mod primitives;
mod sequence;

use thiserror::Error;

/// Which method table a target uses above index 26. The shared
/// SGWSpawnableEntity/SGWBeing range is the same for both kinds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TargetKind {
    /// A player character, whose table adds the player-only methods.
    Player,
    /// A mob or NPC.
    Npc,
}

/// One decoded server-to-client method.
#[derive(Debug, Clone, PartialEq)]
pub enum S2CEvent {
    /// `onDialogDisplay` (105, player).
    DialogDisplay(DialogDisplay),
    /// `InteractionType` (3, shared).
    InteractionType {
        /// The interaction the entity offers.
        type_id: u64,
    },
    /// `onMissionUpdate` (80, player).
    MissionUpdate {
        /// Mission the update is about.
        mission_id: i32,
        /// Mission state as the server sends it.
        status: i8,
        /// Name id of the NPC who gave the mission.
        giver_name_id: i32,
    },
    /// `onStepUpdate` (81, player).
    StepUpdate {
        /// Step within the mission.
        step_id: i32,
        /// Step state as the server sends it.
        status: i8,
    },
    /// `onObjectiveUpdate` (82, player).
    ObjectiveUpdate {
        /// Objective the update is about.
        objective_id: i32,
        /// Objective state as the server sends it.
        status: i8,
        /// Non-zero when the objective is hidden from the journal.
        hidden: i8,
        /// Non-zero when the objective is optional.
        optional: i8,
    },
    /// `onSequence` (1, shared).
    Sequence(Sequence),
    /// `onKnownAbilitiesUpdate` (101, player).
    KnownAbilitiesUpdate {
        /// Ability ids the player now knows.
        ability_ids: Vec<i32>,
    },
    /// `onPlayMovie` (155, player).
    PlayMovie {
        /// Movie to play.
        movie_name: String,
        /// Non-zero to play full screen.
        full_screen: u8,
    },
    /// `onActiveSlotUpdate` (70, player).
    ActiveSlotUpdate {
        /// Bag holding the active slot.
        bag_id: i32,
        /// Slot within that bag.
        slot_id: i32,
    },
    /// `onRemoveItem` (71, player).
    RemoveItem {
        /// Items to remove from the inventory.
        item_ids: Vec<i32>,
    },
    /// `onUpdateItem` (72, player).
    UpdateItem {
        /// Items whose state changed.
        items: Vec<InvItem>,
    },
    /// `onStaticMeshNameUpdate` (0, shared).
    StaticMeshNameUpdate {
        /// Static mesh the entity uses.
        static_mesh: String,
        /// Body set the entity uses.
        body_set: String,
    },
    /// `onEntityProperty` (7, shared).
    EntityProperty {
        /// Property id.
        property: i32,
        /// New value of the property.
        value: i32,
    },
    /// `onVisible` (8, shared).
    Visible {
        /// Non-zero when the entity becomes visible.
        visible: i8,
    },
    /// `onBeingNameIDUpdate` (11, shared).
    BeingNameIdUpdate {
        /// Name id the entity displays.
        name_id: i32,
    },
    /// `onPlayerCommunication` (28, player).
    PlayerCommunication {
        /// Name shown as the speaker.
        speaker: String,
        /// Speaker flags as the server sends them.
        speaker_flags: u8,
        /// Chat channel the text was sent on.
        channel: u8,
        /// The message text.
        text: String,
    },
}

/// `onDialogDisplay`: the dialog an entity opens for the player.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DialogDisplay {
    /// Entity offering the dialog.
    pub entity_id: i32,
    /// Dialog to show.
    pub dialog_id: i32,
    /// Mission flags attached to the dialog.
    pub mission_flags: i32,
    /// Non-zero when the dialog opens at once.
    pub is_immediate: u8,
    /// Mission the dialog belongs to, or zero.
    pub mission_id: i32,
}

/// `onSequence`: a Kismet event set, with its name/value pairs left out.
#[derive(Debug, Clone, PartialEq)]
pub struct Sequence {
    /// Kismet event-set sequence id.
    pub kismet_event_set_seq_id: i32,
    /// Entity that started the sequence.
    pub source_id: i32,
    /// Entity the sequence acts on.
    pub target_id: i32,
    /// Primary target selector.
    pub primary_target: i8,
    /// Seconds until the impact.
    pub impact_time: f32,
    /// Number of name/value pairs that followed on the wire.
    pub nvp_count: u32,
    /// View type selector.
    pub view_type: i8,
    /// Instance the sequence belongs to.
    pub instance_id: i32,
}

/// One item in an `onUpdateItem` batch.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InvItem {
    /// Item id.
    pub id: i32,
    /// Database id of the item.
    pub dbid: i32,
    /// Items in the stack.
    pub stack_size: i32,
    /// Slot the item sits in.
    pub slot_id: i32,
    /// Container (bag) the item sits in.
    pub container_id: i32,
    /// Whether the item is bound to the character.
    pub is_bound: bool,
    /// Current durability.
    pub durability: i32,
    /// Ammunition types the item accepts.
    pub ammo_types: Vec<i32>,
    /// Ammunition type currently loaded.
    pub cur_ammo_type: i32,
    /// Charges remaining.
    pub charges: i32,
}

/// Why a semantic decoder rejected a body. `method` is the wire name, so a
/// failure in a long bundle points at the method, not only at an offset.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum SemanticError {
    /// The body ends before a field (or a counted array) does.
    #[error("{method}: need {need} bytes at {at}, args are {len}")]
    Short {
        /// Wire name of the method.
        method: &'static str,
        /// Offset where the read started.
        at: usize,
        /// Bytes the read needed.
        need: usize,
        /// Total length of the args.
        len: usize,
    },
    /// The layout ended but bytes remain.
    #[error("{method}: {extra} trailing bytes")]
    Trailing {
        /// Wire name of the method.
        method: &'static str,
        /// Bytes left unread.
        extra: usize,
    },
    /// A `WSTRING` holds an unpaired surrogate.
    #[error("{method}: invalid UTF-16 in a WSTRING")]
    BadWstring {
        /// Wire name of the method.
        method: &'static str,
    },
    /// A field the layout names is not decoded yet.
    #[error("{method}: {what} not decoded")]
    Unsupported {
        /// Wire name of the method.
        method: &'static str,
        /// The part of the layout that is skipped.
        what: &'static str,
    },
    /// The method is listed, but its decoder is still a stub.
    #[error("{method}: decoder not written yet")]
    NotImplemented {
        /// Wire name of the method.
        method: &'static str,
    },
}

/// Decodes one server-to-client method.
///
/// `Ok(None)`: no decoder for this index on this kind (not an error).
/// `Err`: a decoder exists and the bytes do not fit it exactly (D-WC5).
pub fn decode_event(
    kind: TargetKind,
    method_index: u16,
    args: &[u8],
) -> Result<Option<S2CEvent>, SemanticError> {
    match (method_index, kind) {
        (0, _) => entity::static_mesh_name_update(args).map(Some),
        (1, _) => sequence::sequence(args).map(Some),
        (3, _) => dialog::interaction_type(args).map(Some),
        (7, _) => entity::entity_property(args).map(Some),
        (8, _) => entity::visible(args).map(Some),
        (11, _) => entity::being_name_id_update(args).map(Some),
        (28, TargetKind::Player) => entity::player_communication(args).map(Some),
        (70, TargetKind::Player) => inventory::active_slot_update(args).map(Some),
        (71, TargetKind::Player) => inventory::remove_item(args).map(Some),
        (72, TargetKind::Player) => inventory::update_item(args).map(Some),
        (80, TargetKind::Player) => mission::mission_update(args).map(Some),
        (81, TargetKind::Player) => mission::step_update(args).map(Some),
        (82, TargetKind::Player) => mission::objective_update(args).map(Some),
        (101, TargetKind::Player) => sequence::known_abilities_update(args).map(Some),
        (105, TargetKind::Player) => dialog::dialog_display(args).map(Some),
        (155, TargetKind::Player) => sequence::play_movie(args).map(Some),
        _ => Ok(None),
    }
}

#[cfg(test)]
mod tests {
    use super::{decode_event, TargetKind};
    use crate::tap_fixture::TapCapture;

    #[test]
    fn index_27_on_an_npc_has_no_decoder() {
        // Fixture #6: the one byte 0x03 sent to NPC 100745. On an NPC index 27
        // is onAggressionOverrideUpdate, which has no decoder yet.
        let cap = TapCapture::praxis_start();
        let args = cap.expect(6, "unknown").args();
        assert_eq!(decode_event(TargetKind::Npc, 27, &args), Ok(None));
    }

    #[test]
    fn an_unlisted_index_has_no_decoder() {
        // Index 50 is onOrganizationRankNameUpdate on a player, not in the
        // contract, so it is not an error.
        assert_eq!(decode_event(TargetKind::Player, 50, &[]), Ok(None));
    }
}
