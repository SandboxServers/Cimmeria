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
    // The once-per-character loot containers already opened (the
    // `open_loot` gate). Read here so a relog restores the flags.
    pub(super) looted_containers: Vec<String>,
}

/// `Ok(None)` when no row has `player_id`.
pub(super) async fn load_player_init_row(
    pool: &PgPool,
    player_id: i32,
) -> sqlx::Result<Option<PlayerInitRow>> {
    sqlx::query_as::<_, PlayerInitRow>(
        "SELECT bandolier_slot, auto_reload, reload_on_activate, \
                known_stargates, trained_abilities, tree_points_spent, \
                training_points, level, bodyset, \
                looted_containers::text[] AS looted_containers \
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
    async fn live_db_purchase_round_trips_through_the_world_entry_read() {
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

    /// The once-per-character loot flag survives a relog: what
    /// `ContainerLooted` appends is what the next world entry hydrates into
    /// `InitPlayerState.looted_containers`. A second append of the same key
    /// (a duplicated message) does not double it, and the wrong account
    /// matches no row. Revert proof: drop `looted_containers` from the
    /// SELECT and the read comes back without the flag (a compile error
    /// here, then an empty list once the field defaults).
    #[tokio::test]
    async fn live_db_looted_container_flag_survives_relog() {
        use crate::base::world_entry::looted_containers::append_looted_container;
        const LOOT_ID: i32 = 0x7030_0311;
        let pool = require_db_or_skip!();
        let _ = sqlx::query("DELETE FROM account WHERE account_id = $1")
            .bind(LOOT_ID)
            .execute(&pool)
            .await;
        sqlx::query("INSERT INTO account (account_id, account_name, password) VALUES ($1, $2, '')")
            .bind(LOOT_ID)
            .bind(format!("loot-flag-{LOOT_ID}"))
            .execute(&pool)
            .await
            .expect("insert account");
        sqlx::query(
            "INSERT INTO sgw_player (\
                account_id, player_id, level, alignment, archetype, gender, \
                player_name, extra_name, world_location, bodyset, \
                pos_x, pos_y, pos_z, skin_color_id\
             ) VALUES ($1, $1, 3, 0, 1, 1, $2, '', 'Castle', \
                       'BS_HumanMale.BS_HumanMale', 0.0, 0.0, 0.0, 0)",
        )
        .bind(LOOT_ID)
        .bind(format!("loot-flag-{LOOT_ID}"))
        .execute(&pool)
        .await
        .expect("insert player");

        let fresh = load_player_init_row(&pool, LOOT_ID).await.unwrap().unwrap();
        let first = append_looted_container(&pool, LOOT_ID, LOOT_ID, "Castle_PreRomneyChest")
            .await
            .unwrap();
        let again = append_looted_container(&pool, LOOT_ID, LOOT_ID, "Castle_PreRomneyChest")
            .await
            .unwrap();
        let stranger = append_looted_container(&pool, LOOT_ID, LOOT_ID + 1, "Other")
            .await
            .unwrap();
        let relogged = load_player_init_row(&pool, LOOT_ID).await.unwrap().unwrap();
        let _ = sqlx::query("DELETE FROM account WHERE account_id = $1")
            .bind(LOOT_ID)
            .execute(&pool)
            .await;

        assert!(
            fresh.looted_containers.is_empty(),
            "a new character has no flags"
        );
        assert_eq!(first, Some(vec!["Castle_PreRomneyChest".to_string()]));
        assert_eq!(
            again,
            Some(vec!["Castle_PreRomneyChest".to_string()]),
            "a duplicated ContainerLooted must not double-append"
        );
        assert_eq!(stranger, None, "the wrong account matches no row");
        assert_eq!(
            relogged.looted_containers,
            vec!["Castle_PreRomneyChest".to_string()],
            "the next world entry reads the flag back"
        );
    }
}
