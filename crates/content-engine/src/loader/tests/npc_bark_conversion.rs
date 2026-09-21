//! `npc_bark` row → [`Action::NpcBark`] conversion.
//!
//! Every param on this verb is reject-on-bad rather than
//! default-through, and each rejection is the difference between "no
//! line" and "a visibly wrong line in the player's chat window". These
//! pin all four rejection shapes plus the two accepted spellings of the
//! `channel` param, with no database.

use super::super::action::convert_action;
use super::super::*;
use crate::actions::Action;

fn bark_row(params: serde_json::Value) -> DbActionRow {
    DbActionRow {
        chain_id: 1,
        action_type: "npc_bark".to_string(),
        target_id: None,
        target_key: None,
        params,
        delay_ms: 0,
        sort_order: 0,
    }
}

/// The shape a DU-07 seed row will actually carry: the real screen id of
/// Col. Marsh's escort line, an explicit speaker, an explicit channel.
#[test]
fn convert_npc_bark_full_row() {
    let action = convert_action(&bark_row(serde_json::json!({
        "screen_id": 96351,
        "speaker": "Col. Marsh",
        "channel": "say"
    })))
    .expect("a fully specified npc_bark row must convert");

    assert_eq!(
        action,
        Action::NpcBark {
            screen_id: 96351,
            speaker: "Col. Marsh".to_string(),
            channel: 0,
        },
        "channel \"say\" is EChannel::CHAN_say = 0"
    );
}

/// `channel` is optional and defaults to `say`. Pinned because the
/// default is the only value the client is verified to render
/// non-modally — a future default drift would be invisible in the seed.
#[test]
fn convert_npc_bark_defaults_channel_to_say() {
    let action = convert_action(&bark_row(serde_json::json!({
        "screen_id": 96352,
        "speaker": "Col. Marsh"
    })))
    .expect("a row omitting `channel` must convert");

    match action {
        Action::NpcBark { channel, .. } => assert_eq!(channel, 0),
        other => panic!("expected NpcBark, got {other:?}"),
    }
}

/// Case is normalised the same way `set_npc_ai_state` normalises its
/// `state` param, so an author writing "Say" is not silently dropped.
#[test]
fn convert_npc_bark_channel_is_case_insensitive() {
    assert!(
        convert_action(&bark_row(serde_json::json!({
            "screen_id": 96352, "speaker": "Col. Marsh", "channel": "SAY"
        })))
        .is_some(),
        "\"SAY\" must convert — the channel match is case-insensitive"
    );
}

/// Any channel other than `say` is dropped. `splash` specifically: its
/// native trigger has not been traced, so sending on `CHAN_splash` would
/// either do nothing or hit the client's unknown-channel path. Rejecting
/// at load means the mistake shows up as a missing line in the seed
/// review, not as a red popup in a playtest.
#[test]
fn convert_npc_bark_rejects_non_say_channel() {
    for bad in ["splash", "yell", "team", "feedback", ""] {
        assert!(
            convert_action(&bark_row(serde_json::json!({
                "screen_id": 96351, "speaker": "Col. Marsh", "channel": bad
            })))
            .is_none(),
            "channel {bad:?} must be rejected — only \"say\" is verified"
        );
    }
}

/// No `screen_id` means no text to resolve from `resources.dialog_screens`.
#[test]
fn convert_npc_bark_rejects_missing_screen_id() {
    assert!(
        convert_action(&bark_row(serde_json::json!({ "speaker": "Col. Marsh" }))).is_none(),
        "a row with no screen_id must be dropped"
    );
}

/// A non-integer `screen_id` is an authoring slip, not a value to coerce.
#[test]
fn convert_npc_bark_rejects_non_integer_screen_id() {
    assert!(
        convert_action(&bark_row(serde_json::json!({
            "screen_id": "96351", "speaker": "Col. Marsh"
        })))
        .is_none(),
        "a string screen_id must be dropped rather than parsed"
    );
    assert!(
        convert_action(&bark_row(serde_json::json!({
            "screen_id": 4_294_967_296i64, "speaker": "Col. Marsh"
        })))
        .is_none(),
        "a screen_id outside i32 must be dropped, not wrapped — \
         dialog_screens.screen_id is an integer column"
    );
}

/// A missing or blank speaker renders as the client's empty-name prefix
/// (`[] says`) — the exact garbling that got the `system_message` stub
/// disconnected from method 28. Whitespace counts as blank.
#[test]
fn convert_npc_bark_rejects_missing_or_blank_speaker() {
    assert!(
        convert_action(&bark_row(serde_json::json!({ "screen_id": 96351 }))).is_none(),
        "a row with no speaker must be dropped"
    );
    for blank in ["", "   "] {
        assert!(
            convert_action(&bark_row(serde_json::json!({
                "screen_id": 96351, "speaker": blank
            })))
            .is_none(),
            "speaker {blank:?} must be dropped — it renders as \"[] says\""
        );
    }
}

/// A speaker with padding converts with the padding stripped, so a
/// trailing space in the seed cannot show up in the chat window.
#[test]
fn convert_npc_bark_trims_speaker() {
    let action = convert_action(&bark_row(serde_json::json!({
        "screen_id": 96351, "speaker": "  Col. Marsh  "
    })))
    .expect("a padded speaker must convert");
    match action {
        Action::NpcBark { speaker, .. } => assert_eq!(speaker, "Col. Marsh"),
        other => panic!("expected NpcBark, got {other:?}"),
    }
}
