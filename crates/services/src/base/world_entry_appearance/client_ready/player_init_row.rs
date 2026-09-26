//! The one `sgw_player` read behind `InitPlayerState`.
//!
//! Split out of `client_ready/mod.rs` so the columns it hydrates onto the
//! cell entity can be round-tripped against a live database: the trainer
//! purchase writes `trained_abilities`, `tree_points_spent` and
//! `training_points`, and a relog must read back exactly what it wrote.

use sqlx::PgPool;

/// Everything `onClientReady` hydrates onto the cell from `sgw_player`.
#[derive(Debug, sqlx::FromRow)]
pub(super) struct PlayerInitRow {
    pub(super) bandolier_slot: i32,
    pub(super) auto_reload: bool,
    pub(super) reload_on_activate: bool,
    pub(super) state_field: i32,
    pub(super) known_stargates: Vec<i32>,
    pub(super) trained_abilities: Vec<i32>,
    pub(super) tree_points_spent: i32,
    // The trainer gates' level and point inputs (AT-03). Read in the same
    // statement as the spend, so the three cannot straddle a purchase.
    pub(super) training_points: i32,
    pub(super) level: i32,
    // The character's body set, for its line-of-sight eye height on the
    // cell (NA31).
    pub(super) bodyset: Option<String>,
}

/// `Ok(None)` when no row has `player_id`.
pub(super) async fn load_player_init_row(
    pool: &PgPool,
    player_id: i32,
) -> sqlx::Result<Option<PlayerInitRow>> {
    sqlx::query_as::<_, PlayerInitRow>(
        "SELECT bandolier_slot, auto_reload, reload_on_activate, state_field, \
                known_stargates, trained_abilities, tree_points_spent, \
                training_points, level, bodyset \
           FROM sgw_player WHERE player_id = $1",
    )
    .bind(player_id)
    .fetch_optional(pool)
    .await
}

#[cfg(test)]
mod tests {
    //! The deferred AT-01 round-trip: what a trainer purchase writes is what
    //! the next world entry hydrates onto the cell.
    use super::*;
    use crate::base::world_entry::methods::progression::persist_purchase;
    use crate::test_support::require_db_or_skip;

    const ID: i32 = 0x7030_0310;

    async fn cleanup(pool: &PgPool) {
        let _ = sqlx::query("DELETE FROM account WHERE account_id = $1")
            .bind(ID)
            .execute(pool)
            .await;
    }

    #[tokio::test]
    async fn purchase_round_trips_through_the_world_entry_read() {
        let pool = require_db_or_skip!();
        cleanup(&pool).await;
        sqlx::query("INSERT INTO account (account_id, account_name, password) VALUES ($1, $2, '')")
            .bind(ID)
            .bind(format!("at03-roundtrip-{ID}"))
            .execute(&pool)
            .await
            .expect("insert account");
        sqlx::query(
            "INSERT INTO sgw_player (\
                account_id, player_id, level, alignment, archetype, gender, \
                player_name, extra_name, world_location, bodyset, \
                pos_x, pos_y, pos_z, skin_color_id, training_points\
             ) VALUES ($1, $1, 7, 0, 1, 1, $2, '', 'CombatSim', \
                       'BS_HumanMale.BS_HumanMale', 0.0, 0.0, 0.0, 0, 6)",
        )
        .bind(ID)
        .bind(format!("at03-roundtrip-{ID}"))
        .execute(&pool)
        .await
        .expect("insert player");

        let fresh = load_player_init_row(&pool, ID).await.unwrap().unwrap();
        persist_purchase(&pool, ID, 597, 2)
            .await
            .unwrap()
            .expect("first buy");
        persist_purchase(&pool, ID, 598, 1)
            .await
            .unwrap()
            .expect("second buy");
        let bought = load_player_init_row(&pool, ID).await.unwrap().unwrap();
        cleanup(&pool).await;

        assert_eq!(
            (fresh.trained_abilities, fresh.tree_points_spent),
            (Vec::<i32>::new(), 0),
            "a new character starts with no provenance and no spend"
        );
        assert_eq!(
            (
                bought.trained_abilities,
                bought.tree_points_spent,
                bought.training_points,
                bought.level,
            ),
            (vec![597, 598], 3, 3, 7),
            "the world-entry read returns the purchases, the spend, the points and the level"
        );
    }
}
