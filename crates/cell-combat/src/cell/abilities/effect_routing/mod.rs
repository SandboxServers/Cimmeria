//! Where each effect of a cast lands (ability-mechanics AB-07, audit B-27
//! and B-28).
//!
//! Before AB-07 every effect of an ability landed on the one target the cast
//! resolved: a self-buff half of an attack buffed the mob, a Self ability's
//! own penalty landed on whatever the client had selected, and a beneficial
//! area half (Morale Boost's "Short Radius AE 35% Focus Heal") never reached
//! anyone but that target. [`route_effect`] is the one place the per-effect
//! rule lives:
//!
//! | Rule | Effect | Lands on |
//! |---|---|---|
//! | 1 | carries `EF_ResolveOnAbilityUser` and deals no damage | the user ([`EffectRoute::User`]) |
//! | 2 | a `TCM_Single` effect of a Self ability with no area effect, dealing no damage | the user (D-AB01: a Self ability targets its user) |
//! | 3 | a beneficial `TCM_AERadius` effect of a player's non-ground cast | the caster's allies in its radius, the caster included ([`EffectRoute::AllyArea`]) |
//! | 4 | `TCM_Group` / `TCM_Aura` | the cast's target, as before: D-AB12 is open with the owner |
//! | 5 | anything else | the cast's target: the single-target pipeline, the cone fan-out and the ground collector, as before |
//!
//! "Beneficial" in rule 3 is the cast's (`ability_is_beneficial`), the
//! effect's own `EF_Beneficial_Effect` bit, or a heal script: a heal never
//! fans out to hostiles.
//!
//! Damage never routes onto the user: rules 1 and 2 leave an effect with a
//! damage NVP or a damage script on the target. Rule 2 skips a Self ability
//! that has an area effect, because there the single effects are the
//! follow-ups of the area hit (Whirlwind's knockdown lands on what the
//! whirlwind hit, not on its user).
//!
//! The off-target effects land through [`land::land_effects`]: their script
//! on the recipient, no QR roll (so a miss never drops them), no threat, no
//! in-combat state, and no #444 gate (the launch drops a target the gate
//! would refuse when the cast still has a user half, [`launch_target`]).
//! The target pipeline then runs the rest of the ability
//! ([`RoutedCast::target_def`]).
//!
//! The secondary targets of a ground cast and of an explosive round's splash
//! take only the part of the ability that belongs to them
//! ([`secondary_scope`], [`splash_scope`]), the way the cone fan-out has
//! always scoped its def to the cone effect.

use std::collections::HashMap;

use cimmeria_cell_world::cell::duel::DuelResources;
use cimmeria_entity::abilities::{
    effect_is_implemented, AbilityDef, EffectDef, EF_BENEFICIAL_EFFECT, EF_RESOLVE_ON_ABILITY_USER,
    HEAL_SCRIPTS, TARGET_GROUND, TARGET_SELF, TCM_AE_CONE, TCM_AE_RADIUS, TCM_SINGLE,
};

use super::super::combat;
use super::super::space_manager::SpaceManager;

mod ally_area;
mod land;

pub(in crate::cell::abilities) use land::{land_effects, Landing, LandingRoute};

/// `TCM_Group`: the caster's group. Routing waits for D-AB12.
pub(crate) const TCM_GROUP: &str = "TCM_Group";
/// `TCM_Aura`: an aura around the caster. Routing waits for D-AB12.
pub(crate) const TCM_AURA: &str = "TCM_Aura";

/// `event` of every per-effect routing row (target `abilities`).
pub(crate) const EVENT_EFFECT_ROUTED: &str = "effect_routed";
/// `event` of the launch's routing rows (target `abilities`).
pub(crate) const EVENT_ROUTING_LAUNCH: &str = "effect_routing_launch";

/// `reason`: rule 1.
pub(crate) const REASON_RESOLVE_ON_USER: &str = "resolve_on_ability_user";
/// `reason`: rule 2.
pub(crate) const REASON_SELF_ABILITY: &str = "self_ability";
/// `reason`: rule 3.
pub(crate) const REASON_BENEFICIAL_AREA: &str = "beneficial_area";
/// `reason`: rule 4.
pub(crate) const REASON_GROUP_AURA_PENDING: &str = "group_aura_pending_d_ab12";
/// `reason`: rule 1 or 2 matched, but the effect deals damage.
pub(crate) const REASON_DAMAGE_STAYS_ON_TARGET: &str = "damage_stays_on_target";
/// `reason`: rule 5.
pub(crate) const REASON_TARGET: &str = "target";

/// Where one effect of a cast lands.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum EffectRoute {
    /// The ability's user.
    User,
    /// Every ally of the caster within the effect's radius of the caster.
    AllyArea,
    /// The cast's target, through the pipeline that served it before AB-07.
    Target,
}

/// What [`route_effect`] needs to know about the cast.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) struct CastShape {
    /// The ability is `TargetSelf`.
    pub self_ability: bool,
    /// The ability is `TargetGround`: its area effects belong to the ground
    /// collector.
    pub ground: bool,
    /// The ability has a `TCM_AERadius` or `TCM_AECone` effect.
    pub has_area_effect: bool,
    /// The cast is a player's beneficial cast (AB-01).
    pub beneficial_cast: bool,
    /// The caster is a player. Only a player's area halves fan out to allies:
    /// "ally" is the support-shot rule, which is defined for players.
    pub player_caster: bool,
}

impl CastShape {
    /// The shape of `caster_id` casting `def`.
    pub(crate) fn of(
        space_mgr: &SpaceManager,
        caster_id: u32,
        def: &AbilityDef,
        beneficial_cast: bool,
    ) -> Self {
        let has_area_effect = def.effect_ids.iter().any(|id| {
            space_mgr.effect_defs.get(id).is_some_and(|e| {
                e.target_collection_method == TCM_AE_RADIUS
                    || e.target_collection_method == TCM_AE_CONE
            })
        });
        Self {
            self_ability: def.target_type_id == TARGET_SELF,
            ground: def.target_type_id == TARGET_GROUND,
            has_area_effect,
            beneficial_cast,
            player_caster: space_mgr.get_entity(caster_id).is_some_and(|c| c.is_player),
        }
    }
}

/// Whether `effect` deals damage: a damage NVP or a damage script.
pub(crate) fn deals_damage(effect: &EffectDef) -> bool {
    effect.param_i32("HealthDamage") > 0
        || effect.param_i32("FocusDamage") > 0
        || effect
            .script_name
            .as_deref()
            .is_some_and(super::damage_apply::is_damage_script)
}

/// Whether `effect` helps whoever it lands on, for rule 3.
fn effect_is_beneficial(effect: &EffectDef, shape: CastShape) -> bool {
    shape.beneficial_cast
        || effect.flags & EF_BENEFICIAL_EFFECT != 0
        || effect
            .script_name
            .as_deref()
            .is_some_and(|s| HEAL_SCRIPTS.contains(&s))
}

/// Where `effect` of a cast of `shape` lands, and the `reason` it logs (the
/// module docs' table).
pub(crate) fn route_effect(effect: &EffectDef, shape: CastShape) -> (EffectRoute, &'static str) {
    let tcm = effect.target_collection_method.as_str();
    let user_flag = effect.flags & EF_RESOLVE_ON_ABILITY_USER != 0;
    let pure_self_single = shape.self_ability && !shape.has_area_effect && tcm == TCM_SINGLE;
    if user_flag || pure_self_single {
        if deals_damage(effect) {
            return (EffectRoute::Target, REASON_DAMAGE_STAYS_ON_TARGET);
        }
        let reason = if user_flag {
            REASON_RESOLVE_ON_USER
        } else {
            REASON_SELF_ABILITY
        };
        return (EffectRoute::User, reason);
    }
    if tcm == TCM_AE_RADIUS
        && !shape.ground
        && shape.player_caster
        && effect_is_beneficial(effect, shape)
    {
        return (EffectRoute::AllyArea, REASON_BENEFICIAL_AREA);
    }
    if tcm == TCM_GROUP || tcm == TCM_AURA {
        return (EffectRoute::Target, REASON_GROUP_AURA_PENDING);
    }
    (EffectRoute::Target, REASON_TARGET)
}

/// A cast split by [`plan_cast`].
#[derive(Debug, Default)]
pub(in crate::cell::abilities) struct RoutedCast {
    /// The effects that land through [`land_effects`], in effect order with
    /// the area fan-outs last.
    pub landings: Vec<Landing>,
    /// The ability minus the effects in `landings`: what the target pipeline
    /// runs. `None` when the cast had no def.
    pub target_def: Option<AbilityDef>,
    /// How many effects were taken off the target pipeline.
    pub moved: usize,
}

impl RoutedCast {
    /// `true` when routing took every effect off the target pipeline: the
    /// cast has nothing left for its target (Combat Sprint).
    pub(in crate::cell::abilities) fn target_has_nothing(&self) -> bool {
        self.moved > 0
            && self
                .target_def
                .as_ref()
                .is_none_or(|d| d.effect_ids.is_empty())
    }

    /// `true` when an effect lands on `caster_id`.
    pub(in crate::cell::abilities) fn lands_on(&self, entity_id: u32) -> bool {
        self.landings.iter().any(|l| l.recipient == entity_id)
    }
}

/// Split `caster_id`'s cast of `def` by [`route_effect`].
///
/// `beneficial_target` is the entity a beneficial cast resolved to (AB-01):
/// its `Target` effects land there through [`land_effects`] too, and
/// `target_def` is left empty. For every other cast it is `None`, and the
/// `Target` effects stay in `target_def` for the damage pipeline.
pub(in crate::cell::abilities) fn plan_cast(
    space_mgr: &SpaceManager,
    caster_id: u32,
    def: Option<&AbilityDef>,
    beneficial_target: Option<u32>,
) -> RoutedCast {
    let Some(def) = def else {
        return RoutedCast::default();
    };
    let shape = CastShape::of(space_mgr, caster_id, def, beneficial_target.is_some());
    let mut landings = Vec::new();
    let mut area = Vec::new();
    let mut target_ids = Vec::with_capacity(def.effect_ids.len());
    let mut moved = 0;
    for &eid in &def.effect_ids {
        let Some(effect) = space_mgr.effect_defs.get(&eid) else {
            target_ids.push(eid);
            continue;
        };
        let (route, reason) = route_effect(effect, shape);
        if route != EffectRoute::Target || reason != REASON_TARGET {
            log_route(space_mgr, caster_id, def, effect, route, reason);
        }
        match (route, beneficial_target) {
            (EffectRoute::User, _) => {
                landings.push(Landing::new(effect, caster_id, LandingRoute::User(reason)))
            }
            (EffectRoute::AllyArea, _) => area.push(effect),
            (EffectRoute::Target, Some(target)) => {
                landings.push(Landing::new(effect, target, LandingRoute::BeneficialTarget))
            }
            (EffectRoute::Target, None) => {
                target_ids.push(eid);
                continue;
            }
        }
        moved += 1;
    }
    for effect in area {
        let allies = ally_area::ally_landings(space_mgr, caster_id, def, effect, &landings);
        landings.extend(allies);
    }
    let target_def = beneficial_target.is_none().then(|| AbilityDef {
        effect_ids: target_ids,
        ..def.clone()
    });
    RoutedCast {
        landings,
        target_def,
        moved,
    }
}

fn log_route(
    space_mgr: &SpaceManager,
    caster_id: u32,
    def: &AbilityDef,
    effect: &EffectDef,
    route: EffectRoute,
    reason: &'static str,
) {
    let who = space_mgr.player_identity(caster_id);
    tracing::debug!(
        target: "abilities",
        event = EVENT_EFFECT_ROUTED,
        cast_id = space_mgr.current_cast_id(),
        account_id = who.account_id,
        player_id = who.player_id,
        entity_id = caster_id,
        ability_id = def.ability_id,
        effect_id = effect.effect_id,
        tcm = effect.target_collection_method.as_str(),
        effect_flags = effect.flags,
        route = ?route,
        reason,
        "effect routed: user halves land on the caster, beneficial area halves on its allies"
    );
}

/// The launch's target for a player's non-beneficial cast of `def` at
/// `wire_target`.
///
/// A cast whose every effect that does something lands off the target (a
/// pure Self ability such as Combat Sprint) has no target: 0, so the #444
/// gate, the range and the line-of-sight checks never see the client's
/// selection. A cast with a user half and a target half at a target #444
/// would refuse (an ally, a vendor, the caster) also gets 0: the user half
/// lands and the hostile half has nothing to land on. Any other cast keeps
/// the wire target, and #444 as before.
pub(in crate::cell::abilities) fn launch_target(
    space_mgr: &SpaceManager,
    caster_id: u32,
    def: Option<&AbilityDef>,
    wire_target: i32,
) -> i32 {
    let (Some(def), Some(caster)) = (def, space_mgr.get_entity(caster_id)) else {
        return wire_target;
    };
    if !caster.is_player {
        return wire_target;
    }
    let shape = CastShape::of(space_mgr, caster_id, def, false);
    let (mut off, mut on) = (0usize, 0usize);
    for effect in def
        .effect_ids
        .iter()
        .filter_map(|id| space_mgr.effect_defs.get(id))
        .filter(|e| effect_is_implemented(Some(e)))
    {
        match route_effect(effect, shape).0 {
            EffectRoute::Target => on += 1,
            EffectRoute::User | EffectRoute::AllyArea => off += 1,
        }
    }
    if off == 0 {
        return wire_target;
    }
    let resolution = if on == 0 {
        "off_target_only"
    } else {
        let attackable = (wire_target > 0)
            .then(|| space_mgr.get_entity(wire_target as u32))
            .flatten()
            .is_none_or(|t| combat::player_may_attack(caster, t, space_mgr.resources.duels()));
        if attackable {
            return wire_target;
        }
        "user_half_only"
    };
    let who = space_mgr.player_identity(caster_id);
    let target_player_id =
        (wire_target > 0).then(|| space_mgr.player_identity(wire_target as u32).player_id);
    tracing::debug!(
        target: "abilities",
        event = EVENT_ROUTING_LAUNCH,
        account_id = who.account_id,
        player_id = who.player_id,
        entity_id = caster_id,
        ability_id = def.ability_id,
        wire_target_id = wire_target,
        target_player_id = target_player_id.flatten(),
        off_target_effects = off,
        target_effects = on,
        resolution,
        "cast launched without its client target: what it does lands on the caster or its allies"
    );
    0
}

/// The part of `def` a ground cast's secondary targets take: everything that
/// stays on the target ([`route_effect`]) except the single-target damage,
/// which is the primary's own hit (3170's 4728) or its DoT (Devastating
/// Blast's 2720). The radius effects (3170's 4729), the cones and the
/// non-damage single effects (Flashbang's debuff, authored on `TCM_Single`
/// with "Small Radius AE" text) stay.
///
/// When that leaves no damage at all although the ability deals some (the
/// damage authored only as a single effect), the secondaries keep the whole
/// target part, as before AB-07, and the fallback is logged.
pub(in crate::cell::abilities) fn secondary_scope(
    space_mgr: &SpaceManager,
    caster_id: u32,
    def: &Option<AbilityDef>,
) -> Option<AbilityDef> {
    let def = def.as_ref()?;
    let target = plan_cast(space_mgr, caster_id, Some(def), None).target_def?;
    let effects = |ids: &[i32]| -> Vec<&EffectDef> {
        ids.iter()
            .filter_map(|id| space_mgr.effect_defs.get(id))
            .collect()
    };
    let area: Vec<i32> = effects(&target.effect_ids)
        .into_iter()
        .filter(|e| e.target_collection_method != TCM_SINGLE || !deals_damage(e))
        .map(|e| e.effect_id)
        .collect();
    let area_damage = effects(&area).into_iter().any(deals_damage);
    let any_damage = effects(&target.effect_ids).into_iter().any(deals_damage);
    if !area_damage && any_damage {
        let who = space_mgr.player_identity(caster_id);
        tracing::debug!(
            target: "abilities",
            event = EVENT_EFFECT_ROUTED,
            cast_id = space_mgr.current_cast_id(),
            account_id = who.account_id,
            player_id = who.player_id,
            entity_id = caster_id,
            ability_id = def.ability_id,
            effect_ids = ?target.effect_ids,
            reason = "no_area_damage_effect",
            "ground secondaries keep the whole ability: its damage is authored only on single-target effects"
        );
        return Some(target);
    }
    Some(AbilityDef {
        effect_ids: area,
        ..target
    })
}

/// The part of `def` an explosive round's splash target takes: the shot's
/// direct damage, scaled by the splash fraction in `damage_apply`. A pulsing
/// effect (a DoT) is the primary's alone: its first tick used to land on
/// every splash target with no registration behind it. `def` is already the
/// target part of the cast ([`RoutedCast::target_def`]).
pub(in crate::cell::abilities) fn splash_scope(
    effect_defs: &HashMap<i32, EffectDef>,
    def: &Option<AbilityDef>,
) -> Option<AbilityDef> {
    let def = def.as_ref()?;
    Some(AbilityDef {
        effect_ids: def
            .effect_ids
            .iter()
            .copied()
            .filter(|id| effect_defs.get(id).is_none_or(|e| !e.is_pulsing()))
            .collect(),
        ..def.clone()
    })
}

#[cfg(test)]
mod tests;
