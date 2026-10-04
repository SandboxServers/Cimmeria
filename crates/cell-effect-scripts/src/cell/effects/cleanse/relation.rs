//! Who a cleanse's target is to its caster, from the cell's attack rules:
//! a harmful cleanse needs the caster or an ally, a buff strip a hostile.
//!
//! The player rule is the support darts' (`cell-combat`'s
//! `use_ability::support_shot::classify`): a target the caster may attack
//! ([`player_may_attack`], duels included) is hostile, another player in
//! the caster's space is an ally, anything else is neither. An NPC caster
//! counts its own faction's NPCs as allies and whatever it would attack as
//! hostile.

use cimmeria_cell_world::cell::combat::{
    is_hostile_to_players, npc_may_target_npc, player_may_attack,
};
use cimmeria_cell_world::cell::duel::DuelResources;

use crate::cell::space_manager::SpaceManager;

/// The target's standing toward the caster.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Relation {
    /// The caster itself.
    Caster,
    /// Someone the caster may help.
    Ally,
    /// Someone the caster may attack.
    Hostile,
    /// Neither, or an entity that is gone.
    Other,
}

impl Relation {
    /// Stable `relation` value for logs.
    pub fn label(self) -> &'static str {
        match self {
            Self::Caster => "caster",
            Self::Ally => "ally",
            Self::Hostile => "hostile",
            Self::Other => "other",
        }
    }
}

/// `target_id`'s [`Relation`] to `caster_id`.
pub fn relation(space_mgr: &SpaceManager, caster_id: u32, target_id: u32) -> Relation {
    if caster_id == target_id {
        return Relation::Caster;
    }
    let (Some(caster), Some(target)) = (
        space_mgr.get_entity(caster_id),
        space_mgr.get_entity(target_id),
    ) else {
        return Relation::Other;
    };
    if caster.is_player {
        if player_may_attack(caster, target, space_mgr.resources.duels()) {
            return Relation::Hostile;
        }
        if target.is_player && target.space_id == caster.space_id {
            return Relation::Ally;
        }
        return Relation::Other;
    }
    if target.is_player {
        return if is_hostile_to_players(caster) {
            Relation::Hostile
        } else {
            Relation::Other
        };
    }
    if npc_may_target_npc(caster, target) {
        Relation::Hostile
    } else if caster.faction == target.faction && caster.space_id == target.space_id {
        Relation::Ally
    } else {
        Relation::Other
    }
}
