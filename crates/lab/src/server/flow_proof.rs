//! The end-screen evidence of the flow tools and `lab_ensure_in_world` (C3).
//! Each flow returns `Ok` only after it has read the screen it waited for; the
//! builders here turn that read into one required Client check. No `await`
//! here: the callers stamp the JSON with the result.

use serde_json::{json, Value};

use crate::proof::{Check, Evidence, Source};

/// The screen a flow tool's last wait (or final read) names, or `None` for a
/// tool that is not a judged flow (`lab_characters` is OPEN and stays out).
pub(super) fn end_screen(tool: &str) -> Option<&'static str> {
    Some(match tool {
        "lab_login"
        | "lab_create_character"
        | "lab_delete_character"
        | "lab_ensure_character_slot"
        | "lab_logout" => "character_select",
        "lab_play_character" => "world_hud",
        "lab_finish_dialog" => "dialog_closed",
        "lab_disconnect" => "login_screen",
        _ => return None,
    })
}

/// The evidence of an `Ok` flow result: one required Client check
/// `end_state` that the flow reached the screen it waited for. A flow that
/// returned without a final read would be `ok: None` (`no_post_read`); none
/// does, so every `Ok` flow measures its screen.
pub(super) fn flow_evidence(tool: &str, _v: &Value) -> Evidence {
    let Some(screen) = end_screen(tool) else {
        return Evidence::no_evidence();
    };
    Evidence::judge(vec![Check::new(
        "end_state",
        Source::Client,
        Some(true),
        true,
    )
    .with_after(json!({ "screen": screen }))])
}

/// The evidence of a `lab_ensure_in_world` result: one required Client check
/// `in_world`, true when the result says the character is in the world.
pub(super) fn in_world_evidence(v: &Value) -> Evidence {
    let in_world = v["in_world"] == true;
    Evidence::judge(vec![Check::new(
        "in_world",
        Source::Client,
        Some(in_world),
        true,
    )
    .with_after(json!({ "world_id": v["world_id"], "pos": v["pos"] }))])
}

#[cfg(test)]
#[path = "flow_proof_tests.rs"]
mod tests;
