//! Move the character before world entry reads its row.
//!
//! The statement shape of the logout `persist_position` (world name, the
//! `world_id` resolved from `resources.worlds`, the three coordinates), with
//! each part optional so a fixture can set the world, the point or both.

use sqlx::PgPool;

/// Write `world` and/or `position` to `sgw_player`. Ok(false) when both are
/// None; Err when the row is missing or the write fails (an unknown world
/// fails the `world_location` foreign key and writes nothing).
pub(crate) async fn move_player(
    pool: &PgPool,
    player_id: i32,
    world: Option<&str>,
    position: Option<[f32; 3]>,
) -> Result<bool, String> {
    if world.is_none() && position.is_none() {
        return Ok(false);
    }
    if let Some(p) = position {
        if !p.iter().all(|c| c.is_finite()) {
            return Err(format!("position {p:?} is not finite"));
        }
    }
    let [x, y, z] = match position {
        Some([x, y, z]) => [Some(x), Some(y), Some(z)],
        None => [None; 3],
    };
    let rows = sqlx::query(
        "UPDATE sgw_player \
           SET world_location = COALESCE($1::text, world_location), \
               world_id = COALESCE((SELECT world_id FROM resources.worlds WHERE world = $1::text), world_id), \
               pos_x = COALESCE($2::real, pos_x), \
               pos_y = COALESCE($3::real, pos_y), \
               pos_z = COALESCE($4::real, pos_z) \
         WHERE player_id = $5",
    )
    .bind(world)
    .bind(x)
    .bind(y)
    .bind(z)
    .bind(player_id)
    .execute(pool)
    .await
    .map_err(|e| format!("moving the character failed: {e}"))?
    .rows_affected();
    if rows != 1 {
        return Err(format!(
            "moving the character updated {rows} rows, expected 1"
        ));
    }
    Ok(true)
}
