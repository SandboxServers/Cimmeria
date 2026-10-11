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
//! missing. The checks that need no part's state (ownership, the world and
//! the position) run before the first write.

mod items;
mod missions;
mod position;
mod tutorials;

#[cfg(test)]
mod play_character_live_db_tests;
#[cfg(test)]
mod tests;

use std::sync::Arc;

use sqlx::PgPool;

use cimmeria_base_session::base::world_entry::lab_fixture as store;
use cimmeria_base_session::base::world_entry::lab_fixture::StagedFixture;

use items::apply_items;
use missions::apply_missions;
use position::move_player;
use tutorials::forget_tutorials;

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
/// store. The session must be a GM's too, since the store is keyed by
/// player and the session is what proves who is playing. The fixture is
/// taken whether or not it is applied, so a refused one never waits for a
/// later Play.
pub(crate) async fn apply_at_play(
    db_pool: &Option<Arc<PgPool>>,
    account_id: u32,
    account_name: Option<&str>,
    player_id: i32,
    access_level: u32,
) {
    let Some(f) = store::take_fixture(player_id) else {
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
            %error,
            "lab fixture not applied"
        ),
    }
}

/// Write `f` for `player_id` (owned by `account_id`). Called from
/// [`apply_at_play`], before `query_world_entry`.
pub(crate) async fn apply_staged_fixture(
    pool: &PgPool,
    account_id: i32,
    f: &StagedFixture,
) -> Result<FixtureApplied, String> {
    precheck(pool, account_id, f).await?;
    let mut applied = FixtureApplied {
        missions: apply_missions(pool, f.player_id, &f.missions).await?,
        ..FixtureApplied::default()
    };
    (applied.items_removed, applied.items_granted) =
        apply_items(pool, f.player_id, &f.items).await?;
    applied.tutorials_forgotten = forget_tutorials(pool, f.player_id, &f.tutorials_unseen).await?;
    applied.moved = move_player(pool, f.player_id, f.world.as_deref(), f.position).await?;
    Ok(applied)
}

/// Refuse before any write: the character must be `account_id`'s (the
/// store is keyed by player only, so a fixture staged for one character
/// must never land on a session that plays another account's), the world
/// must be one `sgw_player.world_location` accepts, and the position must
/// be finite (a NaN makes the next world entry undefined).
async fn precheck(pool: &PgPool, account_id: i32, f: &StagedFixture) -> Result<(), String> {
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
    Ok(())
}
