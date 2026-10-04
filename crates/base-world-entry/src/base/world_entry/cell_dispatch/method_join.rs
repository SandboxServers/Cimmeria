//! The join key a base `client_sent` row carries back to the cell's
//! `wire_sent` row for the same send.
//!
//! The cell's `abilities.wire` `wire_sent` row names the cast (`cast_id`);
//! the base's `client_sent` row, written once the method left the socket,
//! cannot: `CellToBaseMsg::EntityMethodCall` carries no cast id, and giving
//! it one means touching every one of its ~400 constructors. So the base
//! reads back the same payload fields the cell's row decoded, under the same
//! names, and the forensics query joins on them (colo smoke test,
//! 2026-10-04: the two `client_sent` rows of a Heal Focus cast could only be
//! matched to it by time).
//!
//! Join on `entity_id` + `method` and, per method:
//!
//! - `onTimerUpdate`: `timer_id` (the ability for a cooldown or warmup, the
//!   effect for a duration), `timer_type_code`, `secondary_id`, and
//!   `complete_at`: the absolute expiry on the client's clock, the same
//!   `f32` widened the same way on both rows, so it tells two presses of one
//!   ability apart.
//! - `onEffectResults`: `ability_id`, `effect_id` (the cast's `effect_seq`,
//!   equal to its `cast_id` for a single-target hit), `target_id`.
//! - `onErrorCode`: `system_id`, `instance_id`, `error_code`.
//!
//! Layouts: `docs/protocol/client-method-dispatch-table.md`, the same ones
//! `cimmeria_cell_combat`'s `wire_ledger::decode` reads.

use cimmeria_wire::cell::client_methods::being::{ON_EFFECT_RESULTS, ON_TIMER_UPDATE};
use cimmeria_wire::cell::client_methods::player::ON_ERROR_CODE;

/// The payload fields one ability method's `client_sent` row joins on.
/// A field the method does not carry, or a payload too short for its
/// layout, is `None` and absent from the row.
#[derive(Debug, Default, Clone, Copy, PartialEq)]
pub(super) struct JoinFields {
    pub timer_id: Option<i32>,
    pub timer_type_code: Option<i8>,
    pub secondary_id: Option<i32>,
    pub complete_at: Option<f64>,
    pub ability_id: Option<i32>,
    pub effect_id: Option<i32>,
    pub target_id: Option<i32>,
    pub system_id: Option<u8>,
    pub instance_id: Option<i32>,
    pub error_code: Option<u16>,
}

fn i32_at(b: &[u8], at: usize) -> Option<i32> {
    Some(i32::from_le_bytes(b.get(at..at + 4)?.try_into().ok()?))
}

/// Decode the join fields of `method_index`'s `args`.
pub(super) fn join_fields(method_index: u16, args: &[u8]) -> JoinFields {
    match method_index {
        // ID:i32, Type:i8, SourceID:i32, SecondaryId:i32, TotalTime:f32,
        // BigWorldTimeComplete:f32.
        ON_TIMER_UPDATE => JoinFields {
            timer_id: i32_at(args, 0),
            timer_type_code: args.get(4).map(|&t| t as i8),
            secondary_id: i32_at(args, 9),
            complete_at: args
                .get(17..21)
                .and_then(|b| b.try_into().ok())
                .map(|b| f64::from(f32::from_le_bytes(b))),
            ..JoinFields::default()
        },
        // SourceID:i32, AbilityID:i32, EffectID:i32, TargetID:i32, ...
        ON_EFFECT_RESULTS => JoinFields {
            ability_id: i32_at(args, 4),
            effect_id: i32_at(args, 8),
            target_id: i32_at(args, 12),
            ..JoinFields::default()
        },
        // SystemID:u8, InstanceID:i32, ErrorCodeID:u16.
        ON_ERROR_CODE => JoinFields {
            system_id: args.first().copied(),
            instance_id: i32_at(args, 1),
            error_code: args
                .get(5..7)
                .and_then(|b| b.try_into().ok())
                .map(u16::from_le_bytes),
            ..JoinFields::default()
        },
        _ => JoinFields::default(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cimmeria_entity::abilities::{
        serialize_effect_results, serialize_timer_update, TIMER_ABILITY_WARMUP,
    };

    /// The fields come from the real serializers' bytes, so a layout change
    /// on the cell side breaks the join here, not in a forensics query.
    #[test]
    fn a_timer_update_yields_the_cells_join_fields() {
        let args = serialize_timer_update(597, TIMER_ABILITY_WARMUP, 2, 0, 2.0, 1234.5);
        let j = join_fields(ON_TIMER_UPDATE, &args);
        assert_eq!(j.timer_id, Some(597));
        assert_eq!(j.timer_type_code, Some(TIMER_ABILITY_WARMUP));
        assert_eq!(j.secondary_id, Some(0));
        // Widened exactly as the cell's `wire_sent` row widens it.
        assert_eq!(j.complete_at, Some(f64::from(1234.5_f32)));
        assert_eq!(j.ability_id, None);
    }

    #[test]
    fn effect_results_yield_the_effect_id_that_is_the_cast_id() {
        let args = serialize_effect_results(2, 597, 41, 9, 1, &[]);
        let j = join_fields(ON_EFFECT_RESULTS, &args);
        assert_eq!(
            (j.ability_id, j.effect_id, j.target_id),
            (Some(597), Some(41), Some(9))
        );
        assert_eq!(j.timer_id, None);
    }

    #[test]
    fn an_error_code_yields_system_instance_and_code() {
        let mut args = vec![0u8];
        args.extend_from_slice(&597i32.to_le_bytes());
        args.extend_from_slice(&7u16.to_le_bytes());
        let j = join_fields(ON_ERROR_CODE, &args);
        assert_eq!(
            (j.system_id, j.instance_id, j.error_code),
            (Some(0), Some(597), Some(7))
        );
    }

    /// A short payload decodes what it holds and never panics.
    #[test]
    fn a_short_payload_leaves_the_missing_fields_absent() {
        let j = join_fields(ON_TIMER_UPDATE, &[1, 0, 0, 0, 2]);
        assert_eq!(j.timer_id, Some(1));
        assert_eq!(j.timer_type_code, Some(2));
        assert_eq!(j.complete_at, None);
        assert_eq!(join_fields(20, &[0; 32]), JoinFields::default());
    }
}
