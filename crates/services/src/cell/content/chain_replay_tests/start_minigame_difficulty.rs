//! `start_minigame` difficulty round-trip — Castle CA04, defect B5.
//!
//! Split out of [`super::livewire_pairs`], which pins the three seeded
//! Livewire launcher/victory pairs. Those pairs all run at the default
//! difficulty of 1; the guards here are about the *param* — that an authored
//! value survives loader to `Action` to `CellToBaseMsg`, and that an
//! out-of-range one is rejected rather than clamped.
//!
//! No seeded row sets `difficulty`, so only a sentinel chain can prove the
//! round trip. The boundary arithmetic itself is covered without a database
//! by `cimmeria_content_engine::loader::tests::action_conversion`; these two
//! cover the wiring the loader tests cannot see.

use sqlx::PgPool;

use super::livewire_pairs::{execute_and_drain_starts, resolve_interact};
use crate::test_support::require_db_or_skip;

/// Sentinel `content_chains.chain_id`. Fits in `i32`. Sibling reservations in
/// `crates/services` currently run `0x7000_1000..0x7000_1B00`, `0x7000_2000`,
/// `0x7000_3000`, `0x7000_4000`, `0x7000_4242` and `0x7000_5000`
/// (`grant_xp`); this steps past all of them. Cleanup deletes the exact ids
/// inserted, never a range.
const DIFFICULTY_CHAIN_ID: i32 = 0x7000_6000;
const DIFFICULTY_TAG: &str = "CIMMERIA_TEST_MINIGAME_DIFFICULTY_TAG";
/// Neither the loader default (1) nor a boundary — a hardcoded default or a
/// clamp-to-range bug would not reproduce it.
const DIFFICULTY_VALUE: u32 = 3;

/// Insert the sentinel chain + trigger + `start_minigame` action carrying
/// an explicit `difficulty`.
async fn seed_difficulty_chain(pool: &PgPool) {
    sqlx::query(
        "INSERT INTO resources.content_chains \
         (chain_id, description, scope_type, scope_id, enabled, priority) \
         VALUES ($1, 'start_minigame difficulty sentinel', 'space', NULL, true, 0)",
    )
    .bind(DIFFICULTY_CHAIN_ID)
    .execute(pool)
    .await
    .expect("sentinel content_chains insert must succeed");

    sqlx::query(
        "INSERT INTO resources.content_triggers \
         (chain_id, event_type, event_key, scope, once, sort_order) \
         VALUES ($1, 'interact_tag', $2, 'player', false, 0)",
    )
    .bind(DIFFICULTY_CHAIN_ID)
    .bind(DIFFICULTY_TAG)
    .execute(pool)
    .await
    .expect("sentinel content_triggers insert must succeed");

    sqlx::query(
        "INSERT INTO resources.content_actions \
         (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order) \
         VALUES ($1, 'start_minigame', NULL, 'Livewire', $2::jsonb, 0, 0)",
    )
    .bind(DIFFICULTY_CHAIN_ID)
    .bind(format!(
        r#"{{"difficulty": {DIFFICULTY_VALUE}, "on_victory_chains": [{DIFFICULTY_CHAIN_ID}]}}"#
    ))
    .execute(pool)
    .await
    .expect("sentinel content_actions insert must succeed");
}

/// Delete by exact chain id, children first (FK order).
async fn cleanup_difficulty_chain(pool: &PgPool) {
    for stmt in [
        "DELETE FROM resources.content_actions WHERE chain_id = $1",
        "DELETE FROM resources.content_triggers WHERE chain_id = $1",
        "DELETE FROM resources.content_chains WHERE chain_id = $1",
    ] {
        sqlx::query(stmt)
            .bind(DIFFICULTY_CHAIN_ID)
            .execute(pool)
            .await
            .expect("sentinel cleanup must succeed");
    }
}

/// **Defect B5 regression guard.** An authored `difficulty` must survive
/// the loader as `Action::StartMinigame { difficulty }` and reach base as
/// `CellToBaseMsg::StartMinigame { difficulty }`.
///
/// No seeded row sets `difficulty` today, so only a sentinel chain can
/// prove this. Fails with the loader's `difficulty` parse removed (the
/// field falls back to the default 1) and fails again with the executor
/// reverted to its hardcoded `difficulty: 1`.
#[tokio::test]
async fn authored_difficulty_round_trips_from_loader_to_base_message() {
    let pool = require_db_or_skip!();

    // Start from a clean slate in case a previous panicking run leaked.
    cleanup_difficulty_chain(&pool).await;
    seed_difficulty_chain(&pool).await;

    let actions = resolve_interact(&pool, DIFFICULTY_CHAIN_ID, DIFFICULTY_TAG, &[]).await;

    // Drop the sentinel rows before asserting so a failure can't leave a
    // live chain registered in the shared test database.
    cleanup_difficulty_chain(&pool).await;

    assert_eq!(
        actions.len(),
        1,
        "the sentinel chain's single start_minigame row must survive \
         convert_action; zero actions means the loader rejected the \
         difficulty param (it must accept 1-5) — got {actions:?}",
    );

    let sends = execute_and_drain_starts(actions).await;
    assert_eq!(
        sends.len(),
        1,
        "the sentinel chain must emit exactly one \
         CellToBaseMsg::StartMinigame; got {sends:?}",
    );
    assert_eq!(
        sends[0].2, DIFFICULTY_VALUE,
        "the authored difficulty must reach base unchanged; a value of 1 \
         means the executor is still hardcoding it (defect B5)",
    );
}

/// The range check is a reject, not a clamp: an out-of-range `difficulty`
/// drops the whole action row at load time so the authoring mistake shows
/// up as a missing minigame plus a WARN rather than as a silently
/// different difficulty tier.
#[tokio::test]
async fn out_of_range_difficulty_rejects_the_action_row() {
    let pool = require_db_or_skip!();

    cleanup_difficulty_chain(&pool).await;
    seed_difficulty_chain(&pool).await;
    // Overwrite the params with an out-of-range value.
    sqlx::query(
        "UPDATE resources.content_actions \
         SET params = '{\"difficulty\": 7}'::jsonb WHERE chain_id = $1",
    )
    .bind(DIFFICULTY_CHAIN_ID)
    .execute(&pool)
    .await
    .expect("sentinel params update must succeed");

    let actions = resolve_interact(&pool, DIFFICULTY_CHAIN_ID, DIFFICULTY_TAG, &[]).await;

    cleanup_difficulty_chain(&pool).await;

    assert!(
        actions.is_empty(),
        "difficulty 7 is outside the 1-5 range the original client \
         asserted; the loader must drop the row rather than clamp it — got \
         {actions:?}",
    );
}
