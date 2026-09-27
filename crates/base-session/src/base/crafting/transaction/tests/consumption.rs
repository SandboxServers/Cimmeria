//! Committed transactions: what is consumed, where the product lands, and
//! what the client is told.

use super::*;
use crate::base::crafting::test_packets::{remove_item_ids, update_item_rows};
use crate::mercury::method_idx;
use crate::test_support::require_db_or_skip;

/// A partial stack shrinks, and a product listing the bank first lands in
/// the crafting bag. The client gets one `onUpdateItem` with both stacks
/// and no `onRemoveItem`.
#[tokio::test]
async fn partial_stack_shrinks_and_a_bank_first_product_lands_in_the_crafting_bag() {
    let pool = require_db_or_skip!();
    assert_seed_shape(&pool).await;
    let f = Fixture::new(&pool, 0).await;
    let component = f.stack(COMPONENT, INV_CRAFTING, 0, 2).await;

    let applied = f
        .apply(&CraftTransaction {
            named_items: vec![NamedItem::new(component, COMPONENT)],
            consume_named: vec![],
            consume: vec![(COMPONENT, 1)],
            grant: vec![(BANK_FIRST_PRODUCT, 1)],
            expertise: vec![],
            learn_blueprints: vec![],
            required_knowledge: None,
        })
        .await
        .expect("craft commits");

    assert_eq!(f.row(component).await, Some((1, INV_CRAFTING)));
    let products = f.stacks_of(BANK_FIRST_PRODUCT).await;
    assert_eq!(products.len(), 1);
    let (product, size, container, slot) = products[0];
    assert_eq!(
        (size, container, slot),
        (1, INV_CRAFTING, 1),
        "a {{17,15}} product goes to the crafting bag's first free slot, never the bank"
    );
    assert!(applied.drained().is_empty());
    assert_eq!(
        applied.consumed_field(),
        format!("{component}:{COMPONENT}:2→1")
    );
    assert_eq!(
        applied.granted_field(),
        format!("{BANK_FIRST_PRODUCT}:{INV_CRAFTING}:1:0→1"),
        "a new stack starts at 0"
    );

    let calls = f.calls();
    assert_eq!(calls.len(), 1, "{calls:?}");
    assert_eq!(calls[0].method, method_idx::ON_UPDATE_ITEM);
    assert_eq!(
        update_item_rows(&calls[0]),
        vec![
            (component, 1, INV_CRAFTING, 1),
            (product, 1, INV_CRAFTING, 2)
        ],
        "(id, stack, container, 1-based wire slot)"
    );
    assert_eq!(f.outbox_rows().await, 1, "one granted event for the cell");
    f.cleanup().await;
}

/// The request names one stack, but the requirement spans two stacks in
/// two bags: consumption is by design, crafting bag first. The emptied
/// stack is deleted and removed on the client with `onRemoveItem`; the
/// product merges into an existing stack with room.
#[tokio::test]
async fn consumption_by_design_drains_across_both_bags_and_the_product_merges() {
    let pool = require_db_or_skip!();
    assert_seed_shape(&pool).await;
    let f = Fixture::new(&pool, 1).await;
    let in_crafting = f.stack(COMPONENT, INV_CRAFTING, 0, 2).await;
    let in_main = f.stack(COMPONENT, INV_MAIN, 0, 2).await;
    let existing = f.stack(STACKABLE_PRODUCT, INV_MAIN, 5, 5).await;

    let applied = f
        .apply(&CraftTransaction {
            named_items: vec![NamedItem::new(in_main, COMPONENT)],
            consume_named: vec![],
            consume: vec![(COMPONENT, 3)],
            grant: vec![(STACKABLE_PRODUCT, 3)],
            expertise: vec![],
            learn_blueprints: vec![],
            required_knowledge: None,
        })
        .await
        .expect("craft commits");

    assert_eq!(f.row(in_crafting).await, None, "crafting-bag stack drained");
    assert_eq!(
        f.row(in_main).await,
        Some((1, INV_MAIN)),
        "main-bag stack keeps 1"
    );
    assert_eq!(
        f.stacks_of(STACKABLE_PRODUCT).await,
        vec![(existing, 8, INV_MAIN, 5)],
        "merged into the stack with room, no new slot"
    );
    assert_eq!(
        applied.consumed_field(),
        format!("{in_crafting}:{COMPONENT}:2→0,{in_main}:{COMPONENT}:2→1"),
        "each stack with its before and after size"
    );
    assert_eq!(
        applied.granted_field(),
        format!("{STACKABLE_PRODUCT}:{INV_MAIN}:5:5→8"),
        "a merge starts at the existing stack size"
    );

    let calls = f.calls();
    assert_eq!(calls.len(), 2, "{calls:?}");
    assert_eq!(calls[0].method, method_idx::ON_REMOVE_ITEM);
    assert_eq!(remove_item_ids(&calls[0]), vec![in_crafting]);
    assert_eq!(calls[1].method, method_idx::ON_UPDATE_ITEM);
    assert_eq!(
        update_item_rows(&calls[1]),
        vec![(in_main, 1, INV_MAIN, 1), (existing, 8, INV_MAIN, 6)]
    );
    assert_eq!(
        f.outbox_rows().await,
        2,
        "one removed and one granted event"
    );
    f.cleanup().await;
}

/// More than one full stack's worth of a product takes one slot per
/// stack.
#[tokio::test]
async fn a_product_larger_than_a_stack_takes_several_slots() {
    let pool = require_db_or_skip!();
    assert_seed_shape(&pool).await;
    let f = Fixture::new(&pool, 2).await;

    f.apply(&CraftTransaction {
        grant: vec![(STACKABLE_PRODUCT, 23)],
        ..CraftTransaction::default()
    })
    .await
    .expect("grant commits");

    let sizes: Vec<(i32, i32, i32)> = f
        .stacks_of(STACKABLE_PRODUCT)
        .await
        .into_iter()
        .map(|(_, size, container, slot)| (size, container, slot))
        .collect();
    assert_eq!(sizes, vec![(10, 1, 0), (10, 1, 1), (3, 1, 2)]);
    f.cleanup().await;
}

/// Expertise moves only for known disciplines, is capped at 100, and the
/// client gets `onUpdateDiscipline` with the stored value.
#[tokio::test]
async fn expertise_is_capped_and_sent() {
    let pool = require_db_or_skip!();
    let f = Fixture::new(&pool, 3).await;
    sqlx::query(
        "INSERT INTO sgw_player_discipline_expertise (player_id, discipline_id, expertise) \
         VALUES ($1, 21, 98)",
    )
    .bind(f.player_id)
    .execute(&pool)
    .await
    .expect("insert expertise");

    let applied = f
        .apply(&CraftTransaction {
            expertise: vec![(21, 5), (22, 5)],
            ..CraftTransaction::default()
        })
        .await
        .expect("commits");

    assert_eq!(applied.expertise_field(), "21:98→100", "22 is not known");
    let stored: Vec<(i32, i32)> = sqlx::query_as(
        "SELECT discipline_id, expertise FROM sgw_player_discipline_expertise \
         WHERE player_id = $1 ORDER BY discipline_id",
    )
    .bind(f.player_id)
    .fetch_all(&pool)
    .await
    .unwrap();
    assert_eq!(stored, vec![(21, 100)], "no row is created for 22");

    let calls = f.calls();
    assert_eq!(calls.len(), 1, "{calls:?}");
    assert_eq!(calls[0].method, method_idx::ON_UPDATE_DISCIPLINE);
    assert_eq!(
        calls[0].args,
        cimmeria_entity::crafting::serialize_on_update_discipline(21, 100)
    );
    f.cleanup().await;
}

/// A product merges only when the whole quantity fits: exactly filling a
/// stack merges, one more takes a new slot.
#[tokio::test]
async fn a_product_merges_only_when_the_whole_quantity_fits() {
    let pool = require_db_or_skip!();
    assert_seed_shape(&pool).await;
    let f = Fixture::new(&pool, 14).await;
    let seven = f.stack(STACKABLE_PRODUCT, INV_MAIN, 0, 7).await;

    let grant3 = CraftTransaction {
        grant: vec![(STACKABLE_PRODUCT, 3)],
        ..CraftTransaction::default()
    };
    f.apply(&grant3).await.expect("fills the stack");
    assert_eq!(
        f.stacks_of(STACKABLE_PRODUCT).await,
        vec![(seven, 10, INV_MAIN, 0)]
    );

    f.apply(&grant3).await.expect("new stack");
    let sizes: Vec<(i32, i32)> = f
        .stacks_of(STACKABLE_PRODUCT)
        .await
        .into_iter()
        .map(|(_, size, _, slot)| (size, slot))
        .collect();
    assert_eq!(sizes, vec![(10, 0), (3, 1)]);
    f.cleanup().await;
}
