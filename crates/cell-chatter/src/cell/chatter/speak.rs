//! Who speaks a line and who hears it.
//!
//! A line's speaker is the living NPC carrying its `speaker_tag` in the
//! group's space; its listeners are the connected players within the group's
//! `hear_radius` of that NPC. The line goes to each listener as
//! `onPlayerCommunication(speaker name, SPEAKER_None, CHAN_say, text)`
//! (client method 28), through the chat broadcaster's own serializer, so it
//! renders like any NPC bark or player say line.

use cimmeria_common::Vector3;
use cimmeria_entity::cell_entity::CellEntity;
use cimmeria_wire::cell::chat::{serialize_on_player_communication, CHAN_SAY};
use cimmeria_wire::cell::client_methods::communicator::ON_PLAYER_COMMUNICATION;
use cimmeria_wire::state_field::BSF_DEAD;

use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceInstance;

/// `ESpeakerFlags::SPEAKER_None`: an NPC line is neither a GM line nor a
/// DND auto-reply.
const SPEAKER_NONE: u8 = 0;

/// The living NPC in `space` tagged `tag`. A corpse (`BSF_DEAD`) does not
/// speak.
pub(crate) fn speaker<'s>(space: &'s SpaceInstance, tag: &str) -> Option<&'s CellEntity> {
    space
        .entities
        .values()
        .filter(|e| !e.is_player && e.state_field & BSF_DEAD == 0)
        .find(|e| e.tag.as_deref() == Some(tag))
}

/// The connected players in `space` within `radius` of `at`, by entity id.
pub(crate) fn listeners(space: &SpaceInstance, at: &Vector3, radius: f32) -> Vec<u32> {
    let r2 = radius * radius;
    let mut out: Vec<u32> = space
        .players
        .iter()
        .copied()
        .filter(|id| {
            space
                .entities
                .get(id)
                .is_some_and(|p| p.position.distance_squared_to(at) <= r2)
        })
        .collect();
    out.sort_unstable();
    out
}

/// Whether any connected player is within `radius` of any of the NPCs
/// tagged `tags`: the test for starting an exchange.
pub(crate) fn anyone_in_earshot(space: &SpaceInstance, tags: &[&str], radius: f32) -> bool {
    tags.iter()
        .filter_map(|t| speaker(space, t))
        .any(|npc| !listeners(space, &npc.position, radius).is_empty())
}

/// One `EntityMethodCall` per listener carrying the say line.
pub(crate) fn line_messages(
    speaker_name: &str,
    text: &str,
    listeners: &[u32],
) -> Vec<CellToBaseMsg> {
    let args = serialize_on_player_communication(speaker_name, SPEAKER_NONE, CHAN_SAY, text);
    listeners
        .iter()
        .map(|&entity_id| CellToBaseMsg::EntityMethodCall {
            entity_id,
            method_index: ON_PLAYER_COMMUNICATION,
            args: args.clone(),
        })
        .collect()
}
