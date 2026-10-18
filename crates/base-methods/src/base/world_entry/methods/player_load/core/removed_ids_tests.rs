//! Live-DB guards: a player row that still carries ids removed from the
//! resources must load (PP-03, D-PP9 "non-FK ids are left in place").
//!
//! The persistent-DB rebuild keeps `sgw_player` arrays untouched, so a seed
//! change can leave ids that no longer resolve. These pin that the loaders
//! carry such ids unchanged and neither fall back to default data nor log a
//! degraded event. `0x7000_2A00` is taken by `cell-content`, so the base is
//! `TEST_BASE = 0x7000_2B00` (account `TEST_BASE`, player `TEST_BASE + 1`).

use super::*;
use crate::base::world_entry::methods::missions::query_saved_missions;
use crate::test_support::{require_db_or_skip, LogCapture};
use tracing::Level;

const TEST_BASE: i32 = 0x7000_2B00;
/// Absent from every resource table; ids are values here, not row keys.
const REMOVED: i32 = 2_147_000_000 - 1;

async fn cleanup(pool: &PgPool, base: i32) {
    let player_id = base + 1;
    for (sql, id) in [
        ("DELETE FROM sgw_mission WHERE player_id = $1", player_id),
        ("DELETE FROM sgw_player WHERE player_id = $1", player_id),
        ("DELETE FROM account WHERE account_id = $1", base),
    ] {
        let _ = sqlx::query(sql).bind(id).execute(pool).await;
    }
}

/// Account, player and one mission, every id column set to [`REMOVED`].
async fn seed_removed_id_player(pool: &PgPool, base: i32) {
    let account_id = base;
    let player_id = base + 1;
    cleanup(pool, base).await;
    sqlx::query("INSERT INTO account (account_id, account_name, password) VALUES ($1, $2, '')")
        .bind(account_id)
        .bind(format!("removed-ids-{account_id}"))
        .execute(pool)
        .await
        .expect("insert account");
    sqlx::query(
        "INSERT INTO sgw_player (\
            account_id, player_id, level, alignment, archetype, gender, \
            player_name, extra_name, world_location, bodyset, \
            pos_x, pos_y, pos_z, skin_color_id, naquadah, bandolier_slot\
         ) VALUES ($1, $2, 1, 0, 1, 1, $3, '', 'CombatSim', 'BS_HumanMale.BS_HumanMale', \
                   0.0, 0.0, 0.0, 0, 0, 0)",
    )
    .bind(account_id)
    .bind(player_id)
    .bind(format!("test-{player_id}"))
    .execute(pool)
    .await
    .expect("insert player");
    sqlx::query(
        "UPDATE sgw_player SET known_stargates = ARRAY[$2], abilities = ARRAY[$2], \
         trained_abilities = ARRAY[$2], discipline_ids = ARRAY[$2], \
         blueprint_ids = ARRAY[$2], known_respawners = ARRAY[$2] WHERE player_id = $1",
    )
    .bind(player_id)
    .bind(REMOVED)
    .execute(pool)
    .await
    .expect("set removed ids");
    sqlx::query(
        "INSERT INTO sgw_mission (player_id, mission_id, current_step_id, status, \
         completed_step_ids, completed_objective_ids, active_objective_ids, \
         failed_objective_ids) VALUES ($1, $2, $2, 1, '{}', '{}', '{}', '{}')",
    )
    .bind(player_id)
    .bind(REMOVED)
    .execute(pool)
    .await
    .expect("insert mission");
}

#[tokio::test]
async fn live_db_player_load_keeps_a_player_with_removed_resource_ids() {
    let pool = require_db_or_skip!();
    seed_removed_id_player(&pool, TEST_BASE).await;
    let db = Some(Arc::new(pool.clone()));

    let data = query_player_load_data(&db, TEST_BASE as u32, TEST_BASE + 1).await;
    let missions = query_saved_missions(&db, TEST_BASE + 1).await;
    cleanup(&pool, TEST_BASE).await;

    assert_eq!(
        data.player_id,
        TEST_BASE + 1,
        "the player must load, not fall back to the default sentinel"
    );
    assert_eq!(data.known_stargates, vec![REMOVED]);
    assert_eq!(data.blueprint_ids, vec![REMOVED]);
    assert_eq!(missions.len(), 1, "the saved mission must load");
    assert_eq!(missions[0].mission_id, REMOVED);
    assert_eq!(missions[0].current_step_id, Some(REMOVED));
}

#[tokio::test]
async fn live_db_player_load_logs_no_degraded_event_for_removed_ids() {
    let pool = require_db_or_skip!();
    seed_removed_id_player(&pool, TEST_BASE).await;
    let db = Some(Arc::new(pool.clone()));
    let capture = LogCapture::install();

    let data = query_player_load_data(&db, TEST_BASE as u32, TEST_BASE + 1).await;
    let _ = query_saved_missions(&db, TEST_BASE + 1).await;
    cleanup(&pool, TEST_BASE).await;

    assert_eq!(data.player_id, TEST_BASE + 1);
    assert!(
        capture
            .find_event(Level::ERROR, "", "player_row_missing")
            .is_none(),
        "removed ids must not look like a missing player row"
    );
    for reason in ["player_query_failed", "missions_query_failed"] {
        assert!(
            capture.find_event(Level::ERROR, "", reason).is_none(),
            "removed ids must not log reason={reason}"
        );
    }
}
