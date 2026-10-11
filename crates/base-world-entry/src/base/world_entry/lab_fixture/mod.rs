//! Apply a staged lab fixture at Play (lab-fixtures FX-02).
//!
//! `handle_play_character` takes the character's fixture from the store in
//! `cimmeria_base_session::base::world_entry::lab_fixture` and calls
//! [`apply_staged_fixture`] before `query_world_entry`, so the world, the
//! position, the saved missions and the tutorial rows every world-entry
//! read sees are the fixture's, and the cell's `player_loaded` restore
//! chains run on them.
//!
//! Fixture writes are silent (D-FX4): plain row writes, no content events,
//! rewards, Discord or tutorials. Each part is its own write, in the order
//! missions, items, tutorials, position; the first failure stops the apply
//! and earlier parts stay written. The runner's verify step reports what is
//! missing. Every check (ownership, no items until FX-03, the world, the
//! position, each mission and step) runs before the first write, so a
//! refused fixture writes nothing; the ERROR's `reason` says which kind of
//! failure it was (`precheck_refused` or `write_failed`).

mod items;
mod missions;
mod position;
mod tutorials;

#[cfg(test)]
mod play_character_live_db_tests;
#[cfg(test)]
mod refusal_live_db_tests;
#[cfg(test)]
mod tests;

use std::sync::Arc;

use sqlx::PgPool;

use cimmeria_base_session::base::world_entry::lab_fixture as store;
use cimmeria_base_session::base::world_entry::lab_fixture::StagedFixture;

use items::apply_items;
use missions::{plan_missions, write_missions, MissionRow};
use position::move_player;
use tutorials::forget_tutorials;

/// Why [`apply_staged_fixture`] failed: the `reason` of the "lab fixture
/// not applied" ERROR (Rule 6, a stable value per class).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ApplyError {
    /// A check before the first write refused the fixture; nothing written.
    Precheck(String),
    /// A write failed part-way; earlier parts may be written.
    Write(String),
}

impl ApplyError {
    fn reason(&self) -> &'static str {
        match self {
            ApplyError::Precheck(_) => "precheck_refused",
            ApplyError::Write(_) => "write_failed",
        }
    }

    /// The human-readable cause.
    pub(crate) fn message(&self) -> &str {
        match self {
            ApplyError::Precheck(m) | ApplyError::Write(m) => m,
        }
    }
}

/// What [`apply_staged_fixture`] wrote.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub(crate) struct FixtureApplied {
    pub(crate) missions: usize,
    pub(crate) items_removed: u64,
    pub(crate) items_granted: u64,
    pub(crate) tutorials_forgotten: u64,
    pub(crate) moved: bool,
}

/// Take the character's staged fixture, if any, and write it: the
/// `handle_play_character` hook, run after the entry latch and before
/// `query_world_entry`.
///
/// Lab-only (D-FX2): only the lab endpoint stages a fixture, for a GM
/// account's offline character, and nothing a client sends reaches the
/// store. The take is scoped to the playing account: `player_id` is the
/// client's `playCharacter` argument, so a session of another account
/// leaves the fixture staged rather than burning it. Once taken, the
/// session must also be a GM's; a taken fixture that is refused (a demoted
/// GM, no database) is discarded, never left for a later Play.
pub(crate) async fn apply_at_play(
    db_pool: &Option<Arc<PgPool>>,
    account_id: u32,
    account_name: Option<&str>,
    player_id: i32,
    access_level: u32,
) {
    let Some(f) = store::take_fixture_for(player_id, account_id as i32) else {
        return;
    };
    if access_level < store::GM_ACCESS_LEVEL {
        tracing::warn!(
            account_id,
            account_name,
            player_id,
            player_name = %f.player_name,
            access_level,
            reason = "not_gm",
            "lab fixture discarded: the session is not a GM"
        );
        return;
    }
    let Some(pool) = db_pool else {
        tracing::warn!(
            account_id,
            account_name,
            player_id,
            player_name = %f.player_name,
            reason = "no_database",
            "lab fixture discarded: no database to write it to"
        );
        return;
    };
    match apply_staged_fixture(pool, account_id as i32, &f).await {
        Ok(applied) => tracing::info!(
            account_id,
            account_name,
            player_id,
            player_name = %f.player_name,
            world = f.world.as_deref(),
            ?applied,
            "lab fixture applied"
        ),
        Err(error) => tracing::error!(
            account_id,
            account_name,
            player_id,
            player_name = %f.player_name,
            reason = error.reason(),
            error = error.message(),
            "lab fixture not applied"
        ),
    }
}

/// Write `f` for `player_id` (owned by `account_id`). Called from
/// [`apply_at_play`], before `query_world_entry`. The error says whether
/// a check refused the fixture (nothing written) or a write failed.
pub(crate) async fn apply_staged_fixture(
    pool: &PgPool,
    account_id: i32,
    f: &StagedFixture,
) -> Result<FixtureApplied, ApplyError> {
    let rows = precheck(pool, account_id, f)
        .await
        .map_err(ApplyError::Precheck)?;
    write_parts(pool, f, &rows).await.map_err(ApplyError::Write)
}

/// The writes, in the contract's order: missions, items, tutorials,
/// position. The first failure stops; earlier parts stay written.
async fn write_parts(
    pool: &PgPool,
    f: &StagedFixture,
    missions: &[MissionRow],
) -> Result<FixtureApplied, String> {
    let mut applied = FixtureApplied {
        missions: write_missions(pool, f.player_id, missions).await?,
        ..FixtureApplied::default()
    };
    (applied.items_removed, applied.items_granted) =
        apply_items(pool, f.player_id, &f.items).await?;
    applied.tutorials_forgotten = forget_tutorials(pool, f.player_id, &f.tutorials_unseen).await?;
    applied.moved = move_player(pool, f.player_id, f.world.as_deref(), f.position).await?;
    Ok(applied)
}

/// Refuse before any write, and build the mission rows: the character must
/// be `account_id`'s (a second check behind the store's account-scoped
/// take, against the row itself), no items until FX-03, the world must be
/// one `sgw_player.world_location` accepts, the position must be finite (a
/// NaN makes the next world entry undefined; `move_player` relies on this
/// check), and every mission and step must match the seed.
async fn precheck(
    pool: &PgPool,
    account_id: i32,
    f: &StagedFixture,
) -> Result<Vec<MissionRow>, String> {
    let owner: Option<i32> =
        sqlx::query_scalar("SELECT account_id FROM sgw_player WHERE player_id = $1")
            .bind(f.player_id)
            .fetch_optional(pool)
            .await
            .map_err(|e| format!("owner lookup failed: {e}"))?;
    match owner {
        None => return Err(format!("character {} does not exist", f.player_name)),
        Some(owner) if owner != account_id => {
            return Err(format!(
                "character {} belongs to account {owner}, not the playing account {account_id}",
                f.player_name
            ))
        }
        Some(_) => {}
    }
    // FX-03 replaces this with its own item checks; until then an item
    // fixture is refused here, so no other part is written for it.
    if !f.items.is_empty() {
        return Err(items::ITEMS_NOT_YET.to_string());
    }
    if let Some(world) = f.world.as_deref() {
        let known: bool =
            sqlx::query_scalar("SELECT EXISTS (SELECT 1 FROM resources.worlds WHERE world = $1)")
                .bind(world)
                .fetch_one(pool)
                .await
                .map_err(|e| format!("world lookup failed: {e}"))?;
        if !known {
            return Err(format!("world {world} is not in resources.worlds"));
        }
    }
    if let Some(p) = f.position {
        if !p.iter().all(|c| c.is_finite()) {
            return Err(format!("position {p:?} is not finite"));
        }
    }
    plan_missions(pool, &f.missions).await
}
