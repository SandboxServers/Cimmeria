//! Squad telemetry (organizations campaign telemetry rule): one INFO
//! outcome row per action with the actor's identity (and the target's only
//! where the action has one), DEBUG transitions with before/after values,
//! and WARN on the negative seams.

use cimmeria_cell_world::cell::squad::SquadResources;
use std::time::Instant;

use crate::test_support::Captured;
use cimmeria_entity::organization::SQUAD_ORG_ID_MIN;

use super::*;
use crate::test_support::LogCapture;

const SID: i32 = SQUAD_ORG_ID_MIN;

/// Every outcome row (an `outcome` field) on the `squad` target.
fn outcome_rows(capture: &LogCaptureGuard) -> Vec<Captured> {
    capture
        .all()
        .into_iter()
        .filter(|c| c.target == "squad" && c.fields.contains_key("outcome"))
        .collect()
}

/// The single DEBUG transition row named `event`.
fn transition(capture: &LogCaptureGuard, event: &str) -> Captured {
    let rows: Vec<_> = capture
        .all()
        .into_iter()
        .filter(|c| c.target == "squad" && c.level == Level::DEBUG && c.has_field("event", event))
        .collect();
    assert_eq!(rows.len(), 1, "{event}: {rows:#?}");
    rows.into_iter().next().unwrap()
}

fn assert_actor(row: &Captured, player_id: i32) {
    assert!(
        row.has_field("player_id", &player_id.to_string())
            && row.has_field("account_id", &account_of(player_id).to_string()),
        "actor identity: {:?}",
        row.fields
    );
}

fn assert_target(row: &Captured, player_id: Option<i32>) {
    match player_id {
        Some(p) => assert!(
            row.has_field("target_player_id", &p.to_string())
                && row.has_field("target_account_id", &account_of(p).to_string()),
            "target identity: {:?}",
            row.fields
        ),
        None => assert!(
            !row.fields.contains_key("target_player_id")
                && !row.fields.contains_key("target_account_id"),
            "a one-player action carries no target: {:?}",
            row.fields
        ),
    }
}

/// Each of the five actions ends in exactly one INFO `ok` row. Invite,
/// response and kick name the second player; leave and loot mode do not.
#[tokio::test]
async fn every_action_emits_exactly_one_outcome_row() {
    let capture = LogCapture::install();
    let mut mgr = world(&["Alice", "Bob", "Cara"]);
    let (tx, _rx) = channel();

    let req = invite_only(&mut mgr, &tx, 11, 12).await;
    squad::respond(12, req, true, &tx, &mut mgr).await;
    let req3 = invite_only(&mut mgr, &tx, 11, 13).await;
    squad::respond(13, req3, true, &tx, &mut mgr).await;
    squad::set_loot_mode(11, 1, &tx, &mut mgr).await;
    squad::handle_kick(1, 11, SID, "Cara", &tx, &mut mgr).await;
    squad::leave(12, SID, &tx, &mut mgr).await;

    let rows = outcome_rows(&capture);
    let events: Vec<&str> = rows.iter().map(|r| r.fields["event"].as_str()).collect();
    assert_eq!(
        events,
        [
            "squad.invite",
            "squad.invite_response",
            "squad.invite",
            "squad.invite_response",
            "squad.loot_mode",
            "squad.kick",
            "squad.leave",
        ]
    );
    for r in &rows {
        assert_eq!(r.level, Level::INFO, "{:?}", r.fields);
        assert!(r.has_field("outcome", "ok"), "{:?}", r.fields);
        assert!(!r.fields.contains_key("reason"), "{:?}", r.fields);
    }
    // Invite: Alice invites Bob, with the request id.
    assert_actor(&rows[0], 1);
    assert_target(&rows[0], Some(2));
    assert!(rows[0].has_field("request_id", &req.to_string()));
    // Response: Bob answers Alice's invite and lands in the new squad.
    assert_actor(&rows[1], 2);
    assert_target(&rows[1], Some(1));
    assert!(rows[1].has_field("squad_id", &SID.to_string()));
    // Loot mode, one player.
    assert_actor(&rows[4], 1);
    assert_target(&rows[4], None);
    // Kick: Alice kicks Cara.
    assert_actor(&rows[5], 1);
    assert_target(&rows[5], Some(3));
    // Leave, one player.
    assert_actor(&rows[6], 2);
    assert_target(&rows[6], None);
    assert!(rows[6].has_field("squad_id", &SID.to_string()));
}

/// A refusal is also exactly one row, INFO, with its reason.
#[tokio::test]
async fn a_refusal_is_one_info_row_with_a_reason() {
    let capture = LogCapture::install();
    let mut mgr = world(&["Alice"]);
    let (tx, _rx) = channel();
    squad::leave(11, SID, &tx, &mut mgr).await;
    let rows = outcome_rows(&capture);
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].level, Level::INFO);
    assert!(rows[0].has_field("outcome", "rejected"));
    assert!(rows[0].has_field("reason", "not_in_squad"));
    assert_actor(&rows[0], 1);
}

/// Creating a squad and joining it log the invite, consume, create and
/// join transitions with both identities.
#[tokio::test]
async fn join_transitions_are_logged() {
    let capture = LogCapture::install();
    let mut mgr = world(&["Alice", "Bob"]);
    let (tx, _rx) = channel();
    let req = invite_accept(&mut mgr, &tx, 11, 12).await;

    let created = transition(&capture, "invite_created");
    assert!(created.has_field("request_id", &req.to_string()));
    assert_actor(&created, 1);
    assert_target(&created, Some(2));
    let consumed = transition(&capture, "invite_consumed");
    assert!(consumed.has_field("accepted", "true"));
    assert_actor(&consumed, 2);
    assert_target(&consumed, Some(1));
    let squad = transition(&capture, "squad_created");
    assert!(squad.has_field("squad_id", &SID.to_string()));
    assert_actor(&squad, 1);
    let joined: Vec<_> = capture
        .all()
        .into_iter()
        .filter(|c| c.has_field("event", "member_joined"))
        .collect();
    assert_eq!(joined.len(), 2, "both founders join");
    assert!(joined[0].has_field("rank", "8") && joined[1].has_field("rank", "2"));
}

/// The leader of three leaves: `member_left` (requested) and
/// `leader_changed` from Alice to Bob, naming Bob as the target. Then the
/// pair dissolves on Bob's leave: `disbanded`.
#[tokio::test]
async fn departure_transitions_carry_before_and_after() {
    let capture = LogCapture::install();
    let mut mgr = world(&["Alice", "Bob", "Cara"]);
    let (tx, _rx) = channel();
    seed_squad(&mut mgr, 11, &[12, 13]);
    squad::leave(11, SID, &tx, &mut mgr).await;

    let left = transition(&capture, "member_left");
    assert!(left.has_field("reason", "requested"));
    assert_actor(&left, 1);
    let lead = transition(&capture, "leader_changed");
    assert!(lead.has_field("from_player_id", "1") && lead.has_field("to_player_id", "2"));
    assert_target(&lead, Some(2));

    squad::on_disconnect(12, &tx, &mut mgr).await;
    let disbanded = transition(&capture, "disbanded");
    assert!(disbanded.has_field("reason", "logout"));
    assert!(capture
        .all()
        .iter()
        .any(|c| c.has_field("event", "member_left") && c.has_field("reason", "logout")));
}

#[tokio::test]
async fn loot_mode_changed_logs_from_and_to() {
    let capture = LogCapture::install();
    let mut mgr = world(&["Alice", "Bob"]);
    let (tx, _rx) = channel();
    seed_squad(&mut mgr, 11, &[12]);
    squad::set_loot_mode(11, 1, &tx, &mut mgr).await;
    let row = transition(&capture, "loot_mode_changed");
    assert!(row.has_field("from", "round_robin") && row.has_field("to", "free_for_all"));
    assert_actor(&row, 1);
}

/// An invite answered after 60 s: the `invite_expired` transition and an
/// `invite_expired` outcome, told apart from an unknown id.
#[tokio::test]
async fn expired_invite_is_logged_as_expired() {
    let capture = LogCapture::install();
    let mut mgr = world(&["Alice", "Bob"]);
    let (tx, mut rx) = channel();
    let past = Instant::now()
        .checked_sub(crate::cell::squad::INVITE_TTL)
        .expect("uptime exceeds the invite TTL");
    let issued = mgr
        .resources
        .squads_mut()
        .invite(1, "Alice", 2, past)
        .unwrap();
    squad::respond(12, issued.request_id, true, &tx, &mut mgr).await;

    assert!(squad_event(
        &capture,
        Level::INFO,
        "squad.invite_response",
        "invite_expired"
    ));
    let row = transition(&capture, "invite_expired");
    assert!(row.has_field("request_id", &issued.request_id.to_string()));
    assert_actor(&row, 1);
    assert_target(&row, Some(2));
    assert_eq!(
        to(&drain(&mut rx), 12),
        rejection(0, "That invitation has expired.")
    );
}

/// Negative seam: a forwarded call for an entity that is no longer that
/// character. WARN `squad.actor_mismatch`, plus the one outcome row.
#[tokio::test]
async fn actor_mismatch_warns_and_is_rejected() {
    let capture = LogCapture::install();
    let mut mgr = world(&["Alice", "Bob"]);
    let (tx, mut rx) = channel();
    squad::handle_invite(9, 11, "Bob", &tx, &mut mgr).await;
    assert!(drain(&mut rx).is_empty(), "a stale entity is not answered");
    let warn = capture
        .all()
        .into_iter()
        .find(|c| c.level == Level::WARN && c.has_field("event", "squad.actor_mismatch"))
        .expect("WARN squad.actor_mismatch");
    assert_eq!(warn.target, "squad");
    assert!(squad_event(
        &capture,
        Level::INFO,
        "squad.invite",
        "actor_mismatch"
    ));
    // The claimed character, not whoever holds the entity now.
    let row = &outcome_rows(&capture)[0];
    assert!(row.has_field("player_id", "9") && !row.fields.contains_key("account_id"));
}

/// Negative seam: the cell-to-base channel is gone, so the refusal cannot
/// be queued. WARN `squad.send_failed` with its reason, per dropped send.
#[tokio::test]
async fn dropped_send_warns() {
    let capture = LogCapture::install();
    let mut mgr = world(&["Alice"]);
    let (tx, rx) = channel();
    drop(rx);
    squad::set_loot_mode(11, 1, &tx, &mut mgr).await;
    let warns: Vec<_> = capture
        .all()
        .into_iter()
        .filter(|c| {
            c.level == Level::WARN
                && c.target == "squad"
                && c.has_field("event", "squad.send_failed")
                && c.has_field("reason", "cell_to_base_closed")
        })
        .collect();
    assert_eq!(warns.len(), 2, "the error code and the feedback line");
    // The action still logged its outcome.
    assert!(squad_event(
        &capture,
        Level::INFO,
        "squad.loot_mode",
        "not_in_squad"
    ));
}
