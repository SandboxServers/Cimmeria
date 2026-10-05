//! The database half of the AM-02 reserve requests: one transaction per
//! request that moves rounds between the bag stacks and the weapon row.
//! The shell that answers the cell and updates the client is
//! [`super::requests`].
//!
//! Locks: the advisory keys for the reserve bags and the bandolier first,
//! then the weapon row `FOR UPDATE`, then (inside `draw` / `return_rounds`)
//! the stacks, the repo-wide order. The locked weapon row, not the cell's
//! numbers, says how many rounds are in the clip, so a duplicated request
//! moves nothing.

use cimmeria_entity::inventory::{INV_BANDOLIER, INV_CRAFTING, INV_MAIN};
use cimmeria_entity::known_names;
use sqlx::{PgPool, Postgres, Transaction};

use super::{draw, return_rounds, AmmoDraw, AmmoReturn};
use crate::base::inventory_locks::take_inventory_locks;
use crate::cell::messages::ReserveRefusal;

/// A committed reload draw.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DrawCommit {
    /// `clip_size - clip_before`, from the weapon's design.
    pub requested: i32,
    pub clip_before: i32,
    /// What the weapon row now holds.
    pub clip_after: i32,
    pub draw: AmmoDraw,
}

/// A committed switch return.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReturnCommit {
    /// `true` when every round fit and the weapon now holds the new type.
    pub switched: bool,
    /// The rounds the weapon row held, all of which were offered back.
    pub rounds: i32,
    pub ret: AmmoReturn,
}

/// The bags the reserve touches plus the bandolier the weapon sits in.
const LOCKED_BAGS: [i32; 3] = [INV_MAIN, INV_BANDOLIER, INV_CRAFTING];

/// The weapon row a reserve request acts on, read under its lock.
#[derive(Debug, sqlx::FromRow)]
struct WeaponRow {
    /// The design's clip size (`resources.items.clip_size`).
    clip_size: i32,
    /// Rounds in the clip (`sgw_inventory.ammo`).
    ammo: i32,
    cur_ammo_type: i32,
}

/// Lock the weapon row still in `slot_id` as `instance_id`, or `None` when
/// the row moved.
async fn lock_weapon(
    tx: &mut Transaction<'_, Postgres>,
    player_id: i32,
    slot_id: i32,
    instance_id: i32,
) -> Result<Option<WeaponRow>, sqlx::Error> {
    sqlx::query_as(
        "SELECT ri.clip_size, i.ammo, i.cur_ammo_type FROM sgw_inventory i \
           JOIN resources.items ri ON ri.item_id = i.type_id \
          WHERE i.character_id = $1 AND i.container_id = $2 \
            AND i.slot_id = $3 AND i.item_id = $4 \
          FOR UPDATE OF i",
    )
    .bind(player_id)
    .bind(INV_BANDOLIER)
    .bind(slot_id)
    .bind(instance_id)
    .fetch_optional(&mut **tx)
    .await
}

async fn write_weapon(
    tx: &mut Transaction<'_, Postgres>,
    instance_id: i32,
    ammo: i32,
    cur_ammo_type: i32,
) -> Result<u64, sqlx::Error> {
    Ok(
        sqlx::query("UPDATE sgw_inventory SET ammo = $1, cur_ammo_type = $2 WHERE item_id = $3")
            .bind(ammo)
            .bind(cur_ammo_type)
            .bind(instance_id)
            .execute(&mut **tx)
            .await?
            .rows_affected(),
    )
}

/// Draw rounds for a special reload and load them into the weapon, in one
/// transaction. `Ok(Err(_))` is a refusal that committed nothing; a
/// `requested` of 0 (the clip was already full) commits nothing either and
/// reports `drawn == 0`.
pub async fn commit_reload_draw(
    pool: &PgPool,
    player_id: i32,
    slot_id: i32,
    instance_id: i32,
    ammo_type: i32,
    clip_before: i32,
) -> Result<Result<DrawCommit, ReserveRefusal>, sqlx::Error> {
    let mut tx = pool.begin().await?;
    take_inventory_locks(&mut tx, player_id, &LOCKED_BAGS).await?;
    let Some(WeaponRow {
        clip_size,
        ammo: row_ammo,
        ..
    }) = lock_weapon(&mut tx, player_id, slot_id, instance_id).await?
    else {
        tx.rollback().await?;
        return Ok(Err(ReserveRefusal::WeaponChanged));
    };
    // The locked row, not the cell's `clip_before`, says how many rounds
    // fit: the cell flushes the clip ahead of the request on the same
    // ordered channel, so the row is current, and a duplicated or replayed
    // request finds the clip already full instead of drawing a second time.
    if row_ammo != clip_before {
        tracing::debug!(
            target: "ammo",
            event = "reload_draw_clip_mismatch",
            player_id,
            player_name = known_names::player_name(player_id),
            slot_id, // nt:id-only slot index, unnamed
            instance_id, // nt:id-only weapon instance in slot_id
            clip_before, row_ammo,
            "reload draw: the cell's clip differs from the weapon row; the row wins"
        );
    }
    let clip_before = row_ammo.clamp(0, clip_size.max(0));
    let requested = clip_size - clip_before;
    if requested <= 0 {
        tx.rollback().await?;
        return Ok(Ok(DrawCommit {
            requested: 0,
            clip_before,
            clip_after: clip_before,
            draw: AmmoDraw::default(),
        }));
    }
    let d = draw(&mut tx, player_id, ammo_type, requested).await?;
    if d.drawn <= 0 {
        tx.rollback().await?;
        return Ok(Err(ReserveRefusal::StackEmpty));
    }
    let clip_after = clip_before + d.drawn;
    if write_weapon(&mut tx, instance_id, clip_after, ammo_type).await? != 1 {
        tx.rollback().await?;
        return Ok(Err(ReserveRefusal::WeaponChanged));
    }
    tx.commit().await?;
    Ok(Ok(DrawCommit {
        requested,
        clip_before,
        clip_after,
        draw: d,
    }))
}

/// Return a clip's special rounds to the bags and settle the weapon row, in
/// one transaction (see the module docs for the bags-full rule).
pub async fn commit_switch_return(
    pool: &PgPool,
    player_id: i32,
    slot_id: i32,
    instance_id: i32,
    from_ammo_type: i32,
    to_ammo_type: i32,
    rounds: i32,
) -> Result<Result<ReturnCommit, ReserveRefusal>, sqlx::Error> {
    let mut tx = pool.begin().await?;
    take_inventory_locks(&mut tx, player_id, &LOCKED_BAGS).await?;
    // As for the draw, the locked row is the truth: the cell flushes the
    // clip (rounds and type) ahead of the request. A row already on another
    // type was switched before (a duplicated request) and returns nothing.
    let row = lock_weapon(&mut tx, player_id, slot_id, instance_id).await?;
    let Some(row) = row.filter(|r| r.cur_ammo_type == from_ammo_type) else {
        tx.rollback().await?;
        return Ok(Err(ReserveRefusal::WeaponChanged));
    };
    if row.ammo != rounds {
        tracing::debug!(
            target: "ammo",
            event = "switch_return_clip_mismatch",
            player_id,
            player_name = known_names::player_name(player_id),
            slot_id, // nt:id-only slot index, unnamed
            instance_id, // nt:id-only weapon instance in slot_id
            rounds, row_ammo = row.ammo,
            "switch return: the cell's clip differs from the weapon row; the row wins"
        );
    }
    let rounds = row.ammo.max(0);
    let ret = return_rounds(&mut tx, player_id, from_ammo_type, rounds).await?;
    let switched = ret.remainder == 0;
    let (ammo, cur_type) = if switched {
        (0, to_ammo_type)
    } else {
        (ret.remainder, from_ammo_type)
    };
    if write_weapon(&mut tx, instance_id, ammo, cur_type).await? != 1 {
        tx.rollback().await?;
        return Ok(Err(ReserveRefusal::WeaponChanged));
    }
    tx.commit().await?;
    Ok(Ok(ReturnCommit {
        switched,
        rounds,
        ret,
    }))
}
