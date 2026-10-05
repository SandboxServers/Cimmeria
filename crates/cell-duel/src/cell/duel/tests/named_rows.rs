//! Rule 6 (NT-28b): the `duel.ended` row names both duelists, the winner
//! and the loser, and the world, so a duel can be read from SigNoz without
//! looking up ids. Type 12 (`LogCapture`).

use tokio::sync::mpsc;

use super::end_paths::ended_row;
use super::engage::{aoi_mgr, engage};
use super::*;
use crate::test_support::LogCapture;

/// Character and login names on A (challenger) and B (target), stamped the
/// way `CreateEntity` stamps them.
fn name_duelists(mgr: &mut SpaceManager) {
    mgr.get_entity_mut(A_EID)
        .unwrap()
        .stamp_log_names(Some("Jack O'Neill"), Some("oneill_login"));
    mgr.get_entity_mut(B_EID)
        .unwrap()
        .stamp_log_names(Some("Teal'c"), Some("tealc_login"));
}

/// B disconnects mid-duel. The end runs before B's entity goes, and the
/// row names both sides from the snapshot taken before the clear: the
/// challenger (`player_name`, `entity_name`, `account_name`), the target
/// (`target_*`), the loser (B, the one who left) and the winner, and the
/// world. Fails with any of those name fields removed from `end_engaged`.
#[tokio::test]
async fn duel_end_names_both_duelists_the_winner_and_the_loser() {
    let mut mgr = aoi_mgr();
    name_duelists(&mut mgr);
    let (tx, mut rx) = mpsc::channel(256);
    engage(&mut mgr, &tx, &mut rx).await;
    drain(&mut rx);

    let capture = LogCapture::install();
    mgr.disconnect_entity(B_EID, &tx).await;

    let row = ended_row(&capture);
    for (k, v) in [
        ("reason", "connection"),
        ("player_id", "1000"),
        ("player_name", "Jack O'Neill"),
        ("entity_name", "Jack O'Neill"),
        ("account_name", "oneill_login"),
        ("target_player_id", "2000"),
        ("target_player_name", "Teal'c"),
        ("target_entity_name", "Teal'c"),
        ("target_account_name", "tealc_login"),
        ("loser_player_id", "2000"),
        ("loser_player_name", "Teal'c"),
        ("winner_player_id", "1000"),
        ("winner_player_name", "Jack O'Neill"),
        ("world", "Agnos"),
    ] {
        assert!(row.has_field(k, v), "duel.ended {k}={v}: {row:#?}");
    }
}

/// An aborted duel has no winner or loser, so neither name is on the row
/// (never a blank or "unknown" placeholder, Rule 6).
#[tokio::test]
async fn aborted_duel_end_leaves_winner_and_loser_names_off() {
    let mut mgr = aoi_mgr();
    name_duelists(&mut mgr);
    let (tx, mut rx) = mpsc::channel(256);
    engage(&mut mgr, &tx, &mut rx).await;
    drain(&mut rx);
    let duel_id = {
        use crate::cell::duel::DuelResources;
        mgr.resources.duels().duel_of(A_PID).unwrap().duel_id
    };

    let capture = LogCapture::install();
    crate::cell::duel::end_engaged(
        &tx,
        &mut mgr,
        duel_id,
        crate::cell::duel::EndReason::GmAborted,
    )
    .await
    .expect("the engaged duel ends");

    let row = ended_row(&capture);
    assert!(row.has_field("player_name", "Jack O'Neill"), "{row:#?}");
    assert!(row.has_field("target_player_name", "Teal'c"), "{row:#?}");
    for k in [
        "loser_player_name",
        "winner_player_name",
        "killer_entity_name",
    ] {
        assert!(!row.fields.contains_key(k), "{k} on an abort: {row:#?}");
    }
}

/// The recycled-slot branch of `duelist_log_names` (#889 shape): B left and
/// B's entity id now belongs to another player. A challenge that still
/// names B's old entity is refused `target_gone`, and its row names B from
/// `known_names`, with no `target_entity_name` and nothing of the slot's
/// new occupant. Fails if the helper's `player_id` filter is removed (the
/// row then names the impostor for both fields).
#[tokio::test]
async fn refused_challenge_names_a_departed_target_not_its_slots_new_occupant() {
    let mut mgr = make_mgr();
    name_duelists(&mut mgr);
    cimmeria_entity::known_names::remember_player(B_PID, "Teal'c");
    mgr.destroy_entity(B_EID);
    add_player(&mut mgr, B_EID, 9_999, 900, "Agnos", [5.0, 0.0, 0.0]);
    mgr.get_entity_mut(B_EID)
        .unwrap()
        .stamp_log_names(Some("Impostor"), Some("impostor_login"));
    let (tx, _rx) = mpsc::channel(256);

    let capture = LogCapture::install();
    challenge(
        &mut mgr,
        &tx,
        (A_EID, A_PID),
        (B_EID, B_PID),
        std::time::Instant::now(),
    )
    .await;

    let row = capture
        .all()
        .into_iter()
        .find(|c| c.has_field("event", "duel.challenge_refused"))
        .expect("duel.challenge_refused");
    assert!(row.has_field("reason", "target_gone"), "{row:#?}");
    assert!(row.has_field("target_player_name", "Teal'c"), "{row:#?}");
    assert!(row.has_field("player_name", "Jack O'Neill"), "{row:#?}");
    assert!(
        !row.fields.contains_key("target_entity_name"),
        "a recycled slot is never named: {row:#?}"
    );
    assert!(
        !row.fields
            .values()
            .any(|v| v.contains("Impostor") || v.contains("impostor")),
        "the slot's new occupant is named: {row:#?}"
    );
}
