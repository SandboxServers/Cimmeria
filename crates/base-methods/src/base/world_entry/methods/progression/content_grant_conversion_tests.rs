//! Live-DB guards for the CS-01a review fixes to the content grant write:
//!
//! - **F1**: a node the player bought and content then grants is converted
//!   (OD-CS06, "costs no point"): out of `trained_abilities`, its cost
//!   refunded, a content row written, so a respec keeps it.
//! - **F2**: a grant whose `archetypes` list leaves out the character's real
//!   archetype writes nothing.
//! - **F4**: a character-creation starter of the archetype gets no row and
//!   no branch credit, so overlapping grant lists give no free credit.
//!
//! Sentinels: `0x7030_0Exx`. The test character is a Soldier
//! (`insert_test_player`, archetype 1); its tree node and starter come from
//! the seed.

use cimmeria_entity::cell_entity::AbilityGrantKind;

use super::content_grant_tests::{grant, grants, msg, row, run};
use super::content_grant_write::{persist_content_grant, ContentGrantRefusal};
use super::respec::persist_respec;
use super::tests::{cleanup, insert_test_account, insert_test_player};
use crate::ability_tree::{RespecOutcome, RESPEC_COST_NAQUADAH};
use crate::test_support::require_db_or_skip;

/// Bought from a trainer, no tree node in the seed (refunds 0).
const OTHER_TRAINED: i32 = 0x7030_0E41;
const SOLDIER: i32 = 1;

/// A Soldier tree node with a positive cost that is not a Soldier starter:
/// `(ability_id, skill_point_cost)`.
async fn soldier_node(pool: &sqlx::PgPool) -> (i32, i32) {
    sqlx::query_as(
        "SELECT t.ability_id, t.skill_point_cost \
           FROM resources.archetype_ability_tree t \
          WHERE t.archetype = 'ARCHETYPE_Soldier' AND t.skill_point_cost > 0 \
            AND t.ability_id NOT IN (\
                SELECT ca.ability_id FROM resources.char_creation_abilities ca \
                  JOIN resources.char_creation cc USING (char_def_id) \
                 WHERE cc.archetype = 'ARCHETYPE_Soldier') \
          ORDER BY t.ability_id LIMIT 1",
    )
    .fetch_one(pool)
    .await
    .expect("the seed has a Soldier tree node that is no starter")
}

async fn soldier_starter(pool: &sqlx::PgPool) -> i32 {
    sqlx::query_scalar(
        "SELECT MIN(ca.ability_id) FROM resources.char_creation_abilities ca \
           JOIN resources.char_creation cc USING (char_def_id) \
          WHERE cc.archetype = 'ARCHETYPE_Soldier'",
    )
    .fetch_one(pool)
    .await
    .expect("the seed gives a Soldier a starter")
}

async fn setup(pool: &sqlx::PgPool, id: i32, abilities: &[i32], trained: &[i32], spent: i32) {
    cleanup(pool, id).await;
    insert_test_account(pool, id).await;
    insert_test_player(pool, id, id, 5_000).await;
    let r = sqlx::query(
        "UPDATE sgw_player SET abilities = $1, trained_abilities = $2, \
                training_points = 0, tree_points_spent = $3 WHERE player_id = $4",
    )
    .bind(abilities)
    .bind(trained)
    .bind(spent)
    .bind(id)
    .execute(pool)
    .await
    .expect("seed abilities");
    assert_eq!(r.rows_affected(), 1, "fixture row must exist");
}

/// **Guard (F1): a bought node later granted is converted, and a respec
/// keeps it.** The node leaves `trained_abilities`, its cost comes back to
/// `training_points` and off `tree_points_spent`, and it gets a signature
/// row. The respec afterwards refunds only the other purchase. On the old
/// code the node stayed trained with no row, and the respec stripped the
/// free signature for good.
#[tokio::test]
async fn live_db_content_grant_converts_a_bought_node_and_respec_keeps_it() {
    let pool = require_db_or_skip!();
    const ID: i32 = 0x7030_0E01;
    let (node, cost) = soldier_node(&pool).await;
    setup(
        &pool,
        ID,
        &[node, OTHER_TRAINED],
        &[node, OTHER_TRAINED],
        cost + 1,
    )
    .await;

    let write = persist_content_grant(
        &pool,
        ID,
        &[node],
        AbilityGrantKind::Signature,
        Some(687),
        &[SOLDIER],
    )
    .await
    .unwrap()
    .expect("grant writes");
    assert_eq!(
        (
            write.converted.clone(),
            write.credited.clone(),
            write.refunded
        ),
        (vec![node], vec![node], cost)
    );
    assert_eq!(
        row(&pool, ID).await,
        (vec![node, OTHER_TRAINED], vec![OTHER_TRAINED], cost, 1),
        "out of trained, cost refunded, spend reduced"
    );
    assert_eq!(
        grants(&pool, ID).await,
        vec![grant(node, "signature", Some(687))]
    );

    let outcome = persist_respec(&pool, ID, RESPEC_COST_NAQUADAH)
        .await
        .unwrap();
    assert!(
        matches!(outcome, Some(RespecOutcome::Reset { .. })),
        "the other purchase makes the respec go through ({outcome:?})"
    );
    let (abilities, trained, points, spent) = row(&pool, ID).await;
    assert_eq!(abilities, vec![node], "the respec keeps the converted node");
    assert_eq!((trained, points, spent), (vec![], cost + 1, 0));
    assert_eq!(grants(&pool, ID).await.len(), 1, "its row survives");
    cleanup(&pool, ID).await;
}

/// The refund never drives `tree_points_spent` below 0, even on a row whose
/// spend is already short of the node's cost.
#[tokio::test]
async fn live_db_content_grant_conversion_floors_the_spend_at_zero() {
    let pool = require_db_or_skip!();
    const ID: i32 = 0x7030_0E02;
    let (node, cost) = soldier_node(&pool).await;
    setup(&pool, ID, &[node], &[node], 0).await;
    persist_content_grant(&pool, ID, &[node], AbilityGrantKind::Mission, None, &[])
        .await
        .unwrap()
        .expect("grant writes");
    assert_eq!(row(&pool, ID).await, (vec![node], vec![], cost, 0));
    cleanup(&pool, ID).await;
}

/// **Guard (F2): the wrong class gets nothing.** A Soldier hit by a
/// Commando-only signature: no append, no row, no reply to the cell. Drop
/// the archetype check and the Soldier learns the Commando's signature.
#[tokio::test]
async fn live_db_content_grant_refuses_another_archetype() {
    let pool = require_db_or_skip!();
    const ID: i32 = 0x7030_0E03;
    let (node, _) = soldier_node(&pool).await;
    setup(&pool, ID, &[], &[], 0).await;
    assert_eq!(
        persist_content_grant(
            &pool,
            ID,
            &[node],
            AbilityGrantKind::Signature,
            None,
            &[2, 3],
        )
        .await
        .unwrap(),
        Err(ContentGrantRefusal::ArchetypeMismatch { archetype: SOLDIER })
    );
    let to_cell = run(&pool, msg(ID, vec![node], vec![2]), ID).await;
    assert!(to_cell.is_empty(), "no reply, so no line");
    assert_eq!(row(&pool, ID).await.0, Vec::<i32>::new());
    assert!(grants(&pool, ID).await.is_empty());
    cleanup(&pool, ID).await;
}

/// **Guard (F4): a starter earns no credit.** The Soldier's own starter,
/// known or lost, gets no row and is never credited; a non-starter in the
/// same grant is. Revert to rows for every known id and the starter is
/// credited (free branch credit for every Soldier on an overlapping list).
#[tokio::test]
async fn live_db_content_grant_gives_a_starter_no_row_and_no_credit() {
    let pool = require_db_or_skip!();
    const ID: i32 = 0x7030_0E04;
    let starter = soldier_starter(&pool).await;
    let (node, _) = soldier_node(&pool).await;
    setup(&pool, ID, &[starter], &[], 0).await;

    let write = persist_content_grant(
        &pool,
        ID,
        &[starter, node],
        AbilityGrantKind::RacialCore,
        None,
        &[SOLDIER],
    )
    .await
    .unwrap()
    .expect("grant writes");
    assert_eq!(write.starters, vec![starter]);
    assert_eq!(write.credited, vec![node]);
    assert_eq!(
        grants(&pool, ID).await,
        vec![grant(node, "racial_core", None)]
    );

    // A lost starter comes back, still with no row.
    sqlx::query("UPDATE sgw_player SET abilities = '{}' WHERE player_id = $1")
        .bind(ID)
        .execute(&pool)
        .await
        .expect("drop the starter");
    let again = persist_content_grant(
        &pool,
        ID,
        &[starter],
        AbilityGrantKind::RacialCore,
        None,
        &[SOLDIER],
    )
    .await
    .unwrap()
    .expect("grant writes");
    assert_eq!(
        (again.learned, again.credited),
        (vec![starter], Vec::<i32>::new())
    );
    assert_eq!(
        grants(&pool, ID).await.len(),
        1,
        "still only the node's row"
    );
    cleanup(&pool, ID).await;
}
