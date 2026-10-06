//! Which Castle_CellBlock chains carry Class Start v6 grants (CS-04).
//!
//! Seed-shape guards, straight from `content_actions`: the ability grants
//! and the one-time tutorial live on the first-pistol chain of mission 622
//! and the five class chains of mission 687's crate, and nowhere else in the
//! zone. Mission 641 (the SMG) is deliberately untouched, and the two
//! holding-state chains (1010, 1195) carry neither verb.

use crate::test_support::require_db_or_skip;

/// **Guard: in the Cellblock, `grant_ability` and `show_tutorial` are on
/// exactly the CS-04 chains.** Chain ids 1001-1199 are this zone's block
/// (`castle_cellblock_chains.sql`). Add a grant to the holding chains or to
/// a mission 641 chain, or drop one from a class chain, and the list
/// changes.
#[tokio::test]
async fn live_db_cellblock_grants_and_tutorials_are_on_exactly_the_cs04_chains() {
    let pool = require_db_or_skip!();
    let rows: Vec<(i32, String)> = sqlx::query_as(
        "SELECT chain_id, action_type::text FROM resources.content_actions \
         WHERE action_type::text IN ('grant_ability', 'show_tutorial') \
           AND chain_id BETWEEN 1001 AND 1199 \
         ORDER BY chain_id, sort_order",
    )
    .fetch_all(&pool)
    .await
    .expect("content_actions");
    let expected: Vec<(i32, String)> = [
        (1005, "grant_ability"),
        (1005, "show_tutorial"),
        (1098, "grant_ability"),
        (1099, "grant_ability"),
        (1192, "grant_ability"),
        (1193, "grant_ability"),
        (1194, "grant_ability"),
    ]
    .into_iter()
    .map(|(chain, verb)| (chain, verb.to_string()))
    .collect();
    assert_eq!(rows, expected);
}

/// **Guard: mission 641 is unchanged by Class Start (CS-04 scope).** No
/// chain scoped to mission 641 grants an ability or shows a tutorial, and
/// the SMG (item 21) is still granted by exactly one of them. The first
/// count keeps the check from passing on an empty set.
#[tokio::test]
async fn live_db_mission_641_chains_carry_no_class_start_action() {
    let pool = require_db_or_skip!();
    let (chains, class_start_actions, smg_grants): (i64, i64, i64) = sqlx::query_as(
        "SELECT count(DISTINCT c.chain_id), \
                count(*) FILTER (WHERE a.action_type::text IN ('grant_ability', 'show_tutorial')), \
                count(*) FILTER (WHERE a.action_type::text = 'add_item' AND a.target_id = 21) \
         FROM resources.content_chains c \
         JOIN resources.content_actions a USING (chain_id) \
         WHERE c.scope_type::text = 'mission' AND c.scope_id = 641",
    )
    .fetch_one(&pool)
    .await
    .expect("mission 641 chains");
    assert!(chains > 0, "mission 641 has seeded chains");
    assert_eq!(
        (class_start_actions, smg_grants),
        (0, 1),
        "mission 641 grants the SMG once and nothing from Class Start"
    );
}
