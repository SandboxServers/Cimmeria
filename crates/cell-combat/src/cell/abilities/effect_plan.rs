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

use std::collections::HashMap;
use std::sync::{LazyLock, Mutex, PoisonError};
use std::time::{Duration, Instant};

use cimmeria_entity::abilities::{EffectDef, EF_DONT_USE_QR};
use cimmeria_entity::cell_entity::PlayerIdentity;

use super::super::space_manager::SpaceManager;
use super::metrics::{self, EffectPath};

/// `event` of the per-effect plan row (target `abilities.effect`).
pub(crate) const EVENT_EFFECT_PLANNED: &str = "effect_planned";
/// `event` of the WARN for an effect that does nothing on its target.
pub(crate) const EVENT_EFFECT_INERT: &str = "effect_inert";
/// Minimum gap between two `effect_inert` WARNs for one `(ability_id,
/// effect_id)` pair (Pattern D).
const EFFECT_INERT_WARN_MIN_INTERVAL: Duration = Duration::from_secs(60);

/// The last `effect_inert` WARN per `(ability_id, effect_id)`: when it was
/// written and how many rows were swallowed since. Static because a plan
/// has no `SpaceManager` to hang a throttle on.
static INERT_LOG: LazyLock<Mutex<HashMap<(i32, i32), (Instant, u32)>>> =
    LazyLock::new(Mutex::default);

/// Decide whether one `effect_inert` row is written at `now`. `Some(n)`
/// means write it with `suppressed = n`; `None` means count it only.
fn admit_inert(key: (i32, i32), now: Instant) -> Option<u32> {
    let mut log = INERT_LOG.lock().unwrap_or_else(PoisonError::into_inner);
    match log.get_mut(&key) {
        None => {
            log.insert(key, (now, 0));
            Some(0)
        }
        Some((last, suppressed)) => {
            if now.saturating_duration_since(*last) >= EFFECT_INERT_WARN_MIN_INTERVAL {
                let swallowed = *suppressed;
                *last = now;
                *suppressed = 0;
                Some(swallowed)
            } else {
                *suppressed = suppressed.saturating_add(1);
                None
            }
        }
    }
}

// The paths are `abilities_effect_applied_total`'s label set (AB-T6), so
// the row and the metric share one spelling.
pub(crate) const PATH_SCRIPT: &str = EffectPath::Script.label();
pub(crate) const PATH_NVP: &str = EffectPath::Nvp.label();
pub(crate) const PATH_ROUTED_TO_USER: &str = EffectPath::RoutedToUser.label();
pub(crate) const PATH_ALLY_FANOUT: &str = EffectPath::AllyFanout.label();
pub(crate) const PATH_LEDGER: &str = EffectPath::Ledger.label();
pub(crate) const PATH_PULSE: &str = EffectPath::Pulse.label();
pub(crate) const PATH_SKIPPED: &str = EffectPath::Skipped.label();

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
    pub ability_name: Option<&'static str>,
    pub cast_id: Option<i32>,
    pub caster: PlayerIdentity,
    pub target: PlayerIdentity,
    /// The caster's and recipient's names (Rule 6).
    pub caster_name: Option<&'static str>,
    pub target_name: Option<&'static str>,
    /// The recipient's world: the AB-T6 metrics' `world` label.
    pub world: &'static str,
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
            ability_name: cimmeria_cell_world::cell::effects::content_names::ability_name(
                ability_id,
            ),
            cast_id: space_mgr.current_cast_id(),
            caster: space_mgr.player_identity(caster_id),
            target: space_mgr.player_identity(target_id),
            caster_name: space_mgr.entity_names(caster_id).entity_name,
            target_name: space_mgr.entity_names(target_id).entity_name,
            world: metrics::world_of(space_mgr, target_id),
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

    /// Log this plan row and count its path (`abilities_effect_applied_total`).
    pub(crate) fn log(&self, ids: PlanIds) {
        self.log_at(ids, Instant::now());
    }

    /// [`Self::log`] with the throttle's clock passed in, so tests can walk
    /// the window without sleeping.
    fn log_at(&self, ids: PlanIds, now: Instant) {
        match EffectPath::from_label(self.path) {
            Some(path) => metrics::effect_applied(path, ids.world),
            // Every path is one of the consts above; `metrics::tests` pins it.
            None => debug_assert!(false, "effect path {} has no label", self.path),
        }
        tracing::debug!(
            target: "abilities.effect",
            event = EVENT_EFFECT_PLANNED,
            stage = "route",
            account_id = ids.caster.account_id,
            account_name = ids.caster.account_name,
            player_id = ids.caster.player_id,
            player_name = ids.caster.player_name,
            entity_id = ids.caster_id,
            entity_name = ids.caster_name,
            cast_id = ids.cast_id, // nt:id-only per-cast sequence number, no name exists
            ability_id = ids.ability_id,
            ability_name = ids.ability_name,
            effect_id = self.effect_id,
            effect_name = cimmeria_cell_world::cell::effects::content_names::effect_name(self.effect_id),
            target_id = ids.target_id,
            target_name = ids.target_name,
            target_player_id = ids.target.player_id,
            target_player_name = ids.target.player_name,
            path = self.path,
            reason = self.reason,
            nvp = self.nvp,
            script = self.script,
            pulsing = self.pulsing,
            dont_use_qr = self.dont_use_qr,
            "effect planned on one target"
        );
        if self.path == PATH_SKIPPED && self.reason == REASON_NO_SCRIPT {
            self.warn_inert(ids, now);
        }
    }

    /// The `effect_inert` WARN: an effect with no script and no NVP damage
    /// does nothing on this target. Throttled per `(ability_id, effect_id)`.
    fn warn_inert(&self, ids: PlanIds, now: Instant) {
        let Some(effect_id) = self.effect_id else {
            return;
        };
        let Some(suppressed) = admit_inert((ids.ability_id, effect_id), now) else {
            return;
        };
        // An NPC caster has no account; a player's account is always set
        // once its session identity is threaded in.
        let caster_kind = if ids.caster.account_id.is_some() {
            "player"
        } else {
            "npc"
        };
        tracing::warn!(
            target: "abilities.effect",
            event = EVENT_EFFECT_INERT,
            account_id = ids.caster.account_id,
            account_name = ids.caster.account_name,
            player_id = ids.caster.player_id,
            player_name = ids.caster.player_name,
            entity_id = ids.caster_id,
            entity_name = ids.caster_name,
            caster_kind,
            ability_id = ids.ability_id,
            ability_name = ids.ability_name,
            effect_id,
            effect_name = cimmeria_cell_world::cell::effects::content_names::effect_name(self.effect_id),
            target_id = ids.target_id,
            target_name = ids.target_name,
            target_player_id = ids.target.player_id,
            target_player_name = ids.target.player_name,
            suppressed,
            "effect has no script and no NVP damage: it does nothing here"
        );
    }
}

#[cfg(test)]
mod tests {
    use std::time::{Duration, Instant};

    use cimmeria_entity::abilities::EffectDef;
    use cimmeria_entity::cell_entity::PlayerIdentity;
    use tracing::Level;

    use super::{PlanIds, PlannedEffect, EVENT_EFFECT_INERT};
    use crate::test_support::{Captured, LogCapture};

    /// A plan of `effect_id` in `ability_id` that has no script: the live
    /// case is ability 710 / effect 736. Tests use their own ids so the
    /// process-wide throttle never sees another test's keys.
    fn no_script_plan(ability_id: i32, effect_id: i32) -> (PlannedEffect, PlanIds) {
        let effect = EffectDef {
            effect_id,
            ability_id,
            ..Default::default()
        };
        let plan = PlannedEffect::landing(&effect, false, false, false, "");
        let ids = PlanIds {
            caster_id: 1,
            target_id: 2,
            ability_id,
            ability_name: None,
            cast_id: None,
            caster: PlayerIdentity::default(),
            target: PlayerIdentity::default(),
            caster_name: None,
            target_name: None,
            world: "Agnos",
        };
        (plan, ids)
    }

    /// The `effect_inert` rows written for `ability_id`.
    fn inert_rows(logs: &[Captured], ability_id: &str) -> usize {
        logs.iter()
            .filter(|c| {
                c.level == Level::WARN
                    && c.has_field("event", EVENT_EFFECT_INERT)
                    && c.has_field("ability_id", ability_id)
            })
            .count()
    }

    #[test]
    fn one_no_script_plan_warns_once() {
        let logs = LogCapture::install();
        let (plan, ids) = no_script_plan(97101, 97102);
        plan.log_at(ids, Instant::now());

        let row = logs
            .find_message(Level::WARN, "effect has no script")
            .expect("a no-script effect must warn");
        assert_eq!(row.target, "abilities.effect");
        assert!(row.has_field("effect_id", "97102"), "{row:#?}");
        assert!(row.has_field("suppressed", "0"), "{row:#?}");
        assert_eq!(inert_rows(&logs.all(), "97101"), 1);
    }

    #[test]
    fn burst_writes_one_row_then_reports_suppressed() {
        let logs = LogCapture::install();
        let (plan, ids) = no_script_plan(97111, 97112);
        let base = Instant::now();
        for step in 0..3 {
            plan.log_at(ids, base + Duration::from_millis(step));
        }
        assert_eq!(inert_rows(&logs.all(), "97111"), 1, "burst is one row");

        plan.log_at(ids, base + Duration::from_secs(61));
        assert_eq!(inert_rows(&logs.all(), "97111"), 2);
        let last = logs
            .all()
            .into_iter()
            .rfind(|c| c.has_field("ability_id", "97111"))
            .expect("second row");
        assert!(last.has_field("suppressed", "2"), "{last:#?}");
    }

    #[test]
    fn another_effect_id_is_not_swallowed() {
        let logs = LogCapture::install();
        let base = Instant::now();
        let (first, first_ids) = no_script_plan(97121, 97122);
        let (second, second_ids) = no_script_plan(97121, 97123);
        first.log_at(first_ids, base);
        second.log_at(second_ids, base);
        assert_eq!(inert_rows(&logs.all(), "97121"), 2);
    }
}
