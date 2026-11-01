//! The `observe` result of a world click: what the client showed afterwards.

use serde_json::{json, Value};

use super::click::{Expect, UiSnap};
use super::click_range::{range_hint, range_json};
use super::find::Found;

/// What the click step needs from the resolve and hover steps.
pub(super) struct ClickPlan {
    pub(super) button: usize,
    pub(super) expect: Expect,
    pub(super) entity: Option<u32>,
    pub(super) units: super::lua::UnitSlots,
    pub(super) before: UiSnap,
    pub(super) at: (i32, i32),
    pub(super) hover_verified: bool,
    pub(super) face_json: Value,
    pub(super) found: Option<Found>,
    pub(super) range_m: Option<f64>,
}

/// Everything `click_result` reads once the click has been observed.
pub(super) struct ClickTail<'a> {
    pub expect: Expect,
    pub entity: Option<u32>,
    pub before: &'a UiSnap,
    pub after: &'a UiSnap,
    pub ok: bool,
    pub settle: u64,
    pub button: usize,
    pub range_m: Option<f64>,
    pub found: Option<Found>,
    pub at: (i32, i32),
    pub hover_verified: bool,
    pub face_json: Value,
    pub fallback: Value,
}

/// The tool result, or the failure message and evidence when `ok` is false.
pub(super) fn click_result(tail: ClickTail<'_>) -> Result<Value, (String, Value)> {
    let ClickTail {
        expect,
        entity,
        before,
        after,
        ok,
        settle,
        button,
        range_m,
        found,
        at: (x, y),
        hover_verified,
        face_json,
        fallback,
    } = tail;
    let opened: Vec<&String> = after
        .windows
        .iter()
        .filter(|w| !before.windows.contains(w))
        .collect();
    let closed: Vec<&String> = before
        .windows
        .iter()
        .filter(|w| !after.windows.contains(w))
        .collect();
    let mut evidence = json!({
        "target_before": before.target,
        "target_after": after.target,
        "target_is_entity": entity.is_some_and(|e| after.target == e),
        "windows_opened": opened,
        "windows_closed": closed,
        "mouse_over_after": after.mouse_over,
        "range": range_json(range_m),
    });
    if expect == Expect::Nothing {
        evidence["target_after"] = Value::Null;
        evidence["target_is_entity"] = Value::Null;
        evidence["windows_opened"] = Value::Null;
        evidence["windows_closed"] = Value::Null;
        evidence["mouse_over_after"] = Value::Null;
        evidence["after_observed"] = json!(false);
    }
    if !ok {
        return Err((
            format!(
                "the click did not produce the expected {expect:?} within {settle} ms{}",
                range_hint(button, range_m)
            ),
            evidence,
        ));
    }
    Ok(json!({
        "entity": found.as_ref().map(Found::to_json),
        "button": if button == 0 { "left" } else { "right" },
        "clicked_at": [x, y],
        "hover_verified": hover_verified,
        "expect": format!("{expect:?}").to_lowercase(),
        "result": evidence,
        "camera": face_json,
        "fallback": fallback,
    }))
}
