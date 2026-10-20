//! Seed guard: a victory chain that carries conditions must have no trigger.
//!
//! `on_victory_chains` fires conditioned chains through
//! `fire_guarded_minigame_chain_by_id`, which matches only the loader's
//! `__direct_invoke_{id}` event. The loader builds that event only for a
//! chain with no `content_triggers` row. A trigger added to such a chain
//! silently turns every minigame win into `guarded_callback_not_matched`.
//! Mission 1520 (1372) and 1521 (1375) depend on this today.

use crate::test_support::require_db_or_skip;

#[tokio::test]
async fn live_db_conditioned_victory_chains_have_no_triggers() {
    let pool = require_db_or_skip!();
    let victory_ids: Vec<i32> = sqlx::query_scalar(
        "SELECT DISTINCT (jsonb_array_elements_text(params::jsonb -> 'on_victory_chains'))::int \
         FROM resources.content_actions WHERE action_type = 'start_minigame'",
    )
    .fetch_all(&pool)
    .await
    .expect("victory chain query must succeed");
    let mut conditioned = Vec::new();
    for id in victory_ids {
        let conditions: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM resources.content_conditions WHERE chain_id = $1",
        )
        .bind(id)
        .fetch_one(&pool)
        .await
        .expect("condition count must succeed");
        if conditions == 0 {
            continue; // legacy unguarded path; FZ-09 does not cover it
        }
        let triggers: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM resources.content_triggers WHERE chain_id = $1",
        )
        .bind(id)
        .fetch_one(&pool)
        .await
        .expect("trigger count must succeed");
        assert_eq!(
            triggers, 0,
            "victory chain {id} carries conditions, so it fires only through \
             __direct_invoke_{id}; a trigger row stops the loader building that \
             event and every win is lost"
        );
        conditioned.push(id);
    }
    assert!(
        conditioned.contains(&1372) && conditioned.contains(&1375),
        "expected the conditioned victory chains 1372 and 1375, saw {conditioned:?}"
    );
}
