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

use cimmeria_entity::abilities::{ClientEffectResult, EffectDef, EF_DONT_USE_QR, TCM_SINGLE};
use cimmeria_entity::stats::{StatList, FOCUS, HEALTH};

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
}

impl NvpPlanner {
    /// Add a landing effect that has no damage script.
    pub(super) fn add(&mut self, effect: &EffectDef) {
        let (health, focus) = (
            effect.param_i32("HealthDamage").max(0),
            effect.param_i32("FocusDamage").max(0),
        );
        if health == 0 && focus == 0 {
            return;
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
            return;
        }
        if health > 0 {
            self.area_health = Some((health, effect.effect_id, unrolled));
        }
        if focus > 0 {
            self.area_focus = Some((focus, effect.effect_id, unrolled));
        }
    }

    /// The hit's entries, singles first in effect order.
    pub(super) fn finish(mut self, ids: HitIds) -> Vec<NvpDamage> {
        let sources: Vec<(i32, i32, bool)> = self
            .area_health
            .into_iter()
            .chain(self.area_focus)
            .collect();
        if sources.is_empty() {
            return self.singles;
        }
        let source_ids: Vec<i32> = sources.iter().map(|s| s.1).collect();
        if self.direct_single {
            tracing::debug!(
                target: "abilities",
                event = "effect_path_skipped",
                account_id = ids.actor.account_id,
                player_id = ids.actor.player_id,
                entity_id = ids.entity_id,
                target_player_id = ids.target.player_id,
                target_id = ids.target_eid,
                ability_id = ids.ability_id,
                effect_ids = ?source_ids,
                reason = "area_effect_left_to_fan_out",
                "cone/radius NVP damage skipped on this target: a direct TCM_Single damage effect is its damage"
            );
            return self.singles;
        }
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
        self.singles
    }
}

/// Resolve every entry against the target's stats: HEALTH then FOCUS per
/// entry, each through the full pipeline (resist, armour, absorption) at
/// the hit's scale. Returns the HEALTH `onEffectResults` entries, one per
/// entry that dealt HEALTH damage, and the HEALTH damage dealt (for threat
/// and the death check). The FOCUS change reaches the client in
/// `onStatUpdate`, as it always has.
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
                player_id = ids.actor.player_id,
                entity_id = ids.entity_id,
                target_player_id = ids.target.player_id,
                target_id = ids.target_eid,
                ability_id = ids.ability_id,
                effect_ids = ?entries[i..].iter().map(|e| e.effect_id).collect::<Vec<_>>(),
                reason = "target_dead",
                "remaining NVP damage skipped: an earlier effect of this hit killed the target"
            );
            break;
        }
        let qr = if entry.unrolled { &unrolled } else { hit_qr };
        // Focus first: a shield stands in front of Focus, so a partial one
        // spends itself on the Focus half before the Health half
        // (`combat/damage/absorb.rs`). The two pools are otherwise
        // independent, so the order changes nothing without a shield.
        let mut focus_dealt = 0;
        if entry.focus > 0 {
            focus_dealt = combat::calculate_damage_penetrating(
                qr,
                entry.focus,
                scale,
                penetration_mult,
                damage_type,
                FOCUS,
                attacker,
                defender,
            )
            .1;
        }
        let (health_results, health_dealt) = combat::calculate_damage_penetrating(
            qr,
            entry.health,
            scale,
            penetration_mult,
            damage_type,
            HEALTH,
            attacker,
            defender,
        );
        tracing::debug!(
            target: "abilities",
            event = "nvp_damage_resolved",
            account_id = ids.actor.account_id,
            player_id = ids.actor.player_id,
            entity_id = ids.entity_id,
            target_player_id = ids.target.player_id,
            target_id = ids.target_eid,
            ability_id = ids.ability_id,
            effect_id = entry.effect_id,
            health_base = entry.health,
            focus_base = entry.focus,
            qr_rand = qr.qr_rand,
            reason = if entry.unrolled { "dont_use_qr" } else { "hit_roll" },
            health_dealt,
            focus_dealt,
            "NVP damage resolved for one effect"
        );
        results.extend(health_results);
        total_health += health_dealt;
    }
    (results, total_health)
}
