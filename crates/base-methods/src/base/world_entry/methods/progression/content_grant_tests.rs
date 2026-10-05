//! Live-DB guards for the `grant_ability` content action's base write
//! (Class Start v6, CS-01a).
//!
//! Bug shapes: an append without its provenance row (or the reverse), a
//! replay that appends twice, an already-known ability left without a row
//! (no branch credit, lost on reset), a `gm` row that a content grant does
//! not promote, and a write for a character the session no longer plays.
//! Trained conversion, the archetype gate and starters are in
//! `content_grant_conversion_tests`.
//!
//! Sentinels: `0x7030_0Cxx`. Players and accounts use `0x01..0x0F`; ability
//! ids `0x40..0x5F` (the provenance table has no foreign key to
//! `resources.abilities`, so no seed id is needed).

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use cimmeria_entity::cell_entity::AbilityGrantKind;
use tokio::sync::mpsc;

use super::content_grant_write::{persist_content_grant, ContentGrantRefusal, ContentGrantWrite};
use super::handle_content_grant_abilities;
use super::tests::{cleanup, insert_test_account, insert_test_player, make_connected_state};
use crate::cell::messages::{BaseToCellMsg, ContentAbilitiesGranted, ContentGrantAbilities};
use crate::test_support::require_db_or_skip;

pub(super) const ENTITY: u32 = 9_303_001;
/// Known with no provenance row: a legacy grant (D-AT11), not a seeded
/// starter.
const LEGACY: i32 = 0x7030_0C40;
/// Bought from a trainer; not a tree node in the seed, so it refunds 0.
const TRAINED: i32 = 0x7030_0C41;
const NEW_A: i32 = 0x7030_0C50;
const NEW_B: i32 = 0x7030_0C51;
const SOURCE_MISSION: i32 = 1559;

/// `(abilities, trained_abilities, training_points, tree_points_spent)`.
type Row = (Vec<i32>, Vec<i32>, i32, i32);
/// `(ability_id, source_kind, source_id)`, by ability id.
type Grant = (i32, String, Option<i32>);

/// A Soldier knowing `LEGACY` (no provenance) and `TRAINED` (bought,
/// 1 point spent, 3 unspent).
async fn setup(pool: &sqlx::PgPool, id: i32) {
    cleanup(pool, id).await;
    insert_test_account(pool, id).await;
    insert_test_player(pool, id, id, 0).await;
    let r = sqlx::query(
        "UPDATE sgw_player SET abilities = $1, trained_abilities = $2, \
                training_points = 3, tree_points_spent = 1 WHERE player_id = $3",
    )
    .bind(vec![LEGACY, TRAINED])
    .bind(vec![TRAINED])
    .bind(id)
    .execute(pool)
    .await
    .expect("seed abilities");
    assert_eq!(r.rows_affected(), 1, "fixture row must exist");
}

pub(super) async fn row(pool: &sqlx::PgPool, id: i32) -> Row {
    sqlx::query_as(
        "SELECT abilities, trained_abilities, training_points, tree_points_spent \
           FROM sgw_player WHERE player_id = $1",
    )
    .bind(id)
    .fetch_one(pool)
    .await
    .expect("read player row")
}

pub(super) async fn grants(pool: &sqlx::PgPool, id: i32) -> Vec<Grant> {
    sqlx::query_as(
        "SELECT ability_id, source_kind::text, source_id FROM sgw_player_ability_grants \
          WHERE player_id = $1 ORDER BY ability_id",
    )
    .bind(id)
    .fetch_all(pool)
    .await
    .expect("read provenance rows")
}

pub(super) fn grant(id: i32, kind: &str, source: Option<i32>) -> Grant {
    (id, kind.to_string(), source)
}

/// **Guard: append and provenance commit together.** One call grants two
/// new ids and re-grants the legacy one (row added, not re-appended).
#[tokio::test]
async fn live_db_content_grant_appends_and_records_provenance() {
    let pool = require_db_or_skip!();
    const ID: i32 = 0x7030_0C01;
    setup(&pool, ID).await;

    let write = persist_content_grant(
        &pool,
        ID,
        &[NEW_A, LEGACY, NEW_B],
        AbilityGrantKind::Signature,
        Some(SOURCE_MISSION),
        &[1],
    )
    .await
    .unwrap()
    .expect("grant writes");
    assert_eq!(
        write,
        ContentGrantWrite {
            learned: vec![NEW_A, NEW_B],
            already_known: vec![LEGACY],
            credited: vec![NEW_A, LEGACY, NEW_B],
            rows_written: vec![LEGACY, NEW_A, NEW_B],
            training_points: 3,
            tree_points_spent: 1,
            ..Default::default()
        }
    );
    assert_eq!(
        row(&pool, ID).await,
        (vec![LEGACY, TRAINED, NEW_A, NEW_B], vec![TRAINED], 3, 1),
        "appended in request order; trained, points and spend untouched"
    );
    assert_eq!(
        grants(&pool, ID).await,
        vec![
            grant(LEGACY, "signature", Some(SOURCE_MISSION)),
            grant(NEW_A, "signature", Some(SOURCE_MISSION)),
            grant(NEW_B, "signature", Some(SOURCE_MISSION)),
        ]
    );
    cleanup(&pool, ID).await;
}

/// **Guard: a replay is a no-op**, and a second content source does not
/// overwrite the first one's row.
#[tokio::test]
async fn live_db_content_grant_replay_changes_nothing() {
    let pool = require_db_or_skip!();
    const ID: i32 = 0x7030_0C02;
    setup(&pool, ID).await;
    persist_content_grant(&pool, ID, &[NEW_A], AbilityGrantKind::Tutorial, None, &[])
        .await
        .unwrap()
        .expect("first grant");
    let before = (row(&pool, ID).await, grants(&pool, ID).await);

    let again = persist_content_grant(
        &pool,
        ID,
        &[NEW_A],
        AbilityGrantKind::Mission,
        Some(SOURCE_MISSION),
        &[],
    )
    .await
    .unwrap()
    .expect("replay");
    assert_eq!(
        again,
        ContentGrantWrite {
            already_known: vec![NEW_A],
            credited: vec![NEW_A],
            training_points: 3,
            tree_points_spent: 1,
            ..Default::default()
        }
    );
    assert_eq!((row(&pool, ID).await, grants(&pool, ID).await), before);
    cleanup(&pool, ID).await;
}

/// **Guard: a content grant promotes a `gm` row.** A GM gave the ability
/// first; the signature grant must make it survive the next GM reset.
#[tokio::test]
async fn live_db_content_grant_promotes_a_gm_row() {
    let pool = require_db_or_skip!();
    const ID: i32 = 0x7030_0C03;
    setup(&pool, ID).await;
    super::grant_ability::persist_ability_grant(&pool, ID, NEW_A)
        .await
        .unwrap();
    assert_eq!(grants(&pool, ID).await, vec![grant(NEW_A, "gm", None)]);

    let write = persist_content_grant(
        &pool,
        ID,
        &[NEW_A],
        AbilityGrantKind::Signature,
        Some(SOURCE_MISSION),
        &[1],
    )
    .await
    .unwrap()
    .expect("grant writes");
    assert_eq!(write.rows_written, vec![NEW_A]);
    assert_eq!(
        grants(&pool, ID).await,
        vec![grant(NEW_A, "signature", Some(SOURCE_MISSION))]
    );
    cleanup(&pool, ID).await;
}

#[tokio::test]
async fn live_db_content_grant_refuses_a_gm_kind_and_a_missing_row() {
    let pool = require_db_or_skip!();
    const ID: i32 = 0x7030_0C04;
    setup(&pool, ID).await;
    assert_eq!(
        persist_content_grant(&pool, ID, &[NEW_A], AbilityGrantKind::Gm, None, &[])
            .await
            .unwrap(),
        Err(ContentGrantRefusal::GmKind)
    );
    assert!(grants(&pool, ID).await.is_empty());
    assert_eq!(
        persist_content_grant(
            &pool,
            ID + 1,
            &[NEW_A],
            AbilityGrantKind::Tutorial,
            None,
            &[]
        )
        .await
        .unwrap(),
        Err(ContentGrantRefusal::PlayerRowMissing)
    );
    cleanup(&pool, ID).await;
}

pub(super) fn msg(player_id: i32, ids: Vec<i32>, archetypes: Vec<i32>) -> ContentGrantAbilities {
    ContentGrantAbilities {
        entity_id: ENTITY,
        player_id,
        account_id: None,
        chain_id: 7_301,
        ability_ids: ids,
        source_kind: AbilityGrantKind::Tutorial,
        source_id: Some(SOURCE_MISSION),
        archetypes,
    }
}

pub(super) async fn run(
    pool: &sqlx::PgPool,
    msg: ContentGrantAbilities,
    session_player: i32,
) -> Vec<BaseToCellMsg> {
    let addr: SocketAddr = "127.0.0.1:65394".parse().unwrap();
    let connected = Arc::new(Mutex::new(HashMap::from([(
        addr,
        make_connected_state(Some(session_player)),
    )])));
    let entity_to_addr = Arc::new(Mutex::new(HashMap::from([(ENTITY, addr)])));
    let (tx, mut rx) = mpsc::channel(4);
    handle_content_grant_abilities(
        msg,
        &Some(Arc::new(pool.clone())),
        &connected,
        &entity_to_addr,
        &Some(tx),
    )
    .await;
    std::iter::from_fn(|| rx.try_recv().ok()).collect()
}

/// The handler tells the cell what to mirror: the learned ids, every id
/// that now earns branch credit (the newly recorded legacy grant and the
/// converted purchase too), the conversion and the points after it.
#[tokio::test]
async fn live_db_content_grant_handler_reports_learned_credited_and_converted() {
    let pool = require_db_or_skip!();
    const ID: i32 = 0x7030_0C05;
    setup(&pool, ID).await;

    let to_cell = run(&pool, msg(ID, vec![NEW_A, LEGACY, TRAINED], vec![]), ID).await;
    let expected = ContentAbilitiesGranted {
        entity_id: ENTITY,
        player_id: ID,
        chain_id: 7_301,
        source_kind: AbilityGrantKind::Tutorial,
        learned: vec![NEW_A],
        credited: vec![NEW_A, LEGACY, TRAINED],
        converted: vec![TRAINED],
        // `TRAINED` is no tree node, so it refunds nothing.
        training_points: 3,
        tree_points_spent: 1,
    };
    assert!(
        matches!(to_cell.as_slice(), [BaseToCellMsg::ContentAbilitiesGranted(g)] if *g == expected),
        "one ContentAbilitiesGranted ({} messages)",
        to_cell.len()
    );
    cleanup(&pool, ID).await;
}

/// **Guard: a recycled entity writes nothing.**
#[tokio::test]
async fn live_db_content_grant_refuses_a_session_playing_another_character() {
    let pool = require_db_or_skip!();
    const ID: i32 = 0x7030_0C06;
    setup(&pool, ID).await;

    let to_cell = run(&pool, msg(ID, vec![NEW_A], vec![]), ID + 1).await;
    assert!(to_cell.is_empty(), "nothing to the cell");
    assert_eq!(row(&pool, ID).await.0, vec![LEGACY, TRAINED]);
    assert!(grants(&pool, ID).await.is_empty());
    cleanup(&pool, ID).await;
}

/// The provenance rows go with the character (`ON DELETE CASCADE`).
#[tokio::test]
async fn live_db_provenance_rows_cascade_with_the_character() {
    let pool = require_db_or_skip!();
    const ID: i32 = 0x7030_0C07;
    setup(&pool, ID).await;
    persist_content_grant(
        &pool,
        ID,
        &[NEW_A],
        AbilityGrantKind::RacialCore,
        None,
        &[1],
    )
    .await
    .unwrap()
    .expect("grant");
    cleanup(&pool, ID).await;
    assert!(grants(&pool, ID).await.is_empty());
}

/// The table's `CHECK` refuses a kind outside the five.
#[tokio::test]
async fn live_db_provenance_kind_check_refuses_unknown_kinds() {
    let pool = require_db_or_skip!();
    const ID: i32 = 0x7030_0C08;
    setup(&pool, ID).await;
    let r = sqlx::query(
        "INSERT INTO sgw_player_ability_grants (player_id, ability_id, source_kind) \
         VALUES ($1, $2, 'trained')",
    )
    .bind(ID)
    .bind(NEW_A)
    .execute(&pool)
    .await;
    assert!(r.is_err(), "'trained' is not a provenance kind");
    cleanup(&pool, ID).await;
}
