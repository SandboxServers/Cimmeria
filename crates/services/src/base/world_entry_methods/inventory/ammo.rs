//! SQL helpers for persisting per-bandolier-slot ammo state.
//!
//! Stage A wires the message + handler + writer end-to-end so subsequent
//! stages can call this without touching the channel/handler plumbing. The
//! cell does not yet emit `BandolierAmmoUpdate` (Stages B/C/D).

use sqlx::PgPool;

/// Persist the current ammo and selected ammo subtype for a bandolier slot.
///
/// The bandolier is `container_id = 3` (`INV_Bandolier`); the (character_id,
/// container_id, slot_id) triple is unique under
/// `sgw_inventory_unique_slot`, so this UPDATE touches at most one row.
///
/// A zero-row result means the slot was unequipped between the cell sending
/// `BandolierAmmoUpdate` and this UPDATE running — log and move on, the next
/// equip will re-seed from `default_ammo_type` and full clip.
pub async fn update_bandolier_ammo(
    pool: &PgPool,
    character_id: i32,
    slot_id: i32,
    current_ammo: i32,
    cur_ammo_type: i32,
) -> Result<(), sqlx::Error> {
    let res = sqlx::query(
        "UPDATE sgw_inventory \
         SET ammo = $1, cur_ammo_type = $2 \
         WHERE character_id = $3 AND container_id = 3 AND slot_id = $4",
    )
    .bind(current_ammo)
    .bind(cur_ammo_type)
    .bind(character_id)
    .bind(slot_id)
    .execute(pool)
    .await?;

    if res.rows_affected() == 0 {
        tracing::warn!(
            character_id,
            slot_id,
            current_ammo,
            cur_ammo_type,
            "update_bandolier_ammo: no rows updated (slot empty?)"
        );
    }
    Ok(())
}
