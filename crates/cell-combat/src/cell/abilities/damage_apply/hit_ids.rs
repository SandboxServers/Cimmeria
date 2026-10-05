//! The ids of one attacker → target hit, carried into the damage pipeline's
//! rows.

use cimmeria_entity::cell_entity::PlayerIdentity;

/// One attacker → target hit, carried into the submodules' logs. The
/// identities are the canonical `account_id`/`player_id` correlators
/// (instrumentation-discipline rule 5): the actor's pair and the target's
/// `player_id`, both empty (and so omitted from the log) for an NPC.
#[derive(Debug, Clone, Copy)]
pub(super) struct HitIds {
    pub(super) entity_id: u32,
    pub(super) target_eid: u32,
    pub(super) ability_id: i32,
    /// `ability_id`'s name, resolved once per hit for the hit's rows.
    pub(super) ability_name: Option<&'static str>,
    pub(super) actor: PlayerIdentity,
    pub(super) target: PlayerIdentity,
    /// The attacker's and target's names (Rule 6), resolved once per hit:
    /// the NVP rows log while the defender's stats are borrowed and can't
    /// ask the `SpaceManager`. A player's is the interned character name
    /// (a copy); an NPC's costs one NameBook read.
    pub(super) entity_name: Option<&'static str>,
    pub(super) target_name: Option<&'static str>,
    /// The resolving cast (AB-T1's cast scope); `None` outside a cast.
    pub(super) cast_id: Option<i32>,
    /// The target is in GM god mode (#1170): the hit's pool rows are
    /// logged before `GodModeGuard` puts the loss back, and its
    /// `god_mode_absorbed` row says what was restored.
    pub(super) god_mode: bool,
    /// The target's world: the AB-T6 metrics' `world` label.
    pub(super) world: &'static str,
}

impl HitIds {
    /// The `effect_planned` row's ids for this hit (AB-T3).
    pub(super) fn plan_ids(self) -> super::super::effect_plan::PlanIds {
        super::super::effect_plan::PlanIds {
            caster_id: self.entity_id,
            target_id: self.target_eid,
            ability_id: self.ability_id,
            ability_name: self.ability_name,
            cast_id: self.cast_id,
            caster: self.actor,
            target: self.target,
            caster_name: self.entity_name,
            target_name: self.target_name,
            world: self.world,
        }
    }
}
