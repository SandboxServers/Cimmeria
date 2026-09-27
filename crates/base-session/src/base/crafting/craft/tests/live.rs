//! A craft end to end on the seeded catalog: nothing moves at the request,
//! and at the end of the bar the set is consumed across both bags, the
//! product lands, expertise rises by one and 136 goes out.

use cimmeria_entity::crafting::serialize_on_update_discipline;

use super::*;
use crate::base::crafting::test_packets::update_item_rows;
use crate::test_support::require_db_or_skip;

/// The seed rows the tests lean on; fail here with a clear message if one
/// changes.
async fn assert_seed_shape(pool: &PgPool) {
    let sets: Vec<(i32, i32, i32)> = sqlx::query_as(
        "SELECT component_set_id, item_id, quantity FROM resources.blueprints_components \
         WHERE blueprint_id = $1 AND component_set_id IN (1, 2) ORDER BY 1, 2",
    )
    .bind(BLUEPRINT)
    .fetch_all(pool)
    .await
    .expect("read 412");
    assert_eq!(
        sets,
        vec![
            (1, STEEL_CORE, 14),
            (2, STEEL_CORE, 1),
            (2, TITANIUM_CORE, 5)
        ],
        "blueprint 412 sets 1 and 2"
    );
    let bp: (Option<i32>, Option<i32>, bool, i32) = sqlx::query_as(
        "SELECT discipline_id, product_id, is_alloy, quantity FROM resources.blueprints \
         WHERE blueprint_id = $1",
    )
    .bind(BLUEPRINT)
    .fetch_one(pool)
    .await
    .expect("read 412");
    assert_eq!(bp, (Some(DISCIPLINE), Some(TITANIUM_PLATING), false, 1));
    for item in [STEEL_CORE, TITANIUM_CORE, TITANIUM_PLATING] {
        let row: (Vec<i32>, i32) = sqlx::query_as(
            "SELECT container_sets, max_stack_size FROM resources.items WHERE item_id = $1",
        )
        .bind(item)
        .fetch_one(pool)
        .await
        .expect("read item");
        assert_eq!(row, (vec![17, 15], 1), "seed shape of item {item}");
    }
}

const PLATING_NAME: &str = "Titanium Plating (Materials Subcombine B)";

/// Set 1 needs 14 Steel Cores, held as 10 in the main bag and 5 in the
/// crafting bag, one of them named (the last the page found). At the
/// request only the bar goes out; at its end 14 are consumed from both
/// bags, one plating lands in the crafting bag (its first carried bag),
/// expertise goes 10 → 11, and the client gets `onRemoveItem`,
/// `onUpdateItem`, the byte-exact 136 and the success line.
#[tokio::test]
async fn set_one_consumes_across_both_bags_and_grants_the_product() {
    let pool = require_db_or_skip!();
    assert_seed_shape(&pool).await;
    let f = Fixture::new(&pool, 0).await;
    f.know(&[(DISCIPLINE, 10)], &[BLUEPRINT]).await;
    f.stacks(STEEL_CORE, INV_MAIN, 0, 10).await;
    let crafting_bag = f.stacks(STEEL_CORE, INV_CRAFTING, 0, 5).await;

    f.craft(BLUEPRINT, &[crafting_bag[4]], 1).await;

    let at_request = f.calls();
    assert_eq!(
        at_request.iter().map(|c| c.method).collect::<Vec<_>>(),
        vec![cimmeria_wire::cell::client_methods::being::ON_TIMER_UPDATE],
        "only the induction bar at the request"
    );
    assert_eq!(
        &at_request[0].args[0..4],
        &BLUEPRINT.to_le_bytes(),
        "the bar's ID is the blueprint"
    );
    assert_eq!(f.units(STEEL_CORE).await, 15, "nothing consumed yet");
    assert!(f.stacks_of(TITANIUM_PLATING).await.is_empty());
    f.typed.clear();

    assert_eq!(f.run_inductions().await, 1);

    let left = f.stacks_of(STEEL_CORE).await;
    let plating = f.stacks_of(TITANIUM_PLATING).await;
    let expertise = f.expertise(DISCIPLINE).await;
    let calls = f.calls();
    f.cleanup().await;

    assert_eq!(left.len(), 1, "15 held, 14 consumed: {left:?}");
    assert_eq!(plating.len(), 1);
    let (plating_id, plating_size, plating_bag, _) = plating[0];
    assert_eq!((plating_size, plating_bag), (1, INV_CRAFTING));
    assert_eq!(expertise, Some(11));

    let methods: Vec<u16> = calls.iter().map(|c| c.method).collect();
    assert_eq!(
        methods,
        vec![
            method_idx::ON_REMOVE_ITEM,
            method_idx::ON_UPDATE_ITEM,
            method_idx::ON_UPDATE_DISCIPLINE,
            method_idx::ON_PLAYER_COMMUNICATION,
        ]
    );
    let removed = i32::from_le_bytes(calls[0].args[0..4].try_into().unwrap());
    assert_eq!(removed, 14, "fourteen drained stacks in one onRemoveItem");
    assert!(
        update_item_rows(&calls[1])
            .iter()
            .any(|&(id, size, bag, _)| (id, size, bag) == (plating_id, 1, INV_CRAFTING)),
        "the plating is announced"
    );
    assert_eq!(
        calls[2].args,
        serialize_on_update_discipline(DISCIPLINE, 11),
        "136, byte for byte"
    );
    assert_eq!(
        feedback_text(&calls[3]),
        crafted_text(PLATING_NAME, 1),
        "the success line"
    );
}

/// Set 2 (one Steel Core and five Titanium Cores) twice: the page names
/// one instance of each, and the designs pick set 2, not set 1 whose one
/// design they also cover. Two platings; the expertise gain is one per
/// craft, not per unit.
#[tokio::test]
async fn set_two_is_chosen_by_its_designs_and_runs_quantity_times() {
    let pool = require_db_or_skip!();
    assert_seed_shape(&pool).await;
    let f = Fixture::new(&pool, 1).await;
    f.know(&[(DISCIPLINE, 10)], &[BLUEPRINT]).await;
    let steel = f.stacks(STEEL_CORE, INV_MAIN, 0, 16).await;
    let titanium = f.stacks(TITANIUM_CORE, INV_CRAFTING, 0, 10).await;

    f.craft(BLUEPRINT, &[steel[15], titanium[9]], 2).await;
    assert_eq!(f.run_inductions().await, 1);

    let steel_left = f.units(STEEL_CORE).await;
    let titanium_left = f.units(TITANIUM_CORE).await;
    let plating = f.stacks_of(TITANIUM_PLATING).await;
    let expertise = f.expertise(DISCIPLINE).await;
    let lines = f.lines();
    f.cleanup().await;

    assert_eq!(steel_left, 14, "set 2 takes 2 Steel Cores, not set 1's 28");
    assert_eq!(titanium_left, 0);
    assert_eq!(plating.len(), 2, "two non-stacking platings");
    assert!(plating.iter().all(|p| p.2 == INV_CRAFTING));
    assert_eq!(expertise, Some(11));
    assert_eq!(lines, vec![crafted_text(PLATING_NAME, 2)]);
}

/// A second craft while the first runs waits its turn, and the player is
/// told at once; each runs in order at its own deadline. Both name the
/// same instance, as the page does (the last stack it found), and the
/// first craft drains it: the crafting bag is consumed first. The second
/// still completes from the main bag, because the named instances only
/// chose the set.
#[tokio::test]
async fn a_craft_behind_another_is_queued_and_told() {
    let pool = require_db_or_skip!();
    let f = Fixture::new(&pool, 2).await;
    f.know(&[(DISCIPLINE, 10)], &[BLUEPRINT]).await;
    let crafting_bag = f.stacks(STEEL_CORE, INV_CRAFTING, 0, 14).await;
    f.stacks(STEEL_CORE, INV_MAIN, 0, 14).await;

    f.craft(BLUEPRINT, &[crafting_bag[13]], 1).await;
    f.craft(BLUEPRINT, &[crafting_bag[13]], 1).await;
    assert_eq!(f.sessions.pending(f.entity_id), 2);
    assert_eq!(f.lines(), vec![queued_text(PLATING_NAME, 1, 1)]);

    assert_eq!(f.run_inductions().await, 2);
    let steel_left = f.units(STEEL_CORE).await;
    let plating = f.stacks_of(TITANIUM_PLATING).await.len();
    let expertise = f.expertise(DISCIPLINE).await;
    f.cleanup().await;

    assert_eq!(steel_left, 0);
    assert_eq!(plating, 2);
    assert_eq!(expertise, Some(12));
}
