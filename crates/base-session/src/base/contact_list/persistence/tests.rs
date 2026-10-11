use super::*;
use crate::base::contact_list::handlers::presence_fanout;
use crate::base::contact_list::wire::EVENT_DEATH;
use crate::test_support::{require_db_or_skip, LogCapture, TestTransport};
use cimmeria_mercury::transport::Transport;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

/// Sentinel base for contact-list persistence tests.
/// Distinct from crafting (the `0x7000_Cxxx` block) to avoid collisions.
const TEST_BASE: i32 = 0x7000_4000;

async fn cleanup(pool: &PgPool, account_id: i32, player_id: i32) {
    // Members + lists cascade from player DELETE.
    let _ = sqlx::query("DELETE FROM sgw_player WHERE player_id = $1")
        .bind(player_id)
        .execute(pool)
        .await;
    let _ = sqlx::query("DELETE FROM account WHERE account_id = $1")
        .bind(account_id)
        .execute(pool)
        .await;
}

async fn insert_minimal_player(pool: &PgPool, account_id: i32, player_id: i32) {
    sqlx::query("INSERT INTO account (account_id, account_name, password) VALUES ($1, $2, '')")
        .bind(account_id)
        .bind(format!("cl-persist-test-{account_id}"))
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
    .bind(format!("cl-persist-{player_id}"))
    .execute(pool)
    .await
    .expect("insert player");
}

/// `load_contact_lists` returns empty vec for a player with no lists.
#[tokio::test]
async fn live_db_load_empty_returns_empty() {
    let pool = require_db_or_skip!();
    let account_id = TEST_BASE;
    let player_id = TEST_BASE + 1;
    cleanup(&pool, account_id, player_id).await;
    insert_minimal_player(&pool, account_id, player_id).await;

    let lists = load_contact_lists(&pool, player_id)
        .await
        .expect("load should not error");
    assert!(lists.is_empty());

    cleanup(&pool, account_id, player_id).await;
}

/// `ensure_system_lists` creates Friends + Ignore on first call and is idempotent.
#[tokio::test]
async fn live_db_ensure_system_lists_creates_and_is_idempotent() {
    let pool = require_db_or_skip!();
    let account_id = TEST_BASE + 10;
    let player_id = TEST_BASE + 11;
    cleanup(&pool, account_id, player_id).await;
    insert_minimal_player(&pool, account_id, player_id).await;

    let (f1, i1) = ensure_system_lists(&pool, player_id)
        .await
        .expect("first call");
    let (f2, i2) = ensure_system_lists(&pool, player_id)
        .await
        .expect("second call — must be idempotent");

    // Same ids on both calls.
    assert_eq!(f1, f2, "Friends list_id must be stable across ensure calls");
    assert_eq!(i1, i2, "Ignore list_id must be stable across ensure calls");
    assert_ne!(f1, i1, "Friends and Ignore must have different list_ids");

    let lists = load_contact_lists(&pool, player_id)
        .await
        .expect("load after ensure");
    assert_eq!(lists.len(), 2);
    let names: Vec<&str> = lists.iter().map(|l| l.name.as_str()).collect();
    assert!(names.contains(&"Friends"));
    assert!(names.contains(&"Ignore"));

    cleanup(&pool, account_id, player_id).await;
}

/// `create_list` inserts a new list and returns its id.
#[tokio::test]
async fn live_db_create_list_inserts_and_returns_id() {
    let pool = require_db_or_skip!();
    let account_id = TEST_BASE + 20;
    let player_id = TEST_BASE + 21;
    cleanup(&pool, account_id, player_id).await;
    insert_minimal_player(&pool, account_id, player_id).await;

    let list_id = create_list(&pool, player_id, "Allies", 0)
        .await
        .expect("create");
    assert!(list_id > 0);

    let lists = load_contact_lists(&pool, player_id).await.expect("load");
    assert_eq!(lists.len(), 1);
    assert_eq!(lists[0].list_id, list_id);
    assert_eq!(lists[0].name, "Allies");

    cleanup(&pool, account_id, player_id).await;
}

/// `delete_list` returns false when the list does not belong to the player
/// (ownership rejection guard).
#[tokio::test]
async fn live_db_delete_list_rejects_wrong_owner() {
    let pool = require_db_or_skip!();
    let account_id = TEST_BASE + 30;
    let player_id = TEST_BASE + 31;
    let other_account = TEST_BASE + 32;
    let other_player = TEST_BASE + 33;
    cleanup(&pool, account_id, player_id).await;
    cleanup(&pool, other_account, other_player).await;
    insert_minimal_player(&pool, account_id, player_id).await;
    insert_minimal_player(&pool, other_account, other_player).await;

    let list_id = create_list(&pool, player_id, "MyList", 0)
        .await
        .expect("create");

    // other_player trying to delete player_id's list must return false.
    let deleted = delete_list(&pool, other_player, list_id)
        .await
        .expect("delete call should not error");
    assert!(
        !deleted,
        "delete_list must return false when the list belongs to a different player"
    );

    // The list must still exist.
    let lists = load_contact_lists(&pool, player_id).await.expect("load");
    assert_eq!(
        lists.len(),
        1,
        "list must survive wrong-owner delete attempt"
    );

    cleanup(&pool, account_id, player_id).await;
    cleanup(&pool, other_account, other_player).await;
}

/// `add_members` inserts names and ignores duplicates; `remove_members`
/// deletes them and returns only the actually-removed names.
#[tokio::test]
async fn live_db_add_and_remove_members_round_trip() {
    let pool = require_db_or_skip!();
    let account_id = TEST_BASE + 40;
    let player_id = TEST_BASE + 41;
    cleanup(&pool, account_id, player_id).await;
    insert_minimal_player(&pool, account_id, player_id).await;

    let list_id = create_list(&pool, player_id, "Contacts", 0)
        .await
        .expect("create");

    let names = vec!["Alice".to_string(), "Bob".to_string()];
    let added = add_members(&pool, player_id, list_id, &names)
        .await
        .expect("add");
    assert_eq!(added.len(), 2);

    // Duplicate add must be a no-op (ON CONFLICT DO NOTHING).
    let added_again = add_members(&pool, player_id, list_id, &["Alice".to_string()])
        .await
        .expect("add again");
    assert!(
        added_again.is_empty(),
        "duplicate add must not insert a new row"
    );

    let lists = load_contact_lists(&pool, player_id).await.expect("load");
    assert_eq!(lists[0].members.len(), 2);

    // Remove one.
    let removed = remove_members(&pool, player_id, list_id, &["Alice".to_string()])
        .await
        .expect("remove");
    assert_eq!(removed, vec!["Alice".to_string()]);

    let lists2 = load_contact_lists(&pool, player_id)
        .await
        .expect("load after remove");
    assert_eq!(lists2[0].members, vec!["Bob".to_string()]);

    cleanup(&pool, account_id, player_id).await;
}

/// `find_watchers` returns the player_ids of everyone who has a given name
/// in any list, without duplicates.
#[tokio::test]
async fn live_db_find_watchers_returns_correct_player_ids() {
    let pool = require_db_or_skip!();
    let account_id = TEST_BASE + 50;
    let player_id = TEST_BASE + 51;
    cleanup(&pool, account_id, player_id).await;
    insert_minimal_player(&pool, account_id, player_id).await;

    // Two lists, both containing "WatchedPlayer".
    let list1 = create_list(&pool, player_id, "L1", 0).await.unwrap();
    let list2 = create_list(&pool, player_id, "L2", 0).await.unwrap();
    add_members(&pool, player_id, list1, &["WatchedPlayer".to_string()])
        .await
        .unwrap();
    add_members(&pool, player_id, list2, &["WatchedPlayer".to_string()])
        .await
        .unwrap();

    let watchers: Vec<i32> = find_watchers_with_flags(&pool, "WatchedPlayer")
        .await
        .expect("find_watchers_with_flags")
        .into_iter()
        .map(|(id, _)| id)
        .collect();

    // player_id should appear exactly once despite appearing in two lists.
    assert_eq!(
        watchers.iter().filter(|&&id| id == player_id).count(),
        1,
        "find_watchers_with_flags must return distinct rows (DISTINCT clause regression guard)"
    );

    // Not-watched player should not appear.
    assert!(
        !watchers.contains(&(TEST_BASE + 99)),
        "unrelated player must not appear in watchers"
    );

    cleanup(&pool, account_id, player_id).await;
}

/// A subject on one Friends list and one Ignore list: the fanout summary
/// counts two watchers and `ignoring_watchers = 1`.
#[tokio::test]
async fn live_db_presence_fanout_counts_ignoring_watcher() {
    let pool = require_db_or_skip!();
    let friend_account = TEST_BASE + 70;
    let friend_player = TEST_BASE + 71;
    let ignorer_account = TEST_BASE + 72;
    let ignorer_player = TEST_BASE + 73;
    cleanup(&pool, friend_account, friend_player).await;
    cleanup(&pool, ignorer_account, ignorer_player).await;
    insert_minimal_player(&pool, friend_account, friend_player).await;
    insert_minimal_player(&pool, ignorer_account, ignorer_player).await;

    let friends = create_list(&pool, friend_player, "Friends", 300)
        .await
        .unwrap();
    let ignore = create_list(&pool, ignorer_player, "Ignore", 301)
        .await
        .unwrap();
    add_members(
        &pool,
        friend_player,
        friends,
        &["FanoutSubject".to_string()],
    )
    .await
    .unwrap();
    add_members(
        &pool,
        ignorer_player,
        ignore,
        &["FanoutSubject".to_string()],
    )
    .await
    .unwrap();

    let rows = find_watchers_with_flags(&pool, "FanoutSubject")
        .await
        .expect("find_watchers_with_flags");
    let (watchers, ignoring) = presence_fanout::watcher_summary(&rows);
    assert!(watchers.contains(&friend_player));
    assert!(watchers.contains(&ignorer_player));
    assert_eq!(watchers.len(), 2);
    assert_eq!(
        ignoring, 1,
        "only the Ignore-list watcher counts as ignoring"
    );

    cleanup(&pool, friend_account, friend_player).await;
    cleanup(&pool, ignorer_account, ignorer_player).await;
}

/// Fan-out summary row, end to end: `fanout_contact_event` itself logs the
/// DEBUG `contacts.presence_fanout` row with `watchers=2`,
/// `ignoring_watchers=1`, and the subject's `player_id` beside `player_name`.
/// No client is connected, so `online_watchers` and `sent` are both 0.
#[tokio::test]
async fn live_db_presence_fanout_logs_summary_row() {
    let pool = require_db_or_skip!();
    let friend_account = TEST_BASE + 90;
    let friend_player = TEST_BASE + 91;
    let ignorer_account = TEST_BASE + 92;
    let ignorer_player = TEST_BASE + 93;
    let subject_player = TEST_BASE + 94;
    cleanup(&pool, friend_account, friend_player).await;
    cleanup(&pool, ignorer_account, ignorer_player).await;
    insert_minimal_player(&pool, friend_account, friend_player).await;
    insert_minimal_player(&pool, ignorer_account, ignorer_player).await;

    let friends = create_list(&pool, friend_player, "Friends", 300)
        .await
        .unwrap();
    let ignore = create_list(&pool, ignorer_player, "Ignore", 301)
        .await
        .unwrap();
    add_members(
        &pool,
        friend_player,
        friends,
        &["FanoutRowSubject".to_string()],
    )
    .await
    .unwrap();
    add_members(
        &pool,
        ignorer_player,
        ignore,
        &["FanoutRowSubject".to_string()],
    )
    .await
    .unwrap();

    let capture = LogCapture::install();
    let transport: Arc<dyn Transport> = Arc::new(TestTransport::new());
    let db_pool = Some(Arc::new(pool.clone()));
    let connected = Arc::new(Mutex::new(HashMap::new()));
    let entity_to_addr = Arc::new(Mutex::new(HashMap::new()));
    presence_fanout::fanout_contact_event(
        "FanoutRowSubject",
        subject_player,
        EVENT_DEATH,
        0,
        &db_pool,
        &transport,
        &connected,
        &entity_to_addr,
    )
    .await;

    let row = capture
        .find_message(
            tracing::Level::DEBUG,
            "ContactList: presence fanout complete",
        )
        .expect("fanout must log the summary row");
    assert!(row.has_field("event", "contacts.presence_fanout"));
    assert!(row.has_field("player_name", "FanoutRowSubject"));
    assert!(row.has_field("player_id", &subject_player.to_string()));
    assert!(row.has_field("watchers", "2"));
    assert!(row.has_field("ignoring_watchers", "1"));
    assert!(row.has_field("skipped_ignoring", "1"));
    assert!(row.has_field("online_watchers", "0"));
    assert!(row.has_field("sent", "0"));

    cleanup(&pool, friend_account, friend_player).await;
    cleanup(&pool, ignorer_account, ignorer_player).await;
}

/// D-TG10 guard: with both a Friends watcher and an Ignore watcher online,
/// only the friend receives CM 89. The summary row's `sent` counts the friend
/// alone and `skipped_ignoring` counts the ignorer. Reverting the ignorer
/// filter in `fanout_contact_event` sends to both and fails every assertion.
#[tokio::test]
async fn live_db_ignorer_gets_no_presence() {
    use crate::test_support::test_default_connected_client_state;
    use std::net::SocketAddr;

    let pool = require_db_or_skip!();
    let friend_account = TEST_BASE + 100;
    let friend_player = TEST_BASE + 101;
    let ignorer_account = TEST_BASE + 102;
    let ignorer_player = TEST_BASE + 103;
    let subject_player = TEST_BASE + 104;
    cleanup(&pool, friend_account, friend_player).await;
    cleanup(&pool, ignorer_account, ignorer_player).await;
    insert_minimal_player(&pool, friend_account, friend_player).await;
    insert_minimal_player(&pool, ignorer_account, ignorer_player).await;

    let friends = create_list(&pool, friend_player, "Friends", 300)
        .await
        .unwrap();
    let ignore = create_list(&pool, ignorer_player, "Ignore", 301)
        .await
        .unwrap();
    let subject = "IgnorerPresenceSubject".to_string();
    add_members(
        &pool,
        friend_player,
        friends,
        std::slice::from_ref(&subject),
    )
    .await
    .unwrap();
    add_members(
        &pool,
        ignorer_player,
        ignore,
        std::slice::from_ref(&subject),
    )
    .await
    .unwrap();

    let friend_addr: SocketAddr = "127.0.0.1:6101".parse().unwrap();
    let ignorer_addr: SocketAddr = "127.0.0.1:6102".parse().unwrap();
    let mut friend = test_default_connected_client_state();
    friend.active_player_id = Some(friend_player);
    friend.player_entity_id = Some(6101);
    let mut ignorer = test_default_connected_client_state();
    ignorer.active_player_id = Some(ignorer_player);
    ignorer.player_entity_id = Some(6102);
    let connected = Arc::new(Mutex::new(HashMap::from([
        (friend_addr, friend),
        (ignorer_addr, ignorer),
    ])));
    let entity_to_addr = Arc::new(Mutex::new(HashMap::from([
        (6101u32, friend_addr),
        (6102u32, ignorer_addr),
    ])));

    let capture = LogCapture::install();
    let tt = Arc::new(TestTransport::new());
    let transport: Arc<dyn Transport> = tt.clone();
    let db_pool = Some(Arc::new(pool.clone()));
    presence_fanout::fanout_contact_event(
        &subject,
        subject_player,
        EVENT_DEATH,
        0,
        &db_pool,
        &transport,
        &connected,
        &entity_to_addr,
    )
    .await;

    assert_eq!(
        tt.filter_to(friend_addr).len(),
        1,
        "the Friends watcher must receive the presence event"
    );
    assert!(
        tt.filter_to(ignorer_addr).is_empty(),
        "a watcher who ignores the subject must receive no presence event"
    );
    let row = capture
        .find_message(
            tracing::Level::DEBUG,
            "ContactList: presence fanout complete",
        )
        .expect("fanout must log the summary row");
    assert!(row.has_field("watchers", "2"));
    assert!(row.has_field("ignoring_watchers", "1"));
    assert!(row.has_field("skipped_ignoring", "1"));
    assert!(row.has_field("online_watchers", "1"));
    assert!(row.has_field("sent", "1"));

    cleanup(&pool, friend_account, friend_player).await;
    cleanup(&pool, ignorer_account, ignorer_player).await;
}

/// Zero watchers: the fanout skips the connected scan and still logs the
/// summary row, with `watchers=0`, `online_watchers=0` and `sent=0`.
#[tokio::test]
async fn live_db_presence_fanout_empty_watchers_logs_zero_row() {
    let pool = require_db_or_skip!();
    let capture = LogCapture::install();
    let transport: Arc<dyn Transport> = Arc::new(TestTransport::new());
    let db_pool = Some(Arc::new(pool.clone()));
    let connected = Arc::new(Mutex::new(HashMap::new()));
    let entity_to_addr = Arc::new(Mutex::new(HashMap::new()));
    presence_fanout::fanout_contact_event(
        "FanoutEmptySubject",
        TEST_BASE + 95,
        EVENT_DEATH,
        0,
        &db_pool,
        &transport,
        &connected,
        &entity_to_addr,
    )
    .await;

    let row = capture
        .find_message(
            tracing::Level::DEBUG,
            "ContactList: presence fanout complete",
        )
        .expect("empty watcher set must still log the summary row");
    assert!(row.has_field("event", "contacts.presence_fanout"));
    assert!(row.has_field("player_name", "FanoutEmptySubject"));
    assert!(row.has_field("player_id", &(TEST_BASE + 95).to_string()));
    assert!(row.has_field("watchers", "0"));
    assert!(row.has_field("ignoring_watchers", "0"));
    assert!(row.has_field("online_watchers", "0"));
    assert!(row.has_field("sent", "0"));
}

/// Character delete cascades: after deleting the owning `sgw_player` row,
/// no `sgw_contact_list` or `sgw_contact_list_member` rows may survive.
/// This is invariant #4 (no orphaned social data after character deletion).
#[tokio::test]
async fn live_db_character_delete_cascades_to_lists_and_members() {
    let pool = require_db_or_skip!();
    let account_id = TEST_BASE + 60;
    let player_id = TEST_BASE + 61;
    cleanup(&pool, account_id, player_id).await;
    insert_minimal_player(&pool, account_id, player_id).await;

    let list_id = create_list(&pool, player_id, "ToBeGone", 0).await.unwrap();
    add_members(&pool, player_id, list_id, &["Ghost".to_string()])
        .await
        .unwrap();

    // Delete the character.
    sqlx::query("DELETE FROM sgw_player WHERE player_id = $1")
        .bind(player_id)
        .execute(&pool)
        .await
        .expect("delete player");

    // Assert no orphaned rows remain.
    let orphan_lists: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM sgw_contact_list WHERE player_id = $1")
            .bind(player_id)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(
        orphan_lists, 0,
        "sgw_contact_list rows must cascade-delete with the player (invariant #4)"
    );

    let orphan_members: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM sgw_contact_list_member WHERE list_id = $1")
            .bind(list_id)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(
        orphan_members, 0,
        "sgw_contact_list_member rows must cascade-delete with the list (invariant #4)"
    );

    sqlx::query("DELETE FROM account WHERE account_id = $1")
        .bind(account_id)
        .execute(&pool)
        .await
        .ok();
}
