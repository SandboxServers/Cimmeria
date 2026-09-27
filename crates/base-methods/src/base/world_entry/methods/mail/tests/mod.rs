//! Mail handler tests: the read side (`read`, `read_scoping`), the send
//! path (`send_live`, `send_limits`, `send_race`), attachments and escrow
//! (`attach_live`, `attach_race`, SS-M2), the delete guard
//! (`delete_guard`, SS-M2), the attachment ops (`take_live`, `cod_live`,
//! `return_live`, `take_race`, `return_race`, SS-M3), system mail and
//! the GM tools (`system_live`, `gm_live`, SS-U1), the content engine's
//! `send_system_mail` with its cooldown (`content_live`, SS-U3), and
//! expiry, quarantine and new-mail notification (`expiry_live`,
//! `expiry_race`, `quarantine_live`, `notify_live`, SS-M4).
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
mod cod_live;
mod content_live;
mod delete_guard;
mod expiry_live;
mod expiry_race;
mod gm_live;
mod notify_live;
mod packets;
mod quarantine_live;
mod read;
mod read_scoping;
mod return_live;
mod return_race;
mod send_ignore;
mod send_limits;
mod send_live;
mod send_race;
mod system_live;
mod take_live;
mod take_race;

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

/// A mail as the attachment-op tests (SS-M3) set it up, inserted directly
/// so any state can be built, including ones the send path never makes.
#[derive(Debug, Clone)]
pub(super) struct AttachedMail<'a> {
    pub(super) owner: i32,
    pub(super) sender_id: Option<i32>,
    pub(super) sender_name: &'a str,
    pub(super) cash: i64,
    pub(super) flags: i32,
    /// `(instance id, type id, stack size)` of an escrowed item.
    pub(super) item: Option<(i32, i32, i32)>,
}

impl<'a> AttachedMail<'a> {
    /// Plain mail from `sender` to `owner`, nothing attached yet.
    pub(super) fn from(owner: i32, sender: i32, sender_name: &'a str) -> Self {
        Self {
            owner,
            sender_id: Some(sender),
            sender_name,
            cash: 0,
            flags: 0,
            item: None,
        }
    }

    pub(super) fn cash(mut self, cash: i64) -> Self {
        self.cash = cash;
        self
    }

    pub(super) fn cod(mut self, price: i64) -> Self {
        self.cash = price;
        self.flags |= crate::cell::mail::codes::flags::MAIL_COD;
        self
    }

    pub(super) fn item(mut self, item_id: i32, type_id: i32, stack_size: i32) -> Self {
        self.item = Some((item_id, type_id, stack_size));
        self
    }

    /// Insert the mail and its escrow row; returns the mail id.
    pub(super) async fn insert(&self, pool: &PgPool) -> i32 {
        let mail_id: i32 = sqlx::query_scalar(
            "INSERT INTO sgw_gate_mail \
                (character_id, sender_id, sender_name, subject, message, cash, \
                 sent_time, read_time, flags) \
             VALUES ($1, $2, $3, 'Attached', 'body', $4, 1, 0, $5) RETURNING mail_id",
        )
        .bind(self.owner)
        .bind(self.sender_id)
        .bind(self.sender_name)
        .bind(self.cash)
        .bind(self.flags)
        .fetch_one(pool)
        .await
        .expect("insert attached mail");
        if let Some((item_id, type_id, stack_size)) = self.item {
            // Every instance column off its default, so a restore that
            // drops one is visible.
            sqlx::query(
                "INSERT INTO sgw_gate_mail_item \
                    (mail_id, item_id, type_id, stack_size, charges, durability, flags, \
                     bound, ammo, cur_ammo_type, ammo_type, ammo_types, \
                     source_character_id, escrowed_at) \
                 VALUES ($1, $2, $3, $4, $5, $6, 5, true, 9, 2, 'Bullet_EMP', \
                         '{Bullet_Default,Bullet_EMP}', $7, 1)",
            )
            .bind(mail_id)
            .bind(item_id)
            .bind(type_id)
            .bind(stack_size)
            .bind(TEST_CHARGES)
            .bind(TEST_DURABILITY)
            .bind(self.sender_id.unwrap_or(0))
            .execute(pool)
            .await
            .expect("insert escrow row");
        }
        mail_id
    }
}

/// `(owner, cash, flags, returned)` of a mail, `None` once it is gone.
pub(super) async fn mail_state(pool: &PgPool, mail_id: i32) -> Option<(i32, i64, i32, bool)> {
    sqlx::query_as(
        "SELECT character_id, cash, flags, returned FROM sgw_gate_mail WHERE mail_id = $1",
    )
    .bind(mail_id)
    .fetch_optional(pool)
    .await
    .unwrap()
}

/// `(owner, container, slot, stack)` of every inventory row with this
/// instance id (the tests expect zero or one).
pub(super) async fn inventory_rows(pool: &PgPool, item_id: i32) -> Vec<(i32, i32, i32, i32)> {
    sqlx::query_as(
        "SELECT character_id, container_id, slot_id, stack_size FROM sgw_inventory \
         WHERE item_id = $1",
    )
    .bind(item_id)
    .fetch_all(pool)
    .await
    .unwrap()
}

/// Does `mail_id` still hold an escrow row?
pub(super) async fn has_escrow(pool: &PgPool, mail_id: i32) -> bool {
    sqlx::query_scalar("SELECT EXISTS (SELECT 1 FROM sgw_gate_mail_item WHERE mail_id = $1)")
        .bind(mail_id)
        .fetch_one(pool)
        .await
        .unwrap()
}

/// Assert one `mail.op_refused` WARN for `op` / `reason` / `mail_id`,
/// carrying the rule-5 identity fields.
pub(super) fn assert_refused(
    capture: &crate::test_support::LogCaptureGuard,
    op: &str,
    reason: &str,
    mail_id: i32,
) {
    let ev = capture
        .all()
        .into_iter()
        .find(|e| {
            e.level == tracing::Level::WARN
                && e.has_field("event", "mail.op_refused")
                && e.has_field("op", op)
                && e.has_field("reason", reason)
                && e.has_field("mail_id", &mail_id.to_string())
        })
        .unwrap_or_else(|| panic!("mail.op_refused op={op} reason={reason} mail_id={mail_id}"));
    for key in ["account_id", "player_id", "entity_id"] {
        assert!(ev.fields.contains_key(key), "{key} missing: {ev:?}");
    }
}

/// A mail's expiry state (SS-M4), as the expiry tests read it back.
#[derive(Debug, Clone, PartialEq, Eq, sqlx::FromRow)]
pub(super) struct ExpiryRow {
    pub(super) character_id: i32,
    pub(super) sender_id: Option<i32>,
    pub(super) cash: i64,
    pub(super) flags: i32,
    pub(super) returned: bool,
    pub(super) quarantined: bool,
    pub(super) sent_time: i32,
    pub(super) expires_at: Option<i32>,
}

/// `mail_id`'s expiry state, `None` once the row is gone.
pub(super) async fn expiry_row(pool: &PgPool, mail_id: i32) -> Option<ExpiryRow> {
    sqlx::query_as(
        "SELECT character_id, sender_id, cash, flags, returned, quarantined, sent_time, \
                expires_at \
         FROM sgw_gate_mail WHERE mail_id = $1",
    )
    .bind(mail_id)
    .fetch_optional(pool)
    .await
    .unwrap()
}

/// Force a mail's expiry-relevant state, so a test can build any state the
/// writers would reach only over 30 days.
pub(super) async fn set_expiry_state(
    pool: &PgPool,
    mail_id: i32,
    expires_at: Option<i32>,
    returned: bool,
    cod_paid: bool,
) {
    sqlx::query(
        "UPDATE sgw_gate_mail SET expires_at = $2, returned = $3, cod_paid = $4 \
         WHERE mail_id = $1",
    )
    .bind(mail_id)
    .bind(expires_at)
    .bind(returned)
    .bind(cod_paid)
    .execute(pool)
    .await
    .expect("set expiry state");
}
