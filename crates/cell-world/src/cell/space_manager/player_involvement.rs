//! Whether a player has a stake in an NPC event: the one gate for the
//! "nobody saw it" WARNs on the NPC path (DA-F2).
//!
//! An NPC method or attack sequence that reaches no witness is only a bug
//! when a player should have seen it. Two NPCs fighting with no player in
//! AoI drop every `onStatUpdate`, `onStateFieldUpdate` and `onSequence` by
//! design: nobody is there to draw them. The Debug Area arena logged about
//! 55 such WARNs in its first 30 minutes on the colo (release
//! v2026-10-05.1), none of them a fault. Owner decision: NPC-vs-NPC
//! no-witness warnings with no player present go to null; with a player
//! involved they stay WARNs.
//!
//! [`SpaceManager::player_involved`] is that rule. Its callers are the
//! no-witness WARNs themselves (`abilities.wire event=wire_npc_no_witnesses`
//! and `abilities.sequence outcome=no_witnesses`), which only run once the
//! witness list is known to be empty, so "no player among the witnesses" is
//! already true when this is asked. Rules:
//!
//! - **Player side** ([`SpaceManager::is_player_side`]): a player, or an
//!   entity a player controls or owns (a pet, a deployable, a GM's lab
//!   dummy). A pet counts as its owner.
//! - **Involved**: the entity itself, the event's counterpart (the attack's
//!   target, when the caller has one), or anyone on the entity's threat
//!   list is player side. The threat list covers the paths that name no
//!   counterpart, such as a health update on an NPC a player is fighting.

use super::SpaceManager;
use cimmeria_entity::cell_entity::PetState;

impl SpaceManager {
    /// Whether `entity_id` is a player, or an entity a player owns: a pet
    /// (its owner), a deployable or a lab dummy. Unknown ids are not.
    pub fn is_player_side(&self, entity_id: u32) -> bool {
        if self.pets.is_pet(entity_id) || self.deployables.get(entity_id).is_some() {
            return true;
        }
        self.get_entity(entity_id).is_some_and(|e| {
            e.is_player
                || e.extensions.contains::<PetState>()
                || e.extensions.contains::<super::LabDummy>()
        })
    }

    /// The DA-F2 rule: whether a player has a stake in an event on
    /// `entity_id` whose other party (an attack's target) is `counterpart`.
    ///
    /// True when `entity_id`, `counterpart` or anyone on `entity_id`'s
    /// threat list is [player side](Self::is_player_side). A no-witness
    /// WARN on the NPC path is written only when this is true; an
    /// NPC-only event with nobody watching writes nothing.
    pub fn player_involved(&self, entity_id: u32, counterpart: Option<u32>) -> bool {
        if self.is_player_side(entity_id) || counterpart.is_some_and(|c| self.is_player_side(c)) {
            return true;
        }
        self.get_entity(entity_id).is_some_and(|e| {
            e.threat_list
                .keys()
                .any(|&threat| self.is_player_side(threat))
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const NPC_A: u32 = 10;
    const NPC_B: u32 = 11;
    const PLAYER: u32 = 1;
    const PET: u32 = 12;

    fn mgr() -> SpaceManager {
        let mut mgr = SpaceManager::new(1);
        mgr.parse_spaces_xml(
            r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle" Instanced="false" MinX="-800" MaxX="800" MinY="-800" MaxY="800" /></Spaces>"#,
        )
        .unwrap();
        mgr.create_startup_spaces(
            r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle" /></Spaces>"#,
        )
        .unwrap();
        for id in [PLAYER, NPC_A, NPC_B, PET] {
            mgr.create_entity(id, "Castle", [id as f32, 0.0, 0.0], [0.0; 3])
                .unwrap();
        }
        mgr.get_entity_mut(PLAYER).unwrap().is_player = true;
        mgr.pets.register(PLAYER, PET, mgr.player_identity(PLAYER));
        mgr
    }

    #[test]
    fn two_npcs_with_no_player_anywhere_are_not_involved() {
        let mut mgr = mgr();
        mgr.get_entity_mut(NPC_A)
            .unwrap()
            .threat_list
            .insert(NPC_B, 5.0);
        assert!(!mgr.player_involved(NPC_A, Some(NPC_B)));
        assert!(!mgr.player_involved(NPC_A, None));
    }

    #[test]
    fn a_player_source_target_or_threat_entry_involves_a_player() {
        let mut mgr = mgr();
        assert!(mgr.player_involved(PLAYER, Some(NPC_A)));
        assert!(mgr.player_involved(NPC_A, Some(PLAYER)));
        mgr.get_entity_mut(NPC_A)
            .unwrap()
            .threat_list
            .insert(PLAYER, 5.0);
        assert!(mgr.player_involved(NPC_A, None));
    }

    #[test]
    fn a_pet_counts_as_its_owner() {
        let mut mgr = mgr();
        assert!(mgr.is_player_side(PET));
        assert!(mgr.player_involved(NPC_A, Some(PET)));
        mgr.get_entity_mut(NPC_B)
            .unwrap()
            .threat_list
            .insert(PET, 1.0);
        assert!(mgr.player_involved(NPC_B, None));
    }
}
