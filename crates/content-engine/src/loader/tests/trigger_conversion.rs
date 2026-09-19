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

#[test]
fn convert_player_flanked_npc_with_template() {
    let trigger =
        convert_trigger(&cover_trigger_row("player_flanked_npc", Some("NID Guard"))).unwrap();
    match trigger {
        Trigger::OnPlayerFlankedNpc { npc_template } => {
            assert_eq!(npc_template, Some("NID Guard".to_string()));
        }
        other => panic!("Expected OnPlayerFlankedNpc, got {:?}", other),
    }
}

#[test]
fn convert_player_flanked_npc_wildcard() {
    let trigger = convert_trigger(&cover_trigger_row("player_flanked_npc", None)).unwrap();
    match trigger {
        Trigger::OnPlayerFlankedNpc { npc_template } => {
            assert_eq!(npc_template, None);
        }
        other => panic!("Expected wildcard OnPlayerFlankedNpc, got {:?}", other),
    }
}

// ─── entity_health_below (Harset H04) ───────────────────────────────

fn health_row(event_key: Option<&str>) -> DbTriggerRow {
    DbTriggerRow {
        chain_id: 6311,
        event_type: "entity_health_below".to_string(),
        event_key: event_key.map(|s| s.to_string()),
        scope: "player".to_string(),
        once: false,
        sort_order: 0,
    }
}

// ─── Stargate trigger conversions (CA10) ───────────────────────────

fn stargate_trigger_row(event_type: &str, event_key: Option<&str>) -> DbTriggerRow {
    DbTriggerRow {
        chain_id: 0x7000_6200,
        event_type: event_type.to_string(),
        event_key: event_key.map(|s| s.to_string()),
        scope: "player".to_string(),
        once: false,
        sort_order: 0,
    }
}

/// The authored form: `"<tag>:<pct>"` round-trips into a tag + integer
/// percentage. This is the row Harset mission 1325 seeds for the Rin'la
/// duel's submit beat.
#[test]
fn convert_entity_health_below_splits_tag_and_percentage() {
    match convert_trigger(&health_row(Some("Rinla_Malac:30"))) {
        Some(Trigger::OnEntityHealthBelow { entity_tag, pct }) => {
            assert_eq!(entity_tag, "Rinla_Malac");
            assert_eq!(pct, 30);
        }
        other => panic!("expected OnEntityHealthBelow(Rinla_Malac, 30), got {other:?}"),
    }
}

/// Split from the RIGHT: a tag containing a colon keeps its colon and
/// only the trailing field is read as the percentage. Splitting from the
/// left would silently truncate the tag and produce a chain that never
/// matches a real entity.
#[test]
fn convert_entity_health_below_splits_from_the_right() {
    match convert_trigger(&health_row(Some("Harset_Market:Malac:30"))) {
        Some(Trigger::OnEntityHealthBelow { entity_tag, pct }) => {
            assert_eq!(entity_tag, "Harset_Market:Malac");
            assert_eq!(pct, 30);
        }
        other => panic!("expected the tag to keep its embedded colon, got {other:?}"),
    }
}

/// Every malformed shape must reject the chain outright. A trigger that
/// degrades to "any tag" or "any percentage" would fire an unrelated
/// mission step on an unrelated NPC — the same bug shape the
/// `item_equipped` wildcard guard exists for.
#[test]
fn convert_entity_health_below_rejects_malformed_keys() {
    for bad in [
        None,                     // no key at all
        Some("Rinla_Malac"),      // no separator
        Some("Rinla_Malac:"),     // empty percentage
        Some("Rinla_Malac:abc"),  // non-integer percentage
        Some(":30"),              // empty tag
        Some("Rinla_Malac:0"),    // 0% is death; routes to entity_dead_tag
        Some("Rinla_Malac:-10"),  // negative
        Some("Rinla_Malac:100"),  // unmatchable: the crossing test is strict
        Some("Rinla_Malac:101"),  // above full health
        Some("Rinla_Malac:30.5"), // fractional
    ] {
        assert!(
            convert_trigger(&health_row(bad)).is_none(),
            "malformed event_key {bad:?} must reject the chain, not load a \
             degraded trigger",
        );
    }
}

/// The inclusive bounds are legal: `:99` is the first threshold a hit off
/// full health can cross (100 → 99), `:1` is the last threshold above
/// death.
#[test]
fn convert_entity_health_below_accepts_the_boundary_percentages() {
    for (key, expected) in [("Boss:1", 1), ("Boss:99", 99)] {
        match convert_trigger(&health_row(Some(key))) {
            Some(Trigger::OnEntityHealthBelow { pct, .. }) => assert_eq!(pct, expected),
            other => panic!("expected {key} to load, got {other:?}"),
        }
    }
}

/// PR #662 review, finding 2. The matcher is a **strict** downward
/// crossing (`pct_before > threshold && pct_after <= threshold`), so a
/// threshold of 100 is unmatchable: full health is 100, `100 > 100` is
/// false, and any later hit starts from below. The loader used to accept
/// `1..=100`, which let an author seed a chain that reads as wired and
/// never runs — the worst failure mode for content, because there is no
/// error to grep for.
///
/// Reverting `HEALTH_PCT_RANGE` to `1..=100` fails the `100` row.
#[test]
fn convert_entity_health_below_rejects_the_unmatchable_hundred_percent_band() {
    for bad in [0, 100, 101] {
        assert!(
            convert_trigger(&health_row(Some(&format!("Boss:{bad}")))).is_none(),
            "threshold {bad} is outside the matchable band and must drop the \
             trigger row rather than load a chain that can never fire",
        );
    }
    for good in [1, 99] {
        assert!(
            convert_trigger(&health_row(Some(&format!("Boss:{good}")))).is_some(),
            "threshold {good} is inside the matchable band and must load",
        );
    }
}

/// `event_key` carries the destination world name straight through.
/// The chain-replay guards in `cimmeria-services` cover the same arm,
/// but self-skip without a database — this is the no-DB signal.
#[test]
fn stargate_dialed_row_loads_with_its_destination_world() {
    match convert_trigger(&stargate_trigger_row("stargate_dialed", Some("Harset"))) {
        Some(Trigger::OnStargateDialed { destination_world }) => {
            assert_eq!(destination_world.as_deref(), Some("Harset"));
        }
        other => panic!("expected OnStargateDialed(Harset), got {other:?}"),
    }
}

#[test]
fn stargate_crossed_row_loads_with_its_destination_world() {
    match convert_trigger(&stargate_trigger_row("stargate_crossed", Some("Harset"))) {
        Some(Trigger::OnStargateCrossed { destination_world }) => {
            assert_eq!(destination_world.as_deref(), Some("Harset"));
        }
        other => panic!("expected OnStargateCrossed(Harset), got {other:?}"),
    }
}

/// A NULL `event_key` is the documented wildcard for these two, unlike
/// the integer-keyed triggers where NULL rejects the row. Pinned so a
/// future "reject NULL everywhere" sweep can't silently disable every
/// wildcard gate chain.
#[test]
fn stargate_rows_with_no_event_key_load_as_wildcards() {
    match convert_trigger(&stargate_trigger_row("stargate_dialed", None)) {
        Some(Trigger::OnStargateDialed { destination_world }) => {
            assert_eq!(destination_world, None);
        }
        other => panic!("expected wildcard OnStargateDialed, got {other:?}"),
    }
    match convert_trigger(&stargate_trigger_row("stargate_crossed", None)) {
        Some(Trigger::OnStargateCrossed { destination_world }) => {
            assert_eq!(destination_world, None);
        }
        other => panic!("expected wildcard OnStargateCrossed, got {other:?}"),
    }
}

/// A near-miss `event_type` drops the row rather than binding one of the
/// two arms. Catches a `starts_with`-style match if anyone rewrites the
/// dispatch, and pins the exact strings content authors must write.
#[test]
fn a_misspelled_stargate_event_type_loads_nothing() {
    for bad in ["stargate_dial", "stargate_dialled", "stargate", "dialed"] {
        assert!(
            convert_trigger(&stargate_trigger_row(bad, Some("Harset"))).is_none(),
            "event_type {bad:?} must not bind a stargate trigger"
        );
    }
}

// ─── mission_abandoned (Harset H54) ───────────────────────────

/// `event_type = "mission_abandoned"` with the mission id in `event_key`
/// round-trips into `Trigger::OnMissionAbandoned`. This is the only way a
/// seed row can react to an abandon, so a missing arm here would leave every
/// authored repaint chain silently unregistered.
#[test]
fn mission_abandoned_event_type_loads_as_on_mission_abandoned_trigger() {
    use crate::triggers::Trigger;

    let row = DbTriggerRow {
        chain_id: 6308,
        event_type: "mission_abandoned".to_string(),
        event_key: Some("1324".to_string()),
        scope: "player".to_string(),
        once: false,
        sort_order: 0,
    };

    match convert_trigger(&row) {
        Some(Trigger::OnMissionAbandoned { mission_id }) => assert_eq!(mission_id, 1324),
        other => panic!("expected OnMissionAbandoned(1324), got {:?}", other),
    }
}

/// No `event_key` means no target mission. Dropping the row is the only safe
/// disposal: a wildcard abandon trigger would repaint an offer on every
/// abandon the player ever performs.
#[test]
fn mission_abandoned_without_key_returns_none() {
    let row = DbTriggerRow {
        chain_id: 6308,
        event_type: "mission_abandoned".to_string(),
        event_key: None,
        scope: "player".to_string(),
        once: false,
        sort_order: 0,
    };
    assert!(convert_trigger(&row).is_none());
}

/// A non-numeric key is a typo, not a wildcard.
#[test]
fn mission_abandoned_with_non_numeric_key_returns_none() {
    let row = DbTriggerRow {
        chain_id: 6308,
        event_type: "mission_abandoned".to_string(),
        event_key: Some("1324a".to_string()),
        scope: "player".to_string(),
        once: false,
        sort_order: 0,
    };
    assert!(convert_trigger(&row).is_none());
}
