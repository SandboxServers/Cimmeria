//! Unit tests for [`super::UserChannelRegistry`], each on its own fresh
//! instance (never the process-wide [`super::user_channel_registry`]), so
//! they never share state with each other or with the dispatch-level tests
//! in `cimmeria-base`.

use super::*;
use cimmeria_entity::organization::org_text::name_key;

fn key(name: &str) -> String {
    name_key(name)
}

#[test]
fn join_creates_a_new_channel() {
    let reg = UserChannelRegistry::new();
    let outcome = reg.join("chat", &key("chat"), 1);
    assert_eq!(
        outcome,
        JoinOutcome::Joined {
            wire_id: CHAN_CHAT,
            display_name: "chat".to_string(),
            created: true,
        }
    );
    assert_eq!(reg.channel_count(), 1);
}

#[test]
fn second_join_by_name_joins_the_existing_channel_not_a_new_one() {
    let reg = UserChannelRegistry::new();
    let first = reg.join("chat", &key("chat"), 1);
    let JoinOutcome::Joined {
        wire_id: first_id, ..
    } = first
    else {
        panic!("expected Joined, got {first:?}");
    };
    let second = reg.join("chat", &key("chat"), 2);
    assert_eq!(
        second,
        JoinOutcome::Joined {
            wire_id: first_id,
            display_name: "chat".to_string(),
            created: false,
        }
    );
    assert_eq!(reg.channel_count(), 1, "still one channel, not two");
}

/// D-SS13-style fold: "Chat" and "CHAT" land in the same channel as "chat".
#[test]
fn join_is_case_insensitive_on_the_name_key() {
    let reg = UserChannelRegistry::new();
    let JoinOutcome::Joined {
        wire_id: first_id, ..
    } = reg.join("Chat", &key("Chat"), 1)
    else {
        panic!("expected Joined");
    };
    let second = reg.join("CHAT", &key("CHAT"), 2);
    assert_eq!(
        second,
        JoinOutcome::Joined {
            wire_id: first_id,
            // The first creator's spelling wins; the second joiner's own
            // casing is not stored.
            display_name: "Chat".to_string(),
            created: false,
        }
    );
}

#[test]
fn joining_a_channel_already_joined_is_refused() {
    let reg = UserChannelRegistry::new();
    reg.join("chat", &key("chat"), 1);
    let again = reg.join("chat", &key("chat"), 1);
    assert_eq!(
        again,
        JoinOutcome::AlreadyMember {
            display_name: "chat".to_string(),
        }
    );
}

#[test]
fn joining_past_the_per_player_limit_is_refused() {
    let reg = UserChannelRegistry::new();
    for i in 0..MAX_CHANNELS_PER_PLAYER {
        let name = format!("chan{i}");
        let outcome = reg.join(&name, &key(&name), 1);
        assert!(
            matches!(outcome, JoinOutcome::Joined { .. }),
            "channel {i} should have been joined, got {outcome:?}"
        );
    }
    let one_more = reg.join("overflow", &key("overflow"), 1);
    assert_eq!(one_more, JoinOutcome::PlayerLimitReached);
    // Another player is unaffected by entity 1's cap.
    let other_player = reg.join("overflow", &key("overflow"), 2);
    assert!(matches!(other_player, JoinOutcome::Joined { .. }));
}

/// Regression guard: revert the `inner.channels.len() >= MAX_USER_CHANNELS`
/// check (or the id-space fallback) and this test fails because the
/// `MAX_USER_CHANNELS + 1`th distinct channel is created instead of
/// refused.
#[test]
fn joining_past_the_server_wide_channel_limit_is_refused() {
    let reg = UserChannelRegistry::new();
    for i in 0..MAX_USER_CHANNELS {
        // A fresh entity id per channel: this test is about the
        // server-wide channel count, not the per-player cap above.
        let name = format!("chan{i}");
        let outcome = reg.join(&name, &key(&name), 1000 + i as u32);
        assert!(
            matches!(outcome, JoinOutcome::Joined { .. }),
            "channel {i} should have been joined, got {outcome:?}"
        );
    }
    let overflow = reg.join("overflow", &key("overflow"), 999_999);
    assert_eq!(overflow, JoinOutcome::ServerLimitReached);
}

#[test]
fn leave_removes_the_member() {
    let reg = UserChannelRegistry::new();
    let JoinOutcome::Joined { wire_id, .. } = reg.join("chat", &key("chat"), 1) else {
        panic!("expected Joined");
    };
    reg.join("chat", &key("chat"), 2);
    let outcome = reg.leave(wire_id, 1);
    assert_eq!(
        outcome,
        LeaveOutcome::Left {
            display_name: "chat".to_string(),
            deleted: false,
        },
        "one member left, one remains: channel survives"
    );
    assert_eq!(
        reg.members_if_joined(wire_id, 1),
        None,
        "left, not a member"
    );
    assert_eq!(reg.members_if_joined(wire_id, 2).map(|m| m.len()), Some(1));
}

/// The last member's leave deletes the channel and frees its wire id.
#[test]
fn leaving_as_the_last_member_deletes_the_channel() {
    let reg = UserChannelRegistry::new();
    let JoinOutcome::Joined { wire_id, .. } = reg.join("chat", &key("chat"), 1) else {
        panic!("expected Joined");
    };
    let outcome = reg.leave(wire_id, 1);
    assert_eq!(
        outcome,
        LeaveOutcome::Left {
            display_name: "chat".to_string(),
            deleted: true,
        }
    );
    assert_eq!(reg.channel_count(), 0, "empty channel is not kept around");
    // Rejoining by the same name mints a fresh channel; it happens to
    // reuse the freed id (the allocator picks the lowest free one), but a
    // caller should not depend on that -- what matters is it is a *new*
    // channel (created = true), not the ghost of the deleted one.
    let rejoined = reg.join("chat", &key("chat"), 2);
    assert!(matches!(
        rejoined,
        JoinOutcome::Joined { created: true, .. }
    ));
}

#[test]
fn leaving_an_unknown_channel_id_is_not_found() {
    let reg = UserChannelRegistry::new();
    assert_eq!(reg.leave(200, 1), LeaveOutcome::NotFound);
}

#[test]
fn leaving_a_channel_you_never_joined_is_not_member() {
    let reg = UserChannelRegistry::new();
    let JoinOutcome::Joined { wire_id, .. } = reg.join("chat", &key("chat"), 1) else {
        panic!("expected Joined");
    };
    assert_eq!(reg.leave(wire_id, 999), LeaveOutcome::NotMember);
    // The channel is untouched: the one real member is still in it.
    assert_eq!(reg.members_if_joined(wire_id, 1).map(|m| m.len()), Some(1));
}

/// The server-authority check: a client cannot post to a channel it never
/// joined. Reproduces the shape a reverted membership check would leave --
/// [`super::UserChannelRegistry::members_if_joined`] must return `None` for
/// both "channel never existed" and "channel exists, wrong entity".
#[test]
fn members_if_joined_refuses_a_non_member() {
    let reg = UserChannelRegistry::new();
    let JoinOutcome::Joined { wire_id, .. } = reg.join("chat", &key("chat"), 1) else {
        panic!("expected Joined");
    };
    assert_eq!(reg.members_if_joined(wire_id, 999), None);
    assert_eq!(
        reg.members_if_joined(250, 1),
        None,
        "no such channel at all"
    );
}

#[test]
fn members_if_joined_lists_every_member_including_the_caller() {
    let reg = UserChannelRegistry::new();
    let JoinOutcome::Joined { wire_id, .. } = reg.join("chat", &key("chat"), 1) else {
        panic!("expected Joined");
    };
    reg.join("chat", &key("chat"), 2);
    reg.join("chat", &key("chat"), 3);
    let mut members = reg.members_if_joined(wire_id, 1).expect("is a member");
    members.sort_unstable();
    assert_eq!(members, vec![1, 2, 3]);
}

/// `leave_all` (session teardown) removes an entity from every channel it
/// is in and deletes any channel that empties, silently -- there is no
/// client left to send `onChatLeft` to.
#[test]
fn leave_all_removes_the_entity_from_every_channel() {
    let reg = UserChannelRegistry::new();
    let JoinOutcome::Joined {
        wire_id: chat_id, ..
    } = reg.join("chat", &key("chat"), 1)
    else {
        panic!("expected Joined");
    };
    let JoinOutcome::Joined {
        wire_id: roleplay_id,
        ..
    } = reg.join("roleplay", &key("roleplay"), 1)
    else {
        panic!("expected Joined");
    };
    // A bystander keeps "chat" alive; "roleplay" has only entity 1.
    reg.join("chat", &key("chat"), 2);

    reg.leave_all(1);

    assert_eq!(
        reg.members_if_joined(chat_id, 2).map(|m| m.len()),
        Some(1),
        "chat survives with the bystander"
    );
    assert_eq!(reg.members_if_joined(chat_id, 1), None);
    assert_eq!(
        reg.members_if_joined(roleplay_id, 1),
        None,
        "roleplay had no other member and is now gone"
    );
    assert_eq!(
        reg.channel_count(),
        1,
        "only chat (with its bystander) remains"
    );
}

#[test]
fn leave_all_on_an_entity_in_no_channel_is_a_no_op() {
    let reg = UserChannelRegistry::new();
    reg.join("chat", &key("chat"), 1);
    reg.leave_all(999);
    assert_eq!(reg.channel_count(), 1, "untouched");
}
