//! Interaction range reporting for world clicks.

use serde_json::{json, Value};

/// The server's interaction range, metres, straight line
/// (`MAX_INTERACT_DISTANCE` in `cimmeria-cell-world`, `interact_range.rs`).
pub const INTERACT_RANGE_M: f64 = 5.0;

/// `{distance_m, interact_range_m, in_interact_range}` for an entity click.
pub(super) fn range_json(range_m: Option<f64>) -> Value {
    match range_m {
        Some(d) => json!({
            "distance_m": (d * 100.0).round() / 100.0,
            "interact_range_m": INTERACT_RANGE_M,
            "in_interact_range": d <= INTERACT_RANGE_M,
        }),
        None => Value::Null,
    }
}

/// Why a right-click may have opened nothing: the target was out of range.
pub(super) fn range_hint(button: usize, range_m: Option<f64>) -> String {
    match range_m {
        Some(d) if button == 1 && d > INTERACT_RANGE_M => format!(
            "; the target is {d:.1} m away, beyond the server's {INTERACT_RANGE_M} m interact range: stand 3.5-4.5 m from it"
        ),
        _ => String::new(),
    }
}
