//! NVP damage, one resolution per effect (AB-03, B-21).
//!
//! Before AB-03 the hit kept one `HealthDamage` and one `FocusDamage`: the
//! last positive value of any effect. An ability with a direct hit and a
//! DoT, or two single-target damage effects, dealt only one of them, and a
//! mixed ability's `EF_DontUseQR` effect took the hit's roll (so it landed
//! partly on a miss). Now:
//!
//! - Every landing `TCM_Single` effect with NVP damage is its own entry,
//!   resolved through the damage pipeline on its own.
//! - An effect carrying `EF_DontUseQR` resolves at the unrolled QR
//!   ([`super::qr_gate::unrolled_qr`]): its authored base, whatever the
//!   hit rolled. Any other effect takes the hit's roll.
//! - Cone and radius effects are their fan-outs' damage (the cone fan-out
//!   and the ground cast pass a scoped or full ability per target). On a
//!   hit, they collapse into one entry as before (the last positive value
//!   per pool), which lands only when no direct (non-pulsing) `TCM_Single`
//!   damage effect does: a pure cone ability still hurts its primary, and
//!   "Target -100F" + "Medium Cone -100F" deals the primary 100, not 200.

use cimmeria_cell_world::cell::combat_debug::{NvpNote, Pools};
use cimmeria_entity::abilities::{ClientEffectResult, EffectDef, EF_DONT_USE_QR, TCM_SINGLE};
use cimmeria_entity::stats::{StatList, FOCUS, HEALTH};

use super::super::effect_plan::{REASON_AREA_COLLAPSED, REASON_AREA_LEFT_TO_FAN_OUT};
use super::super::metrics::{self, Pool};
use super::qr_gate::unrolled_qr;
use super::HitIds;
use crate::cell::combat::{self, QrResult};

/// One NVP damage resolution on the hit's target.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct NvpDamage {
    /// The effect it came from; `None` for an unknown ability's swing, or
    /// an area collapse whose pools came from two effects.
    pub effect_id: Option<i32>,
    pub health: i32,
    pub focus: i32,
    /// `EF_DontUseQR`: resolve at the unrolled QR, not the hit's roll.
    pub unrolled: bool,
}

/// Collects a hit's NVP damage while `plan_hit_effects` walks the effects.
#[derive(Debug, Default)]
pub(super) struct NvpPlanner {
    singles: Vec<NvpDamage>,
    /// A landing non-pulsing `TCM_Single` damage effect: the target's own
    /// damage, so the area effects stay with their fan-outs.
    direct_single: bool,
    /// The legacy collapse of the area effects: (value, effect, unrolled)
    /// per pool, last positive wins.
    area_health: Option<(i32, i32, bool)>,
    area_focus: Option<(i32, i32, bool)>,
    /// Every area effect added, in order, for the plan rows of the ones
    /// the collapse drops.
    area_ids: Vec<i32>,
}

impl NvpPlanner {
    /// Add a landing effect that has no damage script. Returns whether it
    /// deals NVP damage (`false`: no positive `HealthDamage`/`FocusDamage`).
    pub(super) fn add(&mut self, effect: &EffectDef) -> bool {
        let (health, focus) = (
            effect.param_i32("HealthDamage").max(0),
            effect.param_i32("FocusDamage").max(0),
        );
        if health == 0 && focus == 0 {
            return false;
        }
        let unrolled = effect.flags & EF_DONT_USE_QR != 0;
        if effect.target_collection_method == TCM_SINGLE {
            self.direct_single |= !effect.is_pulsing();
            self.singles.push(NvpDamage {
                effect_id: Some(effect.effect_id),
                health,
                focus,
                unrolled,
            });
            return true;
        }
        self.area_ids.push(effect.effect_id);
        if health > 0 {
            self.area_health = Some((health, effect.effect_id, unrolled));
        }
        if focus > 0 {
            self.area_focus = Some((focus, effect.effect_id, unrolled));
        }
        true
    }

    /// The hit's entries, singles first in effect order, and every area
    /// effect that deals nothing here, with the reason its `effect_planned`
    /// row gives: `area_effect_left_to_fan_out` (a direct `TCM_Single`
    /// damage effect is this target's damage) or `area_collapsed` (a later
    /// area effect replaced its pool in the legacy collapse).
    pub(super) fn finish(mut self) -> (Vec<NvpDamage>, Vec<(i32, &'static str)>) {
        if self.area_ids.is_empty() {
            return (self.singles, Vec::new());
        }
        if self.direct_single {
            let left = self
                .area_ids
                .iter()
                .map(|&id| (id, REASON_AREA_LEFT_TO_FAN_OUT))
                .collect();
            return (self.singles, left);
        }
        let kept: Vec<i32> = self
            .area_health
            .iter()
            .chain(self.area_focus.iter())
            .map(|s| s.1)
            .collect();
        let collapsed = self
            .area_ids
            .iter()
            .filter(|id| !kept.contains(id))
            .map(|&id| (id, REASON_AREA_COLLAPSED))
            .collect();

        // Each pool keeps its own QR provenance: when the collapsed Health
        // and Focus came from effects with different `EF_DontUseQR` policies,
        // they resolve as two entries, so a flagged pool is never rolled.
        match (self.area_health, self.area_focus) {
            (Some(h), Some(f)) if h.2 == f.2 => self.singles.push(NvpDamage {
                effect_id: (h.1 == f.1).then_some(h.1),
                health: h.0,
                focus: f.0,
                unrolled: h.2,
            }),
            (h, f) => {
                self.singles.extend(h.map(|h| NvpDamage {
                    effect_id: Some(h.1),
                    health: h.0,
                    focus: 0,
                    unrolled: h.2,
                }));
                self.singles.extend(f.map(|f| NvpDamage {
                    effect_id: Some(f.1),
                    health: 0,
                    focus: f.0,
                    unrolled: f.2,
                }));
            }
        }
        (self.singles, collapsed)
    }
}

/// Resolve every entry against the target's stats: HEALTH then FOCUS per
/// entry, each through the full pipeline (resist, armour, absorption) at
/// the hit's scale. Returns the HEALTH `onEffectResults` entries, one per
/// entry that dealt HEALTH damage, and the HEALTH damage dealt (for threat
/// and the death check). The FOCUS change reaches the client in
/// `onStatUpdate`, as it always has.
///
/// Each entry logs `nvp_damage_resolved` (AB-T3): the roll it took, the
/// damage type, the target's pools before and after, and what the absorb
/// pools took; a shield that absorbed some of it also logs
/// `shield_absorbed_damage` with the hit's ids. With `debug`, the same
/// values are pushed there for the in-game combat debug (AB-N1).
#[allow(clippy::too_many_arguments)]
pub(super) fn apply_nvp_damage(
    entries: &[NvpDamage],
    hit_qr: &QrResult,
    scale: f64,
    penetration_mult: f64,
    damage_type: i8,
    attacker: &StatList,
    defender: &mut StatList,
    ids: HitIds,
    mut debug: Option<&mut Vec<NvpNote>>,
) -> (Vec<ClientEffectResult>, i32) {
    let unrolled = unrolled_qr();
    let mut results = Vec::new();
    let mut total_health = 0;
    for (i, entry) in entries.iter().enumerate() {
        // A target an earlier entry killed takes nothing more: the hit
        // carries one SRC_MORTAL entry, not one per remaining effect.
        if defender.get(HEALTH).is_some_and(|s| s.cur <= 0) {
            tracing::debug!(
                target: "abilities",
                event = "effect_path_skipped",
                account_id = ids.actor.account_id,
                account_name = ids.actor.account_name,
                player_id = ids.actor.player_id,
                player_name = ids.actor.player_name,
                entity_id = ids.entity_id,
                entity_name = ids.entity_name,
                cast_id = ids.cast_id, // nt:id-only per-cast sequence number, no name exists
                target_player_id = ids.target.player_id,
                target_player_name = ids.target.player_name,
                target_id = ids.target_eid,
                target_name = ids.target_name,
                ability_id = ids.ability_id,
                ability_name = ids.ability_name,
                effect_ids = ?entries[i..].iter().map(|e| e.effect_id).collect::<Vec<_>>(),
                reason = "target_dead",
                "remaining NVP damage skipped: an earlier effect of this hit killed the target"
            );
            break;
        }
        let qr = if entry.unrolled { &unrolled } else { hit_qr };
        let (health_before, focus_before) = pools(defender);
        // Focus first: a shield stands in front of Focus, so a partial one
        // spends itself on the Focus half before the Health half
        // (`combat/damage/absorb.rs`). The two pools are otherwise
        // independent, so the order changes nothing without a shield.
        let mut focus = combat::DamageOutcome::default();
        if entry.focus > 0 {
            focus = combat::resolve_damage(
                qr,
                entry.focus,
                scale,
                penetration_mult,
                damage_type,
                FOCUS,
                attacker,
                defender,
            );
        }
        let health = combat::resolve_damage(
            qr,
            entry.health,
            scale,
            penetration_mult,
            damage_type,
            HEALTH,
            attacker,
            defender,
        );
        let (health_after, focus_after) = pools(defender);
        let absorbed = focus.absorbed + health.absorbed;
        if absorbed > 0 {
            tracing::debug!(
                target: "abilities",
                event = "shield_absorbed_damage",
                stage = "apply",
                account_id = ids.actor.account_id,
                account_name = ids.actor.account_name,
                player_id = ids.actor.player_id,
                player_name = ids.actor.player_name,
                entity_id = ids.entity_id,
                entity_name = ids.entity_name,
                cast_id = ids.cast_id, // nt:id-only per-cast sequence number, no name exists
                target_player_id = ids.target.player_id,
                target_player_name = ids.target.player_name,
                target_id = ids.target_eid,
                target_name = ids.target_name,
                ability_id = ids.ability_id,
                ability_name = ids.ability_name,
                effect_id = entry.effect_id,
                effect_name = cimmeria_cell_world::cell::effects::content_names::effect_name(entry.effect_id),
                damage_type,
                absorbed,
                focus_absorbed = focus.absorbed,
                health_absorbed = health.absorbed,
                focus_through = focus.dealt,
                health_through = health.dealt,
                "a shield absorbed NVP damage"
            );
        }
        tracing::debug!(
            target: "abilities.effect",
            event = "nvp_damage_resolved",
            stage = "apply",
            account_id = ids.actor.account_id,
            account_name = ids.actor.account_name,
            player_id = ids.actor.player_id,
            player_name = ids.actor.player_name,
            entity_id = ids.entity_id,
            entity_name = ids.entity_name,
            cast_id = ids.cast_id, // nt:id-only per-cast sequence number, no name exists
            target_player_id = ids.target.player_id,
            target_player_name = ids.target.player_name,
            target_id = ids.target_eid,
            target_name = ids.target_name,
            ability_id = ids.ability_id,
            ability_name = ids.ability_name,
            effect_id = entry.effect_id,
            effect_name = cimmeria_cell_world::cell::effects::content_names::effect_name(entry.effect_id),
            health_base = entry.health,
            focus_base = entry.focus,
            qr_rand = qr.qr_rand,
            result_code = qr.result_code,
            result = super::qr_gate::result_label(qr.result_code),
            reason = if entry.unrolled { "dont_use_qr" } else { "hit_roll" },
            damage_type,
            health_before,
            health_after,
            focus_before,
            focus_after,
            health_dealt = health.dealt,
            focus_dealt = focus.dealt,
            absorbed,
            // `*_after` is before a god-mode restore (`god_mode_absorbed`).
            god_mode = ids.god_mode,
            "NVP damage resolved for one effect"
        );
        if let Some(notes) = debug.as_deref_mut() {
            notes.push(NvpNote {
                target_id: ids.target_eid,
                effect_id: entry.effect_id,
                health_base: entry.health,
                focus_base: entry.focus,
                health_dealt: health.dealt,
                focus_dealt: focus.dealt,
                absorbed,
                before: Pools {
                    health: health_before,
                    focus: focus_before,
                },
                after: Pools {
                    health: health_after,
                    focus: focus_after,
                },
                reason: if entry.unrolled {
                    "dont_use_qr"
                } else {
                    "hit_roll"
                },
            });
        }
        // AB-T6: what the effect took, per pool, before a god-mode restore.
        for (pool, amount) in [
            (Pool::Health, health.dealt),
            (Pool::Focus, focus.dealt),
            (Pool::Absorb, absorbed),
        ] {
            metrics::damage_dealt(pool, amount, ids.world);
        }
        results.extend(health.results);
        total_health += health.dealt;
    }
    (results, total_health)
}

/// The target's HEALTH and FOCUS.
fn pools(stats: &StatList) -> (i32, i32) {
    let cur = |id| stats.get(id).map_or(0, |s| s.cur);
    (cur(HEALTH), cur(FOCUS))
}
