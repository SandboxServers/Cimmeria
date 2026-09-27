//! Mail handler tests: the read side (`read`, `read_scoping`) and the send
//! path (`send_live`, `send_limits`, `send_race`).
//!
//! The live-DB tests assert on SQL side effects and, where the invariant is
//! what the client is told, on the decoded packets the handler sent.

use super::*;
pub(super) use crate::test_support::require_db_or_skip;
pub(super) use crate::test_support::TestTransport;

mod packets;
mod read;
mod read_scoping;
mod send_limits;
mod send_live;
mod send_race;

pub(super) async fn cleanup(pool: &PgPool, account_id: i32) {
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

pub(super) async fn insert_account_with_two_chars(
    pool: &PgPool,
    account_id: i32,
    char_a: i32,
    char_b: i32,
) {
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
pub(super) async fn insert_mail(pool: &PgPool, character_id: i32, subject: &str) -> i32 {
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
pub(super) fn unix_now() -> i32 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i32
}

pub(super) fn make_state(
    entity_id: u32,
) -> (
    Arc<dyn Transport>,
    Arc<Mutex<HashMap<u32, SocketAddr>>>,
    Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
) {
    let transport: Arc<dyn Transport> = Arc::new(TestTransport::new());
    let fake_addr: SocketAddr = "127.0.0.1:65535".parse().unwrap();
    let entity_to_addr = Arc::new(Mutex::new({
        let mut m = HashMap::new();
        m.insert(entity_id, fake_addr);
        m
    }));
    let connected = Arc::new(Mutex::new(HashMap::new()));
    (transport, entity_to_addr, connected)
}

/// Insert an account and one character per `(player_id, name)`.
pub(super) async fn insert_players(pool: &PgPool, account_id: i32, players: &[(i32, &str)]) {
    sqlx::query(
        "INSERT INTO account (account_id, account_name, password) \
         VALUES ($1, $2, '')",
    )
    .bind(account_id)
    .bind(format!("mail-test-{account_id}"))
    .execute(pool)
    .await
    .expect("insert account");
    for (player_id, name) in players {
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
        .bind(name)
        .execute(pool)
        .await
        .expect("insert player");
    }
}

/// Insert `count` mails for `character_id` with `flags`, in one statement.
pub(super) async fn fill_mailbox(pool: &PgPool, character_id: i32, count: i32, flags: i32) {
    sqlx::query(
        "INSERT INTO sgw_gate_mail \
            (character_id, sender_id, subject, message, cash, sent_time, read_time, flags) \
         SELECT $1, NULL, 'filler', 'body', 0, 0, 0, $3 FROM generate_series(1, $2)",
    )
    .bind(character_id)
    .bind(count)
    .bind(flags)
    .execute(pool)
    .await
    .expect("fill mailbox");
}

/// Mail rows `character_id` holds.
pub(super) async fn mail_count(pool: &PgPool, character_id: i32) -> i64 {
    sqlx::query_scalar("SELECT COUNT(*) FROM sgw_gate_mail WHERE character_id = $1")
        .bind(character_id)
        .fetch_one(pool)
        .await
        .unwrap()
}
