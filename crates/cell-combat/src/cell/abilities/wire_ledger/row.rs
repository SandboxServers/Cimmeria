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
            method_name = $c.method_name,
            origin = $c.origin,
            entity_id = $c.entity_id,
            entity_name = $c.player_name,
            account_id = $c.account_id,
            account_name = $c.account_name,
            player_id = $c.player_id,
            player_name = $c.player_name,
            route = $c.route,
            self_sent = $c.self_sent,
            witness_count = $c.witness_count,
            witness_player_ids = $c.witness_player_ids.as_str(),
            failed_count = $c.failed_count,
            cast_id = $c.cast_id, // nt:id-only per-cast sequence number, no name exists
            $($extra)+
        )
    };
}

/// The fields every row shares.
struct Common {
    /// The ledger's closed label: the decoded methods, else `"other"`.
    method: &'static str,
    method_index: u16,
    /// The method's name on this entity's type, for every index.
    method_name: Option<&'static str>,
    origin: &'static str,
    entity_id: u32,
    account_id: Option<u32>,
    account_name: Option<&'static str>,
    player_id: Option<i32>,
    /// Also the recipient's `entity_name`: a row's entity is the player
    /// whose client the method is queued for.
    player_name: Option<&'static str>,
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

/// The label of the entity a wire argument names, when the row has a
/// `SpaceManager` to ask.
fn label(space_mgr: Option<&SpaceManager>, entity_id: i32) -> Option<&str> {
    let entity_id = u32::try_from(entity_id).ok()?;
    space_mgr.and_then(|m| m.entity_label(entity_id))
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
    // One book read per row, not one per name (a row per ability send).
    let book = cimmeria_names::book();
    let c = Common {
        method: method_name(method_index),
        method_index,
        method_name: space_mgr.map_or_else(
            || cimmeria_wire::names::any_entity_client_method(method_index),
            |m| m.client_method_name(entity_id, method_index),
        ),
        origin: ctx.origin,
        entity_id,
        account_id: who.account_id,
        account_name: who.account_name,
        player_id: who.player_id,
        player_name: who.player_name,
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
            let target_who = u32::try_from(target_id)
                .ok()
                .zip(space_mgr)
                .map_or(PlayerIdentity::UNKNOWN, |(t, m)| m.player_identity(t));
            wire_row!(
                c,
                source_id,
                source_name = label(space_mgr, source_id),
                ability_id,
                ability_name = book.ability(ability_id),
                // The wire's EffectID carries the cast's `cast_id`, not an
                // `effects` row (`hit_wire.rs`).
                effect_id, // nt:id-only the wire EffectID is the cast_id sequence
                target_id,
                target_name = label(space_mgr, target_id),
                target_player_id = target_who.player_id,
                target_player_name = target_who.player_name,
                result_code,
                results_count = count,
                results = results.as_str(),
                "onEffectResults queued for the client"
            );
        }
        Decoded::StatUpdate { count, ref stats } => wire_row!(
            c,
            ability_id = ctx.ability_id,
            ability_name = ctx.ability_id.and_then(|a| book.ability(a)),
            stat_count = count,
            stats = stats.as_str(),
            "stat update queued for the client"
        ),
        Decoded::KnownAbilities { count, ref ids } => wire_row!(
            c,
            ability_count = count,
            ability_ids = ids.as_str(),
            reason = ctx.reason,
            "onKnownAbilitiesUpdate queued for the client"
        ),
        Decoded::AbilityTree {
            lists,
            ref sizes,
            total,
        } => wire_row!(
            c,
            tree_lists = lists,
            tree_sizes = sizes.as_str(),
            tree_total = total,
            reason = ctx.reason,
            "onAbilityTreeInfo queued for the client"
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
                ability_name = ability_id.and_then(|a| book.ability(a)),
                effect_id,
                effect_name = effect_id.and_then(|e| book.effect(e)),
                timer_type = kind,
                timer_type_code = timer_type,
                timer_id = id, // nt:id-only runtime timer handle, nothing to name
                source_id,
                source_name = label(space_mgr, source_id),
                secondary_id, // nt:id-only a duration timer's effect id, named as effect_name
                total_secs = f64::from(total_secs),
                complete_at = f64::from(complete_at),
                action,
                "onTimerUpdate queued for the client"
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
                ability_name = ability_id.and_then(|a| book.ability(a)),
                system_id,   // nt:id-only ERRORCODE_SYSTEM enum code, no name table
                instance_id, // nt:id-only the ability (named as ability_name) or a system instance
                error_code,
                error_name = book.error_code(error_code),
                reason = ctx.reason,
                "onErrorCode queued for the client"
            );
        }
        Decoded::StateField { state_field } => {
            let prev = ctx.prev_state_field;
            wire_row!(
                c,
                state_field,
                state_field_names = %cimmeria_wire::state_field::STATE_FLAGS.render(state_field),
                prev_state_field = prev,
                prev_state_field_names = prev.map(|p| tracing::field::display(cimmeria_wire::state_field::STATE_FLAGS.render(p))),
                bits_set = prev.map(|p| state_field & !p),
                bits_set_names = prev
                    .map(|p| tracing::field::display(cimmeria_wire::state_field::STATE_FLAGS.render(state_field & !p))),
                bits_cleared = prev.map(|p| p & !state_field),
                bits_cleared_names = prev
                    .map(|p| tracing::field::display(cimmeria_wire::state_field::STATE_FLAGS.render(p & !state_field))),
                refcounts = space_mgr
                    .map(|m| refcounts(m, entity_id))
                    .unwrap_or_default()
                    .as_str(),
                reason = ctx.reason,
                "onStateFieldUpdate queued for the client"
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
            ability_name = ctx.ability_id.and_then(|a| book.ability(a)),
            sequence_id,
            sequence_name = book.sequence(sequence_id),
            source_id,
            source_name = label(space_mgr, source_id),
            target_id,
            target_name = label(space_mgr, target_id),
            instance_id, // nt:id-only the phase's cast_id sequence, no name
            reason = ctx.reason,
            "onSequence queued for the client"
        ),
        Decoded::Communication { channel, ref text } => wire_row!(
            c,
            ability_id = ctx.ability_id,
            ability_name = ctx.ability_id.and_then(|a| book.ability(a)),
            channel,
            text = text.as_str(),
            reason = ctx.reason,
            "onPlayerCommunication queued for the client"
        ),
        Decoded::Other | Decoded::Short => wire_row!(
            c,
            ability_id = ctx.ability_id,
            ability_name = ctx.ability_id.and_then(|a| book.ability(a)),
            reason = ctx.reason,
            decode = if matches!(decoded, Decoded::Short) {
                "short"
            } else {
                "none"
            },
            "client method queued"
        ),
    }
}
