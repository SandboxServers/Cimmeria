//! Live-DB integration tests for the Black Market state machine + sweep, plus
//! the non-DB handler telemetry tests.
//!
//! Live-DB tests skip cleanly when `DATABASE_URL` is unset (via
//! `require_db_or_skip!`). Against the bundled local Postgres they exercise
//! createAuction / placeBid / cancelAuction (escrow into container 18 and
//! back), search, the expiry sweep, the buyout, the refusal seams and the
//! character-delete trigger.
//!
//! Sentinels fit in i32 and cleanup deletes by exact sentinel — never by range.
//! Shared fixtures live here in `mod.rs`; the per-area tests are split into
//! sibling files to keep each under the 500-line soft cap.

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use cimmeria_entity::inventory::{INV_AUCTION, INV_MAIN};
use cimmeria_mercury::transport::Transport;
use sqlx::PgPool;

use crate::base::world_entry::methods::black_market::types::BMSearchOptions;
use crate::base::world_entry::methods::black_market::{bid, cancel, create, search as bm_search};
use crate::base::ConnectedClientState;
use crate::test_support::{test_default_connected_client_state, TestTransport};

mod buyout_and_rules;
mod create_bid_cancel;
mod delete_trigger;
mod helpers;
mod named_telemetry;
mod refusals;
mod search;
mod settlement_mail;
mod state_helpers;
mod sweep;

/// Sentinel base for Black Market live-DB tests: the Black Market owns the
/// `0x7000_Axxx` block. Accounts and players use `TEST_BASE + 0..=800`
/// (decimal offsets, so up to `0x7000_A320`), search uses
/// `TEST_BASE + 0x500..=0x5FF`, the BM-02 tests (rules, refusals, the
/// delete trigger) `TEST_BASE + 0x600..=0x8FF`, and the BM-02b settlement
/// mail tests `TEST_BASE + 0x900..=0x9FF`; the in-memory entity ids are
/// `0x7000_A9xx` / `0x7000_AAxx`. The neighbouring blocks are bank
/// (`0x7000_Bxxx`) and crafting (`0x7000_Cxxx`).
pub(super) const TEST_BASE: i32 = 0x7000_A000;

/// design_id 21 (P90) exists in `resources.items`, so the snapshot mint and
/// the item-name search resolve a real row.
pub(super) const ITEM_DEF_ID: i32 = 21;

pub(super) async fn cleanup(pool: &PgPool, account_ids: &[i32], player_ids: &[i32]) {
    for &pid in player_ids {
        let _ = sqlx::query("DELETE FROM sgw_auction_bid WHERE bidder_id = $1")
            .bind(pid)
            .execute(pool)
            .await;
        let _ = sqlx::query("DELETE FROM sgw_auction WHERE seller_id = $1 OR current_bidder = $1")
            .bind(pid)
            .execute(pool)
            .await;
        let _ = sqlx::query("DELETE FROM sgw_gate_mail WHERE character_id = $1")
            .bind(pid)
            .execute(pool)
            .await;
        let _ = sqlx::query("DELETE FROM sgw_inventory WHERE character_id = $1")
            .bind(pid)
            .execute(pool)
            .await;
    }
    for &aid in account_ids {
        let _ = sqlx::query("DELETE FROM account WHERE account_id = $1")
            .bind(aid)
            .execute(pool)
            .await;
    }
}

pub(super) async fn insert_account_and_player(
    pool: &PgPool,
    account_id: i32,
    player_id: i32,
    naquadah: i32,
) {
    sqlx::query("INSERT INTO account (account_id, account_name, password) VALUES ($1, $2, '')")
        .bind(account_id)
        .bind(format!("bm-test-{account_id}"))
        .execute(pool)
        .await
        .expect("insert account");

    sqlx::query(
        "INSERT INTO sgw_player (\
            account_id, player_id, level, alignment, archetype, gender, \
            player_name, extra_name, world_location, bodyset, \
            pos_x, pos_y, pos_z, skin_color_id, naquadah, bandolier_slot\
         ) VALUES ($1, $2, 1, 0, 1, 1, $3, '', 'CombatSim', 'BS_HumanMale.BS_HumanMale', \
                   0.0, 0.0, 0.0, 0, $4, 0)",
    )
    .bind(account_id)
    .bind(player_id)
    .bind(format!("bmp-{player_id}"))
    .bind(naquadah)
    .execute(pool)
    .await
    .expect("insert player");
}

/// An item of `type_id` in the player's main bag, at the first slot past
/// what the bag already holds.
pub(super) async fn insert_item(pool: &PgPool, player_id: i32, type_id: i32) -> i32 {
    insert_item_in(pool, player_id, type_id, INV_MAIN, false).await
}

/// An item in `container_id`, optionally bound.
pub(super) async fn insert_item_in(
    pool: &PgPool,
    player_id: i32,
    type_id: i32,
    container_id: i32,
    bound: bool,
) -> i32 {
    sqlx::query_scalar(
        "INSERT INTO sgw_inventory \
            (character_id, type_id, stack_size, slot_id, container_id, \
             bound, durability, charges) \
         VALUES ($1, $2, 1, \
                 (SELECT COALESCE(MAX(slot_id), -1) + 1 FROM sgw_inventory \
                   WHERE character_id = $1 AND container_id = $3), \
                 $3, $4, 77, 3) \
         RETURNING item_id",
    )
    .bind(player_id)
    .bind(type_id)
    .bind(container_id)
    .bind(bound)
    .fetch_one(pool)
    .await
    .expect("insert inventory row")
}

pub(super) async fn naquadah_of(pool: &PgPool, player_id: i32) -> i64 {
    sqlx::query_scalar("SELECT naquadah::bigint FROM sgw_player WHERE player_id = $1")
        .bind(player_id)
        .fetch_one(pool)
        .await
        .unwrap()
}

/// Rows the player holds in their bags (every container but escrow).
pub(super) async fn inventory_count(pool: &PgPool, player_id: i32) -> i64 {
    sqlx::query_scalar(
        "SELECT COUNT(*) FROM sgw_inventory WHERE character_id = $1 AND container_id <> $2",
    )
    .bind(player_id)
    .bind(INV_AUCTION)
    .fetch_one(pool)
    .await
    .unwrap()
}

/// `(owner, container, durability, charges)` of an inventory row, `None`
/// if it is gone.
pub(super) async fn item_state(pool: &PgPool, item_id: i32) -> Option<(i32, i32, i32, i32)> {
    sqlx::query_as(
        "SELECT character_id, container_id, durability, charges FROM sgw_inventory \
         WHERE item_id = $1",
    )
    .bind(item_id)
    .fetch_optional(pool)
    .await
    .unwrap()
}

/// One Black Market mail: `(mail_id, cash, escrowed item id)`.
pub(super) type BmMail = (i32, i64, Option<i32>);

/// The Black Market mails `player_id` has, oldest first: system mail
/// (`sender_id` NULL) from "Black Market", with its escrow row if any.
pub(super) async fn bm_mails(pool: &PgPool, player_id: i32) -> Vec<BmMail> {
    sqlx::query_as(
        "SELECT m.mail_id, m.cash, mi.item_id FROM sgw_gate_mail m \
         LEFT JOIN sgw_gate_mail_item mi ON mi.mail_id = m.mail_id \
         WHERE m.character_id = $1 AND m.sender_id IS NULL \
           AND m.sender_name = 'Black Market' \
         ORDER BY m.mail_id",
    )
    .bind(player_id)
    .fetch_all(pool)
    .await
    .unwrap()
}

/// `(mail_id, durability, charges, source_character_id)` of a mail escrow
/// row, `None` if no mail holds that item.
pub(super) async fn mail_escrow_of(pool: &PgPool, item_id: i32) -> Option<(i32, i32, i32, i32)> {
    sqlx::query_as(
        "SELECT mail_id, durability, charges, source_character_id FROM sgw_gate_mail_item \
         WHERE item_id = $1",
    )
    .bind(item_id)
    .fetch_optional(pool)
    .await
    .unwrap()
}

pub(super) async fn status_of(pool: &PgPool, seq: i32) -> i16 {
    sqlx::query_scalar("SELECT status FROM sgw_auction WHERE sequence_id = $1")
        .bind(seq)
        .fetch_one(pool)
        .await
        .unwrap()
}

/// The newest auction a seller has.
pub(super) async fn last_auction_of(pool: &PgPool, seller: i32) -> i32 {
    sqlx::query_scalar("SELECT MAX(sequence_id) FROM sgw_auction WHERE seller_id = $1")
        .bind(seller)
        .fetch_one(pool)
        .await
        .unwrap()
}

/// Force an auction's `expires_at` into the past.
pub(super) async fn expire_now(pool: &PgPool, seq: i32) {
    sqlx::query("UPDATE sgw_auction SET expires_at = 1 WHERE sequence_id = $1")
        .bind(seq)
        .execute(pool)
        .await
        .unwrap();
}

/// Transport plus session maps with no sessions: sends drop, which the
/// handlers tolerate.
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

/// One online player: `(entity_id, account_id, player_id)`.
pub(super) type Session = (u32, i32, i32);

/// A database pool plus online sessions, so every handler reply reaches the
/// test transport.
pub(super) struct Harness {
    pub(super) db: Option<Arc<PgPool>>,
    pub(super) tt: Arc<TestTransport>,
    pub(super) transport: Arc<dyn Transport>,
    pub(super) connected: Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    pub(super) e2a: Arc<Mutex<HashMap<u32, SocketAddr>>>,
}

impl Harness {
    pub(super) fn new(pool: &PgPool, sessions: &[Session]) -> Self {
        let tt = Arc::new(TestTransport::new());
        let mut connected = HashMap::new();
        let mut e2a = HashMap::new();
        for (i, &(entity_id, account_id, player_id)) in sessions.iter().enumerate() {
            let addr: SocketAddr = format!("127.0.0.1:{}", 41_000 + i).parse().unwrap();
            let mut state = test_default_connected_client_state();
            state.account_id = account_id as u32;
            state.active_player_id = Some(player_id);
            state.player_entity_id = Some(entity_id);
            connected.insert(addr, state);
            e2a.insert(entity_id, addr);
        }
        Self {
            db: Some(Arc::new(pool.clone())),
            transport: tt.clone(),
            tt,
            connected: Arc::new(Mutex::new(connected)),
            e2a: Arc::new(Mutex::new(e2a)),
        }
    }

    pub(super) async fn create(&self, s: Session, item: i32, start: i32, buyout: i32, len: u8) {
        create::handle_create_auction(
            s.0,
            s.2,
            item,
            start,
            buyout,
            len,
            &self.db,
            &self.transport,
            &self.connected,
            &self.e2a,
        )
        .await;
    }

    pub(super) async fn bid(&self, s: Session, seq: i32, amount: i32) {
        bid::handle_place_bid(
            s.0,
            s.2,
            seq,
            amount,
            &self.db,
            &self.transport,
            &self.connected,
            &self.e2a,
        )
        .await;
    }

    pub(super) async fn cancel(&self, s: Session, seq: i32) {
        cancel::handle_cancel_auction(
            s.0,
            s.2,
            seq,
            &self.db,
            &self.transport,
            &self.connected,
            &self.e2a,
        )
        .await;
    }

    pub(super) async fn search(&self, s: Session, opts: BMSearchOptions) {
        bm_search::handle_search(
            s.0,
            s.2,
            opts,
            &self.db,
            &self.transport,
            &self.connected,
            &self.e2a,
        )
        .await;
    }
}
