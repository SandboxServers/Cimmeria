//! Live-DB: an accepted alloy consumes nothing at the request, and at the
//! end of its bar consumes exactly the component and the elementary
//! components the rule counted, grants two of the product and adds one
//! expertise.

use cimmeria_observability::testing::{counter_total, install as install_meter};
use cimmeria_wire::cell::client_methods::being::ON_TIMER_UPDATE;

use super::fixture::*;
use crate::base::crafting::telemetry::METRIC_REQUESTS;
use crate::base::crafting::test_packets::{feedback_text, remove_item_ids};
use crate::mercury::method_idx;
use crate::test_support::{require_db_or_skip, LogCapture};

fn accepted() -> u64 {
    counter_total(
        METRIC_REQUESTS,
        &[("verb", "alloying"), ("outcome", "accepted")],
    )
}

/// Blueprint 42 end to end: 1x 5192 and ten Normal elementary components
/// make 2x 5191; the client gets the bar, the removals, the new stacks,
/// the expertise and the success line; `completed` names the current-tier
/// item, each elementary component with its quality and tier, and the
/// bucket met.
#[tokio::test]
async fn blueprint_42_alloys_ten_normal_elementaries_into_two_blends() {
    let pool = require_db_or_skip!();
    assert_seed_shape(&pool).await;
    install_meter();
    let f = Fixture::new(&pool, 0).await;
    let component = f.stack(COMPONENT, INV_CRAFTING, 0, 1).await;
    let normals = f.singles(NORMAL, 0, 10).await;
    let before = f.inventory().await;
    let accepted_before = accepted();

    let capture = LogCapture::install();
    f.alloy(ALLOY, component, &normals).await;
    assert_eq!(
        f.inventory().await,
        before,
        "nothing is consumed at the request"
    );
    assert_eq!(accepted() - accepted_before, 1, "one accepted request");
    let calls = f.calls();
    assert_eq!(calls.len(), 1, "only the induction bar: {calls:?}");
    assert_eq!(calls[0].method, ON_TIMER_UPDATE);
    f.transport.clear();

    assert_eq!(f.finish_inductions().await, 1);
    assert!(f.inventory().await.iter().all(|row| row.1 == PRODUCT));
    let products = f.inventory().await;
    assert_eq!(products.len(), 2, "two Blends, one per slot: {products:?}");
    assert!(products
        .iter()
        .all(|&(_, _, size, bag)| size == 1 && bag == INV_CRAFTING));
    assert_eq!(f.expertise().await, 2);

    let calls = f.calls();
    let removed = calls
        .iter()
        .find(|c| c.method == method_idx::ON_REMOVE_ITEM)
        .map(remove_item_ids)
        .expect("onRemoveItem");
    let mut expected: Vec<i32> = normals.clone();
    expected.push(component);
    let mut removed_sorted = removed.clone();
    removed_sorted.sort_unstable();
    expected.sort_unstable();
    assert_eq!(removed_sorted, expected);
    assert!(calls.iter().any(|c| c.method == method_idx::ON_UPDATE_ITEM));
    assert!(calls
        .iter()
        .any(|c| c.method == method_idx::ON_UPDATE_DISCIPLINE));
    let line = calls
        .iter()
        .find(|c| c.method == method_idx::ON_PLAYER_COMMUNICATION)
        .map(feedback_text)
        .expect("success line");
    assert_eq!(line, "Alloying complete: 2 x Blend (Bio-Medical Alloy).");

    let completed = capture
        .all()
        .into_iter()
        .find(|e| e.target == "crafting" && e.has_field("event", "completed"))
        .expect("completed event");
    for (key, value) in [
        ("verb", "alloying".to_string()),
        ("account_id", f.account_id.to_string()),
        ("player_id", f.player_id.to_string()),
        ("entity_id", f.entity_id.to_string()),
        ("blueprint_id", ALLOY.to_string()),
        ("item_id", component.to_string()),
        ("quality_bucket", "normal".to_string()),
        ("expertise", format!("{DISCIPLINE}:1→2")),
    ] {
        assert!(
            completed.has_field(key, &value),
            "{key}={value}: {completed:#?}"
        );
    }
    let elementary = completed
        .fields
        .get("elementary")
        .expect("elementary field");
    assert_eq!(elementary.split(',').count(), 10);
    assert!(elementary.contains(&format!("{}:{NORMAL}:normal:1:1", normals[0])));
    let consumed = completed.fields.get("consumed").expect("consumed field");
    assert!(
        consumed.contains(&format!("{component}:{COMPONENT}:1→0")),
        "{consumed}"
    );
    f.cleanup().await;
}

/// Good's count met by stack quantity: stacks of 3 and 4 give 7, and the
/// first 5 are used, instance by instance, leaving 2 on the second stack.
#[tokio::test]
async fn stack_quantities_count_and_only_the_bucket_count_is_used() {
    let pool = require_db_or_skip!();
    assert_seed_shape(&pool).await;
    let f = Fixture::new(&pool, 1).await;
    let component = f.stack(COMPONENT, INV_CRAFTING, 0, 1).await;
    let first = f.stack(GOOD, INV_MAIN, 0, 3).await;
    let second = f.stack(GOOD, INV_CRAFTING, 1, 4).await;
    // Another Good stack the request did not name: never touched, even
    // though consumption by design would reach the crafting bag first.
    let unnamed = f.stack(GOOD, INV_CRAFTING, 2, 5).await;

    f.alloy(ALLOY, component, &[first, second]).await;
    assert_eq!(f.finish_inductions().await, 1);
    let left: Vec<(i32, i32)> = f
        .inventory()
        .await
        .into_iter()
        .filter(|row| row.1 == GOOD)
        .map(|row| (row.0, row.2))
        .collect();
    assert_eq!(left, vec![(second, 2), (unnamed, 5)]);
    f.cleanup().await;
}

/// Two Great meet Great's count; the Normal beside them, below its own
/// count, is neither refused nor used. One Fantastic is enough on its own.
#[tokio::test]
async fn an_unmet_second_quality_is_left_and_one_fantastic_is_enough() {
    let pool = require_db_or_skip!();
    assert_seed_shape(&pool).await;
    let f = Fixture::new(&pool, 2).await;
    let component = f.stack(COMPONENT, INV_CRAFTING, 0, 1).await;
    let greats = f.singles(GREAT, 0, 2).await;
    let normal = f.stack(NORMAL, INV_MAIN, 5, 1).await;
    f.alloy(ALLOY, component, &[greats[0], normal, greats[1]])
        .await;
    assert_eq!(f.finish_inductions().await, 1);
    let types: Vec<i32> = f.inventory().await.iter().map(|row| row.1).collect();
    assert_eq!(types, vec![NORMAL, PRODUCT, PRODUCT]);

    let component = f.stack(COMPONENT, INV_CRAFTING, 5, 1).await;
    let fantastic = f.stack(FANTASTIC, INV_MAIN, 9, 1).await;
    f.alloy(ALLOY, component, &[fantastic]).await;
    assert_eq!(f.finish_inductions().await, 1);
    assert!(f
        .inventory()
        .await
        .iter()
        .all(|row| row.1 == NORMAL || row.1 == PRODUCT));
    assert_eq!(f.expertise().await, 3);
    f.cleanup().await;
}

/// An elementary component moved to the bank during the bar: the
/// completion refuses, consumes nothing and says why.
#[tokio::test]
async fn an_elementary_moved_to_the_bank_mid_induction_consumes_nothing() {
    let pool = require_db_or_skip!();
    assert_seed_shape(&pool).await;
    let f = Fixture::new(&pool, 3).await;
    let component = f.stack(COMPONENT, INV_CRAFTING, 0, 1).await;
    let normals = f.singles(NORMAL, 0, 10).await;
    f.alloy(ALLOY, component, &normals).await;
    sqlx::query("UPDATE sgw_inventory SET container_id = $2, slot_id = 0 WHERE item_id = $1")
        .bind(normals[3])
        .bind(INV_BANK)
        .execute(&pool)
        .await
        .expect("move to bank");
    let before = f.inventory().await;
    f.transport.clear();

    assert_eq!(f.finish_inductions().await, 1);
    assert_eq!(
        f.inventory().await,
        before,
        "nothing consumed, nothing granted"
    );
    assert_eq!(f.expertise().await, 1);
    let calls = f.calls();
    let line = calls
        .iter()
        .find(|c| c.method == method_idx::ON_PLAYER_COMMUNICATION)
        .map(feedback_text)
        .expect("refusal line");
    assert_eq!(
        line,
        "Components must be in your backpack or crafting bag. Nothing was used."
    );
    f.cleanup().await;
}

/// The named inputs change during the bar: the component moves to the
/// bank, or an elementary stack shrinks below what the rule counted. Each
/// completion refuses and consumes nothing.
#[tokio::test]
async fn a_named_input_changed_mid_induction_consumes_nothing() {
    let pool = require_db_or_skip!();
    assert_seed_shape(&pool).await;
    let f = Fixture::new(&pool, 4).await;
    let cases = [
        (
            "component to the bank",
            "UPDATE sgw_inventory SET container_id = 17, slot_id = 0 WHERE item_id = $1",
            true,
        ),
        (
            "elementary stack shrunk",
            "UPDATE sgw_inventory SET stack_size = 4 WHERE item_id = $1",
            false,
        ),
    ];
    for (case, sql, on_component) in cases {
        let component = f.stack(COMPONENT, INV_CRAFTING, 0, 1).await;
        let good = f.stack(GOOD, INV_MAIN, 0, 5).await;
        f.alloy(ALLOY, component, &[good]).await;
        sqlx::query(sql)
            .bind(if on_component { component } else { good })
            .execute(&pool)
            .await
            .expect("change input");
        let before = f.inventory().await;
        f.transport.clear();

        assert_eq!(f.finish_inductions().await, 1, "{case}");
        assert_eq!(f.inventory().await, before, "{case}: nothing consumed");
        assert_eq!(f.expertise().await, 1, "{case}");
        let lines: Vec<String> = f
            .calls()
            .iter()
            .filter(|c| c.method == method_idx::ON_PLAYER_COMMUNICATION)
            .map(feedback_text)
            .collect();
        assert_eq!(lines.len(), 1, "{case}: one refusal line, {lines:?}");
        assert!(lines[0].ends_with("Nothing was used."), "{case}: {lines:?}");
        sqlx::query("DELETE FROM sgw_inventory WHERE character_id = $1")
            .bind(f.player_id)
            .execute(&pool)
            .await
            .expect("clear");
    }
    f.cleanup().await;
}

/// A respec (here: the discipline forgotten) during the bar refuses the
/// queued alloy at completion; nothing is consumed and the player is told.
#[tokio::test]
async fn forgetting_the_discipline_during_the_bar_refuses_the_alloy() {
    let pool = require_db_or_skip!();
    assert_seed_shape(&pool).await;
    let f = Fixture::new(&pool, 5).await;
    let component = f.stack(COMPONENT, INV_CRAFTING, 0, 1).await;
    let normals = f.singles(NORMAL, 0, 10).await;
    f.alloy(ALLOY, component, &normals).await;
    sqlx::query("UPDATE sgw_player SET discipline_ids = '{}' WHERE player_id = $1")
        .bind(f.player_id)
        .execute(&pool)
        .await
        .expect("forget discipline");
    let before = f.inventory().await;
    f.transport.clear();
    let capture = LogCapture::install();

    assert_eq!(f.finish_inductions().await, 1);
    assert_eq!(
        f.inventory().await,
        before,
        "nothing consumed, nothing granted"
    );
    let calls = f.calls();
    let methods: Vec<u16> = calls.iter().map(|c| c.method).collect();
    assert_eq!(
        methods,
        vec![
            method_idx::ON_PLAYER_COMMUNICATION,
            method_idx::ON_UPDATE_ITEM
        ],
        "the line, then the resync"
    );
    assert_eq!(
        feedback_text(&calls[0]),
        "You must learn the blueprint's discipline first."
    );
    let rejected = capture
        .all()
        .into_iter()
        .find(|e| e.target == "crafting" && e.has_field("event", "rejected"))
        .expect("rejected");
    assert!(
        rejected.has_field("reason", "discipline_unknown"),
        "{rejected:#?}"
    );
    assert!(
        rejected.has_field("player_id", &f.player_id.to_string()),
        "{rejected:#?}"
    );
    f.cleanup().await;
}

/// Two alloys queued with the same named component: the first uses it,
/// and the second completes from the other stack of that design, because
/// the component is consumed by design and not pinned to the named stack.
#[tokio::test]
async fn a_queued_alloy_whose_named_component_was_used_takes_another() {
    let pool = require_db_or_skip!();
    assert_seed_shape(&pool).await;
    let f = Fixture::new(&pool, 6).await;
    let component = f.stack(COMPONENT, INV_CRAFTING, 0, 1).await;
    f.stack(COMPONENT, INV_CRAFTING, 1, 1).await;
    let first = f.stack(GOOD, INV_MAIN, 0, 5).await;
    let second = f.stack(GOOD, INV_MAIN, 1, 5).await;
    f.alloy(ALLOY, component, &[first]).await;
    f.alloy(ALLOY, component, &[second]).await;
    assert_eq!(f.sessions.pending(f.entity_id), 2);

    assert_eq!(f.finish_inductions().await, 1);
    assert_eq!(f.finish_inductions().await, 1);
    let types: Vec<i32> = f.inventory().await.iter().map(|row| row.1).collect();
    assert_eq!(types, vec![PRODUCT; 4], "both alloys made");
    assert_eq!(f.expertise().await, 3);
    f.cleanup().await;
}

/// Elementary items stay pinned to the named stacks, because those stacks
/// decide the quality consumed. Two alloys queued with the same named Good
/// stack: the first uses it up, and the second is refused at completion
/// with a visible line and nothing consumed, although another unnamed Good
/// stack would meet the count.
#[tokio::test]
async fn a_queued_alloy_whose_named_elementaries_were_used_is_refused() {
    let pool = require_db_or_skip!();
    assert_seed_shape(&pool).await;
    let f = Fixture::new(&pool, 7).await;
    let component = f.stack(COMPONENT, INV_CRAFTING, 0, 1).await;
    f.stack(COMPONENT, INV_CRAFTING, 1, 1).await;
    let named = f.stack(GOOD, INV_MAIN, 0, 5).await;
    let unnamed = f.stack(GOOD, INV_CRAFTING, 2, 5).await;
    f.alloy(ALLOY, component, &[named]).await;
    f.alloy(ALLOY, component, &[named]).await;
    assert_eq!(f.sessions.pending(f.entity_id), 2);

    assert_eq!(f.finish_inductions().await, 1);
    let before = f.inventory().await;
    f.transport.clear();
    assert_eq!(f.finish_inductions().await, 1);
    assert_eq!(
        f.inventory().await,
        before,
        "the second alloy consumes nothing"
    );
    assert!(before.iter().any(|row| row.0 == unnamed && row.2 == 5));
    let line = f
        .calls()
        .iter()
        .find(|c| c.method == method_idx::ON_PLAYER_COMMUNICATION)
        .map(feedback_text)
        .expect("refusal line");
    assert_eq!(
        line,
        "A component is no longer in your inventory. Nothing was used."
    );
    assert_eq!(f.expertise().await, 2, "only the first alloy counted");
    f.cleanup().await;
}
