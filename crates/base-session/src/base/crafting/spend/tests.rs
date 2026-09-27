//! `spendAppliedSciencePoints`: the pure rule order, then every outcome
//! against the seeded database, asserting both what the player's client
//! receives and that a refusal writes nothing.

use std::collections::HashMap;
use std::sync::Arc;

use cimmeria_cell_catalog::crafting::{CraftingCatalog, Discipline};
use cimmeria_entity::crafting::CraftingState;
use cimmeria_mercury::encryption::EncryptionVersion;
use sqlx::PgPool;

use super::*;
use crate::base::crafting::feedback::{error_code_args, feedback_text_args};
use crate::base::crafting::persistence::load_crafting_state;
use crate::base::crafting::telemetry::METRIC_REQUESTS;
use crate::base::crafting::test_players::{cleanup, insert_player, OneSession, SESSION_ACCOUNT_ID};
use crate::mercury::{build_player_entity_method_packet, method_idx};
use crate::test_support::{require_db_or_skip, LogCapture, LogCaptureGuard};
use cimmeria_observability::testing::{counter_total, install as install_meter};

// ── Pure rule checks ──────────────────────────────────────────────────────

fn discipline(id: i32, paradigm: i32, level: i32, prereqs: &[i32], name: &str) -> Discipline {
    Discipline {
        discipline_id: id,
        applied_science_id: 2,
        racial_paradigm_id: paradigm,
        racial_paradigm_level: level,
        tech_competency: 5,
        required_discipline_ids: prereqs.to_vec(),
        name: name.to_string(),
    }
}

/// The seed's Materials tree in miniature: 78 (Common 5), 79 (Common 5,
/// needs 78), 82 (Human 3, needs 78).
fn catalog() -> CraftingCatalog {
    CraftingCatalog::from_rows(
        [
            discipline(78, 1, 5, &[], "Materials Engineering"),
            discipline(79, 1, 5, &[78], "Non-Reactive Coatings"),
            discipline(82, 2, 3, &[78], "Ceramic Composites"),
        ],
        [],
        [],
        [],
    )
}

fn fresh(asp: i32) -> CraftingState {
    let mut state = CraftingState::new();
    state.apply_default_paradigm_levels();
    state.applied_science_points = asp;
    state
}

fn knowing(asp: i32, discipline_id: i32, expertise: i32) -> CraftingState {
    let mut state = fresh(asp);
    state.discipline_ids.push(discipline_id);
    state.set_expertise(discipline_id, expertise);
    state
}

#[test]
fn a_fresh_character_with_asp_may_learn_a_root() {
    assert_eq!(check_spend(&fresh(1), &catalog(), 78), Ok(()));
}

#[test]
fn each_rule_refuses_with_its_reason() {
    let c = catalog();
    assert_eq!(
        check_spend(&fresh(1), &c, 9999),
        Err(CraftReject::UnknownDiscipline {
            discipline_id: 9999
        })
    );
    assert_eq!(
        check_spend(&knowing(1, 78, 1), &c, 78),
        Err(CraftReject::DisciplineAlreadyKnown {
            discipline_id: 78,
            name: "Materials Engineering".into()
        })
    );
    assert_eq!(
        check_spend(&fresh(0), &c, 78),
        Err(CraftReject::NoAppliedSciencePoints { asp: 0 })
    );
    assert_eq!(
        check_spend(&knowing(1, 78, 100), &c, 82),
        Err(CraftReject::ParadigmTooLow {
            discipline_id: 82,
            discipline: "Ceramic Composites".into(),
            paradigm_id: 2,
            paradigm: "Human",
            required: 3,
            have: 1,
        })
    );
    assert_eq!(
        check_spend(&fresh(1), &c, 79),
        Err(CraftReject::PrerequisiteMissing {
            discipline_id: 79,
            discipline: "Non-Reactive Coatings".into(),
            prerequisite_id: 78,
            prerequisite: "Materials Engineering".into(),
        })
    );
    assert_eq!(
        check_spend(&knowing(1, 78, 49), &c, 79),
        Err(CraftReject::PrerequisiteExpertise {
            discipline_id: 79,
            discipline: "Non-Reactive Coatings".into(),
            prerequisite_id: 78,
            prerequisite: "Materials Engineering".into(),
            expertise: 49,
            required: 50,
        })
    );
}

/// A prerequisite counts at expertise 50, not 49.
#[test]
fn prerequisite_needs_expertise_fifty() {
    let c = catalog();
    assert!(matches!(
        check_spend(&knowing(1, 78, 49), &c, 79),
        Err(CraftReject::PrerequisiteExpertise { .. })
    ));
    assert_eq!(check_spend(&knowing(1, 78, 50), &c, 79), Ok(()));
}

/// "Already known" outranks "no ASP", so a replay after the last point was
/// spent still says the discipline is known.
#[test]
fn already_known_is_checked_before_asp() {
    assert!(matches!(
        check_spend(&knowing(0, 78, 1), &catalog(), 78),
        Err(CraftReject::DisciplineAlreadyKnown { .. })
    ));
}

#[test]
fn refusal_texts_name_what_is_missing() {
    let c = catalog();
    let text = |state: &CraftingState, id| check_spend(state, &c, id).unwrap_err().text();
    assert_eq!(text(&fresh(0), 78), "You have no applied science points.");
    assert_eq!(
        text(&knowing(1, 78, 1), 78),
        "You already know Materials Engineering."
    );
    assert_eq!(
        text(&knowing(1, 78, 100), 82),
        "Ceramic Composites requires Human paradigm level 3; yours is 1."
    );
    assert_eq!(
        text(&fresh(1), 79),
        "Non-Reactive Coatings requires Materials Engineering at expertise 50."
    );
    assert_eq!(
        text(&knowing(1, 78, 49), 79),
        "Non-Reactive Coatings requires Materials Engineering at expertise 50; yours is 49."
    );
    assert_eq!(text(&fresh(1), 9999), "There is no discipline 9999.");
}

// ── Live DB ───────────────────────────────────────────────────────────────

/// Sentinels in the crafting `0x7000_Cxxx` block: `0x7000_CE00..0x7000_CE1F`
/// (`sync` uses `0x7000_CD00..`).
const TEST_BASE: i32 = 0x7000_CE00;
const ENTITY: u32 = 4290;

/// Insert a player with `asp` points and the given known disciplines.
async fn player(pool: &PgPool, n: i32, asp: i32, known: &[(i32, i32)]) -> (i32, i32) {
    let (account_id, player_id) = (TEST_BASE + 2 * n, TEST_BASE + 2 * n + 1);
    cleanup(pool, account_id, player_id).await;
    insert_player(pool, account_id, player_id).await;
    let ids: Vec<i32> = known.iter().map(|&(id, _)| id).collect();
    sqlx::query(
        "UPDATE sgw_player SET applied_science_points = $2, discipline_ids = $3 \
         WHERE player_id = $1",
    )
    .bind(player_id)
    .bind(asp)
    .bind(&ids)
    .execute(pool)
    .await
    .expect("seed crafting columns");
    for &(id, expertise) in known {
        sqlx::query(
            "INSERT INTO sgw_player_discipline_expertise (player_id, discipline_id, expertise) \
             VALUES ($1, $2, $3)",
        )
        .bind(player_id)
        .bind(id)
        .bind(expertise)
        .execute(pool)
        .await
        .expect("seed expertise");
    }
    (account_id, player_id)
}

/// Run one spend through the real handler with a live pool.
async fn spend(pool: &PgPool, session: &OneSession, player_id: i32, discipline_id: i32) {
    let db_pool = Some(Arc::new(pool.clone()));
    let ctx = CraftCtx {
        db_pool: &db_pool,
        cell_tx: &None,
        transport: &session.transport,
        connected: &session.connected,
        entity_to_addr: &session.entity_to_addr,
    };
    handle_spend(ENTITY, player_id, discipline_id, &ctx).await;
}

fn packet(seq: u32, method: u16, args: &[u8]) -> Vec<u8> {
    build_player_entity_method_packet(
        &[0u8; 32],
        seq,
        &[],
        ENTITY,
        method,
        args,
        EncryptionVersion::V1,
    )
}

/// The one text line a refusal sends, at sequence `seq`.
fn refusal(seq: u32, text: &str) -> Vec<u8> {
    packet(
        seq,
        method_idx::ON_PLAYER_COMMUNICATION,
        &feedback_text_args(text),
    )
}

/// The crafting columns a refusal must leave alone.
async fn snapshot(pool: &PgPool, player_id: i32) -> (Vec<i32>, i32, Vec<(i32, i32)>) {
    let state = load_crafting_state(pool, player_id).await.expect("load");
    let mut expertise: Vec<(i32, i32)> = state.expertise.into_iter().collect();
    expertise.sort_unstable();
    (
        state.discipline_ids,
        state.applied_science_points,
        expertise,
    )
}

/// The `rejected` event for `player_id`: the reason, the full identity on
/// the event, and the compared values the rule reported.
fn assert_rejected(
    capture: &LogCaptureGuard,
    player_id: i32,
    reason: &str,
    fields: &[(&str, &str)],
) {
    let event = capture
        .all()
        .into_iter()
        .find(|c| {
            c.target == "crafting"
                && c.has_field("event", "rejected")
                && c.has_field("player_id", &player_id.to_string())
        })
        .unwrap_or_else(|| panic!("no rejected event for {player_id}"));
    let identity = [
        ("reason", reason.to_string()),
        ("verb", "spendAppliedSciencePoints".to_string()),
        ("account_id", SESSION_ACCOUNT_ID.to_string()),
        ("entity_id", ENTITY.to_string()),
    ];
    for (k, v) in identity
        .iter()
        .map(|(k, v)| (*k, v.as_str()))
        .chain(fields.iter().copied())
    {
        assert!(event.has_field(k, v), "{k}={v}: {event:#?}");
    }
}

/// Success: 78 is learned at expertise 1, one ASP is spent, no blueprint is
/// granted, and the client gets 136 then the new ASP total.
#[tokio::test]
async fn spend_learns_the_discipline_and_pushes_it() {
    let pool = require_db_or_skip!();
    install_meter();
    let capture = LogCapture::install();
    let (account_id, player_id) = player(&pool, 0, 2, &[]).await;
    let session = OneSession::new(ENTITY, 55750);
    let completed = [
        ("verb", "spendAppliedSciencePoints"),
        ("outcome", "completed"),
    ];
    let completed_before = counter_total(METRIC_REQUESTS, &completed);

    spend(&pool, &session, player_id, 78).await;

    let learned = capture
        .all()
        .into_iter()
        .find(|c| c.target == "crafting" && c.has_field("event", "learned"))
        .expect("learned event");
    for (k, v) in [
        ("account_id", SESSION_ACCOUNT_ID.to_string()),
        ("player_id", player_id.to_string()),
        ("entity_id", ENTITY.to_string()),
        ("discipline_id", "78".to_string()),
        ("expertise_after", "1".to_string()),
        ("asp_before", "2".to_string()),
        ("asp_after", "1".to_string()),
    ] {
        assert!(learned.has_field(k, &v), "{k}={v}: {learned:#?}");
    }
    assert!(
        counter_total(METRIC_REQUESTS, &completed) > completed_before,
        "crafting_requests_total{{outcome=completed}} counted"
    );

    let state = load_crafting_state(&pool, player_id).await.expect("load");
    let sent = session.typed.filter_to(session.addr);
    cleanup(&pool, account_id, player_id).await;
    assert_eq!(state.discipline_ids, vec![78]);
    assert_eq!(state.get_expertise(78), Some(1));
    assert_eq!(state.applied_science_points, 1);
    assert!(
        state.blueprint_ids.is_empty(),
        "learning grants no blueprints"
    );
    assert_eq!(
        sent,
        vec![
            packet(
                0,
                method_idx::ON_UPDATE_DISCIPLINE,
                &[78, 0, 0, 0, 1, 0, 0, 0]
            ),
            packet(1, method_idx::ON_ENTITY_PROPERTY, &[2, 0, 0, 0, 1, 0, 0, 0]),
        ]
    );
}

/// Replay (CAT-F F-02): the same request twice learns once and spends
/// once. The second is refused as already known and writes nothing.
#[tokio::test]
async fn replayed_spend_changes_nothing() {
    let pool = require_db_or_skip!();
    let (account_id, player_id) = player(&pool, 1, 2, &[]).await;
    let session = OneSession::new(ENTITY, 55751);

    spend(&pool, &session, player_id, 78).await;
    let after_first = snapshot(&pool, player_id).await;
    spend(&pool, &session, player_id, 78).await;
    let after_second = snapshot(&pool, player_id).await;
    let sent = session.typed.filter_to(session.addr);
    cleanup(&pool, account_id, player_id).await;

    assert_eq!(after_first, (vec![78], 1, vec![(78, 1)]));
    assert_eq!(after_second, after_first, "the replay wrote nothing");
    assert_eq!(sent.len(), 3, "136 + ASP, then one refusal line");
    assert_eq!(
        sent[2],
        refusal(2, "You already know Materials Engineering.")
    );
}

/// No ASP: the text line, then `onErrorCode(0, 0, 214)`, and nothing
/// written.
#[tokio::test]
async fn spend_without_asp_is_refused_with_code_214() {
    let pool = require_db_or_skip!();
    let capture = LogCapture::install();
    let (account_id, player_id) = player(&pool, 2, 0, &[]).await;
    let session = OneSession::new(ENTITY, 55752);
    let before = snapshot(&pool, player_id).await;

    spend(&pool, &session, player_id, 78).await;

    let after = snapshot(&pool, player_id).await;
    let sent = session.typed.filter_to(session.addr);
    cleanup(&pool, account_id, player_id).await;
    assert_eq!(after, before);
    assert_eq!(
        sent,
        vec![
            refusal(0, "You have no applied science points."),
            packet(1, method_idx::ON_ERROR_CODE, &error_code_args(214)),
        ]
    );
    assert_rejected(&capture, player_id, "no_asp", &[("asp", "0")]);
}

/// Already known (not a replay: the character learned it earlier).
#[tokio::test]
async fn spend_on_a_known_discipline_is_refused() {
    let pool = require_db_or_skip!();
    let capture = LogCapture::install();
    let (account_id, player_id) = player(&pool, 3, 3, &[(78, 40)]).await;
    let session = OneSession::new(ENTITY, 55753);
    let before = snapshot(&pool, player_id).await;

    spend(&pool, &session, player_id, 78).await;

    let after = snapshot(&pool, player_id).await;
    let sent = session.typed.filter_to(session.addr);
    cleanup(&pool, account_id, player_id).await;
    assert_eq!(after, before);
    assert_eq!(
        sent,
        vec![refusal(0, "You already know Materials Engineering.")]
    );
    assert_rejected(
        &capture,
        player_id,
        "already_known",
        &[("discipline_id", "78")],
    );
}

/// Paradigm: Ceramic Composites (82) needs Human 3; the default is 1.
#[tokio::test]
async fn spend_below_the_paradigm_level_is_refused() {
    let pool = require_db_or_skip!();
    let capture = LogCapture::install();
    let (account_id, player_id) = player(&pool, 4, 3, &[(78, 100)]).await;
    let session = OneSession::new(ENTITY, 55754);
    let before = snapshot(&pool, player_id).await;

    spend(&pool, &session, player_id, 82).await;

    let after = snapshot(&pool, player_id).await;
    let sent = session.typed.filter_to(session.addr);
    cleanup(&pool, account_id, player_id).await;
    assert_eq!(after, before);
    assert_eq!(
        sent,
        vec![refusal(
            0,
            "Ceramic Composites requires Human paradigm level 3; yours is 1."
        )]
    );
    assert_rejected(
        &capture,
        player_id,
        "paradigm_too_low",
        &[
            ("paradigm_id", "2"),
            ("paradigm_level", "1"),
            ("required_level", "3"),
        ],
    );
}

/// The prerequisite guard: Non-Reactive Coatings (79) needs Materials
/// Engineering (78) at expertise 50. Unknown, and known at 49, are both
/// refused with nothing written; at 50 it is learned. Removing the
/// prerequisite loop from `check_spend` makes the first two learn and fails
/// this test.
#[tokio::test]
async fn spend_without_the_prerequisite_at_fifty_is_refused() {
    let pool = require_db_or_skip!();
    let capture = LogCapture::install();
    let cases = [
        (
            5,
            vec![],
            55755,
            "Non-Reactive Coatings requires Materials Engineering at expertise 50.",
            "prerequisite_missing",
            vec![("prerequisite_id", "78")],
        ),
        (
            6,
            vec![(78, 49)],
            55756,
            "Non-Reactive Coatings requires Materials Engineering at expertise 50; yours is 49.",
            "prerequisite_expertise",
            vec![
                ("prerequisite_id", "78"),
                ("prerequisite_expertise", "49"),
                ("required_expertise", "50"),
            ],
        ),
    ];
    for (n, known, port, text, reason, fields) in cases {
        let (account_id, player_id) = player(&pool, n, 3, &known).await;
        let session = OneSession::new(ENTITY, port);
        let before = snapshot(&pool, player_id).await;

        spend(&pool, &session, player_id, 79).await;

        let after = snapshot(&pool, player_id).await;
        let sent = session.typed.filter_to(session.addr);
        cleanup(&pool, account_id, player_id).await;
        assert_eq!(after, before, "prerequisite {known:?}: nothing written");
        assert_eq!(sent, vec![refusal(0, text)], "prerequisite {known:?}");
        assert_rejected(&capture, player_id, reason, &fields);
    }

    let (account_id, player_id) = player(&pool, 7, 3, &[(78, 50)]).await;
    let session = OneSession::new(ENTITY, 55757);
    spend(&pool, &session, player_id, 79).await;
    let after = snapshot(&pool, player_id).await;
    cleanup(&pool, account_id, player_id).await;
    assert_eq!(after, (vec![78, 79], 2, vec![(78, 50), (79, 1)]));
}

/// Concurrency guard: the spend reads under `FOR UPDATE`. Another
/// transaction holds the player's row, the spend starts and waits, the
/// holder spends the last point and commits; the spend must then see 0 ASP
/// and refuse. Reading without the lock, it would check a stale 1 ASP,
/// learn 78 and leave the total at -1.
#[tokio::test]
async fn spend_rechecks_after_waiting_for_the_row_lock() {
    let pool = require_db_or_skip!();
    let (account_id, player_id) = player(&pool, 9, 1, &[]).await;
    let catalog = cimmeria_cell_catalog::crafting::shared_crafting_catalog(&pool)
        .await
        .expect("catalog");

    let mut holder = pool.begin().await.expect("begin");
    sqlx::query("SELECT 1 FROM sgw_player WHERE player_id = $1 FOR UPDATE")
        .bind(player_id)
        .execute(&mut *holder)
        .await
        .expect("lock the row");
    let (task_pool, task_catalog) = (pool.clone(), catalog.clone());
    let task =
        tokio::spawn(async move { spend_in_db(&task_pool, &task_catalog, player_id, 78).await });
    // Let the spend reach the row lock before the holder commits.
    tokio::time::sleep(std::time::Duration::from_millis(300)).await;
    assert!(!task.is_finished(), "the spend waits for the row lock");
    sqlx::query("UPDATE sgw_player SET applied_science_points = 0 WHERE player_id = $1")
        .bind(player_id)
        .execute(&mut *holder)
        .await
        .expect("spend the point elsewhere");
    holder.commit().await.expect("commit");

    let outcome = task.await.expect("join").expect("no database error");
    let after = snapshot(&pool, player_id).await;
    cleanup(&pool, account_id, player_id).await;
    assert_eq!(outcome, Err(CraftReject::NoAppliedSciencePoints { asp: 0 }));
    assert_eq!(after, (vec![], 0, vec![]));
}

/// A discipline the catalog does not have.
#[tokio::test]
async fn spend_on_an_unknown_discipline_is_refused() {
    let pool = require_db_or_skip!();
    let capture = LogCapture::install();
    let (account_id, player_id) = player(&pool, 8, 3, &[]).await;
    let session = OneSession::new(ENTITY, 55758);
    let before = snapshot(&pool, player_id).await;

    spend(&pool, &session, player_id, 9999).await;

    let after = snapshot(&pool, player_id).await;
    let sent = session.typed.filter_to(session.addr);
    cleanup(&pool, account_id, player_id).await;
    assert_eq!(after, before);
    assert_eq!(sent, vec![refusal(0, "There is no discipline 9999.")]);
    assert_rejected(
        &capture,
        player_id,
        "unknown_discipline",
        &[("discipline_id", "9999")],
    );
}

/// A player row that is gone under a live session: the locked read finds
/// nothing, which is a `persist_failed` WARN naming the phase with the
/// paired `rows_affected = 0` / `expected = 1`, and the player still gets
/// the "unavailable" line.
#[tokio::test]
async fn spend_for_a_missing_player_row_warns_with_rows_affected() {
    let pool = require_db_or_skip!();
    let capture = LogCapture::install();
    // Inside the spend block, never inserted.
    let missing_player = TEST_BASE + 0x1F;
    let session = OneSession::new(ENTITY, 55761);

    spend(&pool, &session, missing_player, 78).await;

    let warn = capture
        .find_event(
            tracing::Level::WARN,
            "fewer rows than it had to",
            "rows_affected_short",
        )
        .expect("persist_failed WARN");
    for (k, v) in [
        ("event", "persist_failed"),
        ("phase", "lock_player"),
        ("rows_affected", "0"),
        ("expected", "1"),
        ("account_id", &SESSION_ACCOUNT_ID.to_string()),
        ("player_id", &missing_player.to_string()),
        ("entity_id", &ENTITY.to_string()),
    ] {
        assert!(warn.has_field(k, v), "{k}={v}: {warn:#?}");
    }
    assert_eq!(
        session.typed.filter_to(session.addr),
        vec![refusal(
            0,
            "Learning disciplines is unavailable right now. Nothing was changed."
        )]
    );
}

/// Without a database the request is refused visibly, not dropped.
#[tokio::test]
async fn spend_without_a_database_is_refused_visibly() {
    let session = OneSession::new(ENTITY, 55759);
    let ctx = CraftCtx {
        db_pool: &None,
        cell_tx: &None,
        transport: &session.transport,
        connected: &session.connected,
        entity_to_addr: &session.entity_to_addr,
    };
    handle_spend(ENTITY, 1, 78, &ctx).await;
    assert_eq!(
        session.typed.filter_to(session.addr),
        vec![refusal(
            0,
            "Learning disciplines is unavailable right now. Nothing was changed."
        )]
    );
}

/// The request entry point routes `Spend` here, not to the "not available
/// yet" line the other verbs still get.
#[tokio::test]
async fn craft_request_routes_spend_to_the_spend_handler() {
    use crate::base::crafting::request::handle_craft_request;
    use crate::cell::messages::{CraftRequest, CraftVerb};

    let session = OneSession::new(ENTITY, 55760);
    let ctx = CraftCtx {
        db_pool: &None,
        cell_tx: &None,
        transport: &session.transport,
        connected: &session.connected,
        entity_to_addr: &session.entity_to_addr,
    };
    handle_craft_request(
        CraftRequest {
            entity_id: ENTITY,
            player_id: 1,
            verb: CraftVerb::Spend { discipline_id: 78 },
            allowed: 0,
        },
        &ctx,
    )
    .await;
    assert_eq!(
        session.typed.filter_to(session.addr),
        vec![refusal(
            0,
            "Learning disciplines is unavailable right now. Nothing was changed."
        )]
    );
}

/// The paradigm-level map the defaults build is keyed like the catalog's
/// `racial_paradigm_id` (1-5), which `check_spend` indexes by.
#[test]
fn defaults_are_keyed_by_paradigm_id() {
    let state = fresh(0);
    let keys: HashMap<i32, i8> = state.racial_paradigm_levels;
    for id in 1..=5 {
        assert!(keys.contains_key(&id), "paradigm {id}");
    }
}
