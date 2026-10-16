use serde_json::json;

use super::*;

fn click(store_seq: u64, epoch: &str, click_seq: u64, name: Option<&str>) -> StoredEvent {
    StoredEvent {
        seq: store_seq,
        kind: CLICK_KIND.into(),
        ts_ms: 1_000 + store_seq as i64,
        fields: json!({
            "event_id": format!("{epoch}:{click_seq}"),
            "click_epoch": epoch,
            "click_seq": click_seq,
            "window": "TutorialWin",
            "button": "TutorialWin__auto_closebutton__",
            "mouse": "left",
            "selected_entity_id": null,
            "selected_target_name": name,
            "target_present": name.is_some(),
        }),
    }
}

#[test]
fn repeated_widget_lost_row_and_target_change_never_join_later_target() {
    let rows = [
        click(10, "lua-a", 1, Some("First target")),
        click(11, "lua-a", 2, None),
        click(12, "lua-a", 4, Some("Later target")),
    ];
    let result = ClickSequence::from_stored(&rows, 412);
    assert_eq!(result.events.len(), 3);
    assert_eq!(result.missing_ring_rows, 1);
    assert_eq!(result.evicted, 0);
    assert_eq!(result.events[0].event_id, "lua-a:1");
    assert_eq!(result.events[1].event_id, "lua-a:2");
    assert_eq!(result.events[2].event_id, "lua-a:4");
    assert_eq!(
        result.events[0].selected_target_name.as_deref(),
        Some("First target")
    );
    assert_eq!(result.events[1].selected_target_name, None);
    assert_eq!(
        result.events[2].selected_target_name.as_deref(),
        Some("Later target")
    );
    assert!(result.events.iter().all(|e| e.selected_entity_id.is_none()));
    assert!(result
        .events
        .iter()
        .all(|e| e.target_id_gap.as_deref() == Some("target_entity_id_unobservable_at_click")));
    assert_eq!(
        result.events[0].widget_path,
        "TutorialWin/TutorialWin__auto_closebutton__"
    );
    assert_eq!(result.events[0].button, "TutorialWin__auto_closebutton__");
    assert_eq!(result.events[0].mouse, "left");
    assert_eq!(result.events[0].client_pid, 412);
    assert_eq!(result.events[0].host_ms, 1_010);
    assert_eq!(result.events[0].client_tick, None);
}

#[test]
fn relog_changes_event_identity_without_creating_a_false_gap() {
    let rows = [click(1, "lua-a", 1, None), click(2, "lua-b", 1, None)];
    let result = ClickSequence::from_stored(&rows, 412);
    assert_eq!(result.missing_ring_rows, 0);
    assert_ne!(result.events[0].event_id, result.events[1].event_id);
}

#[test]
fn malformed_identity_and_non_click_rows_cannot_be_correlated() {
    let mut malformed = click(1, "lua-a", 1, None);
    malformed.fields["event_id"] = json!("lua-a:2");
    let mut other = click(2, "lua-a", 2, None);
    other.kind = "chat.line".into();
    assert_eq!(
        ClickSequence::from_stored(&[malformed, other], 412),
        ClickSequence::default()
    );
}
