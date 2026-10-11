//! `.despawn <entityId>` (lab-chaos CH-13, D-CH9): name the NPC by id instead
//! of selecting it.
//!
//! The lab runner's `burst` spawns NPCs with `.spawnrandom` and must later
//! remove exactly those, which it knows by id but cannot select. With no
//! argument `.despawn` still acts on the selected NPC, unchanged.

use crate::cell::space_manager::SpaceManager;
use crate::mercury::{SGWMOB_CLASS_ID, SGWPET_CLASS_ID};

/// `SGWBeing`'s class id (clientIndex 1): named NPCs such as Col Marsh are
/// beings, not mobs, and `.despawn` on a selection already accepts them.
const SGWBEING_CLASS_ID: u8 = 0x01;

/// Resolve `.despawn`'s `entityId` argument to an NPC the caller may despawn,
/// or the GM-facing reason it may not.
///
/// An NPC here is a non-player mob or being in the caller's space that is not
/// a pet. Refused: a non-number; an id no entity in the caller's space has
/// (an unknown id and an entity on another map get the same answer); and,
/// in the caller's space, a player, any pet (all pets belong to players;
/// `.pet dismiss` removes one properly) and every other entity class
/// (spawnable props, duel markers). The pet refusal is this id form's only:
/// `.despawn` with no argument still acts on a selected pet, as it did before
/// CH-13.
///
/// The space check comes straight after the existence check, so the reply
/// about an entity on another map says nothing about what it is: a GM can
/// not use the id form to learn whether some id elsewhere is a player.
///
/// Unlike a selection this does not require the NPC to be in the caller's
/// view: the runner may have moved since it spawned the NPCs. The same-space
/// rule keeps a stale or mistyped id from removing something on another map.
pub(in crate::cell::console) fn resolve_npc_id(
    caller_id: u32,
    arg: &str,
    space_mgr: &SpaceManager,
) -> Result<u32, String> {
    let Ok(id) = arg.parse::<u32>() else {
        return Err(format!(
            "entityId must be a non-negative whole number (got {arg:?})"
        ));
    };
    let not_here = || format!("no NPC with id {id} in your space");
    let Some(e) = space_mgr.get_entity(id) else {
        return Err(not_here());
    };
    let caller_space = space_mgr.get_entity_space_id(caller_id);
    if caller_space.is_none() || space_mgr.get_entity_space_id(id) != caller_space {
        return Err(not_here());
    }
    if e.is_player {
        return Err(format!(
            "entity {id} is a player; .despawn only removes NPCs"
        ));
    }
    if space_mgr.pets.is_pet(id) || e.class_id == SGWPET_CLASS_ID {
        return Err(format!(
            "entity {id} is a pet; .despawn only removes NPCs (use .pet dismiss)"
        ));
    }
    if e.class_id != SGWMOB_CLASS_ID && e.class_id != SGWBEING_CLASS_ID {
        return Err(format!(
            "entity {id} is not an NPC (entity class {}); .despawn only removes NPCs",
            e.class_id
        ));
    }
    Ok(id)
}
