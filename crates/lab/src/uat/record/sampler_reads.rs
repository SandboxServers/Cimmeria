//! The sampler's pure parts: the character-name guard, the snapshot SQL,
//! and turning one tool result into one log field.

use serde_json::Value;

use super::super::decode::{decode_call, Call};
use super::super::log::{CameraSample, DialogSample, EntitySample, UiClick};

/// D-LR2: the character name is the one value spliced into SQL as text, so
/// it must match `^[A-Za-z][A-Za-z ]{0,63}$` before any call is made.
pub fn check_character(name: &str) -> Result<(), String> {
    let mut chars = name.chars();
    let ok = chars.next().is_some_and(|c| c.is_ascii_alphabetic())
        && name.len() <= 64
        && chars.all(|c| c.is_ascii_alphabetic() || c == ' ');
    if ok {
        Ok(())
    } else {
        Err(format!(
            "character name {name:?} is not ^[A-Za-z][A-Za-z ]{{0,63}}$"
        ))
    }
}

/// The contract's snapshot statements (missions, player, items), keyed by
/// the integer `player_id` only, never a name. An item's `count` is units
/// (`stack_size` summed, NULL as 1), as FX staging and play count them, so
/// using one of a stack of 5 is a change.
pub fn snapshot_sql(player_id: i32) -> [String; 3] {
    let p = player_id;
    [
        format!("SELECT mission_id, status, current_step_id AS step, repeats FROM sgw_mission WHERE player_id = {p} ORDER BY mission_id"),
        format!("SELECT level, exp, naquadah, world_location FROM sgw_player WHERE player_id = {p}"),
        format!("SELECT i.type_id, i.container_id, coalesce(sum(coalesce(i.stack_size,1)),0)::int AS count, min(t.name) AS name FROM sgw_inventory i LEFT JOIN resources.items t ON t.item_id = i.type_id WHERE i.character_id = {p} GROUP BY i.type_id, i.container_id ORDER BY 1, 2"),
    ]
}

/// `"20261012-143000-p2"`: local start time, then the instance. The id
/// names the recording's directory, so the instance must match
/// `[A-Za-z0-9_-]+`.
pub fn recording_id(now_ms: i64, instance: &str) -> Result<String, String> {
    use chrono::TimeZone as _;
    let ok = !instance.is_empty()
        && instance
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-');
    if !ok {
        return Err(format!("instance {instance:?} is not [A-Za-z0-9_-]+"));
    }
    let at = chrono::Local
        .timestamp_millis_opt(now_ms)
        .single()
        .map(|t| t.format("%Y%m%d-%H%M%S").to_string())
        .unwrap_or_else(|| now_ms.to_string());
    Ok(format!("{at}-{instance}"))
}

/// The entity of the one session carrying `player_id`. Keyed by the id,
/// never the name: a name match can pick another player's session.
pub(super) fn find_session(sessions: &Value, player_id: i32) -> Result<u32, String> {
    let list = sessions
        .get("sessions")
        .and_then(Value::as_array)
        .ok_or("server_sessions returned no sessions list")?;
    let hits: Vec<u32> = list
        .iter()
        .filter(|s| s.get("player_id").and_then(Value::as_i64) == Some(i64::from(player_id)))
        .filter_map(|s| s.get("entity_id").and_then(Value::as_u64))
        .filter_map(|id| u32::try_from(id).ok())
        .collect();
    match hits.as_slice() {
        [one] => Ok(*one),
        [] => Err(format!("no in-world session for player_id {player_id}")),
        many => Err(format!(
            "{} sessions carry player_id {player_id}",
            many.len()
        )),
    }
}

/// The entity an inbound call names (contract mapping table), as LR-02's
/// decoder reads it: the target of `setTargetID`, `interact`, `useItem` and
/// `useAbility`. 0 (`interact`'s "current target") names nothing, and
/// neither does an id that was negative on the wire.
pub(super) fn named_target(row: &Value) -> Option<u32> {
    match decode_call(row)? {
        Call::SetTarget { target }
        | Call::Interact { target }
        | Call::UseItem { target, .. }
        | Call::UseAbility { target, .. } => {
            Some(target).filter(|&id| id != 0 && i32::try_from(id).is_ok())
        }
        _ => None,
    }
}

/// `server_entity_get`'s `{entity}` as a sample; `None` when the entity
/// is gone (null) or the reply has no position.
pub(super) fn entity_sample(v: &Value) -> Option<EntitySample> {
    let e = v.get("entity").filter(|e| e.is_object())?;
    let p = e.get("position").and_then(Value::as_array)?;
    let pos = [
        p.first()?.as_f64()?,
        p.get(1)?.as_f64()?,
        p.get(2)?.as_f64()?,
    ];
    let text = |k: &str| e.get(k).and_then(Value::as_str).map(str::to_string);
    Some(EntitySample {
        entity_id: u32::try_from(e.get("entity_id")?.as_u64()?).ok()?,
        tag: text("tag"),
        template_id: e
            .get("template_id")
            .and_then(Value::as_i64)
            .and_then(|t| i32::try_from(t).ok()),
        name: text("entity_name").or_else(|| text("name")),
        pos,
        space_id: u32::try_from(e.get("space_id")?.as_u64()?).ok()?,
    })
}

/// The camera readout as a sample: `pitch_deg = pitch_offset_deg`, `zoom`,
/// `yaw_gain` (SV-05's signed gain; the raw `gain` is unsigned and would
/// record the wrong turn direction, so a missing one is an error), and the
/// camera actor's `yaw_deg` and server position.
pub(super) fn camera_sample(v: &Value) -> Result<CameraSample, String> {
    let num = |k: &str| {
        v.get(k)
            .and_then(Value::as_f64)
            .ok_or_else(|| format!("no {k}"))
    };
    let pose = v.get("pose").ok_or("no pose")?;
    let server = pose.get("server").ok_or("no pose.server")?;
    let axis = |k: &str| {
        server
            .get(k)
            .and_then(Value::as_f64)
            .ok_or_else(|| format!("no pose.server.{k}"))
    };
    Ok(CameraSample {
        pitch_deg: num("pitch_offset_deg")?,
        zoom: num("zoom")?,
        yaw_gain: num("yaw_gain")?,
        yaw_deg: pose
            .get("yaw_deg")
            .and_then(Value::as_f64)
            .ok_or("no pose.yaw_deg")?,
        pos: [axis("x")?, axis("y")?, axis("z")?],
    })
}

/// `ui_state`'s windows (sorted) and open dialog (title, visible button
/// labels), plus the errors of those two sections. A failed windows
/// section leaves `windows` out rather than recording "no windows".
pub(super) fn ui_sample(v: &Value) -> (Option<Vec<String>>, Option<DialogSample>, Vec<String>) {
    let errors: Vec<String> = v
        .get("errors")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .filter(|e| e.starts_with("windows:") || e.starts_with("dialog:"))
        .map(str::to_string)
        .collect();
    let windows = if errors.iter().any(|e| e.starts_with("windows:")) {
        None
    } else {
        v.get("windows").and_then(Value::as_array).map(|w| {
            let mut w: Vec<String> = w
                .iter()
                .filter_map(Value::as_str)
                .map(str::to_string)
                .collect();
            w.sort();
            w
        })
    };
    let dialog = v
        .get("dialog")
        .filter(|d| d.is_object())
        .map(|d| DialogSample {
            title: d
                .get("title")
                .and_then(Value::as_str)
                .filter(|t| !t.is_empty())
                .map(str::to_string),
            buttons: d
                .get("buttons")
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
                .filter_map(|b| b.get("label").and_then(Value::as_str))
                .filter(|l| !l.is_empty())
                .map(str::to_string)
                .collect(),
        });
    (windows, dialog, errors)
}

/// The probe's `{clicks, gap}` as `UiClick`s, in store order, and whether
/// the store evicted clicks before this read. A field that is not a string
/// (a raw mouse button number) keeps its JSON text; a missing one is empty
/// (`mouse`: `None`). The ring writes `mouse = ""` for a button it cannot
/// name, which is no mouse button either.
pub(super) fn click_sample(v: &Value) -> (Vec<UiClick>, bool) {
    let text = |c: &Value, k: &str| match c.get(k) {
        None | Some(Value::Null) => None,
        Some(Value::String(s)) => Some(s.clone()),
        Some(other) => Some(other.to_string()),
    };
    let clicks = v
        .get("clicks")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .map(|c| UiClick {
            window: text(c, "window").unwrap_or_default(),
            button: text(c, "button").unwrap_or_default(),
            mouse: text(c, "mouse").filter(|m| !m.is_empty()),
        })
        .collect();
    (clicks, v.get("gap").and_then(Value::as_bool) == Some(true))
}
