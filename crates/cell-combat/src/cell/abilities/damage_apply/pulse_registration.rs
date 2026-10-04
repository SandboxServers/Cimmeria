//! Registering a hit's pulsing effects on its target.
//!
//! Walk the ability's effects (and a special round's on-hit effect) again:
//! for any with `pulse_count > 1` and a positive `pulse_duration`, register
//! an `ActiveEffectInstance` on the target. The initial pulse already fired
//! (via NVP damage or script dispatch); registration carries the remaining
//! pulses. See `cell::effects::pulsing::effect_pulse_tick` for the per-tick
//! fire loop. The caller skips a splash target (blast damage only), and a
//! QR-rolled effect the roll missed is skipped here (AB-06: a missed DoT
//! must not tick; `plan_hit_effects` logged the skip).

use tokio::sync::mpsc;

use cimmeria_entity::abilities::AbilityDef;

use super::qr_gate;
use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;

/// Register `caster_id`'s pulsing effects that landed on `target_eid`.
pub(super) async fn register_hit_pulses(
    caster_id: u32,
    target_eid: u32,
    ability_def: Option<&AbilityDef>,
    on_hit_effect_id: Option<i32>,
    result_code: u8,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    let Some(def) = ability_def else {
        return;
    };
    let now = std::time::Instant::now();
    for eid in def.effect_ids.iter().copied().chain(on_hit_effect_id) {
        let effect_clone = match space_mgr.effect_defs.get(&eid) {
            Some(e) if e.is_pulsing() && qr_gate::effect_lands(e, result_code) => e.clone(),
            // A missing def was logged by `plan_hit_effects`.
            _ => continue,
        };
        // `register_active_effect` logs each refusal itself (AB-T2).
        crate::cell::effects::register_active_effect(
            space_mgr,
            target_eid,
            caster_id,
            &effect_clone,
            now,
            tx,
        )
        .await;
    }
}
