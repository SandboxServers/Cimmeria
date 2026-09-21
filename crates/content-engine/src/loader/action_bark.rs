//! `npc_bark` row → [`Action::NpcBark`].
//!
//! A bark is a companion combat line ("Let's move out!") spoken into the
//! triggering player's chat window with no dialog window. The 2009 client
//! has no non-modal dialog path, so the line rides
//! `onPlayerCommunication` instead — see [`Action::NpcBark`] for the
//! client-contract reasoning.
//!
//! Every param is **rejected** on a bad value rather than defaulted
//! through. The failure modes here are not "the line is missing" but
//! "the line is visibly wrong in the chat window":
//!
//! - no `screen_id` → no text to resolve from `resources.dialog_screens`;
//! - no `speaker` → the client renders the empty-name prefix (`[] says`),
//!   the exact garbling that got the `system_message` stub disconnected
//!   from method 28 in the first place;
//! - an unrecognised `channel` → either a channel the client never
//!   registered (its red unknown-channel splash popup) or `CHAN_splash`,
//!   whose native trigger has not been traced.

use tracing::warn;

use crate::actions::Action;

use super::DbActionRow;

/// `EChannel::CHAN_say` (`entities/defs/enumerations.xml`). The only
/// channel a bark may use until someone verifies `CHAN_splash` in the
/// client.
const CHAN_SAY: u8 = 0;

/// Convert one `npc_bark` `content_actions` row.
///
/// Returns `None` — dropping the row with a `warn!` naming the chain —
/// for any param the executor could not turn into a correctly rendered
/// chat line.
pub(super) fn convert_npc_bark(row: &DbActionRow) -> Option<Action> {
    let params = &row.params;

    let screen_id = match params.get("screen_id").and_then(|v| v.as_i64()) {
        Some(v) => match i32::try_from(v) {
            Ok(v) => v,
            Err(_) => {
                warn!(
                    chain_id = row.chain_id,
                    screen_id = v,
                    "npc_bark: screen_id is out of i32 range \
                     (resources.dialog_screens.screen_id is an integer column); \
                     dropping the action row"
                );
                return None;
            }
        },
        None => {
            warn!(
                chain_id = row.chain_id,
                ?params,
                "npc_bark: missing integer `screen_id` param -- the line text is \
                 resolved server-side from resources.dialog_screens, so there is \
                 nothing to speak; dropping the action row"
            );
            return None;
        }
    };

    let speaker = match params
        .get("speaker")
        .and_then(|v| v.as_str())
        .map(str::trim)
        .filter(|s| !s.is_empty())
    {
        Some(s) => s.to_string(),
        None => {
            warn!(
                chain_id = row.chain_id,
                screen_id,
                "npc_bark: missing or empty `speaker` param -- the bark screens \
                 carry speaker_id 0, so the name cannot be recovered server-side; \
                 dropping the action row"
            );
            return None;
        }
    };

    // Case-insensitive, matching `set_npc_ai_state`'s `state` param.
    // Absent means `say`; anything else is an authoring mistake.
    let channel = match params
        .get("channel")
        .map(|v| v.as_str().unwrap_or("").to_ascii_lowercase())
    {
        None => CHAN_SAY,
        Some(name) if name == "say" => CHAN_SAY,
        Some(other) => {
            warn!(
                chain_id = row.chain_id,
                screen_id,
                channel = %other,
                "npc_bark: `channel` must be \"say\" -- it is the only non-modal \
                 client route verified today; dropping the action row"
            );
            return None;
        }
    };

    Some(Action::NpcBark {
        screen_id,
        speaker,
        channel,
    })
}
