//! The hit's QR roll and what a miss stops (AB-06, D-AB07).
//!
//! - An effect carrying `EF_DontUseQR` (16) never rolls and always lands,
//!   at [`unrolled_qr`]: the distribution's midpoint, so the NVP pipeline
//!   deals the authored base (`base × 0.5 × 2 × (1 + 0)`). That holds per
//!   effect (AB-03, `nvp_damage`): in a mixed ability the flagged effect
//!   deals its base whatever the others rolled, even on a miss. A hit
//!   whose every effect carries it takes no roll at all: its result is
//!   `RC_Hit`.
//! - Any other effect lands only when the roll is not `RC_MISS`. A missed
//!   effect deals no NVP damage, runs no script and registers no pulses.
//!
//! **Deviation from the python reference, on purpose.** `DamageCalc.
//! calculateResult(dontUseQr=True)` still sampled the beta distribution at
//! QR 0, so a no-QR effect could miss (about 7 % of the time). D-AB07 reads
//! the flag's name literally: no roll.

use cimmeria_entity::abilities::{
    AbilityDef, EffectDef, EF_DONT_USE_QR, RC_CRITICAL, RC_DOUBLE_CRITICAL, RC_GLANCING, RC_HIT,
    RC_MISS, RC_NONE,
};

use super::cover_roll::HitCover;
use super::HitIds;
use crate::cell::combat::{self, QrResult};
use crate::cell::space_manager::SpaceManager;

/// The `qr_rand` an unrolled hit uses: the mean of the QR-0 distribution,
/// `Beta(1.4, 1.4)`.
const UNROLLED_QR_RAND: f64 = 0.5;

/// The QR result of an effect that takes no roll: `RC_Hit` at the mean
/// of the QR-0 distribution.
pub(super) fn unrolled_qr() -> QrResult {
    QrResult {
        qr_rand: UNROLLED_QR_RAND,
        result_code: RC_HIT,
        qr: 0.0,
    }
}

/// Whether `effect` lands on a hit whose roll came out `result_code`.
pub(super) fn effect_lands(effect: &EffectDef, result_code: u8) -> bool {
    effect.flags & EF_DONT_USE_QR != 0 || result_code != RC_MISS
}

/// Whether every effect of the ability carries `EF_DontUseQR`, so the hit
/// takes no roll. An unknown ability, or one with no known effect, rolls.
pub(super) fn hit_skips_qr(ability_def: Option<&AbilityDef>, space_mgr: &SpaceManager) -> bool {
    let Some(def) = ability_def else {
        return false;
    };
    let mut effects = def
        .effect_ids
        .iter()
        .filter_map(|id| space_mgr.effect_defs.get(id))
        .peekable();
    effects.peek().is_some() && effects.all(|e| e.flags & EF_DONT_USE_QR != 0)
}

/// The hit's QR result: the beta roll, or no roll when [`hit_skips_qr`].
/// Logs the hit's one `abilities.qr` `qr_rolled` row (AB-T3). A hit that
/// takes no roll logs it with `dont_use_qr = true` and the unrolled sample
/// (that row was `qr_roll_skipped` before AB-T3).
pub(super) fn roll_hit(
    ability_def: Option<&AbilityDef>,
    space_mgr: &SpaceManager,
    qr: f64,
    seed: u64,
    cover: HitCover,
    ids: HitIds,
) -> QrResult {
    let dont_use_qr = hit_skips_qr(ability_def, space_mgr);
    let forced_result = forced_roll(space_mgr, ids.entity_id);
    let forced = forced_result.is_some();
    let result = match forced_result {
        Some(forced) => forced,
        None if dont_use_qr => unrolled_qr(),
        None => combat::calculate_result(qr, seed),
    };
    tracing::debug!(
        target: "abilities.qr",
        event = "qr_rolled",
        stage = "qr",
        account_id = ids.actor.account_id,
        player_id = ids.actor.player_id,
        entity_id = ids.entity_id,
        cast_id = ids.cast_id,
        ability_id = ids.ability_id,
        target_player_id = ids.target.player_id,
        target_id = ids.target_eid,
        qr,
        roll = result.qr_rand,
        result_code = result.result_code,
        result = result_label(result.result_code),
        cover_qr = cover.attacker_qr,
        cover_reduction_pct = cover.reduction.final_pct,
        dont_use_qr,
        forced,
        "QR rolled for the hit"
    );
    result
}

/// D-AU2 hook point. A GM `.qr <hit|miss|crit|graze|off>` override of the
/// caster's rolls returns the forced result here, and `qr_rolled` logs
/// `forced = true`. There is no override until the owner decides D-AU2.
fn forced_roll(_space_mgr: &SpaceManager, _caster_id: u32) -> Option<QrResult> {
    None
}

/// The `result` label of a QR result code (the client's `EResultCode`).
pub(crate) fn result_label(result_code: u8) -> &'static str {
    match result_code {
        RC_NONE => "none",
        RC_HIT => "hit",
        RC_MISS => "miss",
        RC_CRITICAL => "critical",
        RC_DOUBLE_CRITICAL => "double_critical",
        RC_GLANCING => "glancing",
        _ => "unknown",
    }
}
