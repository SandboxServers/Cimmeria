//! Load + save round-trip for `CraftingState`.
//!
//! State is split across two tables:
//! - `sgw_player` carries the four scalar/array columns: `discipline_ids`,
//!   `blueprint_ids`, `applied_science_points`, `racial_paradigm_levels`.
//! - `sgw_player_discipline_expertise` carries the normalised
//!   per-(player, discipline) expertise rows.
//!
//! Why split? Python's `Crafter.disciplines` was one `{id -> expertise}`
//! map. Storing expertise as a parallel array on `sgw_player` would require
//! N coordinated array UPDATEs per `gainExpertise` call (one per discipline
//! to adjust); a normalised row lets the UPDATE target a single PK.
//!
//! See `db/sgw/Players/Tables/sgw_player_discipline_expertise.sql` for the
//! schema rationale.

use sqlx::PgPool;

use cimmeria_entity::crafting::CraftingState;

/// Load a player's full crafting state from the DB.
///
/// Returns `Ok(default state)` if the player row doesn't exist — matches the
/// `query_player_load_data` pattern in `player_load/core.rs`, where a
/// missing row surfaces as the offline-mode sentinel rather than an error.
/// A real connection error (DB unavailable, query syntax broken) still
/// propagates as `Err`.
///
/// The two queries are intentionally separate rather than a JOIN: the
/// `sgw_player` row is one tuple, the expertise table is N rows, and
/// `sqlx::query_as` doesn't decompose JOINs into a parent + child shape
/// without a lot of ceremony. Two queries, one connection round-trip each.
//
// Phase 1 only the live-DB tests in this module call this; the world-entry
// load path that will invoke it lands in Phase 2 alongside the cell-side
// activity dispatch. `#[allow(dead_code)]` keeps the function in the
// public surface so Phase 2 doesn't have to flip visibility.
#[allow(dead_code)]
pub async fn load_crafting_state(
    pool: &PgPool,
    player_id: i32,
) -> Result<CraftingState, sqlx::Error> {
    #[derive(sqlx::FromRow)]
    struct PlayerCraftingRow {
        discipline_ids: Vec<i32>,
        blueprint_ids: Vec<i32>,
        applied_science_points: i32,
        racial_paradigm_levels: Vec<i32>,
    }

    let row_opt: Option<PlayerCraftingRow> = sqlx::query_as(
        "SELECT discipline_ids, blueprint_ids, applied_science_points, \
                racial_paradigm_levels \
         FROM sgw_player WHERE player_id = $1",
    )
    .bind(player_id)
    .fetch_optional(pool)
    .await?;

    let row = match row_opt {
        Some(r) => r,
        None => return Ok(CraftingState::new()),
    };

    // Pull expertise rows for the disciplines this player knows. We *could*
    // filter `WHERE discipline_id = ANY($2)` to match `discipline_ids`, but
    // the PK includes `player_id` so `WHERE player_id = $1` is already the
    // tightest index hit. A stray expertise row whose discipline isn't in
    // `discipline_ids` is data corruption — we surface it rather than
    // silently dropping it so a future operator-side check can catch it.
    #[derive(sqlx::FromRow)]
    struct ExpertiseRow {
        discipline_id: i32,
        expertise: i32,
    }

    let expertise_rows: Vec<ExpertiseRow> = sqlx::query_as(
        "SELECT discipline_id, expertise \
         FROM sgw_player_discipline_expertise \
         WHERE player_id = $1 \
         ORDER BY discipline_id",
    )
    .bind(player_id)
    .fetch_all(pool)
    .await?;

    let mut state = CraftingState::new();
    state.discipline_ids = row.discipline_ids;
    state.blueprint_ids = row.blueprint_ids;
    state.applied_science_points = row.applied_science_points;

    // `racial_paradigm_levels` is stored as `integer[]`, parallel to the
    // paradigm-id sequence — `levels[i]` is the level for paradigm id `i+1`
    // (paradigms are 1-indexed in `resources.racial_paradigm`). We re-key
    // it into a `HashMap<paradigm_id, level>` here so the discipline-
    // prerequisite check (which looks up by paradigm id, not array index)
    // doesn't have to remember the indexing convention.
    //
    // Wire level fits in `i8` (Python source caps at 5). We clamp on read
    // to defend against corrupted DB rows that exceed `i8::MAX`.
    for (i, level) in row.racial_paradigm_levels.into_iter().enumerate() {
        let paradigm_id = (i as i32) + 1;
        let level_i8 = i8::try_from(level).unwrap_or_else(|_| {
            tracing::warn!(
                player_id,
                paradigm_id,
                level,
                "racial paradigm level exceeds i8 range — clamping; check DB integrity"
            );
            level.clamp(0, i8::MAX as i32) as i8
        });
        state.racial_paradigm_levels.insert(paradigm_id, level_i8);
    }

    for ExpertiseRow {
        discipline_id,
        expertise,
    } in expertise_rows
    {
        state.expertise.insert(discipline_id, expertise);
    }

    Ok(state)
}

/// Save a player's crafting state to the DB.
///
/// Uses one transaction so the `sgw_player` UPDATE and the expertise
/// upsert can't tear — a partial save would leave `discipline_ids` ahead
/// of (or behind) the expertise rows, which manifests as
/// "client thinks I know a discipline but the server shows 0% expertise"
/// or worse, a discipline silently dropping out of the known list.
///
/// Strategy for expertise: delete-then-insert inside the txn. Upsert
/// (`ON CONFLICT DO UPDATE`) would also work, but a delete-then-insert
/// pattern correctly handles the case where the in-memory state has
/// *removed* a discipline (e.g., respec — Phase 5). Phase 1 doesn't
/// remove, but pinning the contract early avoids a behavior change when
/// respec lands.
//
// Phase 1: Phase 2's spendAppliedSciencePoints handler is the first
// production caller. See note on `load_crafting_state`.
#[allow(dead_code)]
pub async fn save_crafting_state(
    pool: &PgPool,
    player_id: i32,
    state: &CraftingState,
) -> Result<(), sqlx::Error> {
    let mut tx = pool.begin().await?;

    // Convert the paradigm map back to the parallel array shape. We assume
    // paradigm ids are contiguous from 1 (matches `resources.racial_paradigm`
    // seed: 5 paradigms, ids 1..=5). If the map has a gap, we backfill with
    // 0 — but log it, because that signals either a load-side bug (missed
    // a paradigm) or DB corruption (paradigm got deleted from resources).
    let max_paradigm_id = state
        .racial_paradigm_levels
        .keys()
        .copied()
        .max()
        .unwrap_or(0);
    let mut levels_array: Vec<i32> = Vec::with_capacity(max_paradigm_id as usize);
    for paradigm_id in 1..=max_paradigm_id {
        let level = state
            .racial_paradigm_levels
            .get(&paradigm_id)
            .copied()
            .unwrap_or_else(|| {
                tracing::warn!(
                    player_id,
                    paradigm_id,
                    "racial paradigm level missing on save — backfilling with 0"
                );
                0
            });
        levels_array.push(level as i32);
    }

    sqlx::query(
        "UPDATE sgw_player \
         SET discipline_ids = $1, \
             blueprint_ids = $2, \
             applied_science_points = $3, \
             racial_paradigm_levels = $4 \
         WHERE player_id = $5",
    )
    .bind(&state.discipline_ids)
    .bind(&state.blueprint_ids)
    .bind(state.applied_science_points)
    .bind(&levels_array)
    .bind(player_id)
    .execute(&mut *tx)
    .await?;

    sqlx::query("DELETE FROM sgw_player_discipline_expertise WHERE player_id = $1")
        .bind(player_id)
        .execute(&mut *tx)
        .await?;

    // Skip the INSERT loop entirely when there's no expertise to write —
    // saves a per-row round-trip cost on common no-expertise saves (fresh
    // character, post-respec) and keeps the txn shorter on the happy path.
    if !state.expertise.is_empty() {
        // Sort keys for deterministic INSERT order (helps test diffs).
        let mut entries: Vec<(i32, i32)> = state.expertise.iter().map(|(&d, &e)| (d, e)).collect();
        entries.sort_by_key(|(d, _)| *d);

        for (discipline_id, expertise) in entries {
            sqlx::query(
                "INSERT INTO sgw_player_discipline_expertise \
                    (player_id, discipline_id, expertise) \
                 VALUES ($1, $2, $3)",
            )
            .bind(player_id)
            .bind(discipline_id)
            .bind(expertise)
            .execute(&mut *tx)
            .await?;
        }
    }

    tx.commit().await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    //! Live-DB regression guard for the load → mutate → save → reload
    //! round-trip. Self-skips when `DATABASE_URL` is unset via
    //! `require_db_or_skip!`.

    use super::*;
    use crate::test_support::require_db_or_skip;

    /// Sentinel base for the crafting persistence tests. Stepped well
    /// past the player_load sentinels (0x7000_1000 range) so concurrent
    /// live-DB runs don't collide on account/player ids. Fits in i32.
    const TEST_BASE: i32 = 0x7000_2000;

    async fn cleanup(pool: &PgPool, account_id: i32, player_id: i32) {
        // Expertise rows cascade on sgw_player delete, but we delete
        // explicitly to support partial-cleanup paths if a test bails
        // before inserting the player row.
        let _ = sqlx::query("DELETE FROM sgw_player_discipline_expertise WHERE player_id = $1")
            .bind(player_id)
            .execute(pool)
            .await;
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
        sqlx::query(
            "INSERT INTO account (account_id, account_name, password) \
             VALUES ($1, $2, '')",
        )
        .bind(account_id)
        .bind(format!("craft-test-{account_id}"))
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
        .bind(format!("craft-test-{player_id}"))
        .execute(pool)
        .await
        .expect("insert player");
    }

    /// Round-trip regression guard: load defaults, mutate every field,
    /// save, reload, and check every field matches. Bug shape this
    /// catches: a forgotten field in the UPDATE statement (or in the
    /// SELECT), a column rename without updating both queries, or a
    /// transaction rollback that leaves the DB in a half-saved state.
    #[tokio::test]
    async fn round_trip_persists_all_fields() {
        let pool = require_db_or_skip!();
        let account_id = TEST_BASE;
        let player_id = TEST_BASE + 1;
        cleanup(&pool, account_id, player_id).await;
        insert_minimal_player(&pool, account_id, player_id).await;

        // 1. Load the freshly-inserted player — should be all defaults.
        let mut state = load_crafting_state(&pool, player_id)
            .await
            .expect("initial load");
        assert!(
            state.discipline_ids.is_empty(),
            "fresh player has no disciplines",
        );
        assert!(state.expertise.is_empty(), "fresh player has no expertise");
        assert!(
            state.blueprint_ids.is_empty(),
            "fresh player has no blueprints",
        );
        assert_eq!(state.applied_science_points, 0, "fresh player has 0 ASP",);

        // 2. Mutate every field. discipline 7 with expertise 42,
        // discipline 13 with expertise 100 (the cap — verifies the
        // CHECK constraint doesn't reject).
        state.discipline_ids = vec![7, 13];
        state.expertise.insert(7, 42);
        state.expertise.insert(13, 100);
        state.blueprint_ids = vec![200, 201, 202];
        state.applied_science_points = 5;
        state.racial_paradigm_levels.insert(1, 3);
        state.racial_paradigm_levels.insert(2, 1);
        state.racial_paradigm_levels.insert(3, 5);

        // 3. Save.
        save_crafting_state(&pool, player_id, &state)
            .await
            .expect("save");

        // 4. Reload from the DB — must round-trip exactly.
        let reloaded = load_crafting_state(&pool, player_id).await.expect("reload");

        assert_eq!(reloaded.discipline_ids, vec![7, 13]);
        assert_eq!(reloaded.blueprint_ids, vec![200, 201, 202]);
        assert_eq!(reloaded.applied_science_points, 5);
        assert_eq!(reloaded.get_expertise(7), Some(42));
        assert_eq!(reloaded.get_expertise(13), Some(100));
        assert_eq!(reloaded.racial_paradigm_levels.get(&1), Some(&3));
        assert_eq!(reloaded.racial_paradigm_levels.get(&2), Some(&1));
        assert_eq!(reloaded.racial_paradigm_levels.get(&3), Some(&5));

        cleanup(&pool, account_id, player_id).await;
    }

    /// Save can rewrite expertise — the second save with reduced
    /// expertise rows must drop the old rows, not retain them. Bug
    /// shape: an upsert without a prior DELETE would leave stale
    /// rows that the next load picks up, presenting the player with
    /// disciplines they don't actually know.
    #[tokio::test]
    async fn save_replaces_expertise_rows() {
        let pool = require_db_or_skip!();
        let account_id = TEST_BASE + 10;
        let player_id = TEST_BASE + 11;
        cleanup(&pool, account_id, player_id).await;
        insert_minimal_player(&pool, account_id, player_id).await;

        // First save: 3 disciplines.
        let mut state = CraftingState::new();
        state.discipline_ids = vec![1, 2, 3];
        state.expertise.insert(1, 10);
        state.expertise.insert(2, 20);
        state.expertise.insert(3, 30);
        save_crafting_state(&pool, player_id, &state)
            .await
            .expect("first save");

        // Second save: drop discipline 2 entirely.
        state.discipline_ids = vec![1, 3];
        state.expertise.remove(&2);
        save_crafting_state(&pool, player_id, &state)
            .await
            .expect("second save");

        let reloaded = load_crafting_state(&pool, player_id).await.expect("reload");
        assert_eq!(reloaded.discipline_ids, vec![1, 3]);
        assert_eq!(
            reloaded.get_expertise(2),
            None,
            "expertise row for dropped discipline 2 must NOT survive a re-save \
             — a stale row here means the DELETE-then-INSERT pattern regressed \
             to a plain INSERT",
        );
        assert_eq!(reloaded.get_expertise(1), Some(10));
        assert_eq!(reloaded.get_expertise(3), Some(30));

        cleanup(&pool, account_id, player_id).await;
    }

    /// Load on a non-existent player_id returns the default state, not
    /// an error. Matches the offline-mode sentinel pattern used by
    /// `query_player_load_data`. Bug shape: a refactor that changed
    /// `fetch_optional` to `fetch_one` would surface a hard error here
    /// and break login for any account that didn't yet have crafting
    /// state seeded.
    #[tokio::test]
    async fn load_missing_player_returns_default() {
        let pool = require_db_or_skip!();
        let bogus_player_id = TEST_BASE + 999;
        // Don't insert anything — just try to load.
        let state = load_crafting_state(&pool, bogus_player_id)
            .await
            .expect("load missing player should not error");
        assert!(state.discipline_ids.is_empty());
        assert!(state.expertise.is_empty());
        assert_eq!(state.applied_science_points, 0);
    }
}
