//! The Ignore list queries and the three-copy resync (SS-C1).
//!
//! Unit tests for the name matching and the cached check; live-DB tests
//! (`require_db_or_skip!`, sentinels `0x7300_C1xx`) for the flags-301 scope,
//! the offline check mail uses, and the resync that feeds the cell.

use std::collections::{HashMap, HashSet};
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use tokio::sync::mpsc;

use super::*;
use crate::base::contact_list::persistence::{add_members, ensure_system_lists};
use crate::cell::messages::BaseToCellMsg;
use crate::test_support::{require_db_or_skip, test_default_connected_client_state};

fn set(v: &[&str]) -> HashSet<String> {
    v.iter().map(|s| s.to_string()).collect()
}

#[test]
fn match_name_prefers_exact_then_unique_case_fold() {
    let names = ["Bob", "bob", "Carol"];
    assert_eq!(
        match_name(names, "bob"),
        NameMatch::Found("bob".into()),
        "an exact match wins even when a case-fold would be ambiguous"
    );
    assert_eq!(match_name(names, "BOB"), NameMatch::Ambiguous);
    assert_eq!(match_name(names, "carol"), NameMatch::Found("Carol".into()));
    assert_eq!(match_name(names, "Dave"), NameMatch::NotFound);
    assert_eq!(match_name(names, ""), NameMatch::NotFound);
}

#[test]
fn session_ignores_reads_the_recipient_cache_only() {
    let a: SocketAddr = "127.0.0.1:54700".parse().unwrap();
    let b: SocketAddr = "127.0.0.1:54701".parse().unwrap();
    let mut sa = test_default_connected_client_state();
    sa.ignore = IgnoreCache::new(set(&["Spammer"]));
    let clients = HashMap::from([(a, sa), (b, test_default_connected_client_state())]);
    assert!(session_ignores(&clients, a, "Spammer"));
    assert!(
        session_ignores(&clients, a, "sPAMMER"),
        "names compare case-insensitively (D-SS13 fold)"
    );
    assert!(!session_ignores(&clients, b, "Spammer"));
    let gone: SocketAddr = "127.0.0.1:54702".parse().unwrap();
    assert!(!session_ignores(&clients, gone, "Spammer"));
}

/// Out-of-order resyncs: the version is taken before the database read, and
/// an older read that finishes after a newer one is not applied.
#[test]
fn ignore_cache_applies_only_the_newest_resync() {
    let mut cache = IgnoreCache::default();
    let older = cache.begin_sync();
    let newer = cache.begin_sync();
    assert!(cache.apply_sync(newer, &set(&["Now"]), [2].into()));
    assert!(
        !cache.apply_sync(older, &set(&["Before"]), [1].into()),
        "a stale read must not overwrite a newer one"
    );
    assert!(cache.ignores("now") && !cache.ignores("Before"));
    assert!(cache.ignores_player(2) && !cache.ignores_player(1));
}

// ── live DB ────────────────────────────────────────────────────────────────

const TEST_BASE: i32 = 0x7300_C100;

async fn cleanup(pool: &PgPool, account_id: i32, player_id: i32) {
    let _ = sqlx::query("DELETE FROM sgw_player WHERE player_id = $1")
        .bind(player_id)
        .execute(pool)
        .await;
    let _ = sqlx::query("DELETE FROM account WHERE account_id = $1")
        .bind(account_id)
        .execute(pool)
        .await;
}

async fn insert_player(pool: &PgPool, account_id: i32, player_id: i32, name: &str) {
    sqlx::query("INSERT INTO account (account_id, account_name, password) VALUES ($1, $2, '')")
        .bind(account_id)
        .bind(format!("ss-c1-ignore-{account_id}"))
        .execute(pool)
        .await
        .expect("insert account");
    sqlx::query(
        "INSERT INTO sgw_player (\
            account_id, player_id, level, alignment, archetype, gender, \
            player_name, extra_name, world_location, bodyset, \
            pos_x, pos_y, pos_z, skin_color_id\
         ) VALUES ($1, $2, 1, 0, 1, 1, $3, '', 'CombatSim', 'BS_HumanMale.BS_HumanMale', \
                   0.0, 0.0, 0.0, 0)",
    )
    .bind(account_id)
    .bind(player_id)
    .bind(name)
    .execute(pool)
    .await
    .expect("insert player");
}

/// `player_ignores` (the check mail send uses for an offline recipient)
/// reads the flags-301 list only: the same name on Friends is not an ignore.
#[tokio::test]
async fn live_db_player_ignores_reads_only_the_ignore_list() {
    let pool = require_db_or_skip!();
    let (account_id, player_id) = (TEST_BASE, TEST_BASE + 1);
    cleanup(&pool, account_id, player_id).await;
    insert_player(&pool, account_id, player_id, "ssc1-owner-1").await;
    let (friends, ignore) = ensure_system_lists(&pool, player_id).await.unwrap();
    add_members(&pool, player_id, friends, &["Pal".to_string()])
        .await
        .unwrap();
    add_members(&pool, player_id, ignore, &["Spammer".to_string()])
        .await
        .unwrap();

    assert!(player_ignores(&pool, player_id, "Spammer").await.unwrap());
    assert!(
        !player_ignores(&pool, player_id, "Pal").await.unwrap(),
        "a Friends entry is not an ignore"
    );
    assert!(
        player_ignores(&pool, player_id, "sPAMMER").await.unwrap(),
        "the database check folds case too"
    );
    assert_eq!(
        load_ignore_names(&pool, player_id).await.unwrap(),
        set(&["Spammer"])
    );
    cleanup(&pool, account_id, player_id).await;
}

/// `resolve_character` finds an offline character by exact name or a
/// unique case-fold, and refuses a case-fold that matches two characters.
#[tokio::test]
async fn live_db_resolve_character_follows_d_ss13() {
    let pool = require_db_or_skip!();
    let a = (TEST_BASE + 10, TEST_BASE + 11, "SsC1Resolve");
    let b = (TEST_BASE + 12, TEST_BASE + 13, "ssc1resolve");
    for (acc, pid, _) in [a, b] {
        cleanup(&pool, acc, pid).await;
    }
    insert_player(&pool, a.0, a.1, a.2).await;

    assert_eq!(
        resolve_character(&pool, "SSC1RESOLVE").await.unwrap(),
        CharacterLookup::Found {
            player_id: a.1,
            name: a.2.to_string()
        }
    );
    insert_player(&pool, b.0, b.1, b.2).await;
    assert_eq!(
        resolve_character(&pool, "SSC1RESOLVE").await.unwrap(),
        CharacterLookup::Ambiguous
    );
    assert_eq!(
        resolve_character(&pool, "ssc1resolve").await.unwrap(),
        CharacterLookup::Found {
            player_id: b.1,
            name: b.2.to_string()
        }
    );
    assert_eq!(
        resolve_character(&pool, "ssc1-nobody").await.unwrap(),
        CharacterLookup::NotFound
    );
    for (acc, pid, _) in [a, b] {
        cleanup(&pool, acc, pid).await;
    }
}

/// The resync writes the DB list to the base session and pushes the same set
/// to the cell as `UpdateIgnoreList` for the given entity.
#[tokio::test]
async fn live_db_resync_ignore_cache_updates_session_and_cell() {
    let pool = require_db_or_skip!();
    let (account_id, player_id) = (TEST_BASE + 20, TEST_BASE + 21);
    cleanup(&pool, account_id, player_id).await;
    insert_player(&pool, account_id, player_id, "ssc1-owner-2").await;
    let (_friends, ignore) = ensure_system_lists(&pool, player_id).await.unwrap();
    // A real character stored in the wrong case, as the contact-list window
    // can: the cache still ignores it by name and by player_id.
    let (jerk_account, jerk_id) = (TEST_BASE + 22, TEST_BASE + 23);
    cleanup(&pool, jerk_account, jerk_id).await;
    insert_player(&pool, jerk_account, jerk_id, "SsC1Jerk").await;
    add_members(&pool, player_id, ignore, &["ssc1jerk".to_string()])
        .await
        .unwrap();

    let addr: SocketAddr = "127.0.0.1:54710".parse().unwrap();
    let mut state = test_default_connected_client_state();
    state.active_player_id = Some(player_id);
    state.account_id = 0x7300_C1AA;
    let connected = Arc::new(Mutex::new(HashMap::from([(addr, state)])));
    let db_pool = Some(Arc::new(pool.clone()));
    let (tx, mut rx) = mpsc::channel(4);
    let cell_tx = Some(tx);
    let ctx = IgnoreSyncCtx {
        db_pool: &db_pool,
        connected: &connected,
        cell_tx: &cell_tx,
    };

    let got = resync_ignore_cache(ctx, addr, player_id, 9001, "test").await;
    assert_eq!(got, Some(set(&["ssc1jerk"])));
    {
        let g = connected.lock().unwrap();
        assert!(
            g[&addr].ignore.ignores("SsC1Jerk"),
            "case-folded name match"
        );
        assert!(
            g[&addr].ignore.ignores_player(jerk_id),
            "the entry resolves to the character's player_id (the SS-D1 duel seam)"
        );
    }
    match rx.try_recv() {
        Ok(BaseToCellMsg::UpdateIgnoreList {
            entity_id,
            player_id: pid,
            account_id,
            version,
            ignore_names,
        }) => {
            assert!(version >= 1, "every push carries a resync version");
            assert_eq!((entity_id, pid), (9001, player_id));
            assert_eq!(
                account_id, 0x7300_C1AA,
                "the session's account rides the push"
            );
            assert_eq!(ignore_names, set(&["ssc1jerk"]));
        }
        _ => panic!("expected UpdateIgnoreList on the cell channel"),
    }

    // A session that has moved on to another character keeps its own cache.
    connected
        .lock()
        .unwrap()
        .get_mut(&addr)
        .unwrap()
        .active_player_id = Some(player_id + 1);
    connected.lock().unwrap().get_mut(&addr).unwrap().ignore = IgnoreCache::default();
    assert_eq!(
        resync_ignore_cache(ctx, addr, player_id, 9001, "test").await,
        None
    );
    assert!(connected.lock().unwrap()[&addr].ignore.is_empty());
    assert!(rx.try_recv().is_err(), "no push for a stale character");
    cleanup(&pool, account_id, player_id).await;
    cleanup(&pool, jerk_account, jerk_id).await;
}

/// A contact-list UI edit of the Ignore list (the `ContactListAddMembers`
/// path) reloads the session and cell copies at once, like `chatIgnore`; an
/// edit of Friends does not. Fails when `resync_if_ignore_list` is removed
/// from the member ops.
#[tokio::test]
async fn live_db_contact_list_ui_edit_of_ignore_list_resyncs_session_and_cell() {
    use crate::base::contact_list::handlers::handle_add_members;
    use crate::test_support::TestTransport;
    use cimmeria_mercury::transport::Transport;

    let pool = require_db_or_skip!();
    let (account_id, player_id) = (TEST_BASE + 30, TEST_BASE + 31);
    cleanup(&pool, account_id, player_id).await;
    insert_player(&pool, account_id, player_id, "ssc1-owner-3").await;
    let (friends, ignore) = ensure_system_lists(&pool, player_id).await.unwrap();

    let addr: SocketAddr = "127.0.0.1:54720".parse().unwrap();
    let entity_id = 9002;
    let mut state = test_default_connected_client_state();
    state.active_player_id = Some(player_id);
    state.player_entity_id = Some(entity_id);
    let connected = Arc::new(Mutex::new(HashMap::from([(addr, state)])));
    let entity_to_addr = Arc::new(Mutex::new(HashMap::from([(entity_id, addr)])));
    let transport: Arc<dyn Transport> = Arc::new(TestTransport::default());
    let db_pool = Some(Arc::new(pool.clone()));
    let (tx, mut rx) = mpsc::channel(4);
    let cell_tx = Some(tx);

    let added = handle_add_members(
        entity_id,
        player_id,
        friends,
        vec!["Pal".to_string()],
        &db_pool,
        &transport,
        &connected,
        &entity_to_addr,
        &cell_tx,
    )
    .await;
    assert_eq!(added, vec!["Pal".to_string()]);
    assert!(rx.try_recv().is_err(), "a Friends edit pushes nothing");

    handle_add_members(
        entity_id,
        player_id,
        ignore,
        vec!["Troll".to_string()],
        &db_pool,
        &transport,
        &connected,
        &entity_to_addr,
        &cell_tx,
    )
    .await;
    assert!(connected.lock().unwrap()[&addr].ignore.ignores("Troll"));
    assert!(matches!(
        rx.try_recv(),
        Ok(BaseToCellMsg::UpdateIgnoreList { ignore_names, .. }) if ignore_names == set(&["Troll"])
    ));
    cleanup(&pool, account_id, player_id).await;
}

/// PR #893 review: the contact-list UI adds up to 100 names per request, so
/// without a cap on its path two batches could take the Ignore list well past
/// `MAX_IGNORE_LIST_MEMBERS`. With 98 on the list, a 5-name batch adds 2,
/// the other 3 are refused with one feedback line, and the list stops at 100;
/// a second batch adds nothing. Fails when `add_members_bounded` ignores the
/// cap (the list ends at 103).
#[tokio::test]
async fn live_db_contact_list_ui_batch_cannot_push_the_ignore_list_past_the_cap() {
    use crate::base::contact_list::handlers::handle_add_members;
    use crate::test_support::TestTransport;
    use cimmeria_mercury::transport::Transport;

    let pool = require_db_or_skip!();
    let (account_id, player_id) = (TEST_BASE + 40, TEST_BASE + 41);
    cleanup(&pool, account_id, player_id).await;
    insert_player(&pool, account_id, player_id, "ssc1-owner-cap").await;
    let (_friends, ignore) = ensure_system_lists(&pool, player_id).await.unwrap();
    let filler: Vec<String> = (0..MAX_IGNORE_LIST_MEMBERS - 2)
        .map(|i| format!("cap-filler-{i}"))
        .collect();
    sqlx::query(
        "INSERT INTO sgw_contact_list_member (list_id, player_name) \
         SELECT $1, n FROM UNNEST($2::text[]) AS t(n)",
    )
    .bind(ignore)
    .bind(&filler)
    .execute(&pool)
    .await
    .unwrap();

    let addr: SocketAddr = "127.0.0.1:54730".parse().unwrap();
    let entity_id = 9003;
    let mut state = test_default_connected_client_state();
    state.active_player_id = Some(player_id);
    state.player_entity_id = Some(entity_id);
    let connected = Arc::new(Mutex::new(HashMap::from([(addr, state)])));
    let entity_to_addr = Arc::new(Mutex::new(HashMap::from([(entity_id, addr)])));
    let test_transport = Arc::new(TestTransport::default());
    let transport: Arc<dyn Transport> = test_transport.clone();
    let db_pool = Some(Arc::new(pool.clone()));
    let (tx, _rx) = mpsc::channel(8);
    let cell_tx = Some(tx);

    let batch: Vec<String> = (1..=5).map(|i| format!("cap-new-{i}")).collect();
    let added = handle_add_members(
        entity_id,
        player_id,
        ignore,
        batch.clone(),
        &db_pool,
        &transport,
        &connected,
        &entity_to_addr,
        &cell_tx,
    )
    .await;
    assert_eq!(
        added,
        vec!["cap-new-1".to_string(), "cap-new-2".to_string()]
    );
    let count = |pool: PgPool| async move {
        let n: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM sgw_contact_list_member WHERE list_id = $1")
                .bind(ignore)
                .fetch_one(&pool)
                .await
                .unwrap();
        n
    };
    assert_eq!(count(pool.clone()).await, MAX_IGNORE_LIST_MEMBERS as i64);
    assert!(
        !test_transport.filter_to(addr).is_empty(),
        "the refused names get a feedback line (and the added ones their echo)"
    );

    let again = handle_add_members(
        entity_id,
        player_id,
        ignore,
        vec!["cap-more".to_string()],
        &db_pool,
        &transport,
        &connected,
        &entity_to_addr,
        &cell_tx,
    )
    .await;
    assert!(again.is_empty(), "a full list takes nothing more");
    assert_eq!(count(pool.clone()).await, MAX_IGNORE_LIST_MEMBERS as i64);
    cleanup(&pool, account_id, player_id).await;
}

/// The resync's one-query snapshot: every stored name, and the ids of the
/// characters they fold-match; a name with no character adds no id.
#[tokio::test]
async fn live_db_load_ignore_snapshot_reads_names_and_ids_together() {
    let pool = require_db_or_skip!();
    let (account_id, player_id) = (TEST_BASE + 50, TEST_BASE + 51);
    let (other_account, other_id) = (TEST_BASE + 52, TEST_BASE + 53);
    cleanup(&pool, account_id, player_id).await;
    cleanup(&pool, other_account, other_id).await;
    insert_player(&pool, account_id, player_id, "ssc1-owner-snap").await;
    insert_player(&pool, other_account, other_id, "SsC1SnapReal").await;
    let (_friends, ignore) = ensure_system_lists(&pool, player_id).await.unwrap();
    add_members(
        &pool,
        player_id,
        ignore,
        &["ssc1snapreal".to_string(), "ssc1-nobody".to_string()],
    )
    .await
    .unwrap();
    let (names, ids) = load_ignore_snapshot(&pool, player_id).await.unwrap();
    assert_eq!(names, set(&["ssc1snapreal", "ssc1-nobody"]));
    assert_eq!(ids, HashSet::from([other_id]));
    cleanup(&pool, account_id, player_id).await;
    cleanup(&pool, other_account, other_id).await;
}
