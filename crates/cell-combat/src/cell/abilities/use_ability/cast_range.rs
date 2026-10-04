//! The range gate of a targeted cast, shared by the launch
//! (`handle_use_ability`) and the warmup fire-time re-check.
//!
//! The bounds come from `cimmeria_entity::abilities::range`, the one place
//! that knows the units and the defaults. A target farther than the maximum
//! is refused for every caster; one closer than the ability's `min_range` is
//! refused for a player only (#1016; the NPC fight tick owns its own
//! minimum-range behaviour). Both refusals answer a player with the same
//! `onErrorCode(SystemID 0, InstanceID ability_id,
//! CONDITION_FEEDBACK_OutsideWeaponRange 42)`, as the 2009 Python reference
//! did (`AbilityManager.py:561`), and leave one `abilities` DEBUG row
//! `event=cast_refused` with `reason=target_out_of_range` or
//! `reason=target_too_close`.

use tokio::sync::mpsc;

use cimmeria_entity::abilities::{RangeBounds, RangeRefusal};

use crate::cell::abilities::wire_ledger::{self, WireCtx};
use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;

/// `CONDITION_FEEDBACK_OutsideWeaponRange`: the feedback for a target outside
/// either range bound.
pub(crate) const CONDITION_FEEDBACK_OUTSIDE_WEAPON_RANGE: u16 = 42;

/// A failed range check, with the numbers its log row reports.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct RangeFailure {
    pub refusal: RangeRefusal,
    pub distance: f32,
    pub bounds: RangeBounds,
}

/// Check `distance` against `bounds` for a caster. Pure.
pub(crate) fn check_cast_range(
    bounds: RangeBounds,
    distance: f32,
    caster_is_player: bool,
) -> Option<RangeFailure> {
    bounds
        .refusal(distance, caster_is_player)
        .map(|refusal| RangeFailure {
            refusal,
            distance,
            bounds,
        })
}

/// `onErrorCode` args for an ability refused as out of range.
fn out_of_range_error_args(ability_id: i32) -> Vec<u8> {
    let mut args = Vec::with_capacity(7);
    args.push(0u8); // SystemID = ERRORCODE_SYSTEM_Ability
    args.extend_from_slice(&ability_id.to_le_bytes()); // InstanceID
    args.extend_from_slice(&CONDITION_FEEDBACK_OUTSIDE_WEAPON_RANGE.to_le_bytes());
    args
}

/// Log a range refusal and, for a player, send the client its feedback.
/// `phase` is `"launch"` or `"warmup_fire"`.
pub(crate) async fn refuse_out_of_range(
    entity_id: u32,
    ability_id: i32,
    target_id: u32,
    failure: RangeFailure,
    phase: &'static str,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &SpaceManager,
) {
    let is_player = space_mgr.get_entity(entity_id).is_some_and(|e| e.is_player);
    let id = space_mgr.player_identity(entity_id);
    tracing::debug!(
        target: "abilities",
        event = "cast_refused",
        reason = failure.refusal.reason(),
        phase,
        entity_id,
        account_id = id.account_id,
        player_id = id.player_id,
        ability_id,
        target_id,
        distance = failure.distance,
        min_range = failure.bounds.min,
        max_range = failure.bounds.max,
        error_code = CONDITION_FEEDBACK_OUTSIDE_WEAPON_RANGE,
        "useAbility refused: the target is outside the ability's range (onErrorCode 42)"
    );
    if !is_player {
        return;
    }
    let args = out_of_range_error_args(ability_id);
    let row = wire_ledger::prepare(crate::mercury::method_idx::ON_ERROR_CODE, &args);
    if tx
        .send(CellToBaseMsg::EntityMethodCall {
            entity_id,
            method_index: crate::mercury::method_idx::ON_ERROR_CODE,
            args,
        })
        .await
        .is_err()
    {
        crate::cell::abilities::metrics::wire_send_failed_in(
            space_mgr,
            entity_id,
            crate::cell::abilities::metrics::WireMessage::OnErrorCode,
        );
        tracing::warn!(
            target: "abilities",
            event = "cast_refused_send_failed",
            reason = failure.refusal.reason(),
            entity_id,
            account_id = id.account_id,
            player_id = id.player_id,
            ability_id,
            "useAbility: the out-of-range onErrorCode could not be queued (base channel closed)"
        );
    } else {
        row.sent_to_owner(
            space_mgr,
            entity_id,
            WireCtx::new("cast_range").reason(failure.refusal.reason()),
        );
    }
}
