use super::*;
use crate::test_support::require_db_or_skip;

/// Sentinel base for contact-list persistence tests.
/// Distinct from crafting (0x7000_2000 / 0x7000_3000) to avoid collisions.
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
async fn load_empty_returns_empty() {
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
async fn ensure_system_lists_creates_and_is_idempotent() {
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
async fn create_list_inserts_and_returns_id() {
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
async fn delete_list_rejects_wrong_owner() {
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
async fn add_and_remove_members_round_trip() {
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
async fn find_watchers_returns_correct_player_ids() {
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

    let watchers = find_watchers(&pool, "WatchedPlayer")
        .await
        .expect("find_watchers");

    // player_id should appear exactly once despite appearing in two lists.
    assert_eq!(
        watchers.iter().filter(|&&id| id == player_id).count(),
        1,
        "find_watchers must return distinct player_ids (DISTINCT clause regression guard)"
    );

    // Not-watched player should not appear.
    assert!(
        !watchers.contains(&(TEST_BASE + 99)),
        "unrelated player must not appear in watchers"
    );

    cleanup(&pool, account_id, player_id).await;
}

/// chatIgnore add-then-remove drives the 'Ignore' list (flags=301): the name
/// lands on the Ignore list after add and is gone after remove, and the list
/// keeps its 301 flags throughout.
#[tokio::test]
async fn chat_ignore_add_then_remove_updates_ignore_list() {
    let pool = require_db_or_skip!();
    let account_id = TEST_BASE + 70;
    let player_id = TEST_BASE + 71;
    cleanup(&pool, account_id, player_id).await;
    insert_minimal_player(&pool, account_id, player_id).await;

    // ensure_system_lists creates the Ignore list (flags=301) idempotently.
    let (_friends, ignore_id) = ensure_system_lists(&pool, player_id).await.expect("ensure");

    // Add a name to the Ignore list (the chatIgnore add path).
    let added = add_members(&pool, player_id, ignore_id, &["Spammer".to_string()])
        .await
        .expect("add to ignore");
    assert_eq!(added, vec!["Spammer".to_string()]);

    // The name must be on the Ignore list (flags == 301).
    let lists = load_contact_lists(&pool, player_id).await.expect("load");
    let ignore = lists
        .iter()
        .find(|l| l.flags == 301)
        .expect("Ignore list present");
    assert_eq!(ignore.list_id, ignore_id);
    assert!(ignore.members.contains(&"Spammer".to_string()));

    // Remove it (the chatIgnore remove path).
    let removed = remove_members(&pool, player_id, ignore_id, &["Spammer".to_string()])
        .await
        .expect("remove from ignore");
    assert_eq!(removed, vec!["Spammer".to_string()]);

    let lists2 = load_contact_lists(&pool, player_id)
        .await
        .expect("load after remove");
    let ignore2 = lists2
        .iter()
        .find(|l| l.flags == 301)
        .expect("Ignore list still present");
    assert!(
        !ignore2.members.contains(&"Spammer".to_string()),
        "name must be gone from the Ignore list after remove"
    );

    cleanup(&pool, account_id, player_id).await;
}

/// Character delete cascades: after deleting the owning `sgw_player` row,
/// no `sgw_contact_list` or `sgw_contact_list_member` rows may survive.
/// This is invariant #4 (no orphaned social data after character deletion).
#[tokio::test]
async fn character_delete_cascades_to_lists_and_members() {
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
