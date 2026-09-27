//! Mail handler tests: the read side (`read`, `read_scoping`), the send
//! path (`send_live`, `send_limits`, `send_race`), attachments and escrow
//! (`attach_live`, `attach_race`, SS-M2) and the delete guard
//! (`delete_guard`, SS-M2).
//!
//! The live-DB tests assert on SQL side effects and, where the invariant is
//! what the client is told, on the decoded packets the handler sent.

use super::*;
pub(super) use crate::test_support::require_db_or_skip;
pub(super) use crate::test_support::TestTransport;

mod attach_live;
mod attach_race;
mod attach_rollback;
mod attach_vault;
mod delete_guard;
mod packets;
mod read;
mod read_scoping;
mod send_ignore;
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

/// Give `player_id` a balance.
pub(super) async fn set_naquadah(pool: &PgPool, player_id: i32, naquadah: i32) {
    sqlx::query("UPDATE sgw_player SET naquadah = $2 WHERE player_id = $1")
        .bind(player_id)
        .bind(naquadah)
        .execute(pool)
        .await
        .expect("set naquadah");
}

pub(super) async fn naquadah(pool: &PgPool, player_id: i32) -> i32 {
    sqlx::query_scalar("SELECT naquadah FROM sgw_player WHERE player_id = $1")
        .bind(player_id)
        .fetch_one(pool)
        .await
        .unwrap()
}

/// Any item type the resources schema holds; the mail path does not care
/// which.
pub(super) async fn any_type_id(pool: &PgPool) -> i32 {
    sqlx::query_scalar("SELECT item_id FROM resources.items ORDER BY item_id LIMIT 1")
        .fetch_one(pool)
        .await
        .expect("resources.items has a row")
}

/// One inventory item for a test: its fixed instance id and where it sits.
#[derive(Debug, Clone, Copy)]
pub(super) struct TestItem {
    pub(super) item_id: i32,
    pub(super) owner: i32,
    pub(super) container_id: i32,
    pub(super) slot_id: i32,
    pub(super) stack_size: i32,
    pub(super) bound: bool,
}

impl TestItem {
    /// A main-bag stack of `stack_size` in `slot_id`.
    pub(super) fn main(item_id: i32, owner: i32, slot_id: i32, stack_size: i32) -> Self {
        Self {
            item_id,
            owner,
            container_id: cimmeria_entity::inventory::INV_MAIN,
            slot_id,
            stack_size,
            bound: false,
        }
    }
}

/// Durability and charges every test item carries, so a test can see them
/// copied into escrow.
pub(super) const TEST_DURABILITY: i32 = 77;
pub(super) const TEST_CHARGES: i32 = 3;

pub(super) async fn insert_item(pool: &PgPool, item: TestItem, type_id: i32) {
    sqlx::query(
        "INSERT INTO sgw_inventory \
            (item_id, character_id, type_id, stack_size, container_id, slot_id, \
             bound, durability, charges) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)",
    )
    .bind(item.item_id)
    .bind(item.owner)
    .bind(type_id)
    .bind(item.stack_size)
    .bind(item.container_id)
    .bind(item.slot_id)
    .bind(item.bound)
    .bind(TEST_DURABILITY)
    .bind(TEST_CHARGES)
    .execute(pool)
    .await
    .expect("insert item");
}

/// `(owner, stack_size)` of an inventory row, `None` once it left
/// `sgw_inventory`.
pub(super) async fn inventory_row(pool: &PgPool, item_id: i32) -> Option<(i32, i32)> {
    sqlx::query_as("SELECT character_id, stack_size FROM sgw_inventory WHERE item_id = $1")
        .bind(item_id)
        .fetch_optional(pool)
        .await
        .unwrap()
}

/// One escrow row, as the tests read it back.
#[derive(Debug, Clone, PartialEq, Eq, sqlx::FromRow)]
pub(super) struct EscrowRow {
    pub(super) mail_id: i32,
    pub(super) item_id: i32,
    pub(super) type_id: i32,
    pub(super) stack_size: i32,
    pub(super) durability: i32,
    pub(super) charges: i32,
    pub(super) bound: bool,
    pub(super) source_character_id: i32,
}

/// Every escrow row whose mail belongs to `character_id`, by mail id.
pub(super) async fn escrow_for(pool: &PgPool, character_id: i32) -> Vec<EscrowRow> {
    sqlx::query_as(
        "SELECT i.mail_id, i.item_id, i.type_id, i.stack_size, i.durability, i.charges, \
                i.bound, i.source_character_id \
         FROM sgw_gate_mail_item i JOIN sgw_gate_mail m ON m.mail_id = i.mail_id \
         WHERE m.character_id = $1 ORDER BY i.mail_id",
    )
    .bind(character_id)
    .fetch_all(pool)
    .await
    .unwrap()
}

/// Escrow rows sent by `sender` (by `source_character_id`), whatever mail
/// they sit on.
pub(super) async fn escrow_from(pool: &PgPool, sender: i32) -> i64 {
    sqlx::query_scalar("SELECT COUNT(*) FROM sgw_gate_mail_item WHERE source_character_id = $1")
        .bind(sender)
        .fetch_one(pool)
        .await
        .unwrap()
}

/// Mail rows `character_id` holds.
pub(super) async fn mail_count(pool: &PgPool, character_id: i32) -> i64 {
    sqlx::query_scalar("SELECT COUNT(*) FROM sgw_gate_mail WHERE character_id = $1")
        .bind(character_id)
        .fetch_one(pool)
        .await
        .unwrap()
}
