//! Live-DB guards for trading from the crafting bag (15).
//!
//! - A crafting component (`container_sets` `{17,15}`) offered from bag 15
//!   lands in the recipient's bag 15; a backpack item in the same trade
//!   still lands in the backpack.
//! - A full destination bag refuses the whole trade: nothing moves, no
//!   cash changes hands, and both players get a line naming the bag.
//! - Equipment, buyback and vault items stay refused.
//! - A trade and a crafting completion on the same player serialize on
//!   the shared inventory locks instead of deadlocking.
//!
//! Sentinels: accounts, players and entities `0x7000_C500..=0x7000_C58F`
//! (16 ids per fixture, fixtures 0-8).

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use cimmeria_entity::inventory::{
    INV_AUCTION, INV_BUYBACK, INV_COMMAND_BANK, INV_CRAFTING, INV_HEAD, INV_MAIN, INV_TEAM_BANK,
};
use cimmeria_entity::trade::serialize_on_trade_results;
use cimmeria_mercury::encryption::{EncryptionVersion, MercuryEncryption};
use cimmeria_mercury::transport::Transport;
use sqlx::PgPool;
use tracing::Level;

use super::{cleanup, insert_account_and_player, insert_item, naquadah_of, owner_of};
use crate::base::crafting::inventory_locks::take_inventory_locks;
use crate::base::world_entry::methods::trade::handle_execute_trade;
use crate::base::ConnectedClientState;
use crate::mercury::{build_player_entity_method_packet, method_idx};
use crate::test_support::{
    require_db_or_skip, test_default_connected_client_state, LogCapture, TestTransport,
};

const SENTINEL_BASE: i32 = 0x7000_C500;

#[derive(Debug, Clone, Copy)]
struct Ids {
    account_a: i32,
    account_b: i32,
    /// Always the lower player id, so a trade takes A's locks first.
    player_a: i32,
    player_b: i32,
    entity_a: u32,
    entity_b: u32,
}

fn ids(fixture: i32) -> Ids {
    let base = SENTINEL_BASE + fixture * 16;
    Ids {
        account_a: base,
        account_b: base + 1,
        player_a: base + 2,
        player_b: base + 3,
        entity_a: (base + 4) as u32,
        entity_b: (base + 5) as u32,
    }
}

async fn setup(pool: &PgPool, f: Ids, naquadah: i32) {
    cleanup(pool, &[f.account_a, f.account_b], &[f.player_a, f.player_b]).await;
    insert_account_and_player(pool, f.account_a, f.player_a, naquadah, "cr-a").await;
    insert_account_and_player(pool, f.account_b, f.player_b, naquadah, "cr-b").await;
}

async fn teardown(pool: &PgPool, f: Ids) {
    cleanup(pool, &[f.account_a, f.account_b], &[f.player_a, f.player_b]).await;
}

/// A seeded crafting component: `container_sets` `{17,15}`.
async fn component_type(pool: &PgPool) -> i32 {
    sqlx::query_scalar(
        "SELECT item_id FROM resources.items WHERE container_sets = '{17,15}' \
         ORDER BY item_id LIMIT 1",
    )
    .fetch_one(pool)
    .await
    .expect("the seed has a {17,15} crafting component")
}

/// A seeded backpack item: `container_sets` `{1,17}`.
async fn backpack_type(pool: &PgPool) -> i32 {
    sqlx::query_scalar(
        "SELECT item_id FROM resources.items WHERE container_sets = '{1,17}' \
         ORDER BY item_id LIMIT 1",
    )
    .fetch_one(pool)
    .await
    .expect("the seed has a {1,17} item")
}

async fn place_of(pool: &PgPool, item_id: i32) -> (i32, i32, i32) {
    sqlx::query_as(
        "SELECT character_id, container_id, slot_id FROM sgw_inventory WHERE item_id = $1",
    )
    .bind(item_id)
    .fetch_one(pool)
    .await
    .expect("read item place")
}

/// Fill slots `0..slots` of `container_id` with one-item stacks.
async fn fill_bag(pool: &PgPool, player_id: i32, type_id: i32, container_id: i32, slots: i32) {
    sqlx::query(
        "INSERT INTO sgw_inventory (character_id, type_id, stack_size, slot_id, container_id, \
                                    bound, durability, charges) \
         SELECT $1, $2, 1, s, $3, false, 100, 0 FROM generate_series(0, $4 - 1) s",
    )
    .bind(player_id)
    .bind(type_id)
    .bind(container_id)
    .bind(slots)
    .execute(pool)
    .await
    .expect("fill the bag");
}

type Connected = Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>;
type EntityToAddr = Arc<Mutex<HashMap<u32, SocketAddr>>>;

const ADDR_A: &str = "127.0.0.1:41701";
const ADDR_B: &str = "127.0.0.1:41702";

/// Both players in world, so the refusal lines are really sent.
fn in_world(
    f: Ids,
) -> (
    Arc<TestTransport>,
    Arc<dyn Transport>,
    Connected,
    EntityToAddr,
) {
    let transport = Arc::new(TestTransport::new());
    let dyn_transport: Arc<dyn Transport> = transport.clone();
    let mut connected = HashMap::new();
    let mut e2a = HashMap::new();
    for (addr, entity, account) in [
        (ADDR_A, f.entity_a, f.account_a),
        (ADDR_B, f.entity_b, f.account_b),
    ] {
        let addr: SocketAddr = addr.parse().unwrap();
        let mut state = test_default_connected_client_state();
        state.player_entity_id = Some(entity);
        state.account_id = account as u32;
        connected.insert(addr, state);
        e2a.insert(entity, addr);
    }
    (
        transport,
        dyn_transport,
        Arc::new(Mutex::new(connected)),
        Arc::new(Mutex::new(e2a)),
    )
}

/// Whether any packet sent to `addr` carries `text` (feedback lines are
/// UTF-16LE on the wire). The test sessions use the all-zero key.
fn sent_text(transport: &TestTransport, addr: &str, text: &str) -> bool {
    let enc = MercuryEncryption::from_session_key([0u8; 32]);
    let needle: Vec<u8> = text.encode_utf16().flat_map(u16::to_le_bytes).collect();
    transport
        .filter_to(addr.parse().unwrap())
        .iter()
        .filter_map(|p| enc.decrypt(p).ok())
        .any(|pt| pt.windows(needle.len()).any(|w| w == needle.as_slice()))
}

/// The `onTradeResults` codes (1-6) sent to `addr`, each matched
/// byte-for-byte against the packet the server builds for
/// `(entity_id, partner, code)` at the observed sequence number.
fn trade_results_sent(
    transport: &TestTransport,
    addr: &str,
    entity_id: u32,
    partner_entity_id: u32,
) -> Vec<i32> {
    let key = [0u8; 32];
    let enc = MercuryEncryption::from_session_key(key);
    let mut codes = Vec::new();
    for packet in transport.filter_to(addr.parse().unwrap()) {
        let Ok(pt) = enc.decrypt(&packet) else {
            continue;
        };
        let seq = u32::from_le_bytes(pt[pt.len() - 4..].try_into().unwrap());
        for code in 1..=6 {
            let expected = build_player_entity_method_packet(
                &key,
                seq,
                &[],
                entity_id,
                method_idx::ON_TRADE_RESULTS,
                &serialize_on_trade_results(partner_entity_id as i32, code),
                EncryptionVersion::V1,
            );
            if enc.decrypt(&expected).ok().as_deref() == Some(pt.as_slice()) {
                codes.push(code);
            }
        }
    }
    codes
}

async fn run_trade(
    pool: &PgPool,
    f: Ids,
    a_items: Vec<i32>,
    a_cash: i32,
    b_items: Vec<i32>,
    b_cash: i32,
    transport: &Arc<dyn Transport>,
    connected: &Connected,
    e2a: &EntityToAddr,
) {
    handle_execute_trade(
        f.entity_a,
        f.player_a,
        f.entity_b,
        f.player_b,
        a_items,
        a_cash,
        b_items,
        b_cash,
        &Some(Arc::new(pool.clone())),
        transport,
        connected,
        e2a,
    )
    .await;
}

/// A component offered from bag 15 lands in the recipient's bag 15, at
/// its lowest free slot; the backpack item coming back lands in the
/// backpack. The move event names both bags.
///
/// Revert-verifier: with `TRADEABLE_CONTAINERS = [INV_MAIN]` the trade is
/// refused and the component stays with A; with the destination forced
/// to `INV_MAIN` it lands in B's backpack.
#[tokio::test]
async fn live_db_crafting_component_lands_in_the_recipients_crafting_bag() {
    let pool = require_db_or_skip!();
    let f = ids(0);
    setup(&pool, f, 0).await;
    let component = component_type(&pool).await;
    let backpack = backpack_type(&pool).await;

    let offered = insert_item(&pool, f.player_a, component, INV_CRAFTING, 7, false).await;
    // B's crafting bag slot 0 is taken, so the lowest free slot is 1.
    insert_item(&pool, f.player_b, component, INV_CRAFTING, 0, false).await;
    let returned = insert_item(&pool, f.player_b, backpack, INV_MAIN, 3, false).await;

    let (_t, transport, connected, e2a) = in_world(f);
    let capture = LogCapture::install();
    run_trade(
        &pool,
        f,
        vec![offered],
        0,
        vec![returned],
        0,
        &transport,
        &connected,
        &e2a,
    )
    .await;

    assert_eq!(
        place_of(&pool, offered).await,
        (f.player_b, INV_CRAFTING, 1),
        "the component must land in B's crafting bag at its lowest free slot"
    );
    assert_eq!(
        place_of(&pool, returned).await,
        (f.player_a, INV_MAIN, 0),
        "the backpack item must still land in A's backpack"
    );
    let moved = capture
        .all()
        .into_iter()
        .find(|c| {
            c.has_field("event", "trade.item_moved") && c.has_field("item_id", &offered.to_string())
        })
        .expect("trade.item_moved for the component");
    assert_eq!(moved.level, Level::INFO);
    for (key, value) in [
        ("container_before", "15"),
        ("slot_before", "7"),
        ("container_after", "15"),
        ("slot_after", "1"),
        ("player_id", &f.player_a.to_string()),
        ("target_player_id", &f.player_b.to_string()),
        ("account_id", &f.account_a.to_string()),
        ("target_account_id", &f.account_b.to_string()),
    ] {
        assert!(moved.has_field(key, value), "{key} = {value} in {moved:?}");
    }

    teardown(&pool, f).await;
}

/// B's crafting bag is full (100/100) and their backpack is empty. The
/// component A offers has nowhere to go, so the whole trade is refused:
/// both items stay put, no naquadah moves, and each player gets a line
/// naming the crafting bag.
///
/// Both clients get `onTradeResults(Cancelled)`, byte-exact: the client's
/// trade window closes only on Completed or Cancelled.
///
/// Revert-verifier: forcing the destination to `INV_MAIN` makes the trade
/// succeed into B's empty backpack; dropping the refusal lines leaves
/// both clients without the cause; sending the space codes (3/4) again
/// fails the result-code assertions.
#[tokio::test]
async fn live_db_full_destination_crafting_bag_refuses_the_whole_trade() {
    let pool = require_db_or_skip!();
    let f = ids(1);
    setup(&pool, f, 500).await;
    let component = component_type(&pool).await;
    let backpack = backpack_type(&pool).await;

    let offered = insert_item(&pool, f.player_a, component, INV_CRAFTING, 0, false).await;
    fill_bag(&pool, f.player_b, component, INV_CRAFTING, 100).await;
    let returned = insert_item(&pool, f.player_b, backpack, INV_MAIN, 5, false).await;

    let (sent, transport, connected, e2a) = in_world(f);
    let capture = LogCapture::install();
    run_trade(
        &pool,
        f,
        vec![offered],
        100,
        vec![returned],
        0,
        &transport,
        &connected,
        &e2a,
    )
    .await;

    assert_eq!(
        place_of(&pool, offered).await,
        (f.player_a, INV_CRAFTING, 0)
    );
    assert_eq!(place_of(&pool, returned).await, (f.player_b, INV_MAIN, 5));
    assert_eq!(naquadah_of(&pool, f.player_a).await, 500);
    assert_eq!(naquadah_of(&pool, f.player_b).await, 500);

    // Cancelled (2) to both sides, never a space code the client ignores.
    assert_eq!(
        trade_results_sent(&sent, ADDR_A, f.entity_a, f.entity_b),
        vec![2],
        "A gets exactly one onTradeResults, Cancelled"
    );
    assert_eq!(
        trade_results_sent(&sent, ADDR_B, f.entity_b, f.entity_a),
        vec![2],
        "B gets exactly one onTradeResults, Cancelled"
    );
    assert!(
        sent_text(
            &sent,
            ADDR_B,
            super::super::execute::LOCAL_CRAFTING_BAG_FULL
        ),
        "B (the full bag) must be told"
    );
    assert!(
        sent_text(
            &sent,
            ADDR_A,
            super::super::execute::REMOTE_CRAFTING_BAG_FULL
        ),
        "A (the partner) must be told"
    );
    let refused = capture
        .find_event(Level::WARN, "atomic swap failed", "crafting_bag_full")
        .expect("trade.refused with reason crafting_bag_full");
    assert!(refused.has_field("container_id", "15"), "{refused:?}");
    assert!(refused.has_field("account_id", &f.account_a.to_string()));
    assert!(refused.has_field("target_account_id", &f.account_b.to_string()));

    teardown(&pool, f).await;
}

/// Equipment, buyback and every vault stay refused now that bag 15 is a
/// source; the offerer is told the item cannot be traded.
#[tokio::test]
async fn live_db_equipment_buyback_and_vault_items_stay_refused() {
    let pool = require_db_or_skip!();
    let backpack = backpack_type(&pool).await;
    let cases = [
        ("equipment (head)", INV_HEAD, 2),
        ("buyback", INV_BUYBACK, 3),
        ("auction", INV_AUCTION, 4),
        ("team vault", INV_TEAM_BANK, 5),
        ("command vault", INV_COMMAND_BANK, 6),
    ];
    for (label, container, fixture) in cases {
        let f = ids(fixture);
        setup(&pool, f, 0).await;
        let bad = insert_item(&pool, f.player_a, backpack, container, 0, false).await;
        let good = insert_item(&pool, f.player_b, backpack, INV_MAIN, 0, false).await;

        let (sent, transport, connected, e2a) = in_world(f);
        run_trade(
            &pool,
            f,
            vec![bad],
            0,
            vec![good],
            0,
            &transport,
            &connected,
            &e2a,
        )
        .await;

        assert_eq!(
            place_of(&pool, bad).await,
            (f.player_a, container, 0),
            "{label}"
        );
        assert_eq!(owner_of(&pool, good).await, Some(f.player_b), "{label}");
        assert!(
            sent_text(&sent, ADDR_A, super::super::execute::LOCAL_UNTRADEABLE_ITEM),
            "{label}: the offerer must be told"
        );
        teardown(&pool, f).await;
    }
}

/// A refusal line for a partner whose entity has no address is not lost
/// silently: `trade.feedback_send_failed` names the entity and the
/// refusal. The offerer, still mapped, gets their line.
///
/// Revert-verifier: dropping the WARN in `send_refusal_line` leaves the
/// miss unlogged and the `find_event` below fails.
#[tokio::test]
async fn live_db_refusal_line_to_an_unmapped_partner_is_logged() {
    let pool = require_db_or_skip!();
    let f = ids(8);
    setup(&pool, f, 0).await;
    let backpack = backpack_type(&pool).await;
    let bad = insert_item(&pool, f.player_a, backpack, INV_HEAD, 0, false).await;

    let (sent, transport, connected, e2a) = in_world(f);
    e2a.lock().unwrap().remove(&f.entity_b);
    let capture = LogCapture::install();
    run_trade(
        &pool,
        f,
        vec![bad],
        0,
        vec![],
        0,
        &transport,
        &connected,
        &e2a,
    )
    .await;

    let miss = capture
        .find_event(
            Level::WARN,
            "trade refusal line not sent",
            "entity_to_addr_miss",
        )
        .expect("trade.feedback_send_failed for the unmapped partner");
    assert!(miss.has_field("event", "trade.feedback_send_failed"));
    assert!(miss.has_field("entity_id", &f.entity_b.to_string()));
    assert!(miss.has_field("player_id", &f.player_b.to_string()));
    assert!(miss.has_field("refusal", "ineligible_container"));
    assert!(sent_text(
        &sent,
        ADDR_A,
        super::super::execute::LOCAL_UNTRADEABLE_ITEM
    ));

    teardown(&pool, f).await;
}

/// A crafting completion on A is mid-transaction: it holds A's inventory
/// locks (`take_inventory_locks`, key 0 then bag 15) and the component
/// row it consumes, and will lock A's `sgw_player` row next. A trade
/// offering that component starts meanwhile. With the shared lock order
/// the trade waits at A's key 0, the completion finishes, and the trade
/// then runs.
///
/// Revert-verifier: with trade's old order (bag-1 key only, player rows
/// before item rows) the trade takes A's player row and waits on the
/// component row, the completion waits on the player row, and Postgres
/// aborts one of them as a deadlock: either the completion's commit fails
/// or the component never moves.
#[tokio::test]
async fn live_db_trade_and_crafting_completion_on_one_player_serialize() {
    let pool = require_db_or_skip!();
    let f = ids(7);
    setup(&pool, f, 0).await;
    let component = component_type(&pool).await;
    let backpack = backpack_type(&pool).await;
    let offered = insert_item(&pool, f.player_a, component, INV_CRAFTING, 0, false).await;
    let returned = insert_item(&pool, f.player_b, backpack, INV_MAIN, 0, false).await;

    // The completion, first half: locks in the crafting order.
    let mut completion = pool.begin().await.expect("begin completion");
    let completion_pid: i32 = sqlx::query_scalar("SELECT pg_backend_pid()")
        .fetch_one(&mut *completion)
        .await
        .unwrap();
    take_inventory_locks(&mut completion, f.player_a, &[INV_CRAFTING])
        .await
        .expect("completion advisory locks");
    sqlx::query("SELECT 1 FROM sgw_inventory WHERE item_id = $1 FOR UPDATE")
        .bind(offered)
        .execute(&mut *completion)
        .await
        .expect("completion locks the component row");

    let trade = tokio::spawn({
        let pool = pool.clone();
        async move {
            let (_sent, transport, connected, e2a) = in_world(f);
            run_trade(
                &pool,
                f,
                vec![offered],
                0,
                vec![returned],
                0,
                &transport,
                &connected,
                &e2a,
            )
            .await;
        }
    });

    // Wait until the trade is blocked on a lock, so the two really
    // overlap; a trade that never blocks makes this test vacuous.
    let mut blocked = false;
    for _ in 0..200 {
        let waiting: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM pg_stat_activity \
             WHERE datname = current_database() AND wait_event_type = 'Lock' \
               AND pid <> pg_backend_pid() AND pid <> $1",
        )
        .bind(completion_pid)
        .fetch_one(&pool)
        .await
        .unwrap();
        if waiting > 0 {
            blocked = true;
            break;
        }
        tokio::time::sleep(Duration::from_millis(25)).await;
    }
    assert!(blocked, "the trade never waited on the completion's locks");

    // The completion, second half: the player row, the consume, commit.
    sqlx::query("SELECT naquadah FROM sgw_player WHERE player_id = $1 FOR UPDATE")
        .bind(f.player_a)
        .execute(&mut *completion)
        .await
        .expect("completion locks A's player row without a deadlock");
    sqlx::query("UPDATE sgw_player SET naquadah = naquadah WHERE player_id = $1")
        .bind(f.player_a)
        .execute(&mut *completion)
        .await
        .expect("completion writes A's player row");
    completion.commit().await.expect("completion commits");

    tokio::time::timeout(Duration::from_secs(20), trade)
        .await
        .expect("the trade finishes once the completion commits")
        .expect("trade task");
    assert_eq!(
        place_of(&pool, offered).await,
        (f.player_b, INV_CRAFTING, 0),
        "the trade ran after the completion"
    );

    teardown(&pool, f).await;
}
