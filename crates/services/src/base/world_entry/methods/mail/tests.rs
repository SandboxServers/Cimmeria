//! Live-DB tests for the mail handler.
//!
//! `handle_mail_request` only outputs via UDP, so the tests assert on
//! the SQL side-effects (rows present, flags column, read_time) rather
//! than packet bytes. The wire-packet builders themselves are pinned in
//! `cell::mail::tests`.

use super::*;
use crate::test_support::require_db_or_skip;

/// Sentinel base for mail tests. Distinct from prior live-DB sentinels:
/// grant_cash (+0x100), move_inventory (+0x200), grant_item (+0x300),
/// missions (+0x400).
const TEST_BASE: i32 = 0x7000_0500;

async fn cleanup(pool: &PgPool, account_id: i32) {
    // sgw_gate_mail has no FK to account, so delete its rows by character_id
    // first. The account delete cascades sgw_player rows.
    let _ = sqlx::query(
        "DELETE FROM sgw_gate_mail WHERE character_id IN \
         (SELECT player_id FROM sgw_player WHERE account_id = $1)",
    )
    .bind(account_id)
    .execute(pool)
    .await;
    let _ = sqlx::query("DELETE FROM account WHERE account_id = $1")
        .bind(account_id)
        .execute(pool)
        .await;
}

async fn insert_account_with_two_chars(pool: &PgPool, account_id: i32, char_a: i32, char_b: i32) {
    sqlx::query(
        "INSERT INTO account (account_id, account_name, password) \
         VALUES ($1, $2, '')",
    )
    .bind(account_id)
    .bind(format!("mail-test-{account_id}"))
    .execute(pool)
    .await
    .expect("insert account");

    for player_id in [char_a, char_b] {
        sqlx::query(
            "INSERT INTO sgw_player (\
                account_id, player_id, level, alignment, archetype, gender, \
                player_name, extra_name, world_location, bodyset, \
                pos_x, pos_y, pos_z, skin_color_id, naquadah\
             ) VALUES ($1, $2, 1, 0, 1, 1, $3, '', 'CombatSim', 'BS_HumanMale.BS_HumanMale', \
                       0.0, 0.0, 0.0, 0, 0)",
        )
        .bind(account_id)
        .bind(player_id)
        .bind(format!("test-{player_id}"))
        .execute(pool)
        .await
        .expect("insert player");
    }
}

/// Insert a mail for `character_id`. Returns the auto-generated mail_id.
async fn insert_mail(pool: &PgPool, character_id: i32, subject: &str) -> i32 {
    sqlx::query_scalar(
        "INSERT INTO sgw_gate_mail \
            (character_id, sender_id, subject, message, cash, sent_time, read_time, flags) \
         VALUES ($1, NULL, $2, 'body', 0, 0, 0, 0) RETURNING mail_id",
    )
    .bind(character_id)
    .bind(subject)
    .fetch_one(pool)
    .await
    .expect("insert mail")
}

/// UNIX-epoch second sampled BEFORE the test calls handle_mail_request.
/// Used to assert read_time lands inside [before, after] rather than
/// merely > 0 — the latter would match a regression that hard-coded
/// `read_time = 1`.
fn unix_now() -> i32 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i32
}

fn make_state(
    entity_id: u32,
) -> (
    Arc<UdpSocket>,
    Arc<Mutex<HashMap<u32, SocketAddr>>>,
    Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
) {
    let std_sock = std::net::UdpSocket::bind("127.0.0.1:0").expect("bind UDP");
    std_sock.set_nonblocking(true).unwrap();
    let socket = Arc::new(UdpSocket::from_std(std_sock).expect("from_std"));
    let fake_addr: SocketAddr = "127.0.0.1:65535".parse().unwrap();
    let entity_to_addr = Arc::new(Mutex::new({
        let mut m = HashMap::new();
        m.insert(entity_id, fake_addr);
        m
    }));
    let connected = Arc::new(Mutex::new(HashMap::new()));
    (socket, entity_to_addr, connected)
}

/// Regression guard: Delete must scope by character_id. Character A
/// requesting Delete on a mail_id owned by character B (same account)
/// must affect 0 rows — B's mail stays. The pre-fix bug had the
/// WHERE clause matching on account-wide criteria, so any character
/// could delete any sibling's mail.
#[tokio::test]
async fn delete_only_affects_target_character_not_account_siblings() {
    let pool = require_db_or_skip!();
    let account_id = TEST_BASE;
    let char_a = TEST_BASE + 1;
    let char_b = TEST_BASE + 2;
    cleanup(&pool, account_id).await;
    insert_account_with_two_chars(&pool, account_id, char_a, char_b).await;

    let mail_a = insert_mail(&pool, char_a, "for A").await;
    let mail_b = insert_mail(&pool, char_b, "for B").await;

    let (socket, e2a, conn) = make_state(0x7000_0501);
    let db_pool = Some(Arc::new(pool.clone()));

    // Character A tries to delete character B's mail. Must NOT delete it.
    handle_mail_request(
        0x7000_0501,
        char_a,
        MailOp::Delete { mail_id: mail_b },
        &socket,
        &conn,
        &e2a,
        &db_pool,
    )
    .await;

    let b_still_exists: bool =
        sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM sgw_gate_mail WHERE mail_id = $1)")
            .bind(mail_b)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert!(
        b_still_exists,
        "character B's mail must survive when character A issues Delete on it"
    );

    // Sanity: A's own mail is still there too.
    let a_still_exists: bool =
        sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM sgw_gate_mail WHERE mail_id = $1)")
            .bind(mail_a)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert!(a_still_exists, "A's own mail unaffected");

    // Now A deletes its own mail — must succeed.
    handle_mail_request(
        0x7000_0501,
        char_a,
        MailOp::Delete { mail_id: mail_a },
        &socket,
        &conn,
        &e2a,
        &db_pool,
    )
    .await;
    let a_now_gone: bool =
        sqlx::query_scalar("SELECT NOT EXISTS(SELECT 1 FROM sgw_gate_mail WHERE mail_id = $1)")
            .bind(mail_a)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert!(a_now_gone, "A's own mail deletes when A is the requester");

    cleanup(&pool, account_id).await;
}

/// Regression guard: Archive must scope by character_id. Same setup as
/// the Delete test, but verifies the flags column on B's mail stays at 0
/// when A issues Archive on it.
#[tokio::test]
async fn archive_only_affects_target_character_not_account_siblings() {
    let pool = require_db_or_skip!();
    let account_id = TEST_BASE + 100;
    let char_a = TEST_BASE + 101;
    let char_b = TEST_BASE + 102;
    cleanup(&pool, account_id).await;
    insert_account_with_two_chars(&pool, account_id, char_a, char_b).await;

    let mail_a = insert_mail(&pool, char_a, "for A").await;
    let mail_b = insert_mail(&pool, char_b, "for B").await;

    let (socket, e2a, conn) = make_state(0x7000_0511);
    let db_pool = Some(Arc::new(pool.clone()));

    // A tries to archive B's mail. Must NOT flip the flags bit.
    handle_mail_request(
        0x7000_0511,
        char_a,
        MailOp::Archive { mail_id: mail_b },
        &socket,
        &conn,
        &e2a,
        &db_pool,
    )
    .await;

    let b_flags: i32 = sqlx::query_scalar("SELECT flags FROM sgw_gate_mail WHERE mail_id = $1")
        .bind(mail_b)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(
        b_flags, 0,
        "character B's flags must stay 0 when character A issues Archive"
    );

    // A archives its own mail — flags must gain bit 0.
    handle_mail_request(
        0x7000_0511,
        char_a,
        MailOp::Archive { mail_id: mail_a },
        &socket,
        &conn,
        &e2a,
        &db_pool,
    )
    .await;
    let a_flags: i32 = sqlx::query_scalar("SELECT flags FROM sgw_gate_mail WHERE mail_id = $1")
        .bind(mail_a)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(
        a_flags & 1,
        1,
        "Archive sets bit 0 on the target's flags column"
    );

    cleanup(&pool, account_id).await;
}

/// Archive is OR-with-1, so calling it twice on the same mail must leave
/// flags == 1 (not toggle / increment). Pins the bitwise-OR semantics so
/// a future regression to `flags = 1` (assignment instead of OR) would
/// be caught when other flag bits are in play.
#[tokio::test]
async fn archive_is_idempotent_via_bitwise_or() {
    let pool = require_db_or_skip!();
    let account_id = TEST_BASE + 200;
    let char_a = TEST_BASE + 201;
    let char_b = TEST_BASE + 202; // unused but make_account expects two
    cleanup(&pool, account_id).await;
    insert_account_with_two_chars(&pool, account_id, char_a, char_b).await;

    // Seed mail with flags = 4 (some unrelated bit set).
    let mail_id: i32 = sqlx::query_scalar(
        "INSERT INTO sgw_gate_mail \
            (character_id, sender_id, subject, message, cash, sent_time, read_time, flags) \
         VALUES ($1, NULL, 'flags-test', 'body', 0, 0, 0, 4) RETURNING mail_id",
    )
    .bind(char_a)
    .fetch_one(&pool)
    .await
    .unwrap();

    let (socket, e2a, conn) = make_state(0x7000_0521);
    let db_pool = Some(Arc::new(pool.clone()));

    // Archive twice. Both calls must leave flags == 4 | 1 == 5
    // (NOT 1 — that would mean assignment overwrote the unrelated bit).
    for _ in 0..2 {
        handle_mail_request(
            0x7000_0521,
            char_a,
            MailOp::Archive { mail_id },
            &socket,
            &conn,
            &e2a,
            &db_pool,
        )
        .await;
    }

    let flags: i32 = sqlx::query_scalar("SELECT flags FROM sgw_gate_mail WHERE mail_id = $1")
        .bind(mail_id)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(
        flags, 5,
        "Archive must OR bit 0 in (flags |= 1) — pre-existing bit 2 must survive. \
         A flags=1 here means a regression to `flags = 1` (assignment)",
    );

    cleanup(&pool, account_id).await;
}

/// Receive smoke: an admin/system row inserted for character B is
/// queryable via the same SELECT shape RequestHeaders uses. The "send"
/// half of the flow lives in upstream tooling (no in-game send path in
/// the server today). This test does NOT go through `handle_mail_request`
/// for the headers fetch — it asserts the SELECT shape directly because
/// the function output is UDP-only and the relevant invariant here is
/// the column round-trip + WHERE-clause scoping (recipient column is
/// `character_id`, not `sender_id`).
#[tokio::test]
async fn mail_inserted_for_character_b_is_queryable_via_request_headers_select() {
    let pool = require_db_or_skip!();
    let account_id = TEST_BASE + 300;
    let char_a = TEST_BASE + 301; // sender
    let char_b = TEST_BASE + 302; // recipient
    cleanup(&pool, account_id).await;
    insert_account_with_two_chars(&pool, account_id, char_a, char_b).await;

    // "Send" — admin/system inserts a mail addressed to B from A.
    let mail_id: i32 = sqlx::query_scalar(
        "INSERT INTO sgw_gate_mail \
            (character_id, sender_name, sender_id, subject, message, cash, sent_time, read_time, flags) \
         VALUES ($1, 'admin', $2, 'Welcome', 'first body', 0, 0, 0, 0) RETURNING mail_id",
    )
    .bind(char_b)
    .bind(char_a)
    .fetch_one(&pool)
    .await
    .unwrap();

    // "Receive" — RequestHeaders' SELECT shape, run directly so the
    // test asserts the row contents rather than the UDP wire output.
    #[derive(sqlx::FromRow)]
    struct Row {
        mail_id: i32,
        sender_id: Option<i32>,
        subject: String,
    }
    let rows: Vec<Row> = sqlx::query_as(
        "SELECT mail_id, sender_id, subject FROM sgw_gate_mail \
         WHERE character_id = $1 ORDER BY mail_id DESC",
    )
    .bind(char_b)
    .fetch_all(&pool)
    .await
    .unwrap();

    assert_eq!(
        rows.len(),
        1,
        "B must see exactly the one mail addressed to it"
    );
    assert_eq!(rows[0].mail_id, mail_id);
    assert_eq!(
        rows[0].sender_id,
        Some(char_a),
        "sender_id round-trips so the client can render the From column"
    );
    assert_eq!(rows[0].subject, "Welcome");

    // Sanity: A's inbox is empty — the same mail_id must NOT show up
    // under the wrong character_id (a JOIN drift would surface here).
    let a_count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM sgw_gate_mail WHERE character_id = $1")
            .bind(char_a)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(
        a_count, 0,
        "sender (char_a) inbox must be empty — sgw_gate_mail.character_id is the recipient column"
    );

    cleanup(&pool, account_id).await;
}

/// `MailOp::RequestBody` on an unread mail must update `read_time` from 0
/// to the current UNIX-epoch second. The window check (before <=
/// read_time <= after) catches both a regression that hard-codes a
/// constant (e.g. `read_time = 1`) AND a regression that writes a
/// far-future / past value via a unit confusion (millis vs seconds).
#[tokio::test]
async fn request_body_marks_unread_mail_as_read_via_read_time_update() {
    let pool = require_db_or_skip!();
    let account_id = TEST_BASE + 400;
    let char_a = TEST_BASE + 401;
    let char_b = TEST_BASE + 402;
    cleanup(&pool, account_id).await;
    insert_account_with_two_chars(&pool, account_id, char_a, char_b).await;

    let mail_id = insert_mail(&pool, char_a, "unread").await;
    let initial: i32 = sqlx::query_scalar("SELECT read_time FROM sgw_gate_mail WHERE mail_id = $1")
        .bind(mail_id)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(
        initial, 0,
        "fixture sanity: insert_mail leaves read_time = 0"
    );

    let (socket, e2a, conn) = make_state(0x7000_0531);
    let db_pool = Some(Arc::new(pool.clone()));

    let before = unix_now();
    handle_mail_request(
        0x7000_0531,
        char_a,
        MailOp::RequestBody { mail_id },
        &socket,
        &conn,
        &e2a,
        &db_pool,
    )
    .await;
    let after = unix_now();

    let stored: i32 = sqlx::query_scalar("SELECT read_time FROM sgw_gate_mail WHERE mail_id = $1")
        .bind(mail_id)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert!(
        stored >= before && stored <= after,
        "read_time must land inside the [before, after] UNIX-second window — \
         got {stored}, expected {before}..={after}. \
         A constant outside this range means the handler regressed to a hard-coded value."
    );

    cleanup(&pool, account_id).await;
}

/// `RequestBody` re-issued on an already-read mail must NOT overwrite
/// `read_time` — the SQL has `AND read_time = 0` precisely to preserve
/// the *first* read timestamp. Without this guard the mail's "first
/// read" UI label would silently re-anchor on every re-open.
#[tokio::test]
async fn request_body_does_not_overwrite_existing_read_time() {
    let pool = require_db_or_skip!();
    let account_id = TEST_BASE + 500;
    let char_a = TEST_BASE + 501;
    let char_b = TEST_BASE + 502;
    cleanup(&pool, account_id).await;
    insert_account_with_two_chars(&pool, account_id, char_a, char_b).await;

    // Seed mail with a known non-zero read_time.
    const ORIGINAL_READ_TIME: i32 = 12_345;
    let mail_id: i32 = sqlx::query_scalar(
        "INSERT INTO sgw_gate_mail \
            (character_id, sender_id, subject, message, cash, sent_time, read_time, flags) \
         VALUES ($1, NULL, 'already-read', 'body', 0, 0, $2, 0) RETURNING mail_id",
    )
    .bind(char_a)
    .bind(ORIGINAL_READ_TIME)
    .fetch_one(&pool)
    .await
    .unwrap();

    let (socket, e2a, conn) = make_state(0x7000_0541);
    let db_pool = Some(Arc::new(pool.clone()));
    handle_mail_request(
        0x7000_0541,
        char_a,
        MailOp::RequestBody { mail_id },
        &socket,
        &conn,
        &e2a,
        &db_pool,
    )
    .await;

    let after: i32 = sqlx::query_scalar("SELECT read_time FROM sgw_gate_mail WHERE mail_id = $1")
        .bind(mail_id)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(
        after, ORIGINAL_READ_TIME,
        "RequestBody must preserve the first-read timestamp \
         (the `AND read_time = 0` guard exists for this)",
    );

    cleanup(&pool, account_id).await;
}

/// Security boundary: character A requesting RequestBody on a mail
/// owned by character B (same account) must NOT mark B's mail read.
/// The SELECT filters by `character_id = $2`, returns None, and the
/// `Ok(Some(row))` arm's UPDATE never runs — so B's `read_time`
/// stays at 0. A regression that moved the UPDATE outside the
/// `Some(row)` match arm would silently flip B's read indicator.
#[tokio::test]
async fn request_body_by_other_character_does_not_mark_target_mail_read() {
    let pool = require_db_or_skip!();
    let account_id = TEST_BASE + 600;
    let char_a = TEST_BASE + 601;
    let char_b = TEST_BASE + 602;
    cleanup(&pool, account_id).await;
    insert_account_with_two_chars(&pool, account_id, char_a, char_b).await;

    let mail_b = insert_mail(&pool, char_b, "for B").await;

    let (socket, e2a, conn) = make_state(0x7000_0551);
    let db_pool = Some(Arc::new(pool.clone()));
    // A requests body of B's mail.
    handle_mail_request(
        0x7000_0551,
        char_a,
        MailOp::RequestBody { mail_id: mail_b },
        &socket,
        &conn,
        &e2a,
        &db_pool,
    )
    .await;

    let read_time: i32 =
        sqlx::query_scalar("SELECT read_time FROM sgw_gate_mail WHERE mail_id = $1")
            .bind(mail_b)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(
        read_time, 0,
        "B's mail must remain unread when A requests its body — \
         the SELECT's character_id filter is the security boundary"
    );

    cleanup(&pool, account_id).await;
}

/// Full receive lifecycle for one mail across the operations the
/// server handles: insert (admin / upstream "send") → headers
/// SELECT smoke (the WHERE clause is the recipient gate) → RequestBody
/// (read_time bumps inside the [before, after] window) → Delete (row
/// gone). Pins the cross-step state machine.
#[tokio::test]
async fn receive_lifecycle_request_headers_then_body_then_delete() {
    let pool = require_db_or_skip!();
    let account_id = TEST_BASE + 700;
    let char_a = TEST_BASE + 701; // sender
    let char_b = TEST_BASE + 702; // recipient
    cleanup(&pool, account_id).await;
    insert_account_with_two_chars(&pool, account_id, char_a, char_b).await;

    // ── send: row inserted addressed to B from A ────────────────
    let mail_id: i32 = sqlx::query_scalar(
        "INSERT INTO sgw_gate_mail \
            (character_id, sender_name, sender_id, subject, message, cash, sent_time, read_time, flags) \
         VALUES ($1, 'A', $2, 'hello', 'body-text', 0, 0, 0, 0) RETURNING mail_id",
    )
    .bind(char_b)
    .bind(char_a)
    .fetch_one(&pool)
    .await
    .unwrap();

    // ── headers: B's inbox shows the row (SELECT smoke; the function
    //    output is wire-only so we assert the underlying SELECT shape).
    let in_b_inbox: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM sgw_gate_mail \
         WHERE character_id = $1 AND mail_id = $2",
    )
    .bind(char_b)
    .bind(mail_id)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(
        in_b_inbox, 1,
        "B's RequestHeaders SELECT must find the mail"
    );

    // ── body: B reads, read_time bumps to within the test's UNIX-second window ─
    let (socket, e2a, conn) = make_state(0x7000_0561);
    let db_pool = Some(Arc::new(pool.clone()));
    let before = unix_now();
    handle_mail_request(
        0x7000_0561,
        char_b,
        MailOp::RequestBody { mail_id },
        &socket,
        &conn,
        &e2a,
        &db_pool,
    )
    .await;
    let after = unix_now();
    let read_after_body: i32 =
        sqlx::query_scalar("SELECT read_time FROM sgw_gate_mail WHERE mail_id = $1")
            .bind(mail_id)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert!(
        read_after_body >= before && read_after_body <= after,
        "read_time must land inside [before, after] — got {read_after_body}, expected {before}..={after}"
    );

    // ── delete: B removes the mail ─────────────────────────────
    handle_mail_request(
        0x7000_0561,
        char_b,
        MailOp::Delete { mail_id },
        &socket,
        &conn,
        &e2a,
        &db_pool,
    )
    .await;
    let still_exists: bool =
        sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM sgw_gate_mail WHERE mail_id = $1)")
            .bind(mail_id)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert!(
        !still_exists,
        "Delete by the recipient must clear the row at the end of the receive lifecycle"
    );

    cleanup(&pool, account_id).await;
}
