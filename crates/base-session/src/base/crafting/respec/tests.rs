//! Crafting respec: the open-respec rules, then both steps against the
//! seeded database, asserting what the client receives, what is written
//! and what is kept, and that a lone or replayed `respecCrafting` wipes
//! nothing.

use std::sync::Arc;
use std::time::{Duration, Instant};

use cimmeria_mercury::encryption::EncryptionVersion;
use cimmeria_wire::cell::client_methods::player::{
    ON_CRAFTING_RESPEC_PROMPT, ON_DISCIPLINE_RESPEC, ON_UPDATE_KNOWN_CRAFTS,
};
use sqlx::PgPool;

use super::*;
use crate::base::crafting::feedback::feedback_text_args;
use crate::base::crafting::persistence::load_crafting_state;
use crate::base::crafting::request::handle_craft_request;
use crate::base::crafting::telemetry::{METRIC_REJECTIONS, METRIC_REQUESTS};
use crate::base::crafting::test_players::{cleanup, insert_player, OneSession, SESSION_ACCOUNT_ID};
use crate::cell::messages::{CraftRequest, CraftVerb};
use crate::mercury::{build_player_entity_method_packet, method_idx};
use crate::test_support::{require_db_or_skip, Captured, LogCapture, LogCaptureGuard};
use cimmeria_observability::testing::{counter_total, install as install_meter};

pub(super) const ENTITY: u32 = 4310;

// ── The open respec ───────────────────────────────────────────────────────

#[test]
fn an_open_respec_is_taken_once() {
    let now = Instant::now();
    let mut slot = Some(PendingRespec::open(7, now));
    assert_eq!(
        take(&mut slot, 7, now + Duration::from_secs(1)),
        Taken::Open
    );
    assert_eq!(slot, None, "taking clears the slot");
    assert_eq!(take(&mut slot, 7, now), Taken::Nothing);
}

#[test]
fn the_window_is_sixty_seconds() {
    let now = Instant::now();
    let open = PendingRespec::open(7, now);
    assert_eq!(RESPEC_WINDOW, Duration::from_secs(60));
    assert_eq!(take(&mut Some(open), 7, now + RESPEC_WINDOW), Taken::Open);
    assert_eq!(
        take(
            &mut Some(open),
            7,
            now + RESPEC_WINDOW + Duration::from_millis(1)
        ),
        Taken::Expired
    );
}

/// A respec opened by one character is not confirmed by another on the
/// same connection, and is discarded.
#[test]
fn another_characters_confirmation_does_not_match() {
    let now = Instant::now();
    let mut slot = Some(PendingRespec::open(7, now));
    assert_eq!(take(&mut slot, 8, now), Taken::Nothing);
    assert_eq!(slot, None);
}

#[test]
fn respec_refusals_read_and_label_as_documented() {
    let cases = [
        (
            CraftReject::NothingToRespec,
            "nothing_to_respec",
            "You have no crafting disciplines to unlearn. Nothing was changed.",
        ),
        (
            CraftReject::NoPendingRespec,
            "no_pending_respec",
            "No crafting respec is waiting to be confirmed. Type .respeccraft to start one.",
        ),
        (
            CraftReject::RespecExpired { window_secs: 60 },
            "respec_expired",
            "The crafting respec was not confirmed within 60 seconds. \
             Type .respeccraft to start again.",
        ),
    ];
    for (why, reason, text) in cases {
        assert_eq!(why.reason(), reason);
        assert_eq!(why.text(), text);
        assert_eq!(why.error_code(), None, "{why:?}");
    }
}

#[test]
fn cleared_disciplines_format_as_before_to_zero() {
    assert_eq!(format_cleared(&[(78, 60), (79, 0)]), "78:60→0,79:0→0");
    assert_eq!(format_paradigms(&[(1, 5), (2, 3)]), "1:5,2:3");
}

// ── Without a database ────────────────────────────────────────────────────

pub(super) fn ctx<'a>(session: &'a OneSession, db_pool: &'a Option<Arc<PgPool>>) -> CraftCtx<'a> {
    CraftCtx {
        db_pool,
        cell_tx: &None,
        transport: &session.transport,
        connected: &session.connected,
        entity_to_addr: &session.entity_to_addr,
    }
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

fn refusal(seq: u32, text: &str) -> Vec<u8> {
    packet(
        seq,
        method_idx::ON_PLAYER_COMMUNICATION,
        &feedback_text_args(text),
    )
}

fn event<'a>(capture: &'a [Captured], name: &str, player_id: i32) -> &'a Captured {
    capture
        .iter()
        .find(|c| {
            c.target == "crafting"
                && c.has_field("event", name)
                && c.has_field("player_id", &player_id.to_string())
        })
        .unwrap_or_else(|| panic!("no {name} event for {player_id}"))
}

fn assert_fields(event: &Captured, fields: &[(&str, String)]) {
    for (k, v) in fields {
        assert!(event.has_field(k, v), "{k}={v}: {event:#?}");
    }
}

fn identity(player_id: i32) -> Vec<(&'static str, String)> {
    vec![
        ("account_id", SESSION_ACCOUNT_ID.to_string()),
        ("player_id", player_id.to_string()),
        ("entity_id", ENTITY.to_string()),
    ]
}

/// The routing guard: a `respecCrafting` through the base's crafting entry
/// point reaches the respec handler, which refuses it for want of an open
/// respec, not the "not available yet" fallback.
#[tokio::test]
async fn craft_request_routes_respec_to_the_respec_handler() {
    let capture = LogCapture::install();
    let session = OneSession::new(ENTITY, 55800);
    let request = CraftRequest {
        entity_id: ENTITY,
        player_id: 17,
        verb: CraftVerb::Respec,
        allowed: 0,
    };
    handle_craft_request(request, &ctx(&session, &None)).await;

    assert_eq!(
        session.typed.filter_to(session.addr),
        vec![refusal(0, &CraftReject::NoPendingRespec.text())]
    );
    let rejected = event(&capture.all(), "rejected", 17).clone();
    assert_fields(&rejected, &[("reason", "no_pending_respec".into())]);
}

/// With no database, `.respeccraft` is refused visibly with a
/// `persist_failed` WARN, and opens nothing.
#[tokio::test]
async fn respec_open_without_a_database_is_refused_visibly() {
    let capture = LogCapture::install();
    let session = OneSession::new(ENTITY, 55801);
    let open = RespecCraftOpen {
        entity_id: ENTITY,
        player_id: 18,
    };
    handle_respec_open(open, &ctx(&session, &None)).await;

    assert_eq!(
        session.typed.filter_to(session.addr),
        vec![refusal(
            0,
            "Crafting respec is unavailable right now. Nothing was changed."
        )]
    );
    let all = capture.all();
    let warn = event(&all, "persist_failed", 18);
    assert_eq!(warn.level, tracing::Level::WARN);
    let mut expected = identity(18);
    expected.extend([("phase", "no_pool".into()), ("verb", "respeccraft".into())]);
    assert_fields(warn, &expected);
    assert_fields(
        event(&all, "rejected", 18),
        &[("reason", "unavailable".into())],
    );
    let pending = with_session(ENTITY, &session.connected, &session.entity_to_addr, |s| {
        s.pending_respec
    });
    assert_eq!(pending, Some(None), "nothing opened");
}

// ── Live DB ───────────────────────────────────────────────────────────────

/// Sentinels in the crafting `0x7000_Cxxx` block: `0x7000_CFD0..0x7000_CFEF`
/// (the transaction tests hold `0x7000_CF40..0x7000_CF9F`).
pub(super) const TEST_BASE: i32 = 0x7000_CFD0;

/// A player with `asp` points, the given known disciplines and expertise
/// (each bought with one point), blueprints and stored paradigm levels.
pub(super) async fn player(
    pool: &PgPool,
    n: i32,
    asp: i32,
    known: &[(i32, i32)],
    blueprints: &[i32],
    paradigms: &[i32],
) -> (i32, i32) {
    let (account_id, player_id) = (TEST_BASE + 2 * n, TEST_BASE + 2 * n + 1);
    cleanup(pool, account_id, player_id).await;
    insert_player(pool, account_id, player_id).await;
    let ids: Vec<i32> = known.iter().map(|&(id, _)| id).collect();
    // Every known discipline counts as bought with one point, as if learned
    // through the trainer.
    sqlx::query(
        "UPDATE sgw_player SET applied_science_points = $2, discipline_ids = $3, \
         blueprint_ids = $4, racial_paradigm_levels = $5, \
         applied_science_points_spent = cardinality($3) WHERE player_id = $1",
    )
    .bind(player_id)
    .bind(asp)
    .bind(&ids)
    .bind(blueprints)
    .bind(paradigms)
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

/// Everything a respec may touch: known disciplines, ASP, expertise rows,
/// blueprints and the paradigm levels as loaded.
pub(super) type Snapshot = (Vec<i32>, i32, Vec<(i32, i32)>, Vec<i32>, Vec<(i32, i8)>);

pub(super) async fn snapshot(pool: &PgPool, player_id: i32) -> Snapshot {
    let state = load_crafting_state(pool, player_id).await.expect("load");
    let mut expertise: Vec<(i32, i32)> = state.expertise.into_iter().collect();
    expertise.sort_unstable();
    let mut paradigms: Vec<(i32, i8)> = state.racial_paradigm_levels.into_iter().collect();
    paradigms.sort_unstable();
    (
        state.discipline_ids,
        state.applied_science_points,
        expertise,
        state.blueprint_ids,
        paradigms,
    )
}

pub(super) async fn open(pool: &PgPool, session: &OneSession, player_id: i32) {
    let db_pool = Some(Arc::new(pool.clone()));
    let msg = RespecCraftOpen {
        entity_id: ENTITY,
        player_id,
    };
    handle_respec_open(msg, &ctx(session, &db_pool)).await;
}

pub(super) async fn confirm(pool: &PgPool, session: &OneSession, player_id: i32) {
    let db_pool = Some(Arc::new(pool.clone()));
    handle_respec_confirm(ENTITY, player_id, &ctx(session, &db_pool)).await;
}

fn prompt(seq: u32) -> Vec<u8> {
    packet(seq, ON_CRAFTING_RESPEC_PROMPT, &[0, 0, 0, 0])
}

fn rejected(capture: &LogCaptureGuard, player_id: i32, verb: &str, reason: &str) {
    let all = capture.all();
    let event = event(&all, "rejected", player_id);
    let mut expected = identity(player_id);
    expected.extend([("verb", verb.to_string()), ("reason", reason.to_string())]);
    assert_fields(event, &expected);
}

/// The whole respec: the prompt costs 0; the confirmation clears both
/// disciplines and their expertise, refunds two points, keeps blueprints
/// 25 and 42 and the stored paradigm levels, and the client gets 112, then
/// 137, 139 with the kept blueprints, and the new ASP total.
#[tokio::test]
async fn live_db_respec_refunds_clears_and_keeps_blueprints_and_paradigms() {
    let pool = require_db_or_skip!();
    install_meter();
    let capture = LogCapture::install();
    let (account_id, player_id) = player(
        &pool,
        0,
        3,
        &[(78, 60), (79, 12)],
        &[25, 42],
        &[5, 3, 1, 2, 1],
    )
    .await;
    let session = OneSession::new(ENTITY, 55802);
    let accepted = [("verb", "respecCrafting"), ("outcome", "accepted")];
    let accepted_before = counter_total(METRIC_REQUESTS, &accepted);
    let prompted = [("verb", "respeccraft"), ("outcome", "accepted")];
    let prompted_before = counter_total(METRIC_REQUESTS, &prompted);

    open(&pool, &session, player_id).await;
    let after_open = snapshot(&pool, player_id).await;
    confirm(&pool, &session, player_id).await;
    let after = snapshot(&pool, player_id).await;
    let sent = session.typed.filter_to(session.addr);
    cleanup(&pool, account_id, player_id).await;

    let paradigms = vec![(1, 5), (2, 3), (3, 1), (4, 2), (5, 1)];
    assert_eq!(
        after_open,
        (
            vec![78, 79],
            3,
            vec![(78, 60), (79, 12)],
            vec![25, 42],
            paradigms.clone()
        ),
        "the prompt alone writes nothing"
    );
    assert_eq!(after, (vec![], 5, vec![], vec![25, 42], paradigms));
    assert_eq!(
        sent,
        vec![
            prompt(0),
            packet(1, ON_DISCIPLINE_RESPEC, &[]),
            packet(
                2,
                ON_UPDATE_KNOWN_CRAFTS,
                &[2, 0, 0, 0, 25, 0, 0, 0, 42, 0, 0, 0]
            ),
            packet(3, method_idx::ON_ENTITY_PROPERTY, &[2, 0, 0, 0, 5, 0, 0, 0]),
        ]
    );

    let all = capture.all();
    let mut expected = identity(player_id);
    expected.extend([
        ("verb", "respeccraft".into()),
        ("cost", "0".into()),
        ("disciplines", "2".into()),
        ("asp", "3".into()),
        ("window_secs", "60".into()),
        ("replaced", "false".into()),
    ]);
    assert_fields(event(&all, "respec_prompted", player_id), &expected);
    let mut expected = identity(player_id);
    expected.extend([
        ("verb", "respecCrafting".into()),
        ("cleared", "78:60→0,79:12→0".into()),
        ("disciplines_cleared", "2".into()),
        ("expertise_rows_deleted", "2".into()),
        ("asp_before", "3".into()),
        ("asp_after", "5".into()),
        ("blueprints_kept", "2".into()),
        ("paradigm_levels_kept", "1:5,2:3,3:1,4:2,5:1".into()),
    ]);
    assert_fields(event(&all, "respec", player_id), &expected);
    assert!(counter_total(METRIC_REQUESTS, &accepted) > accepted_before);
    assert!(counter_total(METRIC_REQUESTS, &prompted) > prompted_before);
}

/// The guard: `respecCrafting` on its own, as a forged packet or a second
/// client would send it, wipes nothing. The player gets the "nothing
/// waiting" line and the state is untouched.
#[tokio::test]
async fn live_db_a_single_respec_crafting_never_wipes() {
    let pool = require_db_or_skip!();
    install_meter();
    let capture = LogCapture::install();
    let (account_id, player_id) = player(&pool, 1, 1, &[(78, 60)], &[25], &[5, 1, 1, 1, 1]).await;
    let session = OneSession::new(ENTITY, 55803);
    let before = snapshot(&pool, player_id).await;
    let labels = [("verb", "respecCrafting"), ("reason", "no_pending_respec")];
    let rejections_before = counter_total(METRIC_REJECTIONS, &labels);

    confirm(&pool, &session, player_id).await;

    let after = snapshot(&pool, player_id).await;
    let sent = session.typed.filter_to(session.addr);
    cleanup(&pool, account_id, player_id).await;
    assert_eq!(after, before, "nothing written");
    assert_eq!(
        sent,
        vec![refusal(
            0,
            "No crafting respec is waiting to be confirmed. Type .respeccraft to start one."
        )]
    );
    rejected(&capture, player_id, "respecCrafting", "no_pending_respec");
    assert!(counter_total(METRIC_REJECTIONS, &labels) > rejections_before);
}

/// Replay: a second `respecCrafting` after a done respec is refused and
/// refunds nothing more.
#[tokio::test]
async fn live_db_replayed_respec_crafting_changes_nothing() {
    let pool = require_db_or_skip!();
    let (account_id, player_id) = player(&pool, 2, 0, &[(78, 60)], &[], &[5, 1, 1, 1, 1]).await;
    let session = OneSession::new(ENTITY, 55804);

    open(&pool, &session, player_id).await;
    confirm(&pool, &session, player_id).await;
    let after_first = snapshot(&pool, player_id).await;
    confirm(&pool, &session, player_id).await;
    let after_second = snapshot(&pool, player_id).await;
    let sent = session.typed.filter_to(session.addr);
    cleanup(&pool, account_id, player_id).await;

    assert_eq!(after_first.0, Vec::<i32>::new());
    assert_eq!(after_first.1, 1, "one point refunded");
    assert_eq!(after_second, after_first, "the replay wrote nothing");
    assert_eq!(sent.len(), 5, "112, 137, 139, ASP, then one refusal");
    assert_eq!(
        sent[4],
        refusal(
            4,
            "No crafting respec is waiting to be confirmed. Type .respeccraft to start one."
        )
    );
}

/// A confirmation after the window is refused as expired and writes
/// nothing.
#[tokio::test]
async fn live_db_an_expired_respec_is_refused() {
    let pool = require_db_or_skip!();
    let capture = LogCapture::install();
    let (account_id, player_id) = player(&pool, 3, 2, &[(78, 60)], &[], &[5, 1, 1, 1, 1]).await;
    let session = OneSession::new(ENTITY, 55805);
    let before = snapshot(&pool, player_id).await;
    let opened_long_ago = Instant::now()
        .checked_sub(RESPEC_WINDOW + Duration::from_secs(1))
        .expect("an instant a minute ago");
    with_session(ENTITY, &session.connected, &session.entity_to_addr, |s| {
        s.pending_respec = Some(PendingRespec::open(player_id, opened_long_ago));
    })
    .expect("session");

    confirm(&pool, &session, player_id).await;

    let after = snapshot(&pool, player_id).await;
    let sent = session.typed.filter_to(session.addr);
    cleanup(&pool, account_id, player_id).await;
    assert_eq!(after, before);
    assert_eq!(
        sent,
        vec![refusal(
            0,
            "The crafting respec was not confirmed within 60 seconds. \
             Type .respeccraft to start again."
        )]
    );
    rejected(&capture, player_id, "respecCrafting", "respec_expired");
}

/// Nothing learned: `.respeccraft` is refused with a line, sends no prompt
/// and opens nothing, so a following Yes is refused too.
#[tokio::test]
async fn live_db_respec_with_nothing_learned_is_refused_at_the_prompt() {
    let pool = require_db_or_skip!();
    let capture = LogCapture::install();
    let (account_id, player_id) = player(&pool, 4, 1, &[], &[25], &[5, 1, 1, 1, 1]).await;
    let session = OneSession::new(ENTITY, 55806);
    let before = snapshot(&pool, player_id).await;

    open(&pool, &session, player_id).await;
    confirm(&pool, &session, player_id).await;

    let after = snapshot(&pool, player_id).await;
    let sent = session.typed.filter_to(session.addr);
    cleanup(&pool, account_id, player_id).await;
    assert_eq!(after, before);
    assert_eq!(
        sent,
        vec![
            refusal(
                0,
                "You have no crafting disciplines to unlearn. Nothing was changed."
            ),
            refusal(
                1,
                "No crafting respec is waiting to be confirmed. Type .respeccraft to start one."
            ),
        ]
    );
    rejected(&capture, player_id, "respeccraft", "nothing_to_respec");
}

/// The transaction re-checks under the row lock: disciplines cleared
/// between the prompt and the Yes leave nothing to respec, and nothing is
/// refunded.
#[tokio::test]
async fn live_db_respec_rechecks_the_state_it_locked() {
    let pool = require_db_or_skip!();
    let capture = LogCapture::install();
    let (account_id, player_id) = player(&pool, 5, 1, &[(78, 60)], &[], &[5, 1, 1, 1, 1]).await;
    let session = OneSession::new(ENTITY, 55807);

    open(&pool, &session, player_id).await;
    sqlx::query("UPDATE sgw_player SET discipline_ids = '{}' WHERE player_id = $1")
        .bind(player_id)
        .execute(&pool)
        .await
        .expect("clear disciplines");
    sqlx::query("DELETE FROM sgw_player_discipline_expertise WHERE player_id = $1")
        .bind(player_id)
        .execute(&pool)
        .await
        .expect("clear expertise");
    confirm(&pool, &session, player_id).await;

    let after = snapshot(&pool, player_id).await;
    let sent = session.typed.filter_to(session.addr);
    cleanup(&pool, account_id, player_id).await;
    assert_eq!(after.1, 1, "no refund");
    assert_eq!(
        sent[1],
        refusal(
            1,
            "You have no crafting disciplines to unlearn. Nothing was changed."
        )
    );
    rejected(&capture, player_id, "respecCrafting", "nothing_to_respec");
}

/// A respec for a character whose row is gone rolls back with the paired
/// `rows_affected` / `expected` and the player still gets a line.
#[tokio::test]
async fn live_db_respec_for_a_missing_player_row_warns_with_rows_affected() {
    let pool = require_db_or_skip!();
    let capture = LogCapture::install();
    let player_id = TEST_BASE + 31;
    let session = OneSession::new(ENTITY, 55808);
    with_session(ENTITY, &session.connected, &session.entity_to_addr, |s| {
        s.pending_respec = Some(PendingRespec::open(player_id, Instant::now()));
    })
    .expect("session");

    confirm(&pool, &session, player_id).await;

    let all = capture.all();
    let warn = event(&all, "persist_failed", player_id);
    assert_eq!(warn.level, tracing::Level::WARN);
    let mut expected = identity(player_id);
    expected.extend([
        ("phase", "lock_player".into()),
        ("reason", "rows_affected_short".into()),
        ("rows_affected", "0".into()),
        ("expected", "1".into()),
    ]);
    assert_fields(warn, &expected);
    assert_eq!(
        session.typed.filter_to(session.addr),
        vec![refusal(
            0,
            "Crafting respec is unavailable right now. Nothing was changed."
        )]
    );
}

/// Lock order: the respec takes the player-wide inventory key before the
/// player row. While another transaction holds only that key (as an item
/// transaction does before it locks item rows and then the player row),
/// the respec waits, even though the player row itself is free.
#[tokio::test]
async fn live_db_respec_waits_for_the_player_wide_inventory_key() {
    let pool = require_db_or_skip!();
    let (account_id, player_id) = player(&pool, 6, 0, &[(78, 60)], &[], &[5, 1, 1, 1, 1]).await;

    let mut holder = pool.begin().await.expect("begin");
    sqlx::query("SELECT pg_advisory_xact_lock($1, 0)")
        .bind(player_id)
        .execute(&mut *holder)
        .await
        .expect("take the player-wide key");
    let task_pool = pool.clone();
    let task = tokio::spawn(async move { respec_in_db(&task_pool, player_id).await });
    tokio::time::sleep(Duration::from_millis(300)).await;
    let waited = !task.is_finished();
    holder.commit().await.expect("commit");

    let outcome = task.await.expect("join").expect("no database error");
    let after = snapshot(&pool, player_id).await;
    cleanup(&pool, account_id, player_id).await;
    assert!(waited, "the respec waits for the player-wide inventory key");
    assert!(outcome.is_ok(), "{outcome:?}");
    assert_eq!((after.0, after.1), (vec![], 1));
}
