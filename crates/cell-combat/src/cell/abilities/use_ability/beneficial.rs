//! Who a player's cast lands on: the #444 target gate, and the beneficial
//! casts that turn it around (ability-mechanics AB-01, D-AB01, D-AB02).
//!
//! The client sends its current target for every non-ground ability, Self
//! ones included (audit B-10): a press of 597 Heal Focus arrives with target
//! 0, the caster, an ally or a hostile. Before AB-01 the server took that id
//! at its word, so a Self heal with nothing selected charged its cooldown and
//! did nothing, with the caster or an ally selected was refused by #444, and
//! with a mob selected healed the mob (B-12 to B-14).
//!
//! [`resolve_cast_target`] is the one place the rule lives. For a player's
//! **beneficial** cast ([`cimmeria_entity::abilities::ability_is_beneficial`]):
//!
//! - a `TargetSelf` ability lands on the caster, whatever the wire said
//!   (D-AB01; python's `canUse` did the same, `AbilityManager.py:527`);
//! - a `TargetTarget` ability lands on its target when that is the caster or
//!   an ally (another player the caster may not attack, in the same space:
//!   [`super::support_shot::classify`], the support-dart rule);
//! - anything else (a hostile, a neutral NPC, a dead or missing target, no
//!   target) takes the D-AB02 branch, [`no_ally`]: fall back to the caster
//!   (the proposed default), or refuse with feedback.
//!
//! A beneficial cast then fires through [`fire_beneficial`], not the damage
//! pipeline: the effects' scripts run on the resolved entity with no QR roll,
//! no threat, no in-combat state and no channel cancel, and the stat update
//! goes to that entity and its witnesses.
//!
//! Every other cast (an NPC's, or a player's non-beneficial one) passes the
//! wire target through unchanged and keeps the #444 gate in [`target_gate`].
//! The resolver never maps a beneficial cast to a hostile, and the gate
//! refuses one that somehow arrives aimed at a non-ally, so a beneficial
//! effect has no path onto a hostile.

use tokio::sync::mpsc;

use cimmeria_cell_world::cell::duel::DuelResources;
use cimmeria_entity::abilities::{ability_is_beneficial, AbilityDef, TARGET_SELF};
use cimmeria_entity::cell_entity::CellEntity;
use cimmeria_wire::cell::chat::{serialize_on_player_communication, CHAN_FEEDBACK};

use super::super::super::combat;
use super::super::super::messages::CellToBaseMsg;
use super::super::super::space_manager::SpaceManager;
use super::super::effect_routing::{land_effects, plan_cast, Landing};
use super::support_shot::{classify, SupportTarget};

/// `event` of every beneficial-cast resolution (target `abilities`).
pub(crate) const EVENT_BENEFICIAL_CAST: &str = "beneficial_cast";
/// `resolution`: a `TargetSelf` ability, landed on the caster.
pub(crate) const RESOLUTION_SELF_ABILITY: &str = "self_ability";
/// `resolution`: a `TargetTarget` ability at the caster or an ally.
pub(crate) const RESOLUTION_ALLY: &str = "ally";
/// `resolution`: no ally to land on; D-AB02's default lands it on the caster.
pub(crate) const RESOLUTION_FALLBACK_TO_CASTER: &str = "fallback_to_caster";
/// `resolution`: no ally to land on, and D-AB02 refuses.
pub(crate) const RESOLUTION_REFUSED: &str = "refused";

/// The feedback line for a refused beneficial cast (D-AB02's refusal branch).
pub(crate) const NO_ALLY_FEEDBACK: &str = "That ability needs a friendly target.";

/// D-AB02, still open with the owner: what a beneficial `TargetTarget` cast
/// with no ally to land on does. `true` is the proposed default (fall back
/// to the caster); `false` refuses it with [`NO_ALLY_FEEDBACK`] before the
/// cooldown is charged. [`no_ally`] is the only reader.
const FALLBACK_TO_CASTER: bool = true;

/// Where a cast lands, from [`resolve_cast_target`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CastTarget {
    /// The caster (a beneficial cast).
    Caster,
    /// The caster or an ally the client named (a beneficial cast).
    Ally(u32),
    /// A non-beneficial cast's target, left to the #444 gate.
    Hostile(u32),
    /// Nothing: a non-beneficial cast with no target, or a refused
    /// beneficial one.
    None,
}

/// Whether `caster_id` casting `def` is a player's beneficial cast, the only
/// kind this module resolves.
pub(crate) fn is_player_beneficial(
    space_mgr: &SpaceManager,
    caster_id: u32,
    def: Option<&AbilityDef>,
) -> bool {
    def.is_some_and(|d| ability_is_beneficial(d, &space_mgr.effect_defs))
        && space_mgr.get_entity(caster_id).is_some_and(|c| c.is_player)
}

/// Where `caster_id`'s cast of `def` at `wire_target` lands (module docs).
pub(crate) fn resolve_cast_target(
    space_mgr: &SpaceManager,
    caster_id: u32,
    def: Option<&AbilityDef>,
    wire_target: i32,
) -> CastTarget {
    resolve(space_mgr, caster_id, def, wire_target).0
}

/// [`resolve_cast_target`] with the `resolution` it logs.
fn resolve(
    space_mgr: &SpaceManager,
    caster_id: u32,
    def: Option<&AbilityDef>,
    wire_target: i32,
) -> (CastTarget, &'static str) {
    if !is_player_beneficial(space_mgr, caster_id, def) {
        let target = if wire_target > 0 {
            CastTarget::Hostile(wire_target as u32)
        } else {
            CastTarget::None
        };
        return (target, "not_beneficial");
    }
    if def.is_some_and(|d| d.target_type_id == TARGET_SELF) {
        return (CastTarget::Caster, RESOLUTION_SELF_ABILITY);
    }
    let ally = (wire_target > 0)
        .then(|| {
            let caster = space_mgr.get_entity(caster_id)?;
            let target = space_mgr.get_entity(wire_target as u32)?;
            let alive = !combat::is_dead_state(target.state_field);
            (alive && classify(caster, target, space_mgr.resources.duels()) == SupportTarget::Ally)
                .then_some(wire_target as u32)
        })
        .flatten();
    match ally {
        Some(id) => (CastTarget::Ally(id), RESOLUTION_ALLY),
        None => no_ally(),
    }
}

/// D-AB02: a beneficial `TargetTarget` cast with no ally to land on. The
/// fallback and the refusal differ only here.
fn no_ally() -> (CastTarget, &'static str) {
    if FALLBACK_TO_CASTER {
        (CastTarget::Caster, RESOLUTION_FALLBACK_TO_CASTER)
    } else {
        (CastTarget::None, RESOLUTION_REFUSED)
    }
}

/// Log the `beneficial_cast` row. `stage` is `launch` (DEBUG: it runs before
/// the launch's dead, known and cooldown checks, so a forged packet must not
/// buy an INFO row) or `fire` (INFO: once per committed cast).
fn log_resolution(
    space_mgr: &SpaceManager,
    caster_id: u32,
    def: Option<&AbilityDef>,
    wire_target: i32,
    target: CastTarget,
    resolution: &'static str,
    stage: &'static str,
) {
    let who = space_mgr.player_identity(caster_id);
    let resolved_target_id = match target {
        CastTarget::Caster => Some(caster_id),
        CastTarget::Ally(id) | CastTarget::Hostile(id) => Some(id),
        CastTarget::None => None,
    };
    let target_player_id =
        resolved_target_id.and_then(|id| space_mgr.player_identity(id).player_id);
    macro_rules! row {
        ($level:expr) => {
            tracing::event!(
                target: "abilities",
                $level,
                event = EVENT_BENEFICIAL_CAST,
                account_id = who.account_id,
                player_id = who.player_id,
                entity_id = caster_id,
                ability_id = def.map(|d| d.ability_id),
                effect_ids = ?def.map(|d| d.effect_ids.as_slice()),
                target_type_id = def.map(|d| d.target_type_id),
                wire_target_id = wire_target,
                resolved_target_id,
                target_player_id,
                resolution,
                stage,
                "beneficial cast resolved: heals and buffs land on the caster or an ally, never a hostile"
            )
        };
    }
    if stage == "launch" {
        row!(tracing::Level::DEBUG);
    } else {
        row!(tracing::Level::INFO);
    }
}

/// A resolution and the `resolution` reason it logs.
pub(super) type Resolved = (CastTarget, &'static str);

/// The fire-time resolution of a beneficial cast, from the client's original
/// target (not the launch's resolved one, so a `fallback_to_caster` keeps its
/// reason), logged once at INFO. The fire uses it for both `Ability_End` and
/// the effects, so the animation names where the cast lands.
pub(super) fn resolve_at_fire(
    space_mgr: &SpaceManager,
    caster_id: u32,
    def: Option<&AbilityDef>,
    wire_target: i32,
) -> Resolved {
    let (target, resolution) = resolve(space_mgr, caster_id, def, wire_target);
    log_resolution(
        space_mgr,
        caster_id,
        def,
        wire_target,
        target,
        resolution,
        "fire",
    );
    (target, resolution)
}

/// The entity a resolution lands on, as an `onSequence` target id (0 for none).
pub(super) fn landing_id(caster_id: u32, target: CastTarget) -> i32 {
    match target {
        CastTarget::Caster => caster_id as i32,
        CastTarget::Ally(id) => id as i32,
        CastTarget::Hostile(_) | CastTarget::None => 0,
    }
}

/// The launch's view of the client's target: `Some((target_id, beneficial))`
/// to go on with, `None` when the cast is refused (feedback already sent).
///
/// A `diverted` cast (a summon, an owner-pet ability, a deployable) has its
/// own target rule and gets target 0, as before. A non-beneficial cast keeps
/// the wire target, unless its effects all land off the target or its user
/// half would be refused with it (`effect_routing::launch_target`: target 0).
/// A beneficial cast gets the resolved entity, so the
/// launch's range and line-of-sight checks, the warmup's anchor and the
/// `onSequence` all name where it will land.
pub(super) async fn launch_target(
    caster_id: u32,
    def: Option<&AbilityDef>,
    wire_target: i32,
    diverted: bool,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &SpaceManager,
) -> Option<(i32, bool)> {
    if diverted {
        return Some((0, false));
    }
    if !is_player_beneficial(space_mgr, caster_id, def) {
        // A cast whose user half has nowhere else to land, or whose target
        // #444 would refuse, drops its target (AB-07, `effect_routing`).
        let target =
            super::super::effect_routing::launch_target(space_mgr, caster_id, def, wire_target);
        return Some((target, false));
    }
    let (target, resolution) = resolve(space_mgr, caster_id, def, wire_target);
    log_resolution(
        space_mgr,
        caster_id,
        def,
        wire_target,
        target,
        resolution,
        "launch",
    );
    match target {
        CastTarget::Caster => Some((caster_id as i32, true)),
        CastTarget::Ally(id) => Some((id as i32, true)),
        CastTarget::Hostile(_) | CastTarget::None => {
            send_no_ally_feedback(caster_id, tx, space_mgr).await;
            None
        }
    }
}

async fn send_no_ally_feedback(
    caster_id: u32,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &SpaceManager,
) {
    let chat = serialize_on_player_communication("SYSTEM", 0, CHAN_FEEDBACK, NO_ALLY_FEEDBACK);
    if tx
        .send(CellToBaseMsg::EntityMethodCall {
            entity_id: caster_id,
            method_index: crate::mercury::method_idx::ON_PLAYER_COMMUNICATION,
            args: chat,
        })
        .await
        .is_err()
    {
        let who = space_mgr.player_identity(caster_id);
        tracing::warn!(
            target: "abilities",
            event = "beneficial_feedback_send_failed",
            account_id = who.account_id,
            player_id = who.player_id,
            entity_id = caster_id,
            reason = "cell_to_base_closed",
            "beneficial-cast refusal feedback could not be queued (base channel closed)"
        );
    }
}

/// The launch's target gate, from [`target_gate`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum TargetGate {
    /// Go on. `friendly` is a support shot at an ally or a beneficial cast:
    /// it never arms the auto-cycle attack loop.
    Admitted { friendly: bool },
    /// A support shot at a hostile target: refused with its own feedback.
    SupportHostile,
    /// Refused by #444 (logged here); the launch returns `false`.
    Refused,
}

/// The single-target gate for `caster` aiming at `target`.
///
/// **#444, the server-authority target-validity gate**, scoped to PLAYER
/// attackers (that is the forgery vector). A non-beneficial cast resolves as
/// damage (`apply_damage_to_target` has no offensive-vs-supportive branch),
/// so a player may only aim one at what `combat::player_may_attack` admits:
/// a hostile NPC, or the engaged duel partner (SS-D2); pets obey the same
/// function. A non-hostile NPC (a vendor, a quest giver) must never take
/// player damage. Mirrors the AoE (`abilities/dispatch/`) and cone
/// (`abilities/cone_aoe.rs`) faction filters; without it a forged
/// `useAbility` griefs vendors, quest NPCs, party members or other players
/// (the client UI restricts target selection, but the server must enforce
/// it).
///
/// NPC attackers are deliberately NOT gated: the NPC AI fight tick calls the
/// same launch to attack a PLAYER, which is legitimate (the AI already picks
/// valid targets server-side).
///
/// Two casts turn the rule around:
///
/// - a **support shot** (beneficial ammo, AM-11d) admits an ally or the
///   shooter and is refused at a hostile target with feedback; anything else
///   (a vendor) still falls to #444;
/// - a **beneficial cast** (AB-01) arrives already resolved to the caster or
///   an ally ([`launch_target`]) and is admitted; one aimed anywhere else is
///   refused, so a heal can never be pointed at a hostile.
pub(super) fn target_gate(
    caster: &CellEntity,
    target: &CellEntity,
    ability_id: i32,
    support_shot: bool,
    beneficial: bool,
    space_mgr: &SpaceManager,
) -> TargetGate {
    let duels = space_mgr.resources.duels();
    let who = space_mgr.player_identity(caster.entity_id.0 as u32);
    let target_who = space_mgr.player_identity(target.entity_id.0 as u32);
    if support_shot {
        match classify(caster, target, duels) {
            SupportTarget::Hostile => return TargetGate::SupportHostile,
            SupportTarget::Ally => return TargetGate::Admitted { friendly: true },
            SupportTarget::Other => {}
        }
    }
    if beneficial {
        if classify(caster, target, duels) == SupportTarget::Ally {
            return TargetGate::Admitted { friendly: true };
        }
        tracing::warn!(
            target: "abilities",
            event = EVENT_BENEFICIAL_CAST,
            account_id = who.account_id,
            player_id = who.player_id,
            entity_id = caster.entity_id.0,
            ability_id,
            target_id = target.entity_id.0,
            target_player_id = target_who.player_id,
            reason = "beneficial_non_ally_target",
            "useAbility rejected -- beneficial cast aimed at a non-ally after resolution; \
             nothing applied"
        );
        return TargetGate::Refused;
    }
    if caster.is_player && !combat::player_may_attack(caster, target, duels) {
        tracing::warn!(
            account_id = who.account_id,
            player_id = who.player_id,
            entity_id = caster.entity_id.0,
            ability_id,
            target_id = target.entity_id.0,
            target_player_id = target_who.player_id,
            target_is_player = target.is_player,
            target_faction = target.faction,
            "useAbility rejected -- player single-target ability against a \
             non-hostile target (friendly-fire / forged target); \
             damage pipeline not entered (#444)"
        );
        return TargetGate::Refused;
    }
    TargetGate::Admitted { friendly: false }
}

/// Fire a committed beneficial cast at `resolved`, the caller's
/// [`resolve_at_fire`] of the client's original target (a warmup can separate
/// the launch from the fire: the ally died, left, or turned hostile).
///
/// The caller (`fire::fire_cast`) has played `Ability_End` at the same
/// resolved entity and spent any ammo. Runs every effect script on the
/// resolved entity, routed per effect by `effect_routing` (an
/// `EF_ResolveOnAbilityUser` effect on the caster, an area effect on the
/// caster's allies in its radius, AB-07), sends each touched
/// entity's stat change to it and its witnesses, sends any buff timers, and
/// registers pulsing effects (Recuperation's 25 pulses) with the caster as
/// invoker. This path owns every effect of a beneficial cast: it never
/// reaches `damage_apply`, so no QR roll or miss gate can drop one, and
/// there is no `onEffectResults`, threat, in-combat state or channel
/// cancel.
pub(super) async fn fire_beneficial(
    caster_id: u32,
    resolved: Resolved,
    def: &Option<AbilityDef>,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    let (target, resolution) = resolved;
    let target_id = match target {
        CastTarget::Caster => caster_id,
        CastTarget::Ally(id) => id,
        CastTarget::Hostile(_) | CastTarget::None => {
            send_no_ally_feedback(caster_id, tx, space_mgr).await;
            return;
        }
    };
    let Some(def) = def.as_ref() else { return };

    let before = pools(space_mgr, target_id);
    // Per-effect routing (AB-07): an `EF_ResolveOnAbilityUser` effect lands
    // on the caster even when the cast lands on an ally, a beneficial area
    // effect on the caster's allies around them, and every other effect on
    // the resolved target.
    let routed = plan_cast(space_mgr, caster_id, Some(def), Some(target_id));
    let pulsing = land_effects(caster_id, &routed.landings, tx, space_mgr).await;

    let after = pools(space_mgr, target_id);
    let who = space_mgr.player_identity(caster_id);
    tracing::debug!(
        target: "abilities",
        event = "beneficial_cast_applied",
        account_id = who.account_id,
        player_id = who.player_id,
        entity_id = caster_id,
        cast_id = space_mgr.current_cast_id(),
        ability_id = def.ability_id,
        effect_ids = ?def.effect_ids,
        resolved_target_id = target_id,
        target_player_id = space_mgr.player_identity(target_id).player_id,
        resolution,
        pulsing_registered = pulsing,
        recipient_ids = ?recipients(&routed.landings),
        recipient_player_ids = ?recipients(&routed.landings)
            .into_iter()
            .map(|id| space_mgr.player_identity(id).player_id)
            .collect::<Vec<_>>(),
        target_health_before = before.0,
        target_health_after = after.0,
        target_focus_before = before.1,
        target_focus_after = after.1,
        "beneficial cast applied its effects"
    );
}

/// Every entity a landing reaches, in first-landing order.
fn recipients(landings: &[Landing]) -> Vec<u32> {
    let mut out: Vec<u32> = Vec::new();
    for l in landings {
        if !out.contains(&l.recipient) {
            out.push(l.recipient);
        }
    }
    out
}

/// The entity's current `(Health, Focus)`, for the before and after fields.
fn pools(space_mgr: &SpaceManager, entity_id: u32) -> (Option<i32>, Option<i32>) {
    space_mgr.get_entity(entity_id).map_or((None, None), |t| {
        (
            t.stats.get(cimmeria_entity::stats::HEALTH).map(|s| s.cur),
            t.stats.get(cimmeria_entity::stats::FOCUS).map(|s| s.cur),
        )
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The D-AB02 switch. Flipping it to the refusal must change only this.
    #[test]
    fn d_ab02_default_falls_back_to_the_caster() {
        assert_eq!(
            no_ally(),
            (CastTarget::Caster, RESOLUTION_FALLBACK_TO_CASTER)
        );
    }
}
