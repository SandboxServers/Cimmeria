//! The `abilities.effect` `effect_planned` row (ability-mechanics AB-T3):
//! one row per effect per target, saying which path the effect takes and
//! why.
//!
//! Two code paths decide where an effect goes, and each logs its own
//! decisions from the point it makes them:
//!
//! - **The hit pipeline** (`damage_apply::effect_scripts::plan_hit_effects`)
//!   sorts a hit's effects into NVP damage, damage scripts and after-hit
//!   scripts, and drops the missed ones.
//! - **The landing path** (`effect_routing::land_effects`) lands the effects
//!   `plan_cast` routed off the target (user halves, beneficial area halves)
//!   and every effect of a beneficial cast. The route was decided in
//!   `plan_cast`; the landing carries it, and the row is written where the
//!   effect lands, so a ground cast's `secondary_scope` (which calls
//!   `plan_cast` only for its target part) never logs a landing that does
//!   not happen.
//!
//! An ammo support shot's on-hit effect (`use_ability::support_shot`) logs
//! one row too.
//!
//! `path` is the primary way the effect acts on its target, in this order:
//! `skipped` (it does nothing here), then `script` for a damage script,
//! `pulse` (a pulsing effect: its first pulse lands now, the rest from the
//! pulse tick), `nvp` (the NVP damage pipeline), `ledger` (a script that
//! writes the timed-effect ledger), `script`. A routed effect's path is its
//! route: `routed_to_user` or `ally_fanout`. The `nvp`, `script` and
//! `pulsing` fields say what else the effect does.
//!
//! The `effect_path_skipped` rows the hit pipeline wrote before AB-T3 for a
//! miss, a damage script and an area effect left to its fan-out are folded
//! in here (`path = skipped` or `script`, same `reason`). One
//! `effect_path_skipped` remains: `reason = target_dead`, an apply-time skip
//! (`nvp_damage::apply_nvp_damage`), not a plan.

use cimmeria_entity::abilities::{EffectDef, EF_DONT_USE_QR};
use cimmeria_entity::cell_entity::PlayerIdentity;

use super::super::space_manager::SpaceManager;

/// `event` of the per-effect plan row (target `abilities.effect`).
pub(crate) const EVENT_EFFECT_PLANNED: &str = "effect_planned";

pub(crate) const PATH_SCRIPT: &str = "script";
pub(crate) const PATH_NVP: &str = "nvp";
pub(crate) const PATH_ROUTED_TO_USER: &str = "routed_to_user";
pub(crate) const PATH_ALLY_FANOUT: &str = "ally_fanout";
pub(crate) const PATH_LEDGER: &str = "ledger";
pub(crate) const PATH_PULSE: &str = "pulse";
pub(crate) const PATH_SKIPPED: &str = "skipped";

/// The QR roll missed: no damage, script or pulses (AB-06).
pub(crate) const REASON_MISS: &str = "miss";
/// A damage script is its effect's only damage path (AB-06, B-22).
pub(crate) const REASON_DAMAGE_SCRIPT: &str = "damage_script";
/// The effect has no script and no NVP damage: nothing to do.
pub(crate) const REASON_NO_SCRIPT: &str = "no_script";
/// The recipient is gone by the time the effect lands.
pub(crate) const REASON_NOT_REACHABLE: &str = "not_reachable";
/// NVP damage at the unrolled QR (`EF_DontUseQR`).
pub(crate) const REASON_DONT_USE_QR: &str = "dont_use_qr";
/// NVP damage at the hit's roll.
pub(crate) const REASON_HIT_ROLL: &str = "hit_roll";
/// A non-damage script that runs after the hit resolves.
pub(crate) const REASON_AFTER_HIT_SCRIPT: &str = "after_hit_script";
/// A pulsing effect: registered after its first pulse.
pub(crate) const REASON_PULSING: &str = "pulsing";
/// A cone or radius effect's NVP damage left to its fan-out, because a
/// direct single-target damage effect is this target's damage.
pub(crate) const REASON_AREA_LEFT_TO_FAN_OUT: &str = "area_effect_left_to_fan_out";
/// A cone or radius effect whose pool a later area effect of the same
/// ability replaced in the legacy collapse (one area value per pool).
pub(crate) const REASON_AREA_COLLAPSED: &str = "area_collapsed";
/// A splash target takes the shot's damage only (AM-10).
pub(crate) const REASON_SPLASH_TARGET: &str = "splash_target";
/// A special round's on-hit effect (AM-04).
pub(crate) const REASON_AMMO_ON_HIT: &str = "ammo_on_hit";
/// An ammo support shot's on-hit effect on an ally (AM-11d).
pub(crate) const REASON_SUPPORT_SHOT: &str = "support_shot";
/// An unknown ability's generic swing (no `AbilityDef`).
pub(crate) const REASON_UNKNOWN_ABILITY: &str = "unknown_ability";
/// Every effect of a beneficial cast lands on its resolved target (AB-01).
pub(crate) const REASON_BENEFICIAL_CAST: &str = "beneficial_cast";

/// The scripts that write the timed-effect ledger
/// (`SpaceManager::apply_timed_effect`): telemetry classification only.
const LEDGER_SCRIPTS: [&str; 6] = [
    "TimedStat",
    "StatBuff",
    "Stun",
    "Knockdown",
    "AbsorbShield",
    "MovementSlow",
];

pub(crate) fn is_ledger_script(name: &str) -> bool {
    LEDGER_SCRIPTS.contains(&name)
}

/// Who an effect plan is for: the caster and one recipient.
#[derive(Debug, Clone, Copy)]
pub(crate) struct PlanIds {
    pub caster_id: u32,
    pub target_id: u32,
    pub ability_id: i32,
    pub cast_id: Option<i32>,
    pub caster: PlayerIdentity,
    pub target: PlayerIdentity,
}

impl PlanIds {
    /// `caster_id`'s effect landing on `target_id`, in the current cast.
    pub(crate) fn of(
        space_mgr: &SpaceManager,
        caster_id: u32,
        target_id: u32,
        ability_id: i32,
    ) -> Self {
        Self {
            caster_id,
            target_id,
            ability_id,
            cast_id: space_mgr.current_cast_id(),
            caster: space_mgr.player_identity(caster_id),
            target: space_mgr.player_identity(target_id),
        }
    }
}

/// One effect's plan on one target.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct PlannedEffect {
    /// `None` for an unknown ability's swing.
    pub effect_id: Option<i32>,
    pub path: &'static str,
    pub reason: &'static str,
    /// It deals NVP damage on this target.
    pub nvp: bool,
    /// It runs its script on this target.
    pub script: bool,
    /// It registers pulses on this target.
    pub pulsing: bool,
    pub dont_use_qr: bool,
}

impl PlannedEffect {
    /// `effect` doing nothing on this target, for `reason`.
    pub(crate) fn skipped(effect: &EffectDef, reason: &'static str) -> Self {
        Self {
            effect_id: Some(effect.effect_id),
            path: PATH_SKIPPED,
            reason,
            nvp: false,
            script: false,
            pulsing: false,
            dont_use_qr: effect.flags & EF_DONT_USE_QR != 0,
        }
    }

    /// `effect`'s damage script is the hit's damage for it (AB-06).
    pub(crate) fn damage_script(effect: &EffectDef) -> Self {
        Self {
            effect_id: Some(effect.effect_id),
            path: PATH_SCRIPT,
            reason: REASON_DAMAGE_SCRIPT,
            nvp: false,
            script: true,
            pulsing: effect.is_pulsing(),
            dont_use_qr: effect.flags & EF_DONT_USE_QR != 0,
        }
    }

    /// `effect` landing: `nvp` when it deals NVP damage here, `script` when
    /// its script runs here, `pulsing` when it registers pulses. The path
    /// is the primary one (module docs); `reason` explains a script or
    /// ledger path, the NVP and pulse paths name their own.
    pub(crate) fn landing(
        effect: &EffectDef,
        nvp: bool,
        script: bool,
        pulsing: bool,
        reason: &'static str,
    ) -> Self {
        let dont_use_qr = effect.flags & EF_DONT_USE_QR != 0;
        let ledger = script && effect.script_name.as_deref().is_some_and(is_ledger_script);
        let (path, reason) = if pulsing {
            (PATH_PULSE, REASON_PULSING)
        } else if nvp {
            let r = if dont_use_qr {
                REASON_DONT_USE_QR
            } else {
                REASON_HIT_ROLL
            };
            (PATH_NVP, r)
        } else if ledger {
            (PATH_LEDGER, reason)
        } else if script {
            (PATH_SCRIPT, reason)
        } else {
            (PATH_SKIPPED, REASON_NO_SCRIPT)
        };
        Self {
            effect_id: Some(effect.effect_id),
            path,
            reason,
            nvp,
            script,
            pulsing,
            dont_use_qr,
        }
    }

    /// Log this plan row.
    pub(crate) fn log(&self, ids: PlanIds) {
        tracing::debug!(
            target: "abilities.effect",
            event = EVENT_EFFECT_PLANNED,
            stage = "route",
            account_id = ids.caster.account_id,
            player_id = ids.caster.player_id,
            entity_id = ids.caster_id,
            cast_id = ids.cast_id,
            ability_id = ids.ability_id,
            effect_id = self.effect_id,
            target_id = ids.target_id,
            target_player_id = ids.target.player_id,
            path = self.path,
            reason = self.reason,
            nvp = self.nvp,
            script = self.script,
            pulsing = self.pulsing,
            dont_use_qr = self.dont_use_qr,
            "effect planned on one target"
        );
    }
}
