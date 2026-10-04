//! The `abilities.wire` row itself: the common fields, then the decoded
//! method's own (see the table in the module docs).

use cimmeria_entity::cell_entity::PlayerIdentity;

use super::super::messaging::{Delivery, WireRoute};
use super::decode::{method_name, timer_type_name, Decoded};
use super::WireCtx;
use crate::cell::space_manager::SpaceManager;

/// The common fields, then the method's own.
macro_rules! wire_row {
    ($c:expr, $($extra:tt)+) => {
        tracing::debug!(
            target: "abilities.wire",
            event = "wire_sent",
            stage = "wire",
            delivery = "queued_to_base",
            method = $c.method,
            method_index = $c.method_index,
            origin = $c.origin,
            entity_id = $c.entity_id,
            account_id = $c.account_id,
            player_id = $c.player_id,
            route = $c.route,
            self_sent = $c.self_sent,
            witness_count = $c.witness_count,
            witness_player_ids = $c.witness_player_ids.as_str(),
            failed_count = $c.failed_count,
            cast_id = $c.cast_id,
            $($extra)+
        )
    };
}

/// The fields every row shares.
struct Common {
    method: &'static str,
    method_index: u16,
    origin: &'static str,
    entity_id: u32,
    account_id: Option<u32>,
    player_id: Option<i32>,
    route: &'static str,
    self_sent: bool,
    witness_count: usize,
    witness_player_ids: String,
    failed_count: usize,
    cast_id: Option<i32>,
}

/// `bit:count` for every counted `state_field` bit, low bit first.
fn refcounts(space_mgr: &SpaceManager, entity_id: u32) -> String {
    let Some(e) = space_mgr.get_entity(entity_id) else {
        return String::new();
    };
    let mut counts: Vec<(u32, u32)> = e
        .state_flag_counts
        .iter()
        .map(|(&mask, &n)| (mask.trailing_zeros(), n))
        .collect();
    counts.sort_unstable();
    counts
        .iter()
        .map(|(bit, n)| format!("{bit}:{n}"))
        .collect::<Vec<_>>()
        .join(",")
}

/// Write one row. `space_mgr` is `None` for a caller that holds only the
/// recipient's identity (a refusal helper); the fields that need a lookup
/// (the witnesses' player ids, the target's, refcounts, the cast scope) are
/// then absent.
#[allow(clippy::too_many_arguments)]
pub(super) fn emit(
    space_mgr: Option<&SpaceManager>,
    who: PlayerIdentity,
    entity_id: u32,
    method_index: u16,
    decoded: &Decoded,
    route: WireRoute,
    delivery: &Delivery,
    ctx: &WireCtx,
) {
    if delivery.delivered() == 0 {
        return;
    }
    // Sorted: the witness set's order is a hash order.
    let witness_player_ids = space_mgr.map_or_else(String::new, |m| {
        let mut ids: Vec<i32> = delivery
            .witness_ids
            .iter()
            .filter_map(|&w| m.player_identity(w).player_id)
            .collect();
        ids.sort_unstable();
        ids.iter().map(i32::to_string).collect::<Vec<_>>().join(",")
    });
    let c = Common {
        method: method_name(method_index),
        method_index,
        origin: ctx.origin,
        entity_id,
        account_id: who.account_id,
        player_id: who.player_id,
        route: route.label(),
        self_sent: delivery.self_sent,
        witness_count: delivery.witness_ids.len(),
        witness_player_ids,
        failed_count: delivery.failed,
        cast_id: ctx
            .cast_id
            .or_else(|| space_mgr.and_then(SpaceManager::current_cast_id)),
    };
    match *decoded {
        Decoded::EffectResults {
            source_id,
            ability_id,
            effect_id,
            target_id,
            result_code,
            count,
            ref results,
        } => {
            let target_player_id = u32::try_from(target_id)
                .ok()
                .zip(space_mgr)
                .and_then(|(t, m)| m.player_identity(t).player_id);
            wire_row!(
                c,
                source_id,
                ability_id,
                effect_id,
                target_id,
                target_player_id,
                result_code,
                results_count = count,
                results = results.as_str(),
                "onEffectResults sent"
            );
        }
        Decoded::StatUpdate { count, ref stats } => wire_row!(
            c,
            ability_id = ctx.ability_id,
            stat_count = count,
            stats = stats.as_str(),
            "onStatUpdate sent"
        ),
        Decoded::Timer {
            id,
            timer_type,
            source_id,
            secondary_id,
            total_secs,
            complete_at,
        } => {
            let kind = timer_type_name(timer_type);
            let (ability_id, effect_id) = match kind {
                "warmup" | "cooldown" => (Some(id), None),
                "duration" => (ctx.ability_id, Some(secondary_id)),
                _ => (ctx.ability_id, None),
            };
            // `0.0` completes in the past: the client drops the timer.
            let action = if complete_at > 0.0 { "start" } else { "clear" };
            wire_row!(
                c,
                ability_id,
                effect_id,
                timer_type = kind,
                timer_type_code = timer_type,
                timer_id = id,
                source_id,
                secondary_id,
                total_secs = f64::from(total_secs),
                complete_at = f64::from(complete_at),
                action,
                "onTimerUpdate sent"
            );
        }
        Decoded::ErrorCode {
            system_id,
            instance_id,
            error_code,
        } => {
            // SystemID 0 is ERRORCODE_SYSTEM_Ability: InstanceID is the ability.
            let ability_id = if system_id == 0 {
                Some(instance_id)
            } else {
                ctx.ability_id
            };
            wire_row!(
                c,
                ability_id,
                system_id,
                instance_id,
                error_code,
                reason = ctx.reason,
                "onErrorCode sent"
            );
        }
        Decoded::StateField { state_field } => {
            let prev = ctx.prev_state_field;
            wire_row!(
                c,
                state_field,
                prev_state_field = prev,
                bits_set = prev.map(|p| state_field & !p),
                bits_cleared = prev.map(|p| p & !state_field),
                refcounts = space_mgr
                    .map(|m| refcounts(m, entity_id))
                    .unwrap_or_default()
                    .as_str(),
                reason = ctx.reason,
                "onStateFieldUpdate sent"
            );
        }
        Decoded::Sequence {
            sequence_id,
            source_id,
            target_id,
            instance_id,
        } => wire_row!(
            c,
            ability_id = ctx.ability_id,
            sequence_id,
            source_id,
            target_id,
            instance_id,
            reason = ctx.reason,
            "onSequence sent"
        ),
        Decoded::Other | Decoded::Short => wire_row!(
            c,
            ability_id = ctx.ability_id,
            decode = if matches!(decoded, Decoded::Short) {
                "short"
            } else {
                "none"
            },
            "client method sent"
        ),
    }
}
