use super::*;
use chrono::Utc;

fn world_entry(account_id: u32, account: &str, character: &str) -> Event {
    Event::PlayerWorldEntry {
        account: Named::new(account_id, Some(account.into())),
        character: Named::name_only(character),
        world: Named::name_only("Castle_CellBlock"),
        position: [0.0; 3],
        timestamp: Utc::now(),
    }
}

fn level_up(character: &str) -> Event {
    Event::PlayerLevelUp {
        character: Named::name_only(character),
        new_level: 2,
        timestamp: Utc::now(),
    }
}

#[test]
fn nothing_is_muted_without_a_list() {
    assert!(!is_muted(&[], &world_entry(10, "lab", "Lab Muteless")));
}

/// Regression guard for the lab spamming Discord: the lab account's world
/// entry is muted by name in any case, and so is a later event that only
/// names the character it played.
#[test]
fn a_muted_account_and_the_characters_it_plays_are_muted() {
    let muted = vec!["LAB".to_string()];
    assert!(is_muted(&muted, &world_entry(10, "lab", "Mutetest Alpha")));
    assert!(is_muted(&muted, &level_up("mutetest alpha")));
    assert!(!is_muted(&muted, &level_up("Someone Else")));
    assert!(!is_muted(&muted, &world_entry(3, "cady", "Real Player")));
}

#[test]
fn accounts_can_be_muted_by_id() {
    let muted = vec!["10".to_string()];
    assert!(is_muted(
        &muted,
        &world_entry(10, "anything", "Mutetest Beta")
    ));
    let tracing = Event::TracingEvent {
        kind: crate::TracingEventKind::Warn,
        target: "spawner".into(),
        message: "x".into(),
        fields: vec![("account_id".into(), "10".into())],
        timestamp: Utc::now(),
    };
    assert!(is_muted(&muted, &tracing));
}
