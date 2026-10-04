//! Read back the fields an `abilities.wire` row reports from the bytes that
//! went on the wire, so the row says what the client was sent rather than
//! what the caller meant to send.
//!
//! Layouts (`docs/protocol/client-method-dispatch-table.md`):
//!
//! - `onSequence` (1): `KismetEventSetSeqID:i32, SourceID:i32, TargetID:i32,
//!   PrimaryTarget:i8, ImpactTime:f32, NameValuePairs[], ViewType:i8,
//!   InstanceId:i32`. `InstanceId` is the last four bytes whatever the pair
//!   count.
//! - `onTimerUpdate` (12): `ID:i32, Type:i8, SourceID:i32, SecondaryId:i32,
//!   TotalTime:f32, BigWorldTimeComplete:f32`.
//! - `onEffectResults` (14): `SourceID:i32, AbilityID:i32, EffectID:i32,
//!   TargetID:i32, ResultCode:u8, count:u32, count x (stat_id:i8, delta:i32,
//!   damage_code:i8, stat_result_code:i8)`.
//! - `onStateFieldUpdate` (19): `bStateField:i32`.
//! - `onStatUpdate` (20): `count:u32, count x (stat_id:i32, min:i32, cur:i32,
//!   max:i32)`.
//! - `onErrorCode` (121): `SystemID:u8, InstanceID:i32, ErrorCodeID:u16`.

use std::fmt::Write as _;

use cimmeria_entity::abilities::{
    TIMER_ABILITY_COOLDOWN, TIMER_ABILITY_WARMUP, TIMER_CATEGORY_COOLDOWN, TIMER_DURATION_EFFECT,
};

use crate::cell::client_methods::being::{
    ON_EFFECT_RESULTS, ON_STATE_FIELD_UPDATE, ON_TIMER_UPDATE,
};
use crate::cell::client_methods::combatant::ON_STAT_UPDATE;
use crate::cell::client_methods::communicator::ON_PLAYER_COMMUNICATION;
use crate::cell::client_methods::player::ON_ERROR_CODE;
use crate::cell::client_methods::spawnable_entity::ON_SEQUENCE;

/// One decoded payload. `Short` is a payload too small for its layout (a
/// caller bug the row still reports, with the length).
#[derive(Debug, Clone, PartialEq)]
pub(super) enum Decoded {
    Sequence {
        sequence_id: i32,
        source_id: i32,
        target_id: i32,
        instance_id: i32,
    },
    Timer {
        id: i32,
        timer_type: i8,
        source_id: i32,
        secondary_id: i32,
        total_secs: f32,
        complete_at: f32,
    },
    EffectResults {
        source_id: i32,
        ability_id: i32,
        effect_id: i32,
        target_id: i32,
        result_code: u8,
        count: u32,
        /// `stat_id:delta` pairs, comma-separated.
        results: String,
    },
    StateField {
        state_field: u32,
    },
    StatUpdate {
        count: u32,
        /// `stat_id:cur/max` triples, comma-separated, in wire order.
        stats: String,
    },
    ErrorCode {
        system_id: u8,
        instance_id: i32,
        error_code: u16,
    },
    /// A method this ledger has no layout for.
    Other,
    Short,
}

fn i32_at(b: &[u8], at: usize) -> Option<i32> {
    Some(i32::from_le_bytes(b.get(at..at + 4)?.try_into().ok()?))
}

fn u32_at(b: &[u8], at: usize) -> Option<u32> {
    Some(u32::from_le_bytes(b.get(at..at + 4)?.try_into().ok()?))
}

fn f32_at(b: &[u8], at: usize) -> Option<f32> {
    Some(f32::from_le_bytes(b.get(at..at + 4)?.try_into().ok()?))
}

/// The client method's name for `method_index`, as the dispatch table
/// spells it; `"other"` for a method this ledger does not decode.
pub(crate) fn method_name(method_index: u16) -> &'static str {
    match method_index {
        ON_SEQUENCE => "onSequence",
        ON_TIMER_UPDATE => "onTimerUpdate",
        ON_EFFECT_RESULTS => "onEffectResults",
        ON_STATE_FIELD_UPDATE => "onStateFieldUpdate",
        ON_STAT_UPDATE => "onStatUpdate",
        ON_ERROR_CODE => "onErrorCode",
        ON_PLAYER_COMMUNICATION => "onPlayerCommunication",
        _ => "other",
    }
}

/// `onTimerUpdate.Type` as a label.
pub(super) fn timer_type_name(timer_type: i8) -> &'static str {
    match timer_type {
        TIMER_ABILITY_WARMUP => "warmup",
        TIMER_ABILITY_COOLDOWN => "cooldown",
        TIMER_DURATION_EFFECT => "duration",
        TIMER_CATEGORY_COOLDOWN => "category",
        _ => "other",
    }
}

impl Decoded {
    pub(super) fn parse(method_index: u16, b: &[u8]) -> Self {
        let parsed = match method_index {
            ON_SEQUENCE => Self::sequence(b),
            ON_TIMER_UPDATE => Self::timer(b),
            ON_EFFECT_RESULTS => Self::effect_results(b),
            ON_STATE_FIELD_UPDATE => {
                u32_at(b, 0).map(|state_field| Self::StateField { state_field })
            }
            ON_STAT_UPDATE => Self::stat_update(b),
            ON_ERROR_CODE => Self::error_code(b),
            _ => return Self::Other,
        };
        parsed.unwrap_or(Self::Short)
    }

    fn error_code(b: &[u8]) -> Option<Self> {
        Some(Self::ErrorCode {
            system_id: *b.first()?,
            instance_id: i32_at(b, 1)?,
            error_code: u16::from_le_bytes(b.get(5..7)?.try_into().ok()?),
        })
    }

    fn sequence(b: &[u8]) -> Option<Self> {
        // 26 bytes with no name-value pairs; InstanceId closes the payload.
        if b.len() < 26 {
            return None;
        }
        Some(Self::Sequence {
            sequence_id: i32_at(b, 0)?,
            source_id: i32_at(b, 4)?,
            target_id: i32_at(b, 8)?,
            instance_id: i32_at(b, b.len() - 4)?,
        })
    }

    fn timer(b: &[u8]) -> Option<Self> {
        Some(Self::Timer {
            id: i32_at(b, 0)?,
            timer_type: *b.get(4)? as i8,
            source_id: i32_at(b, 5)?,
            secondary_id: i32_at(b, 9)?,
            total_secs: f32_at(b, 13)?,
            complete_at: f32_at(b, 17)?,
        })
    }

    fn effect_results(b: &[u8]) -> Option<Self> {
        let count = u32_at(b, 17)?;
        let mut results = String::new();
        for i in 0..count as usize {
            let at = 21 + i * 7;
            let stat_id = *b.get(at)? as i8;
            let delta = i32_at(b, at + 1)?;
            if !results.is_empty() {
                results.push(',');
            }
            let _ = write!(results, "{stat_id}:{delta}");
        }
        Some(Self::EffectResults {
            source_id: i32_at(b, 0)?,
            ability_id: i32_at(b, 4)?,
            effect_id: i32_at(b, 8)?,
            target_id: i32_at(b, 12)?,
            result_code: *b.get(16)?,
            count,
            results,
        })
    }

    fn stat_update(b: &[u8]) -> Option<Self> {
        let count = u32_at(b, 0)?;
        let mut stats = String::new();
        for i in 0..count as usize {
            let at = 4 + i * 16;
            let (id, cur, max) = (i32_at(b, at)?, i32_at(b, at + 8)?, i32_at(b, at + 12)?);
            if !stats.is_empty() {
                stats.push(',');
            }
            let _ = write!(stats, "{id}:{cur}/{max}");
        }
        Some(Self::StatUpdate { count, stats })
    }
}

#[cfg(test)]
mod tests {
    use cimmeria_entity::abilities::ClientEffectResult;
    use cimmeria_entity::abilities::{serialize_effect_results, serialize_timer_update};

    use super::*;

    #[test]
    fn effect_results_round_trip() {
        let bytes = serialize_effect_results(
            7,
            579,
            41,
            9,
            1,
            &[ClientEffectResult {
                stat_id: 3,
                delta: -12,
                damage_code: 0,
                stat_result_code: 0,
            }],
        );
        assert_eq!(
            Decoded::parse(ON_EFFECT_RESULTS, &bytes),
            Decoded::EffectResults {
                source_id: 7,
                ability_id: 579,
                effect_id: 41,
                target_id: 9,
                result_code: 1,
                count: 1,
                results: "3:-12".into(),
            }
        );
    }

    #[test]
    fn timer_round_trip_and_short_payloads() {
        let bytes = serialize_timer_update(579, TIMER_ABILITY_COOLDOWN, 7, 0, 1.5, 100.0);
        assert!(matches!(
            Decoded::parse(ON_TIMER_UPDATE, &bytes),
            Decoded::Timer {
                id: 579,
                timer_type: TIMER_ABILITY_COOLDOWN,
                ..
            }
        ));
        assert_eq!(
            Decoded::parse(ON_TIMER_UPDATE, &bytes[..20]),
            Decoded::Short
        );
        assert_eq!(Decoded::parse(ON_ERROR_CODE, &[0, 1]), Decoded::Short);
        assert_eq!(Decoded::parse(9999, &[]), Decoded::Other);
    }
}
