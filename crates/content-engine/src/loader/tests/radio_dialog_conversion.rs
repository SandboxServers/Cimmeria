//! `display_radio_dialog` row → [`Action::DisplayRadioDialog`] (DK-06b,
//! D-DK10). No database.

use super::super::action::convert_action;
use super::super::*;
use crate::actions::Action;

fn radio_row(target_id: Option<i32>) -> DbActionRow {
    DbActionRow {
        chain_id: 8160,
        action_type: "display_radio_dialog".to_string(),
        target_id,
        target_key: None,
        params: serde_json::json!({}),
        delay_ms: 0,
        sort_order: 0,
    }
}

/// The Dakara radio row converts to its own variant, not `DisplayDialog`.
#[test]
fn convert_display_radio_dialog_row() {
    assert_eq!(
        convert_action(&radio_row(Some(5816))),
        Some(Action::DisplayRadioDialog { dialog_id: 5816 })
    );
}

/// No dialog id, no action: the row is dropped, as a `display_dialog` row is.
#[test]
fn convert_display_radio_dialog_without_target_is_dropped() {
    assert_eq!(convert_action(&radio_row(None)), None);
}
