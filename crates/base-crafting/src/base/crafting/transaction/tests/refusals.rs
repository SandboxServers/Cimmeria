//! Refused transactions: nothing is consumed or granted, the player reads
//! why, and the client's inventory is resynced.

use super::*;
use crate::base::crafting::test_packets::{feedback_text, update_item_rows};
use crate::mercury::method_idx;
use crate::test_support::require_db_or_skip;

/// The `persist_failed` WARN of a rollback: its phase and reason, and the
/// player's identity.
fn assert_persist_failed(
    capture: &crate::test_support::LogCaptureGuard,
    f: &Fixture,
    phase: &str,
    reason: &str,
) {
    let e = capture
        .find_event(tracing::Level::WARN, "rolled back", reason)
        .unwrap_or_else(|| panic!("persist_failed {reason}: {:#?}", capture.all()));
    assert_eq!(e.target, "crafting");
    assert!(e.has_field("event", "persist_failed"));
    assert!(e.has_field("phase", phase), "{e:#?}");
    assert!(
        e.has_field("account_id", &f.account_id.to_string()),
        "{e:#?}"
    );
    assert!(e.has_field("player_id", &f.player_id.to_string()), "{e:#?}");
    assert!(e.has_field("entity_id", &f.entity_id.to_string()), "{e:#?}");
}

/// The refusal is the feedback line, then a full `onUpdateItem` of every
/// item the player holds.
fn assert_refused_with(f: &Fixture, text: &str, items_held: usize) {
    let calls = f.calls();
    assert_eq!(calls.len(), 2, "feedback line, then resync: {calls:?}");
    assert_eq!(calls[0].method, method_idx::ON_PLAYER_COMMUNICATION);
    assert_eq!(feedback_text(&calls[0]), text);
    assert_eq!(calls[1].method, method_idx::ON_UPDATE_ITEM);
    assert_eq!(update_item_rows(&calls[1]).len(), items_held);
}

/// A full crafting bag rolls back the consumption that already ran in the
/// same transaction.
#[tokio::test]
async fn live_db_full_bag_rolls_back_and_tells_the_player() {
    let pool = require_db_or_skip!();
    assert_seed_shape(&pool).await;
    let f = Fixture::new(&pool, 4).await;
    sqlx::query(
        "INSERT INTO sgw_inventory \
            (character_id, type_id, stack_size, slot_id, container_id, bound, durability, charges) \
         SELECT $1, $2, 1, s, $3, false, 100, 0 FROM generate_series(0, 99) AS s",
    )
    .bind(f.player_id)
    .bind(FILLER)
    .bind(INV_CRAFTING)
    .execute(&pool)
    .await
    .expect("fill the crafting bag");
    let component = f.stack(COMPONENT, INV_MAIN, 0, 2).await;

    let result = f
        .apply(&CraftTransaction {
            named_items: vec![NamedItem::new(component, COMPONENT)],
            consume_named: vec![],
            consume: vec![(COMPONENT, 2)],
            grant: vec![(BANK_FIRST_PRODUCT, 1)],
            expertise: vec![],
            learn_blueprints: vec![],
            required_knowledge: None,
            research: None,
        })
        .await;

    assert_eq!(
        result,
        Err(CraftReject::InventoryFull {
            design_id: BANK_FIRST_PRODUCT,
            container_id: INV_CRAFTING
        })
    );
    assert_eq!(
        f.row(component).await,
        Some((2, INV_MAIN)),
        "nothing consumed"
    );
    assert!(f.stacks_of(BANK_FIRST_PRODUCT).await.is_empty());
    assert_eq!(f.outbox_rows().await, 0);
    assert_refused_with(
        &f,
        "Not enough room in your bags for the result. Nothing was used.",
        101,
    );
    f.cleanup().await;
}

/// The player moved a named component to the bank during the induction:
/// the completion re-checks it and refuses, though the crafting bag alone
/// would cover the quantity.
#[tokio::test]
async fn live_db_a_component_moved_to_the_bank_before_completion_rolls_back() {
    let pool = require_db_or_skip!();
    assert_seed_shape(&pool).await;
    let f = Fixture::new(&pool, 5).await;
    let other = f.stack(COMPONENT, INV_CRAFTING, 0, 2).await;
    let named = f.stack(COMPONENT, INV_CRAFTING, 1, 1).await;
    sqlx::query("UPDATE sgw_inventory SET container_id = $1, slot_id = 0 WHERE item_id = $2")
        .bind(INV_BANK)
        .bind(named)
        .execute(&pool)
        .await
        .expect("move to the bank");

    let result = f
        .apply(&CraftTransaction {
            named_items: vec![NamedItem::new(named, COMPONENT)],
            consume_named: vec![],
            consume: vec![(COMPONENT, 1)],
            grant: vec![(BANK_FIRST_PRODUCT, 1)],
            expertise: vec![],
            learn_blueprints: vec![],
            required_knowledge: None,
            research: None,
        })
        .await;

    assert_eq!(
        result,
        Err(CraftReject::ComponentNotInCraftingBags {
            item_id: named,
            container_id: INV_BANK
        })
    );
    assert_eq!(f.row(other).await, Some((2, INV_CRAFTING)));
    assert_eq!(f.row(named).await, Some((1, INV_BANK)));
    assert!(f.stacks_of(BANK_FIRST_PRODUCT).await.is_empty());
    assert_refused_with(
        &f,
        "Components must be in your backpack or crafting bag. Nothing was used.",
        2,
    );
    f.cleanup().await;
}

/// Only the main and crafting bags feed a craft: a bank stack does not
/// make up a shortfall, and is not touched.
#[tokio::test]
async fn live_db_bank_stacks_do_not_count_toward_consumption() {
    let pool = require_db_or_skip!();
    assert_seed_shape(&pool).await;
    let f = Fixture::new(&pool, 6).await;
    let named = f.stack(COMPONENT, INV_CRAFTING, 0, 1).await;
    let banked = f.stack(COMPONENT, INV_BANK, 0, 2).await;

    let result = f
        .apply(&CraftTransaction {
            named_items: vec![NamedItem::new(named, COMPONENT)],
            consume_named: vec![],
            consume: vec![(COMPONENT, 2)],
            grant: vec![(BANK_FIRST_PRODUCT, 1)],
            expertise: vec![],
            learn_blueprints: vec![],
            required_knowledge: None,
            research: None,
        })
        .await;

    assert_eq!(
        result,
        Err(CraftReject::NotEnoughComponents {
            design_id: COMPONENT,
            needed: 2,
            available: 1
        })
    );
    assert_eq!(f.row(named).await, Some((1, INV_CRAFTING)));
    assert_eq!(f.row(banked).await, Some((2, INV_BANK)));
    assert_refused_with(
        &f,
        "You do not have enough components. Nothing was used.",
        2,
    );
    f.cleanup().await;
}

/// A request naming another player's item is refused as missing; the
/// other player's stack is untouched.
#[tokio::test]
async fn live_db_a_component_of_another_player_is_missing() {
    let pool = require_db_or_skip!();
    assert_seed_shape(&pool).await;
    let f = Fixture::new(&pool, 7).await;
    let victim = Fixture::new(&pool, 8).await;
    let theirs = victim.stack(COMPONENT, INV_CRAFTING, 0, 2).await;
    let mine = f.stack(COMPONENT, INV_CRAFTING, 0, 2).await;

    let result = f
        .apply(&CraftTransaction {
            named_items: vec![NamedItem::new(theirs, COMPONENT)],
            consume_named: vec![],
            consume: vec![(COMPONENT, 1)],
            grant: vec![(BANK_FIRST_PRODUCT, 1)],
            expertise: vec![],
            learn_blueprints: vec![],
            required_knowledge: None,
            research: None,
        })
        .await;

    assert_eq!(
        result,
        Err(CraftReject::ComponentMissing { item_id: theirs })
    );
    assert_eq!(victim.row(theirs).await, Some((2, INV_CRAFTING)));
    assert_eq!(f.row(mine).await, Some((2, INV_CRAFTING)));
    assert_refused_with(
        &f,
        "A component is no longer in your inventory. Nothing was used.",
        1,
    );
    victim.cleanup().await;
    f.cleanup().await;
}

/// A product whose `container_sets` allow no carried bag is refused
/// before anything is consumed.
#[tokio::test]
async fn live_db_a_product_with_no_carried_bag_is_refused() {
    let pool = require_db_or_skip!();
    assert_seed_shape(&pool).await;
    let f = Fixture::new(&pool, 9).await;
    let component = f.stack(COMPONENT, INV_CRAFTING, 0, 2).await;

    let result = f
        .apply(&CraftTransaction {
            named_items: vec![NamedItem::new(component, COMPONENT)],
            consume_named: vec![],
            consume: vec![(COMPONENT, 1)],
            grant: vec![(MISSION_ONLY, 1)],
            expertise: vec![],
            learn_blueprints: vec![],
            required_knowledge: None,
            research: None,
        })
        .await;

    assert_eq!(
        result,
        Err(CraftReject::NoCarriedBagForProduct {
            design_id: MISSION_ONLY
        })
    );
    assert_eq!(f.row(component).await, Some((2, INV_CRAFTING)));
    assert_refused_with(
        &f,
        "The result cannot be placed in your bags. Nothing was used.",
        1,
    );
    f.cleanup().await;
}

/// No database: the player still reads a line.
#[tokio::test]
async fn no_database_is_reported_to_the_player() {
    use crate::base::crafting::telemetry::{METRIC_REJECTIONS, METRIC_REQUESTS};
    use cimmeria_observability::testing::{counter_total, install as install_meter};

    let capture = crate::test_support::LogCapture::install();
    install_meter();
    let rejections = [
        ("verb", "no_database_probe"),
        ("reason", "induction_failed"),
    ];
    let answered = [("verb", "no_database_probe"), ("outcome", "rejected")];
    let before = (
        counter_total(METRIC_REJECTIONS, &rejections),
        counter_total(METRIC_REQUESTS, &answered),
    );
    let transport = Arc::new(TestTransport::new());
    let dyn_transport: Arc<dyn Transport> = transport.clone();
    let addr: SocketAddr = "127.0.0.1:56100".parse().unwrap();
    let env = InductionEnv {
        db_pool: None,
        cell_tx: None,
        transport: dyn_transport,
        connected: Arc::new(Mutex::new(HashMap::from([(
            addr,
            test_default_connected_client_state(),
        )]))),
        entity_to_addr: Arc::new(Mutex::new(HashMap::from([(77, addr)]))),
    };

    let ids = JobIds {
        job_id: 0,
        verb: "no_database_probe",
        account_id: 76,
        player_id: 78,
        entity_id: 77,
        gm_entity_id: None,
        gm_name: None,
    };
    let result = apply_craft_transaction(&env, &ids, &CraftTransaction::default()).await;

    assert_eq!(result, Err(CraftReject::InductionFailed));
    let calls = decode_all(&transport.filter_to(addr));
    assert_eq!(calls.len(), 1);
    assert_eq!(
        feedback_text(&calls[0]),
        "Crafting failed. Nothing was used."
    );
    let e = capture
        .find_event(tracing::Level::WARN, "rolled back", "no_database")
        .expect("persist_failed no_database");
    assert!(e.has_field("event", "persist_failed"));
    assert!(e.has_field("phase", "begin"));
    assert!(e.has_field("account_id", "76"), "{e:#?}");
    assert!(e.has_field("player_id", "78"), "{e:#?}");
    assert!(e.has_field("entity_id", "77"), "{e:#?}");
    let e = capture
        .find_event(
            tracing::Level::INFO,
            "crafting request rejected",
            "induction_failed",
        )
        .expect("rejected induction_failed");
    assert!(e.has_field("event", "rejected"));
    // The queued job's account, not whatever the live session map says.
    assert!(e.has_field("account_id", "76"), "{e:#?}");
    assert!(e.has_field("player_id", "78"), "{e:#?}");
    assert!(e.has_field("entity_id", "77"), "{e:#?}");
    // The request was answered when the job was queued: a completion
    // refusal counts the rejection only.
    assert_eq!(counter_total(METRIC_REJECTIONS, &rejections) - before.0, 1);
    assert_eq!(counter_total(METRIC_REQUESTS, &answered) - before.1, 0);
}

/// A non-positive cost is a verb bug, never a free craft: the grant does
/// not happen.
#[tokio::test]
async fn live_db_a_non_positive_consume_quantity_refuses_the_grant() {
    let pool = require_db_or_skip!();
    let capture = crate::test_support::LogCapture::install();
    assert_seed_shape(&pool).await;
    let f = Fixture::new(&pool, 11).await;
    let component = f.stack(COMPONENT, INV_CRAFTING, 0, 2).await;

    let result = f
        .apply(&CraftTransaction {
            named_items: vec![NamedItem::new(component, COMPONENT)],
            consume_named: vec![],
            consume: vec![(COMPONENT, -1)],
            grant: vec![(BANK_FIRST_PRODUCT, 1)],
            expertise: vec![],
            learn_blueprints: vec![],
            required_knowledge: None,
            research: None,
        })
        .await;

    assert_eq!(result, Err(CraftReject::InductionFailed));
    assert_eq!(f.row(component).await, Some((2, INV_CRAFTING)));
    assert!(f.stacks_of(BANK_FIRST_PRODUCT).await.is_empty());
    assert_refused_with(&f, "Crafting failed. Nothing was used.", 1);
    assert_persist_failed(&capture, &f, "consume", "invalid_quantity");
    f.cleanup().await;
}

/// A product id the item table does not know is a data error: refused,
/// nothing consumed.
#[tokio::test]
async fn live_db_an_unknown_product_is_refused() {
    let pool = require_db_or_skip!();
    let capture = crate::test_support::LogCapture::install();
    let f = Fixture::new(&pool, 12).await;
    let component = f.stack(COMPONENT, INV_CRAFTING, 0, 2).await;

    let result = f
        .apply(&CraftTransaction {
            named_items: vec![NamedItem::new(component, COMPONENT)],
            consume_named: vec![],
            consume: vec![(COMPONENT, 1)],
            grant: vec![(TEST_BASE + 0xFFF, 1)],
            expertise: vec![],
            learn_blueprints: vec![],
            required_knowledge: None,
            research: None,
        })
        .await;

    assert_eq!(result, Err(CraftReject::InductionFailed));
    assert_eq!(f.row(component).await, Some((2, INV_CRAFTING)));
    assert_refused_with(&f, "Crafting failed. Nothing was used.", 1);
    assert_persist_failed(&capture, &f, "resolve", "unknown_product");
    f.cleanup().await;
}

/// A database failure (here: the player row is gone) rolls back and is
/// reported as a failed craft.
#[tokio::test]
async fn live_db_a_database_failure_rolls_back_and_is_reported() {
    let pool = require_db_or_skip!();
    let capture = crate::test_support::LogCapture::install();
    let f = Fixture::new(&pool, 13).await;
    sqlx::query("DELETE FROM sgw_player WHERE player_id = $1")
        .bind(f.player_id)
        .execute(&pool)
        .await
        .expect("delete the player row");

    let result = f
        .apply(&CraftTransaction {
            grant: vec![(BANK_FIRST_PRODUCT, 1)],
            ..CraftTransaction::default()
        })
        .await;

    assert_eq!(result, Err(CraftReject::InductionFailed));
    assert!(f.stacks_of(BANK_FIRST_PRODUCT).await.is_empty());
    assert_refused_with(&f, "Crafting failed. Nothing was used.", 0);
    assert_persist_failed(&capture, &f, "check_player", "player_missing");
    f.cleanup().await;
}

/// An inventory read that fails is logged, and nothing is sent: an empty
/// list would be read as an empty inventory.
#[tokio::test]
async fn a_failed_inventory_read_is_logged_and_sends_nothing() {
    use crate::test_support::LogCapture;

    let capture = LogCapture::install();
    let unreachable = sqlx::postgres::PgPoolOptions::new()
        .max_connections(1)
        .acquire_timeout(std::time::Duration::from_millis(1))
        .connect_lazy("postgres://nobody:nobody@127.0.0.1:1/none")
        .expect("lazy pool");
    let pool = Arc::new(unreachable);
    let transport = Arc::new(TestTransport::new());
    let dyn_transport: Arc<dyn Transport> = transport.clone();
    let addr: SocketAddr = "127.0.0.1:56101".parse().unwrap();
    let env = InductionEnv {
        db_pool: Some(pool.clone()),
        cell_tx: None,
        transport: dyn_transport,
        connected: Arc::new(Mutex::new(HashMap::from([(
            addr,
            test_default_connected_client_state(),
        )]))),
        entity_to_addr: Arc::new(Mutex::new(HashMap::from([(77, addr)]))),
    };
    let ids = JobIds {
        job_id: 5,
        verb: "test_plan",
        account_id: 76,
        player_id: 78,
        entity_id: 77,
        gm_entity_id: None,
        gm_name: None,
    };

    resync_inventory(&env, &pool, &ids).await;

    assert!(transport.is_empty(), "no partial inventory is sent");
    let e = capture
        .find_event(
            tracing::Level::WARN,
            "crafting inventory read failed",
            "inventory_read_failed",
        )
        .expect("client_sync_failed WARN");
    assert!(e.has_field("event", "client_sync_failed"));
    assert!(e.has_field("what", "resync"));
    assert!(e.has_field("account_id", "76"));
    assert!(e.has_field("player_id", "78"));
    assert!(e.has_field("entity_id", "77"));
}
