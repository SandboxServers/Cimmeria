//! Live-DB guards and `LogCapture` guards for the vault purchase (BV-05):
//! one click is charged once, a short purse changes nothing, the ceiling is
//! 100, and `expand` / every `expand_rejected` reason logs its fields and
//! tells the player.
//!
//! The cell's verdict and offer are passed in directly: how the cell takes
//! them is pinned in `cimmeria-cell-interactions` `bank/expand_tests.rs`.
//! These tests pin what the base does with them. The prices are the seeded
//! `resources.bank_expansion_price` rows (100 naquadah a step), so the
//! tests also prove the seed loads.
//!
//! Sentinels: accounts and players `0x7000_BB00..=0x7000_BB71`, entities
//! `0x7000_BBE0..=0x7000_BBEF`, Banker `0x7000_BBD0`. Skip when
//! `DATABASE_URL` is unset.

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use cimmeria_entity::cell_entity::{ExpansionOffer, VaultScope};
use cimmeria_mercury::transport::Transport;
use sqlx::PgPool;
use tracing::Level;

use super::persist::{persist_expansion, ExpandOutcome};
use super::*;
use crate::test_support::{
    require_db_or_skip, test_default_connected_client_state, Captured, LogCapture, LogCaptureGuard,
    TestTransport,
};

pub(super) const BASE: i32 = 0x7000_BB00;
pub(super) const BANKER: u32 = 0x7000_BBD0;

/// Next to the Banker, 2.5 units away.
pub(super) const AT_BANKER: VaultAccess = VaultAccess::Open {
    scope: VaultScope::Personal,
    banker_id: Some(BANKER),
    distance: Some(2.5),
};

/// A test client in world as one entity, so the sends reach it.
pub(super) struct TestClient {
    pub(super) transport: Arc<TestTransport>,
    pub(super) dyn_transport: Arc<dyn Transport>,
    pub(super) addr: SocketAddr,
    pub(super) e2a: Arc<Mutex<HashMap<u32, SocketAddr>>>,
    pub(super) conn: Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
}

/// The session at `port` playing `c`'s character as `c.entity_id`.
pub(super) fn in_world(c: ExpandCaller, port: u16) -> TestClient {
    in_world_as(c.entity_id, c.player_id, port)
}

/// A session playing `player_id` as `entity_id`.
pub(super) fn in_world_as(entity_id: u32, player_id: i32, port: u16) -> TestClient {
    let transport = Arc::new(TestTransport::new());
    let dyn_transport: Arc<dyn Transport> = transport.clone();
    let addr: SocketAddr = format!("127.0.0.1:{port}").parse().unwrap();
    let mut state = test_default_connected_client_state();
    state.player_entity_id = Some(entity_id);
    state.active_player_id = Some(player_id);
    TestClient {
        transport,
        dyn_transport,
        addr,
        e2a: Arc::new(Mutex::new(HashMap::from([(entity_id, addr)]))),
        conn: Arc::new(Mutex::new(HashMap::from([(addr, state)]))),
    }
}

impl TestClient {
    /// Every packet to the client, decrypted. The test session's key is all
    /// zeros (`test_default_connected_client_state`).
    fn packets(&self) -> Vec<Vec<u8>> {
        let enc = cimmeria_mercury::encryption::MercuryEncryption::from_session_key([0u8; 32]);
        self.transport
            .filter_to(self.addr)
            .iter()
            .filter_map(|p| enc.decrypt(p).ok())
            .collect()
    }

    /// Whether any packet carries `needle` byte for byte.
    pub(super) fn saw_bytes(&self, needle: &[u8]) -> bool {
        self.packets()
            .iter()
            .any(|p| p.windows(needle.len()).any(|w| w == needle))
    }

    /// Whether any packet carries `text` as UTF-16LE, the encoding of
    /// `onPlayerCommunication`'s text.
    pub(super) fn saw_text(&self, text: &str) -> bool {
        let want: Vec<u8> = text.encode_utf16().flat_map(u16::to_le_bytes).collect();
        self.saw_bytes(&want)
    }

    pub(super) fn sent(&self) -> usize {
        self.transport.filter_to(self.addr).len()
    }
}

pub(super) fn caller(n: i32, entity_id: u32) -> ExpandCaller {
    ExpandCaller {
        entity_id,
        account_id: Some((BASE + n) as u32),
        player_id: BASE + n + 1,
    }
}

pub(super) async fn cleanup(pool: &PgPool, c: ExpandCaller) {
    let _ = sqlx::query("DELETE FROM sgw_player WHERE player_id = $1")
        .bind(c.player_id)
        .execute(pool)
        .await;
    let _ = sqlx::query("DELETE FROM account WHERE account_id = $1")
        .bind(c.account_id.unwrap() as i32)
        .execute(pool)
        .await;
}

/// The sentinel account and character, with `bank_slots` and `naquadah`.
pub(super) async fn setup(pool: &PgPool, c: ExpandCaller, bank_slots: i16, naquadah: i32) {
    cleanup(pool, c).await;
    let account_id = c.account_id.unwrap() as i32;
    sqlx::query("INSERT INTO account (account_id, account_name, password) VALUES ($1, $2, '')")
        .bind(account_id)
        .bind(format!("bv05-expand-{account_id}"))
        .execute(pool)
        .await
        .expect("insert account");
    sqlx::query(
        "INSERT INTO sgw_player (\
            account_id, player_id, level, alignment, archetype, gender, \
            player_name, extra_name, world_location, bodyset, \
            pos_x, pos_y, pos_z, skin_color_id, naquadah, bank_slots\
         ) VALUES ($1, $2, 1, 0, 1, 1, $3, '', 'CombatSim', 'BS_HumanMale.BS_HumanMale', \
                   0.0, 0.0, 0.0, 0, $4, $5)",
    )
    .bind(account_id)
    .bind(c.player_id)
    .bind(format!("Bv05Expand{}", c.player_id))
    .bind(naquadah)
    .bind(bank_slots)
    .execute(pool)
    .await
    .expect("insert player");
}

/// `(bank_slots, naquadah)` as committed.
pub(super) async fn row(pool: &PgPool, player_id: i32) -> (i16, i32) {
    sqlx::query_as("SELECT bank_slots, naquadah FROM sgw_player WHERE player_id = $1")
        .bind(player_id)
        .fetch_one(pool)
        .await
        .expect("player row")
}

/// The offer the seed quotes at `from_slots`: 100 naquadah a step.
pub(super) fn offered(from_slots: i16) -> Option<ExpansionOffer> {
    Some(ExpansionOffer {
        from_slots,
        price: 100,
    })
}

pub(super) async fn expand(
    pool: &PgPool,
    client: &TestClient,
    c: ExpandCaller,
    offer: Option<ExpansionOffer>,
    vault: VaultAccess,
) {
    handle_expand(
        c,
        offer,
        vault,
        &Some(Arc::new(pool.clone())),
        &client.dyn_transport,
        &client.conn,
    )
    .await;
}

pub(super) fn bank_rows(capture: &LogCaptureGuard, name: &str) -> Vec<Captured> {
    capture
        .all()
        .into_iter()
        .filter(|c| c.target == "bank" && c.has_field("event", name))
        .collect()
}

/// Exactly one `name` row at `level`, with the caller's correlators and
/// every `(field, value)` in `want`.
pub(super) fn one(
    capture: &LogCaptureGuard,
    name: &str,
    level: Level,
    c: ExpandCaller,
    want: &[(&str, &str)],
) -> Captured {
    let found = bank_rows(capture, name);
    assert_eq!(found.len(), 1, "exactly one {name}: {:#?}", capture.all());
    let row = found.into_iter().next().unwrap();
    assert_eq!(row.level, level, "{name} level: {row:#?}");
    let account = c.account_id.unwrap().to_string();
    let player = c.player_id.to_string();
    let entity = c.entity_id.to_string();
    for (k, v) in [
        ("account_id", account.as_str()),
        ("player_id", player.as_str()),
        ("entity_id", entity.as_str()),
    ]
    .iter()
    .chain(want)
    {
        assert!(row.has_field(k, v), "{name}: {k}={v} missing: {row:#?}");
    }
    row
}

/// One step at a Banker: 40 to 50, 100 naquadah paid, in the row; INFO
/// `expand` with the size and the cash before and after; the player
/// receives the re-declared `onBagInfo`, the new balance and a chat line.
#[tokio::test]
async fn a_purchase_adds_ten_slots_charges_the_price_and_redeclares_the_vault() {
    let pool = require_db_or_skip!();
    let c = caller(0x00, 0x7000_BBE0);
    setup(&pool, c, 40, 250).await;
    let client = in_world(c, 40900);
    let capture = LogCapture::install();

    expand(&pool, &client, c, offered(40), AT_BANKER).await;
    let after = row(&pool, c.player_id).await;
    cleanup(&pool, c).await;

    assert_eq!(after, (50, 150));
    one(
        &capture,
        "expand",
        Level::INFO,
        c,
        &[
            ("bank_slots_before", "40"),
            ("bank_slots_after", "50"),
            ("price", "100"),
            ("cash_before", "250"),
            ("cash_after", "150"),
            ("banker_id", &BANKER.to_string()),
            ("gm_override", "false"),
            ("distance", "2.5"),
        ],
    );
    assert!(bank_rows(&capture, "expand_rejected").is_empty());
    assert!(capture
        .all()
        .iter()
        .any(|r| r.target == "span:bank.expand_purchase" && r.level == Level::INFO));
    assert!(
        client.saw_bytes(&vault_resize_bag_info_args(50)),
        "onBagInfo must re-declare the vault at 50"
    );
    assert!(
        client.saw_bytes(&150i32.to_le_bytes()),
        "onCashChanged(150)"
    );
    assert!(client.saw_text("Your vault now has 50 slots. You paid 100 naquadah."));
    assert_eq!(client.sent(), 3, "bag info, cash and one line");
}

/// A GM `.bank` session buys too, with no Banker: `gm_override=true` and
/// no `banker_id` or `distance` field.
#[tokio::test]
async fn a_gm_session_expands_without_a_banker() {
    let pool = require_db_or_skip!();
    let c = caller(0x10, 0x7000_BBE1);
    setup(&pool, c, 60, 100).await;
    let client = in_world(c, 40901);
    let gm = VaultAccess::Open {
        scope: VaultScope::Personal,
        banker_id: None,
        distance: None,
    };
    let capture = LogCapture::install();

    expand(&pool, &client, c, offered(60), gm).await;
    let after = row(&pool, c.player_id).await;
    cleanup(&pool, c).await;

    assert_eq!(after, (70, 0));
    let e = one(
        &capture,
        "expand",
        Level::INFO,
        c,
        &[("gm_override", "true")],
    );
    assert!(!e.fields.contains_key("banker_id"), "{e:#?}");
    assert!(!e.fields.contains_key("distance"), "{e:#?}");
}

/// The double purchase from one click: the same offer sent twice, one
/// after the other and then concurrently, is charged once. The second is
/// WARN `expand_rejected reason=replay` with the size it found, and the
/// player is told. Fails if the `bank_slots = from_slots` guard is
/// removed: both sends then buy (60 slots, 200 paid).
#[tokio::test]
async fn a_double_purchase_from_one_click_is_charged_once() {
    let pool = require_db_or_skip!();
    let c = caller(0x20, 0x7000_BBE2);
    setup(&pool, c, 40, 1000).await;
    let client = in_world(c, 40902);
    let capture = LogCapture::install();

    expand(&pool, &client, c, offered(40), AT_BANKER).await;
    expand(&pool, &client, c, offered(40), AT_BANKER).await;
    let sequential = row(&pool, c.player_id).await;

    // Concurrently: two statements on two connections for one offer at 50.
    let (a, b) = tokio::join!(
        persist_expansion(&pool, c.player_id, offered(50).unwrap()),
        persist_expansion(&pool, c.player_id, offered(50).unwrap())
    );
    let concurrent = row(&pool, c.player_id).await;
    cleanup(&pool, c).await;

    assert_eq!(sequential, (50, 900), "one click, one charge");
    one(
        &capture,
        "expand",
        Level::INFO,
        c,
        &[("bank_slots_after", "50")],
    );
    one(
        &capture,
        "expand_rejected",
        Level::WARN,
        c,
        &[
            ("reason", "replay"),
            ("offered_slots", "40"),
            ("offered_price", "100"),
            ("bank_slots", "50"),
            ("cash", "900"),
        ],
    );
    assert!(client.saw_text("That expansion was already handled. Nothing more was charged."));

    let outcomes = [a.expect("first statement"), b.expect("second statement")];
    let bought = outcomes
        .iter()
        .filter(|o| matches!(o, ExpandOutcome::Expanded { .. }))
        .count();
    let replays = outcomes
        .iter()
        .filter(|o| matches!(o, ExpandOutcome::Replay { .. }))
        .count();
    assert_eq!((bought, replays), (1, 1), "{outcomes:?}");
    assert_eq!(concurrent, (60, 800), "the concurrent pair is charged once");
}

/// Short of the price: nothing changes, WARN `expand_rejected
/// reason=insufficient_cash` with the cash, the size and the price, and a
/// line naming the price. Fails if the cash guard is removed (the row goes
/// to 50 slots and -1 naquadah).
#[tokio::test]
async fn insufficient_funds_change_nothing() {
    let pool = require_db_or_skip!();
    let c = caller(0x30, 0x7000_BBE3);
    setup(&pool, c, 40, 99).await;
    let client = in_world(c, 40903);
    let capture = LogCapture::install();

    expand(&pool, &client, c, offered(40), AT_BANKER).await;
    let after = row(&pool, c.player_id).await;
    cleanup(&pool, c).await;

    assert_eq!(after, (40, 99));
    one(
        &capture,
        "expand_rejected",
        Level::WARN,
        c,
        &[
            ("reason", "insufficient_cash"),
            ("bank_slots", "40"),
            ("cash", "99"),
            ("price", "100"),
            ("banker_id", &BANKER.to_string()),
        ],
    );
    assert!(bank_rows(&capture, "expand").is_empty());
    assert!(client.saw_text("You need 100 naquadah to expand your vault. Nothing was charged."));
    assert_eq!(client.sent(), 1, "only the line: no bag info, no cash");
}

/// 100 is the ceiling: 90 buys the last step, then an offer at 100 is
/// WARN `expand_rejected reason=at_ceiling` and changes nothing. Fails if
/// the ceiling branch is removed from the classification (the refusal is
/// then `price_missing`, the wrong line).
#[tokio::test]
async fn the_ceiling_is_100() {
    let pool = require_db_or_skip!();
    let c = caller(0x40, 0x7000_BBE4);
    setup(&pool, c, 90, 1000).await;
    let client = in_world(c, 40904);
    let capture = LogCapture::install();

    expand(&pool, &client, c, offered(90), AT_BANKER).await;
    let at_full = row(&pool, c.player_id).await;
    expand(&pool, &client, c, offered(100), AT_BANKER).await;
    let after = row(&pool, c.player_id).await;
    cleanup(&pool, c).await;

    assert_eq!(at_full, (100, 900));
    assert_eq!(after, (100, 900), "nothing past the ceiling");
    one(
        &capture,
        "expand",
        Level::INFO,
        c,
        &[("bank_slots_after", "100")],
    );
    one(
        &capture,
        "expand_rejected",
        Level::WARN,
        c,
        &[
            ("reason", "at_ceiling"),
            ("bank_slots", "100"),
            ("cash", "900"),
        ],
    );
    assert!(client.saw_text("Your vault is already at its full size of 100 slots."));
}

/// The verdict refuses before any write: no session, and a Banker the
/// player walked away from. Each is WARN `expand_rejected` with the
/// verdict's own label, the size and the cash read for the log, and a line.
/// Fails if the verdict check is removed (the purchase goes through).
#[tokio::test]
async fn a_closed_vault_verdict_buys_nothing() {
    let pool = require_db_or_skip!();
    let cases = [
        (
            0x50,
            0x7000_BBE5,
            VaultAccess::NO_SESSION,
            "no_vault_session",
            "Talk to a Banker again to expand your vault. Nothing was charged.",
        ),
        (
            0x60,
            0x7000_BBE6,
            VaultAccess::Closed {
                reason: "banker_out_of_range",
                banker_id: Some(BANKER),
                distance: Some(9.0),
            },
            "banker_out_of_range",
            "You are too far from the Banker. Your vault was not expanded.",
        ),
    ];
    for (n, entity_id, vault, reason, line) in cases {
        let c = caller(n, entity_id);
        setup(&pool, c, 40, 500).await;
        let client = in_world(c, 40905);
        let capture = LogCapture::install();

        expand(&pool, &client, c, offered(40), vault).await;
        let after = row(&pool, c.player_id).await;
        cleanup(&pool, c).await;

        assert_eq!(after, (40, 500), "{reason}");
        one(
            &capture,
            "expand_rejected",
            Level::WARN,
            c,
            &[("reason", reason), ("bank_slots", "40"), ("cash", "500")],
        );
        assert!(client.saw_text(line), "{reason}: {line}");
    }
}

/// An open verdict with no offer on the session (the dialog was answered
/// twice, or after a re-open): WARN `expand_rejected reason=no_offer`.
#[tokio::test]
async fn an_answer_without_an_offer_buys_nothing() {
    let pool = require_db_or_skip!();
    let c = caller(0x70, 0x7000_BBE7);
    setup(&pool, c, 40, 500).await;
    let client = in_world(c, 40906);
    let capture = LogCapture::install();

    expand(&pool, &client, c, None, AT_BANKER).await;
    let after = row(&pool, c.player_id).await;
    cleanup(&pool, c).await;

    assert_eq!(after, (40, 500));
    let e = one(
        &capture,
        "expand_rejected",
        Level::WARN,
        c,
        &[("reason", "no_offer"), ("bank_slots", "40")],
    );
    assert!(!e.fields.contains_key("offered_slots"), "{e:#?}");
    assert!(client.saw_text("Talk to a Banker again to expand your vault. Nothing was charged."));
}

/// A character id with no row: WARN `reason=player_row_missing`.
#[tokio::test]
async fn a_missing_player_row_is_refused() {
    let pool = require_db_or_skip!();
    let c = caller(0x80, 0x7000_BBE8);
    cleanup(&pool, c).await;
    let client = in_world(c, 40907);
    let capture = LogCapture::install();

    expand(&pool, &client, c, offered(40), AT_BANKER).await;

    one(
        &capture,
        "expand_rejected",
        Level::WARN,
        c,
        &[("reason", "player_row_missing")],
    );
    assert!(client.saw_text("Your vault could not be expanded right now. Nothing was charged."));
}

/// No pool: WARN `reason=db_unavailable`, and a line.
#[tokio::test]
async fn no_pool_logs_db_unavailable() {
    let c = caller(0x90, 0x7000_BBE9);
    let client = in_world(c, 40908);
    let capture = LogCapture::install();

    handle_expand(
        c,
        offered(40),
        AT_BANKER,
        &None,
        &client.dyn_transport,
        &client.conn,
    )
    .await;

    one(
        &capture,
        "expand_rejected",
        Level::WARN,
        c,
        &[("reason", "db_unavailable"), ("offered_slots", "40")],
    );
    assert!(client.saw_text("Your vault could not be expanded right now. Nothing was charged."));
}

/// A pool that cannot connect: WARN `reason=query_failed` with the error.
#[tokio::test]
async fn an_unreachable_database_logs_query_failed() {
    let unreachable = sqlx::postgres::PgPoolOptions::new()
        .acquire_timeout(std::time::Duration::from_millis(200))
        .connect_lazy("postgres://nobody:nothing@127.0.0.1:1/none")
        .expect("lazy pool");
    let c = caller(0xA0, 0x7000_BBEA);
    let client = in_world(c, 40909);
    let capture = LogCapture::install();

    expand(&unreachable, &client, c, offered(40), AT_BANKER).await;

    let e = one(
        &capture,
        "expand_rejected",
        Level::WARN,
        c,
        &[("reason", "query_failed")],
    );
    assert!(e.fields.contains_key("error"), "{e:#?}");
}

/// `price_missing` needs a seed gap, which a shared test database must not
/// get. The classification is pinned in `wire_tests`; this pins the row and
/// the line the refusal path produces for it.
#[tokio::test]
async fn price_missing_logs_its_reason_and_tells_the_player() {
    let c = caller(0xB0, 0x7000_BBEB);
    let client = in_world(c, 40910);
    let sends = Client {
        caller: c,
        transport: &client.dyn_transport,
        connected: &client.conn,
    };
    let capture = LogCapture::install();

    let snapshot = Snapshot {
        bank_slots: Some(40),
        cash: Some(500),
        price: None,
    };
    reject(
        &sends,
        ExpandRefusal::PriceMissing,
        &AT_BANKER,
        offered(40),
        snapshot,
        None,
    )
    .await;

    let e = one(
        &capture,
        "expand_rejected",
        Level::WARN,
        c,
        &[
            ("reason", "price_missing"),
            ("bank_slots", "40"),
            ("cash", "500"),
        ],
    );
    assert!(!e.fields.contains_key("price"), "{e:#?}");
    assert!(client.saw_text("Your vault could not be expanded right now. Nothing was charged."));
}

/// The character logged off before the answer: the purchase still commits,
/// and the dropped sends log `bank_feedback_send_failed
/// reason=no_client_address`.
#[tokio::test]
async fn a_player_with_no_client_address_logs_the_dropped_sends() {
    let pool = require_db_or_skip!();
    let c = caller(0xC0, 0x7000_BBEC);
    setup(&pool, c, 40, 100).await;
    // A session playing another character only.
    let client = in_world_as(0x7000_BBEF, c.player_id + 0x100, 40911);
    let capture = LogCapture::install();

    expand(&pool, &client, c, offered(40), AT_BANKER).await;
    let after = row(&pool, c.player_id).await;
    cleanup(&pool, c).await;

    assert_eq!(after, (50, 0));
    let dropped = bank_rows(&capture, "bank_feedback_send_failed");
    assert_eq!(dropped.len(), 3, "bag info, cash, line: {dropped:#?}");
    for d in &dropped {
        assert_eq!(d.level, Level::WARN);
        assert!(d.has_field("reason", "no_client_address"), "{d:#?}");
        assert!(d.has_field("player_id", &c.player_id.to_string()), "{d:#?}");
    }
}

/// The entity id the cell sent now belongs to **another** character's
/// session (the buyer gated and the id was reused): the purchase commits,
/// and nothing reaches that session, neither the buyer's vault size nor the
/// buyer's balance. Each dropped send logs `bank_feedback_send_failed
/// reason=no_client_address`. Fails if the sends are addressed by entity id.
#[tokio::test]
async fn a_recycled_entity_id_receives_nothing() {
    let pool = require_db_or_skip!();
    let c = caller(0xD0, 0x7000_BBED);
    setup(&pool, c, 40, 300).await;
    // The same entity id, played by someone else.
    let other = in_world_as(c.entity_id, c.player_id + 0x100, 40912);
    let capture = LogCapture::install();

    expand(&pool, &other, c, offered(40), AT_BANKER).await;
    let after = row(&pool, c.player_id).await;
    cleanup(&pool, c).await;

    assert_eq!(after, (50, 200), "the purchase itself commits");
    assert_eq!(
        other.sent(),
        0,
        "the other character's session gets nothing"
    );
    let dropped = bank_rows(&capture, "bank_feedback_send_failed");
    assert_eq!(dropped.len(), 3, "{dropped:#?}");
}

/// The price was retuned while the offer was open: nothing is charged,
/// WARN `expand_rejected reason=price_changed`. The seed price stays 100;
/// the offer claims 90.
#[tokio::test]
async fn a_price_the_player_was_not_shown_is_never_charged() {
    let pool = require_db_or_skip!();
    let c = caller(0xE0, 0x7000_BBEE);
    setup(&pool, c, 40, 300).await;
    let client = in_world(c, 40913);
    let capture = LogCapture::install();

    let shown = Some(ExpansionOffer {
        from_slots: 40,
        price: 90,
    });
    expand(&pool, &client, c, shown, AT_BANKER).await;
    let after = row(&pool, c.player_id).await;
    cleanup(&pool, c).await;

    assert_eq!(after, (40, 300));
    one(
        &capture,
        "expand_rejected",
        Level::WARN,
        c,
        &[
            ("reason", "price_changed"),
            ("offered_price", "90"),
            ("price", "100"),
        ],
    );
    assert!(client.saw_text("Talk to a Banker again to expand your vault. Nothing was charged."));
}
