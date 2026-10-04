//! Which of a hit's effects deal damage, and how (AB-06, D-AB07).
//!
//! An effect whose `script_name` is a **damage script** deals its damage
//! through that script only. The legacy NVP pipeline skips it: before
//! AB-06 the same `FocusDamage`/`HealthDamage` NVPs were taken once by the
//! pipeline and again by the script, so Pistol Shot and Strike hit twice
//! (ability-mechanics B-22). The damage script now runs where the NVP
//! damage runs, before the hit's death check and `onEffectResults`, so its
//! HEALTH change is in the packet, in the threat and in the death check.
//!
//! Every other script runs after the hit resolves ([`run_scripts`]), as
//! before: a heal sees the post-hit state.
//!
//! A QR-rolled effect whose roll misses lands nothing: no NVP damage, no
//! script, no pulses (B-23; [`super::qr_gate`]).

use cimmeria_entity::abilities::{
    AbilityDef, ClientEffectResult, EffectDef, RC_MISS, SRC_MORTAL, SRC_NONE,
};
use cimmeria_entity::stats::{FOCUS, HEALTH};

use super::nvp_damage::{NvpDamage, NvpPlanner};
use super::qr_gate::effect_lands;
use super::silent_rows;
use super::HitIds;
use crate::cell::abilities::effect_plan::{
    PlannedEffect, PATH_NVP, PATH_SKIPPED, REASON_AFTER_HIT_SCRIPT, REASON_AMMO_ON_HIT,
    REASON_MISS, REASON_SPLASH_TARGET, REASON_UNKNOWN_ABILITY,
};
use crate::cell::space_manager::SpaceManager;

/// The scripts that are an effect's damage. They read `FocusDamage` and
/// `HealthDamage`, the same NVPs the legacy pipeline reads.
const DAMAGE_SCRIPTS: [&str; 4] = [
    "RangedPhysicalDamage",
    "MeleePhysicalDamage",
    "RangedEnergyDamage",
    "MeleeDamage",
];

/// The NVPs a damage script reads, scaled by the hit's damage scale.
const DAMAGE_NVPS: [&str; 2] = ["FocusDamage", "HealthDamage"];

/// HEALTH damage an unknown ability (no `AbilityDef`) deals through the
/// NVP pipeline: a generic 15-HP swing.
const UNKNOWN_ABILITY_HEALTH_DAMAGE: i32 = 15;

pub(in crate::cell::abilities) fn is_damage_script(name: &str) -> bool {
    DAMAGE_SCRIPTS.contains(&name)
}

/// How a hit's effects deal their damage.
#[derive(Debug, Default)]
pub(super) struct HitEffects {
    /// NVP damage from effects with no damage script, one entry per
    /// `TCM_Single` effect (AB-03, B-21; [`super::nvp_damage`]).
    pub nvp: Vec<NvpDamage>,
    /// Effects whose damage script is this hit's damage.
    pub damage_scripts: Vec<i32>,
    /// Every other landing script, run after the hit resolves.
    pub after_scripts: Vec<i32>,
    /// The `effect_planned` rows this plan logged, for the in-game combat
    /// debug (AB-N1).
    pub planned: Vec<PlannedEffect>,
}

/// Sort the ability's effects (and a special ammo round's on-hit effect)
/// into the hit's damage paths. `direct` is false for an explosive round's
/// splash target: it keeps the NVP damage and the damage scripts (both
/// scaled by the splash fraction in the caller), and `after_scripts` is
/// emptied.
///
/// Every effect it sorts logs one `abilities.effect` `effect_planned` row
/// (AB-T3, [`crate::cell::abilities::effect_plan`]), written once the plan
/// is final, so an area effect the NVP planner leaves to its fan-out logs
/// `skipped`, not `nvp`.
pub(super) fn plan_hit_effects(
    space_mgr: &SpaceManager,
    ability_def: Option<&AbilityDef>,
    on_hit_effect_id: Option<i32>,
    direct: bool,
    result_code: u8,
    ids: HitIds,
) -> HitEffects {
    let mut plan = HitEffects::default();
    let mut rows: Vec<PlannedEffect> = Vec::new();
    let cast_id = space_mgr.current_cast_id();
    let Some(def) = ability_def else {
        let landed = result_code != RC_MISS;
        if landed {
            // WARN beside the `effect_planned` row: the numbers are made up.
            silent_rows::unknown_ability_fallback(ids, cast_id, UNKNOWN_ABILITY_HEALTH_DAMAGE);
            plan.nvp.push(NvpDamage {
                effect_id: None,
                health: UNKNOWN_ABILITY_HEALTH_DAMAGE,
                focus: 0,
                unrolled: false,
            });
        }
        let row = PlannedEffect {
            effect_id: None,
            path: if landed { PATH_NVP } else { PATH_SKIPPED },
            reason: if landed {
                REASON_UNKNOWN_ABILITY
            } else {
                REASON_MISS
            },
            nvp: landed,
            script: false,
            pulsing: false,
            dont_use_qr: false,
        };
        row.log(ids.plan_ids());
        plan.planned.push(row);
        return plan;
    };
    let mut nvp = NvpPlanner::default();
    for &eid in &def.effect_ids {
        let Some(effect) = space_mgr.effect_defs.get(&eid) else {
            silent_rows::effect_def_missing(ids, cast_id, eid, "plan");
            continue;
        };
        if !effect_lands(effect, result_code) {
            rows.push(PlannedEffect::skipped(effect, REASON_MISS));
            continue;
        }
        let script = effect.script_name.as_deref();
        if script.is_some_and(is_damage_script) {
            rows.push(PlannedEffect::damage_script(effect));
            plan.damage_scripts.push(eid);
            continue;
        }
        let deals_nvp = nvp.add(effect);
        // A splash target runs no after-hit script and registers no pulses.
        let runs_script = script.is_some() && direct;
        if script.is_some() {
            plan.after_scripts.push(eid);
        }
        if !direct && !deals_nvp && script.is_some() {
            rows.push(PlannedEffect::skipped(effect, REASON_SPLASH_TARGET));
            continue;
        }
        rows.push(PlannedEffect::landing(
            effect,
            deals_nvp,
            runs_script,
            direct && effect.is_pulsing(),
            REASON_AFTER_HIT_SCRIPT,
        ));
    }
    let (entries, dropped) = nvp.finish();
    plan.nvp = entries;
    for row in rows.iter_mut() {
        let Some(&(_, reason)) = dropped.iter().find(|d| Some(d.0) == row.effect_id) else {
            continue;
        };
        // Its NVP damage never resolves on this target. An effect whose
        // NVP was its primary path is skipped; one with a script or pulses
        // keeps that path, without the `nvp` flag.
        row.nvp = false;
        if row.path == PATH_NVP {
            row.path = PATH_SKIPPED;
            row.reason = reason;
        }
    }

    if let Some(eid) = on_hit_effect_id.filter(|eid| !space_mgr.effect_defs.contains_key(eid)) {
        silent_rows::effect_def_missing(ids, cast_id, eid, "on_hit");
    }
    if let Some(effect) = on_hit_effect_id.and_then(|eid| space_mgr.effect_defs.get(&eid)) {
        if effect.script_name.is_some() {
            plan.after_scripts.push(effect.effect_id);
        }
        rows.push(PlannedEffect::landing(
            effect,
            false,
            effect.script_name.is_some() && direct,
            direct && effect.is_pulsing(),
            REASON_AMMO_ON_HIT,
        ));
    }
    // A splash target takes the shot's damage only: its damage scripts stay
    // (the caller runs them at the splash scale, since they ARE the shot's
    // damage), but no other script (a bleed, a stun, the on-hit effect)
    // lands a second time.
    if !direct {
        plan.after_scripts.clear();
    }
    let plan_ids = ids.plan_ids();
    for row in &rows {
        row.log(plan_ids);
    }
    plan.planned = rows;
    plan
}

/// Run the hit's damage scripts on the target, each effect's
/// `FocusDamage`/`HealthDamage` scaled by `scale` (cover, special ammo,
/// splash share: the same scale the NVP pipeline applies). Returns the
/// `onEffectResults` entry for the HEALTH change, if any, and the HEALTH
/// damage dealt (for threat).
///
/// Only HEALTH is reported, the one stat the server has ever sent in an
/// `onEffectResults` list; a Focus entry waits for client evidence
/// (AB-E1). The Focus change still reaches the client in `onStatUpdate`.
pub(super) fn apply_damage_scripts(
    space_mgr: &mut SpaceManager,
    ids: HitIds,
    effect_ids: &[i32],
    scale: f64,
    damage_type: i8,
) -> (Vec<ClientEffectResult>, i32) {
    if effect_ids.is_empty() {
        return (Vec::new(), 0);
    }
    let Some((health_before, focus_before)) = pools(space_mgr, ids.target_eid) else {
        return (Vec::new(), 0);
    };
    for &eid in effect_ids {
        let Some(mut effect) = space_mgr.effect_defs.get(&eid).cloned() else {
            silent_rows::effect_def_missing(ids, space_mgr.current_cast_id(), eid, "damage_script");
            continue;
        };
        scale_damage_nvps(&mut effect, scale);
        absorb_script_damage(space_mgr, ids, &mut effect, damage_type);
        dispatch(space_mgr, ids, &effect);
    }
    let (health_after, focus_after) = pools(space_mgr, ids.target_eid).unwrap_or_default();
    let health_delta = health_after - health_before;
    tracing::debug!(
        target: "abilities",
        event = "damage_script_hit",
        account_id = ids.actor.account_id,
        player_id = ids.actor.player_id,
        entity_id = ids.entity_id,
        target_player_id = ids.target.player_id,
        target_id = ids.target_eid,
        ability_id = ids.ability_id,
        effect_ids = ?effect_ids,
        scale,
        health_delta,
        focus_delta = focus_after - focus_before,
        god_mode = ids.god_mode,
        "damage scripts resolved the hit's damage"
    );
    if health_delta == 0 {
        return (Vec::new(), 0);
    }
    let result = ClientEffectResult {
        stat_id: HEALTH as i8,
        delta: health_delta,
        damage_code: damage_type,
        stat_result_code: if health_after <= 0 {
            SRC_MORTAL
        } else {
            SRC_NONE
        },
    };
    (vec![result], -health_delta)
}

/// Let the target's absorb shields take the script's damage first
/// (AB-10, `combat::damage::absorb`): the script then deals what got
/// through. The hit settles the shields' ledger entries afterwards.
fn absorb_script_damage(
    space_mgr: &mut SpaceManager,
    ids: HitIds,
    effect: &mut EffectDef,
    damage_type: i8,
) {
    let Some(target) = space_mgr.get_entity_mut(ids.target_eid) else {
        return;
    };
    let absorbed = crate::cell::combat::absorb_damage_nvps(&mut target.stats, effect, damage_type);
    if absorbed > 0 {
        tracing::debug!(
            target: "abilities",
            event = "shield_absorbed_damage",
            stage = "apply",
            account_id = ids.actor.account_id,
            player_id = ids.actor.player_id,
            entity_id = ids.entity_id,
            cast_id = ids.cast_id,
            target_player_id = ids.target.player_id,
            target_id = ids.target_eid,
            ability_id = ids.ability_id,
            effect_id = effect.effect_id,
            damage_type,
            absorbed,
            focus_through = effect.param_i32("FocusDamage"),
            health_through = effect.param_i32("HealthDamage"),
            "a shield absorbed a damage script's damage"
        );
    }
}

/// Run each effect's script on the target, unscaled. A script that deals
/// damage (an on-hit burn, a Suppression chip) passes the target's shields
/// first at the shot's `damage_type`, as the hit's own damage scripts do;
/// the caller settles the shields after.
pub(super) fn run_scripts(
    space_mgr: &mut SpaceManager,
    ids: HitIds,
    effect_ids: &[i32],
    damage_type: i8,
) {
    for &eid in effect_ids {
        let Some(mut effect) = space_mgr.effect_defs.get(&eid).cloned() else {
            silent_rows::effect_def_missing(ids, space_mgr.current_cast_id(), eid, "after_script");
            continue;
        };
        absorb_script_damage(space_mgr, ids, &mut effect, damage_type);
        dispatch(space_mgr, ids, &effect);
    }
}

fn dispatch(space_mgr: &mut SpaceManager, ids: HitIds, effect: &EffectDef) {
    let Some(script_name) = effect.script_name.as_deref() else {
        silent_rows::script_name_missing(ids, space_mgr.current_cast_id(), effect.effect_id);
        return;
    };
    let mut ctx = crate::cell::effects::EffectContext {
        source_id: ids.entity_id,
        target_id: ids.target_eid,
        effect,
        space_mgr,
    };
    crate::cell::effects::dispatch_by_name(script_name, &mut ctx);
}

/// The target's HEALTH and FOCUS, or `None` when it is gone.
fn pools(space_mgr: &SpaceManager, target_eid: u32) -> Option<(i32, i32)> {
    let stats = &space_mgr.get_entity(target_eid)?.stats;
    let cur = |id| stats.get(id).map_or(0, |s| s.cur);
    Some((cur(HEALTH), cur(FOCUS)))
}

fn scale_damage_nvps(effect: &mut EffectDef, scale: f64) {
    if (scale - 1.0).abs() < f64::EPSILON {
        return;
    }
    for name in DAMAGE_NVPS {
        if effect.params.contains_key(name) {
            let scaled = (f64::from(effect.param_i32(name)) * scale).round() as i32;
            effect.params.insert(name.to_string(), scaled.to_string());
        }
    }
}
