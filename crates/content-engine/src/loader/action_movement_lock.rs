//! `movement_lock` row → [`Action::MovementLock`] (cellblock-boot-lock D-BL3).
//!
//! ```text
//! target_id  = NULL
//! target_key = NULL
//! params     = {"op": "hold", "anchor_tag": "Cellblock_PrisonBootControl",
//!               "at": [-333.03, 73.472, -227.32], "max_distance": 3.0,
//!               "client_flag": false, "message": "Locked to the floor."}
//! params     = {"op": "release", "message": "The prison boot unlocks."}
//! ```
//!
//! `max_distance` (default 3.0), `client_flag` (default `false`) and
//! `message` are optional. A malformed param drops the row with a `warn!`
//! naming the chain: a hold that silently never pins, or a release that never
//! lets go, is worse than one the loader refuses where an author will see it.

use tracing::warn;

use crate::actions::Action;
use crate::movement_lock::{
    MovementLockOp, DEFAULT_MAX_DISTANCE, MAX_MAX_DISTANCE, MAX_MESSAGE_CHARS,
};

use super::DbActionRow;

/// `anchor_tag` names a spawn tag; keep it short and printable.
const MAX_TAG_CHARS: usize = 64;

/// Convert one `movement_lock` `content_actions` row.
pub(super) fn convert_movement_lock(row: &DbActionRow) -> Option<Action> {
    let params = &row.params;
    let drop_row = |why: &str| {
        warn!(
            chain_id = row.chain_id,
            chain_name = cimmeria_names::book().chain(row.chain_id),
            ?params,
            reason = "malformed_movement_lock",
            "movement_lock: {why}; dropping the action row"
        );
        None
    };

    let message = match params.get("message") {
        None => None,
        Some(v) => match v.as_str().map(str::trim) {
            Some(m) if (1..=MAX_MESSAGE_CHARS).contains(&m.chars().count()) => Some(m.to_string()),
            _ => return drop_row("`message` must be a string of 1-200 chars once trimmed"),
        },
    };
    match params.get("op").and_then(|v| v.as_str()) {
        Some("release") => Some(Action::MovementLock(MovementLockOp::Release { message })),
        Some("hold") => {
            let anchor_tag = match params.get("anchor_tag").and_then(|v| v.as_str()) {
                Some(t)
                    if !t.is_empty()
                        && t.chars().count() <= MAX_TAG_CHARS
                        && t.chars().all(|c| c.is_ascii_graphic()) =>
                {
                    t.to_string()
                }
                _ => {
                    return drop_row(
                        "`anchor_tag` must be 1-64 printable ASCII characters with no spaces",
                    )
                }
            };
            let at = match params
                .get("at")
                .and_then(|v| v.as_array())
                .and_then(|xs| xs.iter().map(finite_f32).collect::<Option<Vec<f32>>>())
            {
                Some(v) if v.len() == 3 => [v[0], v[1], v[2]],
                _ => return drop_row("`at` must be an array of 3 finite numbers"),
            };
            let max_distance = match params.get("max_distance") {
                None => DEFAULT_MAX_DISTANCE,
                Some(v) => match finite_f32(v) {
                    Some(d) if d > 0.0 && d <= MAX_MAX_DISTANCE => d,
                    _ => return drop_row("`max_distance` must be a number > 0 and <= 10"),
                },
            };
            let client_flag = match params.get("client_flag") {
                None => false,
                Some(v) => match v.as_bool() {
                    Some(b) => b,
                    None => return drop_row("`client_flag` must be a boolean"),
                },
            };
            Some(Action::MovementLock(MovementLockOp::Hold {
                anchor_tag,
                at,
                max_distance,
                client_flag,
                message,
            }))
        }
        _ => drop_row("`op` must be \"hold\" or \"release\""),
    }
}

/// A JSON number as a finite `f32`. `None` for non-numbers, and for values an
/// `f64` cast cannot hold (1e300 casts to infinity).
fn finite_f32(v: &serde_json::Value) -> Option<f32> {
    v.as_f64().map(|x| x as f32).filter(|x| x.is_finite())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row(params: serde_json::Value) -> DbActionRow {
        DbActionRow {
            chain_id: 1023,
            action_type: "movement_lock".to_string(),
            target_id: None,
            target_key: None,
            params,
            delay_ms: 0,
            sort_order: 0,
        }
    }

    fn convert(params: serde_json::Value) -> Option<Action> {
        super::super::action::convert_action(&row(params))
    }

    #[test]
    fn converts_a_full_hold() {
        let a = convert(serde_json::json!({
            "op": "hold",
            "anchor_tag": "Cellblock_PrisonBootControl",
            "at": [-333.5, 73.5, -227.25],
            "max_distance": 2.5,
            "client_flag": true,
            "message": "Locked to the floor."
        }));
        assert_eq!(
            a,
            Some(Action::MovementLock(MovementLockOp::Hold {
                anchor_tag: "Cellblock_PrisonBootControl".to_string(),
                at: [-333.5, 73.5, -227.25],
                max_distance: 2.5,
                client_flag: true,
                message: Some("Locked to the floor.".to_string()),
            }))
        );
    }

    #[test]
    fn hold_defaults() {
        let a = convert(serde_json::json!({
            "op": "hold",
            "anchor_tag": "Cellblock_PrisonBootControl",
            "at": [1.0, 2.0, 3.0]
        }));
        match a {
            Some(Action::MovementLock(MovementLockOp::Hold {
                max_distance,
                client_flag,
                message,
                ..
            })) => {
                assert_eq!(max_distance, DEFAULT_MAX_DISTANCE);
                assert!(!client_flag);
                assert_eq!(message, None);
            }
            other => panic!("expected a hold, got {other:?}"),
        }
    }

    #[test]
    fn converts_release_with_and_without_message() {
        assert_eq!(
            convert(serde_json::json!({"op": "release"})),
            Some(Action::MovementLock(MovementLockOp::Release {
                message: None
            }))
        );
        assert_eq!(
            convert(serde_json::json!({"op": "release", "message": "The prison boot unlocks."})),
            Some(Action::MovementLock(MovementLockOp::Release {
                message: Some("The prison boot unlocks.".to_string()),
            }))
        );
    }

    #[test]
    fn trims_the_message() {
        assert_eq!(
            convert(serde_json::json!({"op": "release", "message": "  hi  "})),
            Some(Action::MovementLock(MovementLockOp::Release {
                message: Some("hi".to_string()),
            }))
        );
    }

    #[test]
    fn malformed_rows_are_dropped() {
        let cases = [
            serde_json::json!({"anchor_tag": "T", "at": [1, 2, 3]}),
            serde_json::json!({"op": "freeze", "anchor_tag": "T", "at": [1, 2, 3]}),
            serde_json::json!({"op": "hold", "at": [1, 2, 3]}),
            serde_json::json!({"op": "hold", "anchor_tag": "", "at": [1, 2, 3]}),
            serde_json::json!({"op": "hold", "anchor_tag": "has space", "at": [1, 2, 3]}),
            serde_json::json!({"op": "hold", "anchor_tag": "T"}),
            serde_json::json!({"op": "hold", "anchor_tag": "T", "at": [1, 2]}),
            serde_json::json!({"op": "hold", "anchor_tag": "T", "at": [1, "2", 3]}),
            serde_json::json!({"op": "hold", "anchor_tag": "T", "at": [1e300, 2, 3]}),
            serde_json::json!({"op": "hold", "anchor_tag": "T", "at": [1, 2, 3], "max_distance": 0}),
            serde_json::json!({"op": "hold", "anchor_tag": "T", "at": [1, 2, 3], "max_distance": 10.5}),
            serde_json::json!({"op": "hold", "anchor_tag": "T", "at": [1, 2, 3], "max_distance": "3"}),
            serde_json::json!({"op": "hold", "anchor_tag": "T", "at": [1, 2, 3], "client_flag": "yes"}),
            serde_json::json!({"op": "release", "message": "   "}),
            serde_json::json!({"op": "release", "message": "x".repeat(201)}),
            serde_json::json!({"op": "release", "message": 5}),
        ];
        for p in cases {
            assert_eq!(convert(p.clone()), None, "{p}");
        }
    }
}
