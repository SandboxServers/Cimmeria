//! Adjacent-negative, relog and refusal guards for the Marsh letter receipt
//! (HA-1360b2 batch 2). Every test drives the harness in the parent module;
//! negatives assert the absence of the request, the consume, the completion and
//! the success frames, not only a refusal reason.
//!
//! Test-only.

use cimmeria_wire::cell::messages::{LetterTurnInOutcome, LetterTurnInRefusal, PluginMsg};

use super::*;

/// The Praxis offer 6511 binds on Marsh's slot 10.
const PRAXIS_OFFER_DSM: i32 = 5254;
/// The letter bind 6502 installs and 6501 clears.
const LETTER_BIND_DSM: i32 = 120101;
/// A container outside the accessible bags (the bank side).
const STORAGE_CONTAINER: i32 = 99;

/// Account and player ids for scenario `n` (each scenario owns two ids).
fn ids(n: i32) -> (i32, i32) {
    (TEST_BASE + 0x10 + 2 * n, TEST_BASE + 0x11 + 2 * n)
}

async fn setup(pool: &PgPool, n: i32, chains: &[i32]) -> Driver {
    let (account_id, player_id) = ids(n);
    cleanup(pool, account_id, player_id).await;
    insert_account_and_player(pool, account_id, player_id).await;
    Driver::new(pool.clone(), account_id, player_id, chains).await
}

async fn setup_production(pool: &PgPool, n: i32) -> Driver {
    let (account_id, player_id) = ids(n);
    cleanup(pool, account_id, player_id).await;
    insert_account_and_player(pool, account_id, player_id).await;
    Driver::production(pool.clone(), account_id, player_id).await
}

async fn teardown(pool: &PgPool, n: i32) {
    let (account_id, player_id) = ids(n);
    cleanup(pool, account_id, player_id).await;
}

/// The dialog sets bound on Marsh's slot 10 for the character.
fn slot10(harset: &SpaceManager) -> Vec<i32> {
    harset
        .get_entity(PLAYER_EID)
        .unwrap()
        .available_interactions
        .get(&10)
        .map(|e| e.iter().map(|&(dsm, _, _)| dsm).collect())
        .unwrap_or_default()
}

/// Everything that must still hold after a refused click: the letter is held,
/// 1360 is not completed and no receipt row exists.
async fn problems_if_consumed(driver: &Driver) -> Vec<String> {
    let mut problems = Vec::new();
    if driver.letter_count().await != 1 {
        problems.push("the letter was consumed".to_owned());
    }
    if driver.row_1360().await.0 == i32::from(MISSION_COMPLETED) {
        problems.push("mission 1360 completed".to_owned());
    }
    if driver.receipt_rows().await != 0 {
        problems.push("a receipt row exists".to_owned());
    }
    problems
}

/// Marsh in a state that must not turn the letter in: no request reaches the
/// base, nothing is consumed or completed, and (where asserted) no mission or
/// dialog frame goes out.
#[tokio::test]
async fn live_db_marsh_receipt_wrong_state_refuses_without_success() {
    let pool = require_db_or_skip!();
    let cases: [(&str, Stage, bool, bool); 4] = [
        (
            "still on 4037",
            Stage {
                cross: false,
                ..Stage::MARSH
            },
            false,
            true,
        ),
        (
            "wrong world",
            Stage {
                world: "Castle",
                ..Stage::MARSH
            },
            false,
            true,
        ),
        (
            "wrong recipient",
            Stage {
                tag: "CmdCenter_Other",
                ..Stage::MARSH
            },
            false,
            true,
        ),
        // The Praxis turn-in legitimately answers this click, so only the
        // letter-side outcome is asserted, not the absence of every frame.
        ("1361 at 4694", Stage::MARSH, true, false),
    ];
    let mut failures = Vec::new();
    for (n, (label, stage_opts, overlap, frames_must_be_silent)) in cases.into_iter().enumerate() {
        let n = n as i32;
        let driver = setup_production(&pool, n).await;
        let (tx, mut rx) = mpsc::channel(128);
        let mut harset = stage(&driver, &tx, &mut rx, stage_opts).await;
        if overlap {
            harset
                .get_entity_mut(PLAYER_EID)
                .unwrap()
                .missions
                .add_mission(MissionInstance::new(1361, 4694, vec![]));
        }
        let click = click_marsh(&driver, &tx, &mut rx, &mut harset).await;
        if request_count(&click) != 0 {
            failures.push(format!("{label}: a turn-in request was queued"));
        }
        let methods = frame_methods(&click);
        if frames_must_be_silent
            && (methods.iter().any(|m| MISSION_FRAMES.contains(m)) || shows_receipt_dialog(&click))
        {
            failures.push(format!("{label}: success frames {methods:?}"));
        }
        driver.route(click).await;
        failures.extend(
            problems_if_consumed(&driver)
                .await
                .into_iter()
                .map(|p| format!("{label}: {p}")),
        );
        teardown(&pool, n).await;
    }
    assert!(failures.is_empty(), "{failures:#?}");
}

/// The letter is absent, foreign-owned or in storage: the base refuses with a
/// reason, nothing is consumed and the mission stays active.
#[tokio::test]
async fn live_db_marsh_receipt_missing_item_is_a_refusal() {
    let pool = require_db_or_skip!();
    let mut failures = Vec::new();
    for (i, label) in ["no letter", "foreign-owned", "storage only"]
        .into_iter()
        .enumerate()
    {
        let n = 8 + 2 * i as i32; // scenario ids 8, 10, 12; 9, 11, 13 hold the other character
        let mut driver = setup(&pool, n, &RECEIPT_CHAINS).await;
        let (other_account, other_player) = ids(n + 1);
        cleanup(&pool, other_account, other_player).await;
        insert_account_and_player(&pool, other_account, other_player).await;
        let (tx, mut rx) = mpsc::channel(128);
        let mut harset = stage(&driver, &tx, &mut rx, Stage::MARSH).await;
        let mine = driver.player_id;
        let sql = match label {
            "no letter" => "DELETE FROM sgw_inventory WHERE character_id = $1 AND type_id = 3730",
            "foreign-owned" => {
                "UPDATE sgw_inventory SET character_id = $2 WHERE character_id = $1 AND type_id = 3730"
            }
            _ => "UPDATE sgw_inventory SET container_id = $2 WHERE character_id = $1 AND type_id = 3730",
        };
        sqlx::query(sql)
            .bind(mine)
            .bind(if label == "storage only" {
                STORAGE_CONTAINER
            } else {
                other_player
            })
            .execute(&pool)
            .await
            .expect("stage the missing letter");

        let click = click_marsh(&driver, &tx, &mut rx, &mut harset).await;
        if request_count(&click) != 1 {
            failures.push(format!("{label}: expected one request"));
        }
        driver.route(click).await;
        let result = driver.next_result().await;
        if !matches!(
            result.outcome,
            LetterTurnInOutcome::Refused(LetterTurnInRefusal::NoAccessibleLetter)
        ) {
            failures.push(format!("{label}: outcome {:?}", result.outcome));
        }
        apply_letter_turn_in_result(result, &tx, &mut harset, &driver.engine).await;
        let queued = drain(&mut rx);
        let methods = frame_methods(&queued);
        if methods.iter().any(|m| MISSION_FRAMES.contains(m)) || shows_receipt_dialog(&queued) {
            failures.push(format!("{label}: success frames {methods:?}"));
        }
        if driver.row_1360().await.0 == i32::from(MISSION_COMPLETED) {
            failures.push(format!("{label}: mission completed"));
        }
        if driver.receipt_rows().await != 0 {
            failures.push(format!("{label}: receipt row written"));
        }
        if driver.step_1360().await != Some(4038) {
            failures.push(format!("{label}: mission left step 4038"));
        }
        if label != "no letter" && label != "foreign-owned" {
            // The storage letter is still the character's own, untouched.
            let kept: i64 = sqlx::query_scalar(
                "SELECT count(*) FROM sgw_inventory WHERE character_id = $1 AND type_id = 3730",
            )
            .bind(mine)
            .fetch_one(&pool)
            .await
            .unwrap();
            if kept != 1 {
                failures.push(format!("{label}: the stored letter was consumed"));
            }
        }
        cleanup(&pool, other_account, other_player).await;
        teardown(&pool, n).await;
    }
    assert!(failures.is_empty(), "{failures:#?}");
}

/// A second click while the first is unresolved is one attempt: one request,
/// one consume, no second result.
#[tokio::test]
async fn live_db_marsh_receipt_duplicate_click_is_one_attempt() {
    let pool = require_db_or_skip!();
    let mut driver = setup_production(&pool, 14).await;
    let (tx, mut rx) = mpsc::channel(128);
    let mut harset = stage(&driver, &tx, &mut rx, Stage::MARSH).await;

    let first = click_marsh(&driver, &tx, &mut rx, &mut harset).await;
    assert_eq!(request_count(&first), 1);
    driver.route(first).await;
    let second = click_marsh(&driver, &tx, &mut rx, &mut harset).await;
    assert_eq!(
        request_count(&second),
        0,
        "the second click is not a request"
    );

    let result = driver.next_result().await;
    assert!(matches!(result.outcome, LetterTurnInOutcome::Committed(_)));
    apply_letter_turn_in_result(result, &tx, &mut harset, &driver.engine).await;
    drain(&mut rx);
    tokio::time::sleep(Duration::from_millis(100)).await;
    assert!(
        driver.base_rx.try_recv().is_err(),
        "one attempt gets one result"
    );
    assert_eq!(driver.letter_count().await, 0, "one consume");
    assert_eq!(driver.receipt_rows().await, 1);
    teardown(&pool, 14).await;
}

/// After the commit the character relogs: the database holds the terminal
/// state, the letter bind is gone, the Praxis offer is restored on entry and
/// nothing is replayed.
#[tokio::test]
async fn live_db_marsh_receipt_relog_after_commit_does_not_replay() {
    let pool = require_db_or_skip!();
    let mut driver = setup_production(&pool, 15).await;
    let (tx, mut rx) = mpsc::channel(128);
    let mut harset = stage(&driver, &tx, &mut rx, Stage::MARSH).await;
    let frames = deliver(&mut driver, &tx, &mut rx, &mut harset).await;
    driver.route(frames).await;
    drop(harset);

    let mut again = enter(&driver, &tx, &mut rx, Stage::MARSH).await;
    assert!(
        !again.methods.iter().any(|m| MISSION_FRAMES.contains(m)) && !again.receipt_dialog,
        "relog replays no dialog or mission frame: {:?}",
        again.methods
    );
    assert_eq!(again.requests, 0);
    let binds = slot10(&again.harset);
    assert!(
        binds.contains(&PRAXIS_OFFER_DSM),
        "offer restored: {binds:?}"
    );
    assert!(!binds.contains(&LETTER_BIND_DSM), "letter bind stays gone");
    let click = click_marsh(&driver, &tx, &mut rx, &mut again.harset).await;
    assert_eq!(request_count(&click), 0, "no second turn-in");
    assert_eq!(driver.letter_count().await, 0);
    let row = driver.row_1360().await;
    assert_eq!(row.0, i32::from(MISSION_COMPLETED));
    assert_eq!(row.6, 1);
    assert_eq!(driver.receipt_rows().await, 1);
    teardown(&pool, 15).await;
}

/// The character relogs with the request unresolved (never reached the base):
/// the letter is still held, 6502 re-adds the letter bind and a new click works.
#[tokio::test]
async fn live_db_marsh_receipt_relog_before_commit_keeps_the_offer() {
    let pool = require_db_or_skip!();
    let mut driver = setup_production(&pool, 16).await;
    let (tx, mut rx) = mpsc::channel(128);
    let mut harset = stage(&driver, &tx, &mut rx, Stage::MARSH).await;
    let lost = click_marsh(&driver, &tx, &mut rx, &mut harset).await;
    assert_eq!(request_count(&lost), 1);
    drop(lost); // the request never reached the base
    drop(harset);

    let mut again = enter(&driver, &tx, &mut rx, Stage::MARSH).await;
    assert_eq!(driver.letter_count().await, 1, "the letter is still held");
    assert!(
        slot10(&again.harset).contains(&LETTER_BIND_DSM),
        "6502 re-adds the letter bind on entry"
    );
    deliver(&mut driver, &tx, &mut rx, &mut again.harset).await;
    assert_eq!(driver.letter_count().await, 0);
    assert_eq!(driver.receipt_rows().await, 1);
    teardown(&pool, 16).await;
}

/// An ambiguous commit is retried with the same attempt: the base answers
/// `AlreadyCommitted`, the letter is consumed once and the stale answer sends
/// no second set of frames.
#[tokio::test]
async fn live_db_marsh_receipt_recovered_delivery_is_already_committed() {
    let pool = require_db_or_skip!();
    let mut driver = setup_production(&pool, 17).await;
    let (tx, mut rx) = mpsc::channel(128);
    let mut harset = stage(&driver, &tx, &mut rx, Stage::MARSH).await;

    let click = click_marsh(&driver, &tx, &mut rx, &mut harset).await;
    driver.route(click).await;
    let first = driver.next_result().await;
    let request = first.request;
    apply_letter_turn_in_result(first, &tx, &mut harset, &driver.engine).await;
    drain(&mut rx);

    driver
        .route(vec![CellToBaseMsg::Plugin(PluginMsg::new(request))])
        .await;
    let retry = driver.next_result().await;
    assert!(
        matches!(retry.outcome, LetterTurnInOutcome::AlreadyCommitted(_)),
        "{:?}",
        retry.outcome
    );
    apply_letter_turn_in_result(retry, &tx, &mut harset, &driver.engine).await;
    let queued = drain(&mut rx);
    let methods = frame_methods(&queued);
    assert!(
        !methods.iter().any(|m| MISSION_FRAMES.contains(m)) && !shows_receipt_dialog(&queued),
        "the retry replays no frame: {methods:?}"
    );
    assert_eq!(driver.letter_count().await, 0, "one consume in total");
    assert_eq!(driver.receipt_rows().await, 1);
    teardown(&pool, 17).await;
}

/// Two characters on the shared Marsh: A's commit consumes, completes and
/// unbinds nothing for B, and B still delivers its own letter.
#[tokio::test]
async fn live_db_marsh_receipt_two_characters_are_isolated() {
    let pool = require_db_or_skip!();
    let mut a = setup_production(&pool, 18).await;
    let mut b = setup_production(&pool, 19).await;
    let (tx_a, mut rx_a) = mpsc::channel(128);
    let (tx_b, mut rx_b) = mpsc::channel(128);
    let mut harset_a = stage(&a, &tx_a, &mut rx_a, Stage::MARSH).await;
    let mut harset_b = stage(&b, &tx_b, &mut rx_b, Stage::MARSH).await;

    let frames = deliver(&mut a, &tx_a, &mut rx_a, &mut harset_a).await;
    a.route(frames).await;

    assert_eq!(b.letter_count().await, 1, "B keeps its letter");
    assert_eq!(b.row_1360().await.0, i32::from(MISSION_ACTIVE));
    assert_eq!(b.step_1360().await, Some(4038));
    assert_eq!(b.receipt_rows().await, 0);
    assert!(
        slot10(&harset_b).contains(&LETTER_BIND_DSM),
        "B's letter bind stays"
    );

    let frames = deliver(&mut b, &tx_b, &mut rx_b, &mut harset_b).await;
    b.route(frames).await;
    assert_eq!(b.letter_count().await, 0);
    assert_eq!(b.receipt_rows().await, 1);
    assert_eq!(a.receipt_rows().await, 1, "A's receipt is its own");
    teardown(&pool, 18).await;
    teardown(&pool, 19).await;
}

/// Without chain 6501 the click does nothing. The control run with 6501 in the
/// same test proves the harness would have produced the request.
#[tokio::test]
async fn live_db_marsh_receipt_without_6501_does_nothing() {
    let pool = require_db_or_skip!();

    let control = setup(&pool, 20, &RECEIPT_CHAINS).await;
    let (tx, mut rx) = mpsc::channel(128);
    let mut harset = stage(&control, &tx, &mut rx, Stage::MARSH).await;
    let click = click_marsh(&control, &tx, &mut rx, &mut harset).await;
    assert_eq!(
        request_count(&click),
        1,
        "control: with 6501 the click queues the request"
    );
    teardown(&pool, 20).await;

    let without: Vec<i32> = RECEIPT_CHAINS.into_iter().filter(|&c| c != 6501).collect();
    let mut driver = setup(&pool, 21, &without).await;
    let (tx, mut rx) = mpsc::channel(128);
    let mut harset = stage(&driver, &tx, &mut rx, Stage::MARSH).await;
    let click = click_marsh(&driver, &tx, &mut rx, &mut harset).await;
    assert_eq!(request_count(&click), 0, "no Plugin without 6501");
    let methods = frame_methods(&click);
    assert!(
        !methods.iter().any(|m| MISSION_FRAMES.contains(m)) && !shows_receipt_dialog(&click),
        "no success feedback: {methods:?}"
    );
    driver.route(click).await;
    tokio::time::sleep(Duration::from_millis(100)).await;
    while let Ok(msg) = driver.base_rx.try_recv() {
        assert!(
            !matches!(msg, BaseToCellMsg::LetterTurnInResult(_)),
            "the base answered a turn-in nobody requested"
        );
    }
    assert!(problems_if_consumed(&driver).await.is_empty());
    teardown(&pool, 21).await;
}
