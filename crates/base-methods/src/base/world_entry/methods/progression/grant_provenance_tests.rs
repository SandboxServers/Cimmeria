//! Live-DB guards for grant provenance across the GM tools, the GM / Debug
//! NPC reset and the trainer respec (Class Start v6, CS-01a, lock L6).
//!
//! Bug shapes: a reset that wipes a tutorial or signature grant (the pre-
//! CS-01a rebuild from starters only), a reset that keeps a GM grant or its
//! row, a GM grant that writes no `gm` row (the next reset could not tell it
//! from a content grant), a GM grant that downgrades a content row to `gm`,
//! and a respec that removes a granted ability or its row.
//!
//! Sentinels: `0x7030_0Dxx`; ability ids `0x40..0x5F`. The test character
//! is a Soldier (`archetype = 1`); its starters come from the seed.

use cimmeria_entity::cell_entity::AbilityGrantKind;

use super::content_grant_tests::{grants, row};
use super::content_grant_write::persist_content_grant;
use super::gm_ability_bulk::persist_bulk;
use super::grant_ability::persist_ability_grant;
use super::respec::persist_respec;
use super::tests::{cleanup, insert_test_account, insert_test_player};
use crate::ability_tree::{RespecOutcome, RESPEC_COST_NAQUADAH};
use crate::cell::messages::GmAbilityChange;
use crate::test_support::require_db_or_skip;

const TUTORIAL: i32 = 0x7030_0D40;
const SIGNATURE: i32 = 0x7030_0D41;
const GM_GRANT: i32 = 0x7030_0D42;
const TRAINED: i32 = 0x7030_0D43;
const LEGACY: i32 = 0x7030_0D44;
const GIVE_ALL: [i32; 2] = [0x7030_0D50, 0x7030_0D51];

/// A Soldier with its starters, a trained node (1 point spent) and a legacy
/// grant with no provenance row, then a tutorial and a signature content
/// grant and one `.giveability` GM grant.
async fn setup(pool: &sqlx::PgPool, id: i32) -> Vec<i32> {
    cleanup(pool, id).await;
    insert_test_account(pool, id).await;
    insert_test_player(pool, id, id, 5_000).await;
    // Since Class Start v6 CS-02 a canonical Soldier has no starters, so the
    // fixture is a debug-kit Soldier: its reset set is the debug kit.
    let starters: Vec<i32> = sqlx::query_scalar(
        "SELECT ability_id FROM resources.char_creation_debug_kit_abilities ORDER BY 1",
    )
    .fetch_all(pool)
    .await
    .expect("debug kit");
    assert!(!starters.is_empty(), "fixture: the seed has a debug kit");
    let mut held = starters.clone();
    held.extend([LEGACY, TRAINED]);
    sqlx::query(
        "UPDATE sgw_player SET abilities = $1, trained_abilities = $2, \
                training_points = 3, tree_points_spent = 1, debug_kit = true           WHERE player_id = $3",
    )
    .bind(&held)
    .bind(vec![TRAINED])
    .bind(id)
    .execute(pool)
    .await
    .expect("seed abilities");
    persist_content_grant(pool, id, &[TUTORIAL], AbilityGrantKind::Tutorial, None, &[])
        .await
        .unwrap()
        .expect("tutorial grant");
    persist_content_grant(
        pool,
        id,
        &[SIGNATURE],
        AbilityGrantKind::Signature,
        Some(687),
        &[1],
    )
    .await
    .unwrap()
    .expect("signature grant");
    persist_ability_grant(pool, id, GM_GRANT).await.unwrap();
    starters
}

fn kinds(rows: &[(i32, String, Option<i32>)]) -> Vec<(i32, &str)> {
    rows.iter().map(|(id, k, _)| (*id, k.as_str())).collect()
}

/// **Guard: `.giveability` records `gm` provenance**, and never downgrades
/// a content grant's row.
#[tokio::test]
async fn live_db_gm_grant_writes_a_gm_row_and_keeps_content_rows() {
    let pool = require_db_or_skip!();
    const ID: i32 = 0x7030_0D01;
    setup(&pool, ID).await;
    assert_eq!(
        kinds(&grants(&pool, ID).await),
        vec![
            (TUTORIAL, "tutorial"),
            (SIGNATURE, "signature"),
            (GM_GRANT, "gm")
        ]
    );
    // A GM re-granting a known signature changes neither list.
    persist_ability_grant(&pool, ID, SIGNATURE).await.unwrap();
    assert_eq!(kinds(&grants(&pool, ID).await)[1], (SIGNATURE, "signature"));
    cleanup(&pool, ID).await;
}

/// **Guard: give-all records a `gm` row per appended id**, one batch, and
/// none for ids already known.
#[tokio::test]
async fn live_db_gm_give_all_writes_gm_rows_for_appended_ids_only() {
    let pool = require_db_or_skip!();
    const ID: i32 = 0x7030_0D02;
    setup(&pool, ID).await;
    persist_bulk(
        &pool,
        ID,
        GmAbilityChange::GrantAll,
        &[GIVE_ALL[0], TUTORIAL, TRAINED, GIVE_ALL[1]],
    )
    .await
    .unwrap()
    .expect("give-all writes");
    assert_eq!(
        kinds(&grants(&pool, ID).await),
        vec![
            (TUTORIAL, "tutorial"),
            (SIGNATURE, "signature"),
            (GM_GRANT, "gm"),
            (GIVE_ALL[0], "gm"),
            (GIVE_ALL[1], "gm"),
        ]
    );
    cleanup(&pool, ID).await;
}

/// **Guard: the reset keeps non-`gm` grants and drops `gm` ones** (L6).
/// Abilities become the starters plus the tutorial and the signature; the
/// GM grant, the legacy grant and the trained node go; the spend is
/// refunded; only the `gm` row is deleted. Revert to the starters-only
/// rebuild and the tutorial and signature are lost.
#[tokio::test]
async fn live_db_gm_reset_keeps_content_grants_and_drops_gm_grants() {
    let pool = require_db_or_skip!();
    const ID: i32 = 0x7030_0D03;
    let starters = setup(&pool, ID).await;

    let write = persist_bulk(&pool, ID, GmAbilityChange::Reset, &[])
        .await
        .unwrap()
        .expect("reset writes");
    let mut expected = starters.clone();
    expected.extend([TUTORIAL, SIGNATURE]);
    assert_eq!(write.after, expected);
    assert_eq!(row(&pool, ID).await, (expected, vec![], 4, 0));
    assert_eq!(
        kinds(&grants(&pool, ID).await),
        vec![(TUTORIAL, "tutorial"), (SIGNATURE, "signature")]
    );
    cleanup(&pool, ID).await;
}

/// **Guard: a respec keeps every grant and its row.** Only the trained
/// node goes and only its point comes back; the content and GM grants and
/// all three provenance rows survive (L6).
#[tokio::test]
async fn live_db_respec_keeps_granted_abilities_and_their_rows() {
    let pool = require_db_or_skip!();
    const ID: i32 = 0x7030_0D04;
    let starters = setup(&pool, ID).await;
    let rows_before = grants(&pool, ID).await;

    let outcome = persist_respec(&pool, ID, RESPEC_COST_NAQUADAH)
        .await
        .unwrap();
    assert!(
        matches!(outcome, Some(RespecOutcome::Reset { .. })),
        "the trained node makes the respec go through ({outcome:?})"
    );
    let mut expected = starters;
    expected.extend([LEGACY, TUTORIAL, SIGNATURE, GM_GRANT]);
    assert_eq!(row(&pool, ID).await, (expected, vec![], 4, 0));
    assert_eq!(grants(&pool, ID).await, rows_before);
    cleanup(&pool, ID).await;
}
