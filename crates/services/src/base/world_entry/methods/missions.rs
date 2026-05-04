use std::sync::Arc;

use sqlx::PgPool;

use crate::cell::messages::SavedMission;

pub async fn query_saved_missions(
    db_pool: &Option<Arc<PgPool>>,
    player_id: i32,
) -> Vec<SavedMission> {
    let pool = match db_pool {
        Some(p) => p,
        None => return vec![],
    };

    #[derive(sqlx::FromRow)]
    struct MissionRow {
        mission_id: i32,
        status: i32,
        current_step_id: Option<i32>,
        completed_step_ids: Vec<i32>,
        completed_objective_ids: Vec<i32>,
        active_objective_ids: Vec<i32>,
        failed_objective_ids: Vec<i32>,
        repeats: i32,
    }

    match sqlx::query_as::<_, MissionRow>(
        "SELECT mission_id, status, current_step_id, \
         completed_step_ids, completed_objective_ids, active_objective_ids, failed_objective_ids, \
         repeats \
         FROM sgw_mission WHERE player_id = $1",
    )
    .bind(player_id)
    .fetch_all(pool.as_ref())
    .await
    {
        Ok(rows) => {
            let missions: Vec<_> = rows
                .into_iter()
                .map(|r| {
                    let status = i8::try_from(r.status).unwrap_or_else(|_| {
                        tracing::warn!(
                            player_id,
                            mission_id = r.mission_id,
                            db_status = r.status,
                            "Mission status out of i8 range, clamping"
                        );
                        // Clamp at i32, then cast — casting first wraps modulo 256.
                        r.status.clamp(i8::MIN as i32, i8::MAX as i32) as i8
                    });
                    SavedMission {
                        mission_id: r.mission_id,
                        status,
                        current_step_id: r.current_step_id,
                        completed_step_ids: r.completed_step_ids,
                        completed_objective_ids: r.completed_objective_ids,
                        active_objective_ids: r.active_objective_ids,
                        failed_objective_ids: r.failed_objective_ids,
                        repeats: r.repeats,
                    }
                })
                .collect();
            tracing::info!(
                player_id,
                count = missions.len(),
                "Loaded saved missions from DB"
            );
            missions
        }
        Err(e) => {
            tracing::error!(player_id, "Failed to query saved missions: {e}");
            vec![]
        }
    }
}

pub async fn handle_mission_update(
    player_id: i32,
    mission_id: i32,
    status: i8,
    current_step_id: Option<i32>,
    completed_step_ids: &[i32],
    completed_objective_ids: &[i32],
    active_objective_ids: &[i32],
    failed_objective_ids: &[i32],
    repeats: i32,
    db_pool: &Option<Arc<PgPool>>,
) {
    let pool = match db_pool {
        Some(p) => p,
        None => {
            tracing::debug!(player_id, mission_id, "MissionUpdate: no DB pool");
            return;
        }
    };

    // `repeats = EXCLUDED.repeats` is the fix for #118 — the prior UPSERT
    // omitted this column, so re-completing a repeatable mission appeared to
    // reset the counter on relog instead of advancing it. Cell is the
    // authoritative source for the post-bump value (set by
    // `MissionInstance::complete`/`fail`).
    let result = sqlx::query(
        "INSERT INTO sgw_mission (player_id, mission_id, status, current_step_id, \
         completed_step_ids, completed_objective_ids, active_objective_ids, failed_objective_ids, \
         repeats) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9) \
         ON CONFLICT (player_id, mission_id) DO UPDATE SET \
         status = EXCLUDED.status, \
         current_step_id = EXCLUDED.current_step_id, \
         completed_step_ids = EXCLUDED.completed_step_ids, \
         completed_objective_ids = EXCLUDED.completed_objective_ids, \
         active_objective_ids = EXCLUDED.active_objective_ids, \
         failed_objective_ids = EXCLUDED.failed_objective_ids, \
         repeats = EXCLUDED.repeats",
    )
    .bind(player_id)
    .bind(mission_id)
    .bind(status as i32)
    .bind(current_step_id)
    .bind(completed_step_ids)
    .bind(completed_objective_ids)
    .bind(active_objective_ids)
    .bind(failed_objective_ids)
    .bind(repeats)
    .execute(pool.as_ref())
    .await;

    match result {
        Ok(_) => tracing::debug!(
            player_id,
            mission_id,
            status,
            repeats,
            "Mission state persisted"
        ),
        Err(e) => tracing::error!(player_id, mission_id, "Failed to persist mission: {e}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::require_db_or_skip;

    /// Sentinel base for mission tests. Distinct from grant_cash (0x7000_0100),
    /// move_inventory (0x7000_0200), grant_item (0x7000_0300).
    const TEST_PLAYER_BASE: i32 = 0x7000_0400;

    /// sgw_mission has a FK to sgw_player. Cleanup deletes the account, which
    /// cascades sgw_player rows; sgw_mission rows go away when the player rows
    /// they reference are deleted (or via the FK rule). Mission rows are also
    /// deleted explicitly first to avoid relying on cascade ordering.
    async fn cleanup(pool: &PgPool, account_id: i32, player_id: i32) {
        let _ = sqlx::query("DELETE FROM sgw_mission WHERE player_id = $1")
            .bind(player_id)
            .execute(pool)
            .await;
        let _ = sqlx::query("DELETE FROM account WHERE account_id = $1")
            .bind(account_id)
            .execute(pool)
            .await;
    }

    async fn insert_account_and_player(pool: &PgPool, account_id: i32, player_id: i32) {
        sqlx::query(
            "INSERT INTO account (account_id, account_name, password) \
             VALUES ($1, $2, '')",
        )
        .bind(account_id)
        .bind(format!("mission-test-{account_id}"))
        .execute(pool)
        .await
        .expect("insert account");

        sqlx::query(
            "INSERT INTO sgw_player (\
                account_id, player_id, level, alignment, archetype, gender, \
                player_name, extra_name, world_location, bodyset, \
                pos_x, pos_y, pos_z, skin_color_id, naquadah\
             ) VALUES ($1, $2, 1, 0, 1, 1, $3, '', 'CombatSim', 'BS_HumanMale.BS_HumanMale', \
                       0.0, 0.0, 0.0, 0, 0)",
        )
        .bind(account_id)
        .bind(player_id)
        .bind(format!("test-{player_id}"))
        .execute(pool)
        .await
        .expect("insert player");
    }

    /// Happy path: handle_mission_update INSERTs a row with all fields present.
    #[tokio::test]
    async fn inserts_new_mission_row_with_all_fields() {
        let pool = require_db_or_skip!();
        let account_id = TEST_PLAYER_BASE;
        let player_id = TEST_PLAYER_BASE + 1;
        cleanup(&pool, account_id, player_id).await;
        insert_account_and_player(&pool, account_id, player_id).await;
        let db_pool = Some(Arc::new(pool.clone()));

        handle_mission_update(
            player_id,
            12345,
            2,
            Some(7),
            &[1, 2, 3],
            &[10, 20],
            &[30],
            &[],
            5,
            &db_pool,
        )
        .await;

        // Filter on (player_id, mission_id) and use fetch_one so a regression
        // that inserted multiple rows would surface either as the count check
        // failing or as fetch_one's "expected one row" error.
        let row_count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM sgw_mission WHERE player_id = $1 AND mission_id = $2",
        )
        .bind(player_id)
        .bind(12345)
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(row_count, 1, "INSERT must produce exactly one row");

        let row: (
            i32,
            i32,
            Option<i32>,
            Vec<i32>,
            Vec<i32>,
            Vec<i32>,
            Vec<i32>,
            i32,
        ) = sqlx::query_as(
            "SELECT mission_id, status, current_step_id, \
                    completed_step_ids, completed_objective_ids, \
                    active_objective_ids, failed_objective_ids, repeats \
             FROM sgw_mission WHERE player_id = $1 AND mission_id = $2",
        )
        .bind(player_id)
        .bind(12345)
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(row.0, 12345, "mission_id");
        assert_eq!(row.1, 2, "status");
        assert_eq!(row.2, Some(7), "current_step_id");
        assert_eq!(row.3, vec![1, 2, 3], "completed_step_ids");
        assert_eq!(row.4, vec![10, 20], "completed_objective_ids");
        assert_eq!(row.5, vec![30], "active_objective_ids");
        assert_eq!(row.6, Vec::<i32>::new(), "failed_objective_ids");
        assert_eq!(row.7, 5, "repeats");

        cleanup(&pool, account_id, player_id).await;
    }

    /// Regression guard: the prior UPSERT omitted `repeats = EXCLUDED.repeats`,
    /// so re-completing a repeatable mission would appear to reset the counter
    /// on relog instead of advancing it. This test seeds a row with repeats=3,
    /// then runs an UPSERT with repeats=4 — the row must persist 4, not 3.
    #[tokio::test]
    async fn upsert_propagates_repeats_column_on_conflict() {
        let pool = require_db_or_skip!();
        let account_id = TEST_PLAYER_BASE + 100;
        let player_id = TEST_PLAYER_BASE + 101;
        cleanup(&pool, account_id, player_id).await;
        insert_account_and_player(&pool, account_id, player_id).await;
        let db_pool = Some(Arc::new(pool.clone()));

        // Seed: mission already at repeats=3, status=1.
        handle_mission_update(player_id, 9999, 1, None, &[], &[], &[], &[], 3, &db_pool).await;

        // Re-complete: cell now sends status=2, repeats=4.
        handle_mission_update(player_id, 9999, 2, None, &[], &[], &[], &[], 4, &db_pool).await;

        let (status, repeats): (i32, i32) = sqlx::query_as(
            "SELECT status, repeats FROM sgw_mission \
             WHERE player_id = $1 AND mission_id = $2",
        )
        .bind(player_id)
        .bind(9999)
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(status, 2, "status must update on conflict");
        assert_eq!(
            repeats, 4,
            "repeats must propagate via EXCLUDED.repeats on conflict — \
             a 3 here means the UPSERT regressed to omitting the column",
        );

        cleanup(&pool, account_id, player_id).await;
    }

    /// Regression guard: query_saved_missions uses i8::try_from(r.status) with
    /// a clamp fallback. A pathological row whose `status` column doesn't fit
    /// in i8 must clamp to i8::MIN/MAX rather than wrap modulo 256 via `as i8`
    /// (which would silently turn a 200 into -56). Done at the query layer
    /// because the `status` column is `integer` so legitimate writes can
    /// outpace the i8 wire/cell representation.
    #[tokio::test]
    async fn query_saved_missions_clamps_out_of_range_status_to_i8_max() {
        let pool = require_db_or_skip!();
        let account_id = TEST_PLAYER_BASE + 200;
        let player_id = TEST_PLAYER_BASE + 201;
        cleanup(&pool, account_id, player_id).await;
        insert_account_and_player(&pool, account_id, player_id).await;

        // Insert directly via SQL so we can write a status that handle_mission_update
        // (which takes i8) couldn't produce. 500 is well outside i8 range.
        sqlx::query(
            "INSERT INTO sgw_mission \
                (player_id, mission_id, status, current_step_id, \
                 completed_step_ids, completed_objective_ids, \
                 active_objective_ids, failed_objective_ids, repeats) \
             VALUES ($1, 1, 500, NULL, '{}', '{}', '{}', '{}', 0)",
        )
        .bind(player_id)
        .execute(&pool)
        .await
        .expect("insert oversize-status mission");

        let db_pool = Some(Arc::new(pool.clone()));
        let missions = query_saved_missions(&db_pool, player_id).await;
        assert_eq!(missions.len(), 1);
        assert_eq!(
            missions[0].status,
            i8::MAX,
            "status 500 (outside i8) must clamp to i8::MAX, not wrap modulo 256 to -12",
        );

        cleanup(&pool, account_id, player_id).await;
    }

    /// Cross-service triangle: cell completion → CellToBaseMsg::MissionUpdate
    /// (routed through `cell_dispatch::handle_cell_message`, the actual
    /// production bridge) → base UPSERT → re-login query. Routing through
    /// `handle_cell_message` instead of calling `handle_mission_update`
    /// directly catches a regression in the dispatch arm itself — e.g.
    /// dropping `repeats` from the destructure or wiring one of the Vec
    /// fields to the wrong handler argument.
    ///
    /// What the test does NOT pin: the `current_step_id`, `completed_step_ids`,
    /// `completed_objective_ids`, `active_objective_ids`, `failed_objective_ids`
    /// fields are all empty / None on the wire because the executor
    /// (cell/content/executor.rs::CompleteMission) hardcodes them empty
    /// for completion — that's the production message shape, lossy by
    /// design. The cell-side `MissionInstance` does carry the completed
    /// step + objective in `completed_steps` / `completed_objectives`,
    /// but those values are not transmitted to base today. Adding wire
    /// fidelity for them is a separate concern; this test pins the
    /// EXISTING contract (lossy completion) round-trips end-to-end.
    #[tokio::test]
    async fn mission_completion_cell_to_base_to_db_to_relogin_round_trip() {
        use crate::base::world_entry::handle_cell_message;
        use crate::cell::messages::CellToBaseMsg;
        use crate::cell::missions::{accept_mission, complete_mission_direct};
        use crate::cell::space_manager::SpaceManager;
        use cimmeria_entity::missions::{MissionObjective, STATUS_ACTIVE};
        use std::collections::HashMap;
        use std::sync::Mutex;
        use tokio::net::UdpSocket;
        use tokio::sync::mpsc;

        let pool = require_db_or_skip!();
        let account_id = TEST_PLAYER_BASE + 400;
        let player_id = TEST_PLAYER_BASE + 401;
        cleanup(&pool, account_id, player_id).await;
        insert_account_and_player(&pool, account_id, player_id).await;
        let db_pool = Some(Arc::new(pool.clone()));

        // ── 1. Cell-side setup ──────────────────────────────────────
        const MISSION_ID: i32 = 0x7000_0410;
        const STEP_ID: i32 = 0x7000_0411;
        const OBJ_ID: i32 = 0x7000_0412;
        const ENTITY_ID: u32 = 1;

        let mut mgr = SpaceManager::new(1);
        let xml = r#"<?xml version="1.0"?><Spaces><Space WorldName="Agnos" Instanced="false" MinX="0" MaxX="100" MinY="0" MaxY="100" /></Spaces>"#;
        let cxml = r#"<?xml version="1.0"?><Spaces><Space WorldName="Agnos" /></Spaces>"#;
        mgr.parse_spaces_xml(xml).unwrap();
        mgr.create_startup_spaces(cxml).unwrap();
        mgr.create_entity(ENTITY_ID, "Agnos", [0.0; 3], [0.0; 3])
            .unwrap();

        let (tx, _rx) = mpsc::channel::<CellToBaseMsg>(64);
        let objectives = vec![MissionObjective {
            objective_id: OBJ_ID,
            status: STATUS_ACTIVE,
            hidden: false,
            optional: false,
        }];
        accept_mission(ENTITY_ID, MISSION_ID, STEP_ID, objectives, &tx, &mut mgr).await;

        // ── 2. Cell-side completion ─────────────────────────────────
        complete_mission_direct(ENTITY_ID, MISSION_ID, &tx, &mut mgr).await;
        let post_repeats = mgr
            .get_entity(ENTITY_ID)
            .and_then(|e| e.missions.get_mission(MISSION_ID))
            .map(|m| m.repeats)
            .unwrap();
        assert_eq!(
            post_repeats, 1,
            "MissionInstance::complete must bump repeats (Python parity)",
        );

        // ── 3. Build the EXACT message the executor emits ───────────
        // Field set mirrors crates/services/src/cell/content/executor.rs
        // CompleteMission arm. Empty Vecs / None are the executor's
        // intentional contract for a "completed" message.
        let msg = CellToBaseMsg::MissionUpdate {
            player_id,
            mission_id: MISSION_ID,
            status: 2,
            current_step_id: None,
            completed_step_ids: vec![],
            completed_objective_ids: vec![],
            active_objective_ids: vec![],
            failed_objective_ids: vec![],
            repeats: post_repeats,
        };

        // ── 4. Drive through the production cell_dispatch arm ──────
        // handle_cell_message is the actual cell→base bridge — a
        // regression in its MissionUpdate destructure (dropped repeats,
        // mis-bound Vec args) breaks the test here rather than silently
        // shipping. The MissionUpdate arm only touches db_pool; the
        // socket / connected / cell_tx / minigame fields aren't used,
        // so we wire the minimum stubs.
        let socket = Arc::new(UdpSocket::bind("127.0.0.1:0").await.expect("bind UDP"));
        let connected = Arc::new(Mutex::new(HashMap::new()));
        let entity_to_addr = Arc::new(Mutex::new(HashMap::new()));
        let cell_tx_opt: Option<mpsc::Sender<crate::cell::messages::BaseToCellMsg>> = None;
        let minigame_registry = None;
        handle_cell_message(
            msg,
            &socket,
            &connected,
            &entity_to_addr,
            &cell_tx_opt,
            &db_pool,
            &minigame_registry,
            "127.0.0.1",
            0,
        )
        .await;

        // ── 5. Re-login: query_saved_missions reads the row back ────
        let restored = query_saved_missions(&db_pool, player_id).await;
        let mission = restored
            .iter()
            .find(|m| m.mission_id == MISSION_ID)
            .expect("mission_id must round-trip across cell→base→DB");
        assert_eq!(mission.status, 2, "status round-trip");
        assert_eq!(
            mission.repeats, post_repeats,
            "repeats from cell-side bump must round-trip via the cell_dispatch bridge",
        );
        assert!(
            mission.current_step_id.is_none(),
            "completion clears current_step_id"
        );
        // Pin the executor's lossy contract end-to-end: the wire shape
        // sends empty Vecs, so the DB row + re-login query reflect that.
        assert!(mission.completed_step_ids.is_empty());
        assert!(mission.completed_objective_ids.is_empty());
        assert!(mission.active_objective_ids.is_empty());
        assert!(mission.failed_objective_ids.is_empty());

        cleanup(&pool, account_id, player_id).await;
    }

    /// Repeat-counter cross-service round-trip: complete the mission twice
    /// across two cell→base→DB cycles. Pins that the post-bump `repeats`
    /// from the SECOND completion is what the DB row carries — the
    /// counter advances rather than stalling. A regression here would
    /// silently re-strand repeatable missions even with the
    /// `EXCLUDED.repeats` UPSERT in place, because the cell→base bridge
    /// or the UPSERT could regress independently.
    #[tokio::test]
    async fn second_completion_advances_persisted_repeats_counter_end_to_end() {
        use crate::cell::missions::{accept_mission, complete_mission_direct};
        use crate::cell::space_manager::SpaceManager;
        use cimmeria_entity::missions::{MissionObjective, STATUS_ACTIVE};
        use tokio::sync::mpsc;

        let pool = require_db_or_skip!();
        let account_id = TEST_PLAYER_BASE + 500;
        let player_id = TEST_PLAYER_BASE + 501;
        cleanup(&pool, account_id, player_id).await;
        insert_account_and_player(&pool, account_id, player_id).await;
        let db_pool = Some(Arc::new(pool.clone()));

        const MISSION_ID: i32 = 0x7000_0510;
        const STEP_ID: i32 = 0x7000_0511;
        const OBJ_ID: i32 = 0x7000_0512;
        const ENTITY_ID: u32 = 1;

        let mut mgr = SpaceManager::new(1);
        let xml = r#"<?xml version="1.0"?><Spaces><Space WorldName="Agnos" Instanced="false" MinX="0" MaxX="100" MinY="0" MaxY="100" /></Spaces>"#;
        let cxml = r#"<?xml version="1.0"?><Spaces><Space WorldName="Agnos" /></Spaces>"#;
        mgr.parse_spaces_xml(xml).unwrap();
        mgr.create_startup_spaces(cxml).unwrap();
        mgr.create_entity(ENTITY_ID, "Agnos", [0.0; 3], [0.0; 3])
            .unwrap();

        let (tx, _rx) = mpsc::channel::<crate::cell::messages::CellToBaseMsg>(64);
        let objectives = || {
            vec![MissionObjective {
                objective_id: OBJ_ID,
                status: STATUS_ACTIVE,
                hidden: false,
                optional: false,
            }]
        };

        // ── First completion: repeats 0 → 1 ─────────────────────────
        accept_mission(ENTITY_ID, MISSION_ID, STEP_ID, objectives(), &tx, &mut mgr).await;
        complete_mission_direct(ENTITY_ID, MISSION_ID, &tx, &mut mgr).await;
        let after_first = mgr
            .get_entity(ENTITY_ID)
            .and_then(|e| e.missions.get_mission(MISSION_ID))
            .map(|m| m.repeats)
            .unwrap();
        assert_eq!(after_first, 1, "first completion bumps repeats to 1");
        handle_mission_update(
            player_id,
            MISSION_ID,
            2,
            None,
            &[],
            &[],
            &[],
            &[],
            after_first,
            &db_pool,
        )
        .await;

        // ── Re-accept: repeats carry forward ────────────────────────
        // accept_mission reads prior repeats off the cell state and sets
        // them on the fresh MissionInstance — so the second completion
        // bumps from 1 to 2, not 0 to 1. Without this carry-forward, a
        // re-accept would reset the counter on the cell and the next
        // UPSERT would persist the reset value.
        accept_mission(ENTITY_ID, MISSION_ID, STEP_ID, objectives(), &tx, &mut mgr).await;
        complete_mission_direct(ENTITY_ID, MISSION_ID, &tx, &mut mgr).await;
        let after_second = mgr
            .get_entity(ENTITY_ID)
            .and_then(|e| e.missions.get_mission(MISSION_ID))
            .map(|m| m.repeats)
            .unwrap();
        assert_eq!(
            after_second, 2,
            "second completion advances repeats: re-accept must carry forward prior count"
        );
        handle_mission_update(
            player_id,
            MISSION_ID,
            2,
            None,
            &[],
            &[],
            &[],
            &[],
            after_second,
            &db_pool,
        )
        .await;

        // ── Re-login query: DB row carries the post-second-bump count ─
        let restored = query_saved_missions(&db_pool, player_id).await;
        let mission = restored
            .iter()
            .find(|m| m.mission_id == MISSION_ID)
            .expect("mission must round-trip");
        assert_eq!(
            mission.repeats, 2,
            "DB row must carry the cell's post-second-completion repeats — \
             a 1 here means the UPSERT regressed to dropping the column on conflict",
        );

        cleanup(&pool, account_id, player_id).await;
    }

    /// Companion: clamp on the negative side too. A status of -200 must
    /// clamp to i8::MIN (-128), not wrap to +56.
    #[tokio::test]
    async fn query_saved_missions_clamps_out_of_range_status_to_i8_min() {
        let pool = require_db_or_skip!();
        let account_id = TEST_PLAYER_BASE + 300;
        let player_id = TEST_PLAYER_BASE + 301;
        cleanup(&pool, account_id, player_id).await;
        insert_account_and_player(&pool, account_id, player_id).await;

        sqlx::query(
            "INSERT INTO sgw_mission \
                (player_id, mission_id, status, current_step_id, \
                 completed_step_ids, completed_objective_ids, \
                 active_objective_ids, failed_objective_ids, repeats) \
             VALUES ($1, 1, -200, NULL, '{}', '{}', '{}', '{}', 0)",
        )
        .bind(player_id)
        .execute(&pool)
        .await
        .expect("insert negative-status mission");

        let db_pool = Some(Arc::new(pool.clone()));
        let missions = query_saved_missions(&db_pool, player_id).await;
        assert_eq!(missions.len(), 1);
        assert_eq!(
            missions[0].status,
            i8::MIN,
            "status -200 must clamp to i8::MIN, not wrap modulo 256 to +56",
        );

        cleanup(&pool, account_id, player_id).await;
    }
}
