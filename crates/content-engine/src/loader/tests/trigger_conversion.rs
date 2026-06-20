//! DB-row → `Trigger` conversions: mission-accepted, item-equipped
//! (wildcard / typed / malformed), interact-tag, the cover-system
//! family, and npc-flanked.

use super::super::trigger::convert_trigger;
use super::super::*;

/// `event_type = "mission_accepted"` with the mission id in `event_key`
/// must round-trip into a `Trigger::OnMissionAccepted`. This is the
/// load-bearing path for chains like Aftermath chain 1097 that need
/// to react to mission start without piggybacking on whichever chain
/// did the accepting.
#[test]
fn mission_accepted_event_type_loads_as_on_mission_accepted_trigger() {
    use crate::triggers::Trigger;

    let row = DbTriggerRow {
        chain_id: 1097,
        event_type: "mission_accepted".to_string(),
        event_key: Some("687".to_string()),
        scope: "player".to_string(),
        once: false,
        sort_order: 0,
    };

    match convert_trigger(&row) {
        Some(Trigger::OnMissionAccepted { mission_id }) => assert_eq!(mission_id, 687),
        other => panic!("expected OnMissionAccepted(687), got {:?}", other),
    }
}

/// `mission_accepted` without an `event_key` cannot resolve a target
/// mission — must drop to None so the loader skips the row rather
/// than firing on every accept event.
#[test]
fn mission_accepted_without_key_returns_none() {
    let row = DbTriggerRow {
        chain_id: 1097,
        event_type: "mission_accepted".to_string(),
        event_key: None,
        scope: "player".to_string(),
        once: false,
        sort_order: 0,
    };
    assert!(convert_trigger(&row).is_none());
}

/// `item_equipped` accepts NULL `event_key` as a wildcard — the chain
/// should match any equipped item.
#[test]
fn item_equipped_null_key_loads_as_wildcard() {
    use crate::triggers::Trigger;
    let row = DbTriggerRow {
        chain_id: 9000,
        event_type: "item_equipped".to_string(),
        event_key: None,
        scope: "player".to_string(),
        once: false,
        sort_order: 0,
    };
    match convert_trigger(&row) {
        Some(Trigger::OnItemEquipped { item_id: None }) => {}
        other => panic!("expected OnItemEquipped(wildcard), got {:?}", other),
    }
}

/// A specific integer key must round-trip as a typed filter.
#[test]
fn item_equipped_numeric_key_loads_as_typed_filter() {
    use crate::triggers::Trigger;
    let row = DbTriggerRow {
        chain_id: 1004,
        event_type: "item_equipped".to_string(),
        event_key: Some("55".to_string()),
        scope: "player".to_string(),
        once: false,
        sort_order: 0,
    };
    match convert_trigger(&row) {
        Some(Trigger::OnItemEquipped { item_id: Some(55) }) => {}
        other => panic!("expected OnItemEquipped(55), got {:?}", other),
    }
}

/// A non-empty `event_key` that fails to parse as i32 must drop the
/// chain entirely. Silently collapsing `Some("bad")` into `None` would
/// turn a typo'd integer into a wildcard that fires for every equip
/// event — visible bug shape: an unrelated equip would advance an
/// unrelated mission.
#[test]
fn item_equipped_malformed_key_returns_none_not_wildcard() {
    let row = DbTriggerRow {
        chain_id: 9001,
        event_type: "item_equipped".to_string(),
        event_key: Some("not_a_number".to_string()),
        scope: "player".to_string(),
        once: false,
        sort_order: 0,
    };
    assert!(
        convert_trigger(&row).is_none(),
        "malformed item_equipped event_key must reject the chain, not silently \
         load as a wildcard match",
    );
}

#[test]
fn convert_interact_tag_trigger() {
    let row = DbTriggerRow {
        chain_id: 1,
        event_type: "interact_tag".to_string(),
        event_key: Some("ArmYourself_FrostBody".to_string()),
        scope: "player".to_string(),
        once: false,
        sort_order: 0,
    };
    let trigger = convert_trigger(&row).unwrap();
    match trigger {
        Trigger::OnInteractTag { entity_tag } => {
            assert_eq!(entity_tag, "ArmYourself_FrostBody")
        }
        other => panic!("Expected OnInteractTag, got {:?}", other),
    }
}

// ─── Cover-system trigger conversions ───────────────────────────────

fn cover_trigger_row(event_type: &str, event_key: Option<&str>) -> DbTriggerRow {
    DbTriggerRow {
        chain_id: 9209,
        event_type: event_type.to_string(),
        event_key: event_key.map(|s| s.to_string()),
        scope: "player".to_string(),
        once: false,
        sort_order: 0,
    }
}

#[test]
fn convert_player_entered_cover_with_set_id() {
    let trigger = convert_trigger(&cover_trigger_row("player_entered_cover", Some("42"))).unwrap();
    match trigger {
        Trigger::OnPlayerEnteredCover { cover_set_id } => {
            assert_eq!(cover_set_id, Some(42));
        }
        other => panic!("Expected OnPlayerEnteredCover, got {:?}", other),
    }
}

#[test]
fn convert_player_entered_cover_wildcard() {
    let trigger = convert_trigger(&cover_trigger_row("player_entered_cover", None)).unwrap();
    match trigger {
        Trigger::OnPlayerEnteredCover { cover_set_id } => {
            assert_eq!(cover_set_id, None);
        }
        other => panic!("Expected wildcard OnPlayerEnteredCover, got {:?}", other),
    }
}

#[test]
fn convert_player_entered_cover_rejects_bad_set_id() {
    // Typo'd integer must reject the chain rather than collapse to
    // wildcard — same shape as the item_equipped guard.
    let trigger = convert_trigger(&cover_trigger_row(
        "player_entered_cover",
        Some("not-a-number"),
    ));
    assert!(
        trigger.is_none(),
        "non-integer event_key must reject the chain, got {:?}",
        trigger
    );
}

#[test]
fn convert_player_left_cover_with_set_id() {
    let trigger = convert_trigger(&cover_trigger_row("player_left_cover", Some("7"))).unwrap();
    match trigger {
        Trigger::OnPlayerLeftCover { cover_set_id } => {
            assert_eq!(cover_set_id, Some(7));
        }
        other => panic!("Expected OnPlayerLeftCover, got {:?}", other),
    }
}

#[test]
fn convert_player_in_cover_duration_seconds_only() {
    let trigger =
        convert_trigger(&cover_trigger_row("player_in_cover_duration", Some("5"))).unwrap();
    match trigger {
        Trigger::OnPlayerInCoverDuration {
            cover_set_id,
            seconds,
        } => {
            assert_eq!(cover_set_id, None);
            assert_eq!(seconds, 5);
        }
        other => panic!("Expected OnPlayerInCoverDuration, got {:?}", other),
    }
}

#[test]
fn convert_player_in_cover_duration_seconds_and_set_id() {
    let trigger = convert_trigger(&cover_trigger_row(
        "player_in_cover_duration",
        Some("10:42"),
    ))
    .unwrap();
    match trigger {
        Trigger::OnPlayerInCoverDuration {
            cover_set_id,
            seconds,
        } => {
            assert_eq!(cover_set_id, Some(42));
            assert_eq!(seconds, 10);
        }
        other => panic!(
            "Expected OnPlayerInCoverDuration with set_id, got {:?}",
            other
        ),
    }
}

#[test]
fn convert_player_in_cover_duration_rejects_bad_seconds() {
    let trigger = convert_trigger(&cover_trigger_row(
        "player_in_cover_duration",
        Some("not-a-number:42"),
    ));
    assert!(trigger.is_none(), "bad seconds must reject the chain");
}

#[test]
fn convert_npc_flanked_with_template() {
    let trigger = convert_trigger(&cover_trigger_row("npc_flanked", Some("HumanGuard"))).unwrap();
    match trigger {
        Trigger::OnNpcFlanked { npc_template } => {
            assert_eq!(npc_template, Some("HumanGuard".to_string()));
        }
        other => panic!("Expected OnNpcFlanked, got {:?}", other),
    }
}

#[test]
fn convert_npc_flanked_wildcard() {
    let trigger = convert_trigger(&cover_trigger_row("npc_flanked", None)).unwrap();
    match trigger {
        Trigger::OnNpcFlanked { npc_template } => {
            assert_eq!(npc_template, None);
        }
        other => panic!("Expected wildcard OnNpcFlanked, got {:?}", other),
    }
}
