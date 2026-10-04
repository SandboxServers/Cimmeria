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
use super::HitIds;
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
}

/// Sort the ability's effects (and a special ammo round's on-hit effect)
/// into the hit's damage paths. `direct` is false for an explosive round's
/// splash target: it keeps the NVP damage and the damage scripts (both
/// scaled by the splash fraction in the caller), and `after_scripts` is
/// emptied.
pub(super) fn plan_hit_effects(
    space_mgr: &SpaceManager,
    ability_def: Option<&AbilityDef>,
    on_hit_effect_id: Option<i32>,
    direct: bool,
    result_code: u8,
    ids: HitIds,
) -> HitEffects {
    let mut plan = HitEffects::default();
    let Some(def) = ability_def else {
        if result_code != RC_MISS {
            plan.nvp.push(NvpDamage {
                effect_id: None,
                health: UNKNOWN_ABILITY_HEALTH_DAMAGE,
                focus: 0,
                unrolled: false,
            });
        }
        return plan;
    };
    let mut nvp = NvpPlanner::default();
    for &eid in &def.effect_ids {
        let Some(effect) = space_mgr.effect_defs.get(&eid) else {
            continue;
        };
        if !effect_lands(effect, result_code) {
            log_skipped(
                ids,
                effect,
                "miss",
                "the QR roll missed: no damage, script or pulses",
            );
            continue;
        }
        let script = effect.script_name.as_deref();
        if script.is_some_and(is_damage_script) {
            log_skipped(
                ids,
                effect,
                "damage_script",
                "NVP pipeline skipped: the damage script is the only damage path",
            );
            plan.damage_scripts.push(eid);
            continue;
        }
        nvp.add(effect);
        if script.is_some() {
            plan.after_scripts.push(eid);
        }
    }
    plan.nvp = nvp.finish(ids);
    if let Some(eid) = on_hit_effect_id {
        if space_mgr
            .effect_defs
            .get(&eid)
            .is_some_and(|e| e.script_name.is_some())
        {
            plan.after_scripts.push(eid);
        }
    }
    // A splash target takes the shot's damage only: its damage scripts stay
    // (the caller runs them at the splash scale, since they ARE the shot's
    // damage), but no other script (a bleed, a stun, the on-hit effect)
    // lands a second time.
    if !direct {
        plan.after_scripts.clear();
    }
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
            continue;
        };
        scale_damage_nvps(&mut effect, scale);
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

/// Run each effect's script on the target, unscaled.
pub(super) fn run_scripts(space_mgr: &mut SpaceManager, ids: HitIds, effect_ids: &[i32]) {
    for eid in effect_ids {
        let Some(effect) = space_mgr.effect_defs.get(eid).cloned() else {
            continue;
        };
        dispatch(space_mgr, ids, &effect);
    }
}

fn dispatch(space_mgr: &mut SpaceManager, ids: HitIds, effect: &EffectDef) {
    let Some(script_name) = effect.script_name.as_deref() else {
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

fn log_skipped(ids: HitIds, effect: &EffectDef, reason: &'static str, msg: &'static str) {
    tracing::debug!(
        target: "abilities",
        event = "effect_path_skipped",
        account_id = ids.actor.account_id,
        player_id = ids.actor.player_id,
        entity_id = ids.entity_id,
        target_player_id = ids.target.player_id,
        target_id = ids.target_eid,
        ability_id = ids.ability_id,
        effect_id = effect.effect_id,
        script = effect.script_name.as_deref().unwrap_or(""),
        reason,
        "{msg}"
    );
}
