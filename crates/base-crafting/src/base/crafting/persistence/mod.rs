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
//!
//! Every load gives each paradigm with no stored level its starting level
//! (`CraftingState::apply_default_paradigm_levels`), so a loaded state
//! always carries all five. A verb
//! that mutates state inside its own transaction uses
//! [`load_crafting_state_locked`] and [`save_crafting_state_in`], so the
//! `sgw_player` row stays locked from the read to the commit.

use cimmeria_entity::known_names;
use sqlx::{PgConnection, PgPool};

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
///
/// It takes no lock; a read-modify-write goes through
/// [`load_crafting_state_locked`].
pub async fn load_crafting_state(
    pool: &PgPool,
    player_id: i32,
) -> Result<CraftingState, sqlx::Error> {
    load_crafting_state_reporting(pool, player_id)
        .await
        .map(|(state, _)| state)
}

/// [`load_crafting_state`], also saying whether any starting paradigm level
/// was filled in because none was stored. The login sync logs it.
#[tracing::instrument(name = "crafting.load", level = "info", skip_all, fields(player_id))]
pub async fn load_crafting_state_reporting(
    pool: &PgPool,
    player_id: i32,
) -> Result<(CraftingState, bool), sqlx::Error> {
    // Wrap the body so the counter fires exactly once on every exit
    // (`ok`, `sqlx_error`, `row_not_found`) without sprinkling counter!
    // calls at each early-return. The inner function returns
    // `Err(RowNotFound)` for the missing-row case so the wrapper can
    // tag the outcome, and the wrapper maps it back to
    // `Ok(CraftingState::new())` to preserve the original public
    // contract ("missing row = default state").
    //
    // Bug shape this guards against (PR #483 review): the previous
    // per-exit counter pattern emitted `sqlx_error` *only* if the first
    // `sgw_player` query failed. If the second `sgw_player_discipline_expertise`
    // query failed via `await?`, the function returned early without
    // any counter — under-counting load failures by exactly the
    // fraction that surface as expertise-side errors.
    let result = match pool.acquire().await {
        Ok(mut conn) => read_state(&mut conn, player_id, false).await,
        Err(e) => Err(e),
    };
    count_load(&result);
    // Preserve the public contract: callers see `Ok(default state)` for
    // a missing player_id, matching the offline-mode sentinel used by
    // `query_player_load_data`.
    match result {
        Err(sqlx::Error::RowNotFound) => {
            let mut state = CraftingState::new();
            let defaults_applied = state.apply_default_paradigm_levels();
            Ok((state, defaults_applied))
        }
        other => other,
    }
}

/// Load a player's crafting state inside the caller's transaction, locking
/// the `sgw_player` row `FOR UPDATE` until the caller commits or rolls
/// back. `Ok(None)` when the player row does not exist.
///
/// Every crafting write reads through this, so two requests for one player
/// (a double click, a GM grant racing a spend) serialise on the row instead
/// of the second overwriting the first.
pub async fn load_crafting_state_locked(
    conn: &mut PgConnection,
    player_id: i32,
) -> Result<Option<CraftingState>, sqlx::Error> {
    let result = read_state(conn, player_id, true).await;
    count_load(&result);
    match result {
        Ok((state, _)) => Ok(Some(state)),
        Err(sqlx::Error::RowNotFound) => Ok(None),
        Err(e) => Err(e),
    }
}

fn count_load<T>(result: &Result<T, sqlx::Error>) {
    let outcome = match result {
        Ok(_) => "ok",
        Err(sqlx::Error::RowNotFound) => "row_not_found",
        Err(_) => "sqlx_error",
    };
    cimmeria_observability::counter!(
        "crafting_persist_attempts_total",
        "kind" => "load",
        "outcome" => outcome,
    );
}

/// The `sgw_player` half of the load, with and without the row lock.
macro_rules! select_player_crafting {
    ($suffix:literal) => {
        concat!(
            "SELECT discipline_ids, blueprint_ids, applied_science_points, ",
            "racial_paradigm_levels FROM sgw_player WHERE player_id = $1",
            $suffix
        )
    };
}

/// Read both tables on `conn` and fill in the starting level of every
/// paradigm with none stored (the `bool`: whether any was filled). A missing player row is `Err(RowNotFound)`;
/// the callers map it to their own contract.
async fn read_state(
    conn: &mut PgConnection,
    player_id: i32,
    lock: bool,
) -> Result<(CraftingState, bool), sqlx::Error> {
    #[derive(sqlx::FromRow)]
    struct PlayerCraftingRow {
        discipline_ids: Vec<i32>,
        blueprint_ids: Vec<i32>,
        applied_science_points: i32,
        racial_paradigm_levels: Vec<i32>,
    }

    let sql = if lock {
        select_player_crafting!(" FOR UPDATE")
    } else {
        select_player_crafting!("")
    };
    let row_opt: Option<PlayerCraftingRow> = sqlx::query_as(sql)
        .bind(player_id)
        .fetch_optional(&mut *conn)
        .await?;

    // Distinguish the missing-row case via `RowNotFound`. The public
    // wrapper maps this back to `Ok(default)` for the caller; the
    // wrapper-emitted counter tags it as `row_not_found` so the
    // dashboard separates "no DB connection" from "no such player".
    let row = row_opt.ok_or(sqlx::Error::RowNotFound)?;

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
    .fetch_all(&mut *conn)
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
    // Wire level fits in `i8` (levels top out at 10). We clamp on read
    // to defend against corrupted DB rows that exceed `i8::MAX`.
    for (i, level) in row.racial_paradigm_levels.into_iter().enumerate() {
        let paradigm_id = (i as i32) + 1;
        let level_i8 = i8::try_from(level).unwrap_or_else(|_| {
            tracing::warn!(
                player_id,
                player_name = known_names::player_name(player_id),
                paradigm_id,
                paradigm_name = cimmeria_names::racial_paradigm_name(paradigm_id),
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

    let defaults_applied = state.apply_default_paradigm_levels();
    Ok((state, defaults_applied))
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
///
/// **Invariant — function must exit only via `tx.commit()` after the
/// DELETE.** The expertise table is rewritten from scratch on every
/// save: DELETE all rows for this player, then re-INSERT the live map.
/// sqlx `Transaction` rolls back on Drop, so an `await?` between the
/// DELETE and the commit fails safely (rollback). The dangerous shape
/// is a future refactor that *adds an Ok(()) early return* between the
/// DELETE and the commit — the txn would drop unsealed and the
/// player's entire expertise set would be wiped silently. Keep the
/// function linear; any new step belongs either before the DELETE or
/// before the commit, never between them with an unguarded `return Ok`.
///
/// **Missing-player guard.** The UPDATE's `WHERE player_id = $5` matches
/// 0 rows for a non-existent player. Combined with an empty `expertise`
/// map, that would commit a no-op transaction and return Ok — silently
/// persisting nothing. We check `rows_affected()` on the UPDATE and
/// return `sqlx::Error::RowNotFound` if the player doesn't exist, so
/// callers see a real failure instead of a phantom success.
#[tracing::instrument(
    name = "crafting.save",
    level = "info",
    skip_all,
    fields(player_id, expertise_count = state.expertise.len()),
)]
pub async fn save_crafting_state(
    pool: &PgPool,
    player_id: i32,
    state: &CraftingState,
) -> Result<(), sqlx::Error> {
    // Wrap the body so the counter fires once on every exit (Ok, Err,
    // or row_not_found) without sprinkling counter! calls at each
    // early-return. The Drop-on-error path keeps the metric balanced
    // even if a future refactor adds a new error arm.
    let result = async {
        let mut tx = pool.begin().await?;
        write_state(&mut tx, player_id, state).await?;
        tx.commit().await
    }
    .await;
    let outcome = match &result {
        Ok(()) => "ok",
        Err(sqlx::Error::RowNotFound) => "row_not_found",
        Err(_) => "sqlx_error",
    };
    cimmeria_observability::counter!(
        "crafting_persist_attempts_total",
        "kind" => "save",
        "outcome" => outcome,
    );
    result
}

/// [`save_crafting_state`] inside the caller's transaction; the caller
/// commits. Pair it with [`load_crafting_state_locked`] on the same
/// transaction. The invariants on [`save_crafting_state`] hold here too: an
/// error leaves the transaction to roll back when it drops.
pub async fn save_crafting_state_in(
    conn: &mut PgConnection,
    player_id: i32,
    state: &CraftingState,
) -> Result<(), sqlx::Error> {
    let result = write_state(conn, player_id, state).await;
    let outcome = match &result {
        Ok(()) => "ok",
        Err(sqlx::Error::RowNotFound) => "row_not_found",
        Err(_) => "sqlx_error",
    };
    cimmeria_observability::counter!(
        "crafting_persist_attempts_total",
        "kind" => "save",
        "outcome" => outcome,
    );
    result
}

async fn write_state(
    conn: &mut PgConnection,
    player_id: i32,
    state: &CraftingState,
) -> Result<(), sqlx::Error> {
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
                    player_name = known_names::player_name(player_id),
                    paradigm_id,
                    paradigm_name = cimmeria_names::racial_paradigm_name(paradigm_id),
                    "racial paradigm level missing on save — backfilling with 0"
                );
                0
            });
        levels_array.push(level as i32);
    }

    let update_result = sqlx::query(
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
    .execute(&mut *conn)
    .await?;

    // Silent-data-loss guard: if the player_id doesn't exist, the UPDATE
    // matches 0 rows and (with an empty expertise map) the rest of the
    // transaction is a no-op. Without this check, the caller sees Ok(())
    // and assumes the state was persisted. Returning RowNotFound mirrors
    // sqlx's idiom for "the row you expected to touch wasn't there".
    if update_result.rows_affected() == 0 {
        tracing::error!(
            player_id,
            player_name = known_names::player_name(player_id),
            "save_crafting_state: UPDATE matched 0 rows — sgw_player row missing"
        );
        return Err(sqlx::Error::RowNotFound);
    }

    sqlx::query("DELETE FROM sgw_player_discipline_expertise WHERE player_id = $1")
        .bind(player_id)
        .execute(&mut *conn)
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
            .execute(&mut *conn)
            .await?;
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests;
