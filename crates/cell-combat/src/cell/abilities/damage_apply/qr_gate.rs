//! The hit's QR roll and what a miss stops (AB-06, D-AB07).
//!
//! - An effect carrying `EF_DontUseQR` (16) never rolls and always lands.
//!   A hit whose every effect carries it takes no roll at all: its result
//!   is `RC_Hit` at the distribution's midpoint, so the NVP pipeline deals
//!   the authored base (`base × 0.5 × 2 × (1 + 0)`).
//! - Any other effect lands only when the roll is not `RC_MISS`. A missed
//!   effect deals no NVP damage, runs no script and registers no pulses.
//!
//! **Deviation from the python reference, on purpose.** `DamageCalc.
//! calculateResult(dontUseQr=True)` still sampled the beta distribution at
//! QR 0, so a no-QR effect could miss (about 7 % of the time). D-AB07 reads
//! the flag's name literally: no roll.

use cimmeria_entity::abilities::{AbilityDef, EffectDef, EF_DONT_USE_QR, RC_HIT, RC_MISS};

use super::HitIds;
use crate::cell::combat::{self, QrResult};
use crate::cell::space_manager::SpaceManager;

/// The `qr_rand` an unrolled hit uses: the mean of the QR-0 distribution,
/// `Beta(1.4, 1.4)`.
const UNROLLED_QR_RAND: f64 = 0.5;

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
pub(super) fn roll_hit(
    ability_def: Option<&AbilityDef>,
    space_mgr: &SpaceManager,
    qr: f64,
    seed: u64,
    ids: HitIds,
) -> QrResult {
    if !hit_skips_qr(ability_def, space_mgr) {
        return combat::calculate_result(qr, seed);
    }
    tracing::debug!(
        target: "abilities",
        event = "qr_roll_skipped",
        entity_id = ids.entity_id,
        target_id = ids.target_eid,
        ability_id = ids.ability_id,
        reason = "every effect carries EF_DontUseQR",
        "hit takes no QR roll: RC_Hit"
    );
    QrResult {
        qr_rand: UNROLLED_QR_RAND,
        result_code: RC_HIT,
        qr: 0.0,
    }
}
