use std::sync::Arc;

use sqlx::PgPool;
use tokio::sync::mpsc;

use cimmeria_entity::manager::EntityManager;

use crate::cell::messages::BaseToCellMsg;
use crate::mercury::{WorldEntryInfo, DEFAULT_SPACE_ID, SGWGMPLAYER_CLASS_ID, SGWPLAYER_CLASS_ID};

use super::super::gm_only_worlds::{gm_only_redirect, note_gm_only_redirect};
use super::super::space_registry::resolve_space_id_fallback;

/// Resolve the CREATE_BASE_PLAYER `class_id` byte from the caller's access
/// level. GMs (access_level > 0) come up as SGWGmPlayer (0x03) so the
/// client binds the GM method table and the native gm* cell surface
/// (flattened indices 109+) is reachable; everyone else stays SGWPlayer
/// (0x02). Inherited indices 0-108 don't shift either way (append-at-end
/// inheritance), so the existing player wire path is byte-identical for
/// access_level 0. See `docs/architecture/gm-cell-method-gating.md` and
/// `crate::mercury::SGWGMPLAYER_CLASS_ID` for the derivation.
fn class_id_for_access_level(access_level: u32) -> u8 {
    if access_level > 0 {
        SGWGMPLAYER_CLASS_ID
    } else {
        SGWPLAYER_CLASS_ID
    }
}

/// Sentinel `player_entity_id` returned by [`query_world_entry`] when no real
/// entity could be allocated/registered (DB error or character not found).
/// Callers must treat this as "world entry failed, do not proceed" rather than
/// hard-coding the literal `0`.
pub const NO_ENTITY_ID: u32 = 0;

/// Query the character's world entry data from the database and allocate a player entity ID.
///
/// If a CellService channel is available, sends `CreateEntity` to resolve the space_id
/// dynamically. Otherwise falls back to the hardcoded space ID table.
pub async fn query_world_entry(
    db_pool: &Option<Arc<PgPool>>,
    account_id: u32,
    account_name: Option<String>,
    player_id: i32,
    access_level: u32,
    entity_manager: &Arc<std::sync::Mutex<EntityManager>>,
    cell_tx: &Option<mpsc::Sender<BaseToCellMsg>>,
) -> WorldEntryInfo {
    // GMs spawn as SGWGmPlayer (0x03) so the gm* cell surface is reachable;
    // regular players stay SGWPlayer (0x02) — byte-unchanged login path.
    let class_id = class_id_for_access_level(access_level);
    // Defer allocating the entity_id until we know the player row loaded —
    // otherwise every DB failure / no-character lookup leaks an entity_id
    // in the EntityManager. On hard failure paths (DB error, no character
    // row) we return `player_entity_id = 0` as a "no entry" sentinel so the
    // caller can detect the failure without us also burning an unregistered
    // ID that the cell service never learns about.
    let alloc_entity =
        || -> u32 { entity_manager.lock().unwrap().create_entity("SGWPlayer").0 as u32 };
    let default_entry_with_eid = |player_eid: u32| WorldEntryInfo {
        player_entity_id: player_eid,
        space_id: DEFAULT_SPACE_ID,
        pos: [0.0; 3],
        rot: [0.0; 3],
        world_name: "CombatSim".to_string(),
        class_id,
        world_stargates: vec![],
    };

    let pool = match db_pool {
        Some(p) => p,
        // No-DB mode (e.g. CombatSim smoke test) is a real entry. If a cell
        // channel is up, dispatch CreateEntity for the allocated id so the
        // cell learns about the entity instead of receiving downstream
        // packets (gate-travel / AoI) for an id it never saw. Without that
        // round-trip the cell silently ignores the entity.
        None => {
            let player_eid = alloc_entity();
            if let Some(tx) = cell_tx {
                let (reply_tx, reply_rx) = tokio::sync::oneshot::channel();
                match tx
                    .send(BaseToCellMsg::CreateEntity {
                        entity_id: player_eid,
                        world_name: "CombatSim".to_string(),
                        position: [0.0; 3],
                        rotation: [0.0; 3],
                        // Login always resolves the destination by world name.
                        destination_space_id: None,
                        // Identity-stamp the cell entity at birth so every
                        // cell-side log for this session is attributable.
                        account_id: Some(account_id),
                        player_id: Some(player_id),
                        account_name: account_name.clone(),
                        // No row is read in no-DB mode, so no name.
                        player_name: None,
                        reply_tx,
                    })
                    .await
                {
                    Ok(_) => {
                        // Wait for the cell to ack the registration; the space_id
                        // reply is unused here (default_entry uses DEFAULT_SPACE_ID),
                        // but we must drive the oneshot so the cell completes the
                        // create.
                        let _ = reply_rx.await;
                    }
                    Err(e) => {
                        // Mirror the DB-path log so a closed cell channel is
                        // visible at world entry instead of silently producing
                        // an unregistered entity id.
                        tracing::warn!(
                            "CellService channel closed sending CreateEntity in no-DB mode ({e}) — entity {} will be unregistered with the cell",
                            player_eid
                        );
                    }
                }
            }
            return default_entry_with_eid(player_eid);
        }
    };

    #[derive(sqlx::FromRow)]
    struct EntryRow {
        player_name: String,
        world_location: String,
        pos_x: f32,
        pos_y: f32,
        pos_z: f32,
        alignment: i32,
    }

    match sqlx::query_as::<_, EntryRow>(
        "SELECT player_name, world_location, pos_x, pos_y, pos_z, alignment \
         FROM sgw_player WHERE player_id = $1 AND account_id = $2",
    )
    .bind(player_id)
    .bind(account_id as i32)
    .fetch_optional(pool.as_ref())
    .await
    {
        Ok(Some(mut row)) => {
            // A non-GM saved in a GM-only world (a relog after a GM summon,
            // a demoted GM) enters at their faction's start instead, before
            // the cell ever places them there (D-DA4).
            if let Some(redirect) =
                gm_only_redirect(&row.world_location, access_level, row.alignment)
            {
                note_gm_only_redirect(
                    "login",
                    player_id,
                    Some(&row.player_name),
                    Some(account_id),
                    account_name.as_deref(),
                    access_level,
                    &redirect,
                );
                row.world_location = redirect.world.to_string();
                [row.pos_x, row.pos_y, row.pos_z] = redirect.position;
            }
            // Kept whole so a refused entry below can hand the id back.
            let player_entity = entity_manager.lock().unwrap().create_entity("SGWPlayer");
            let player_eid = player_entity.0 as u32;
            let pos = [row.pos_x, row.pos_y, row.pos_z];

            // `None` only comes from the fallback table, for a world that must
            // fail closed (see `resolve_space_id_fallback`).
            let resolved_space_id = if let Some(tx) = cell_tx {
                let (reply_tx, reply_rx) = tokio::sync::oneshot::channel();
                match tx
                    .send(BaseToCellMsg::CreateEntity {
                        entity_id: player_eid,
                        world_name: row.world_location.clone(),
                        position: pos,
                        rotation: [0.0; 3],
                        // Login always resolves the destination by world name.
                        destination_space_id: None,
                        // Identity-stamp the cell entity at birth so every
                        // cell-side log for this session is attributable.
                        account_id: Some(account_id),
                        player_id: Some(player_id),
                        account_name: account_name.clone(),
                        player_name: Some(row.player_name.clone()),
                        reply_tx,
                    })
                    .await
                {
                    Ok(_) => match reply_rx.await {
                        Ok(sid) => Some(sid),
                        Err(_) => {
                            tracing::warn!(world = %row.world_location, "CellService oneshot dropped -- using fallback");
                            resolve_space_id_fallback(&row.world_location)
                        }
                    },
                    Err(e) => {
                        tracing::warn!(
                            world = %row.world_location,
                            "CellService channel closed sending CreateEntity ({e}) — using fallback space id"
                        );
                        resolve_space_id_fallback(&row.world_location)
                    }
                }
            } else {
                tracing::warn!(
                    player_id, world = %row.world_location,
                    "query_world_entry: cell_tx is None at world entry — falling back to hardcoded space id table; this is likely a service-startup ordering bug"
                );
                resolve_space_id_fallback(&row.world_location)
            };
            // The cell did not place the entity and the saved world has no
            // safe stand-in space: refuse the entry rather than hand the
            // client a space the entity is not in.
            let Some(space_id) = resolved_space_id else {
                tracing::error!(
                    player_id, account_id, entity_id = player_eid,
                    world = %row.world_location, reason = "no_safe_space_fallback",
                    "World entry refused: the cell did not place the entity and this world \
                     has no fallback space — returning sentinel entity id"
                );
                entity_manager.lock().unwrap().destroy_entity(player_entity);
                return default_entry_with_eid(NO_ENTITY_ID);
            };

            let world_stargates = query_world_stargates(db_pool, &row.world_location).await;

            WorldEntryInfo {
                player_entity_id: player_eid,
                space_id,
                pos,
                rot: [0.0; 3],
                world_name: row.world_location.clone(),
                class_id,
                world_stargates,
            }
        }
        Ok(None) => {
            tracing::warn!(
                player_id,
                account_id,
                "Character not found for world entry — returning sentinel entity id"
            );
            default_entry_with_eid(NO_ENTITY_ID)
        }
        Err(e) => {
            tracing::error!(
                player_id,
                account_id,
                "Failed to query world entry ({e}) — returning sentinel entity id"
            );
            default_entry_with_eid(NO_ENTITY_ID)
        }
    }
}

/// Query stargate IDs physically present in a world.
///
/// These IDs populate `setupStargateInfo(worldStargateIds, ...)` and are
/// separate from the player's learned/known address book.
pub async fn query_world_stargates(db_pool: &Option<Arc<PgPool>>, world_name: &str) -> Vec<i32> {
    let pool = match db_pool {
        Some(p) => p,
        None => return vec![],
    };

    match sqlx::query_scalar::<_, Vec<i32>>(
        "SELECT COALESCE(array_agg(s.stargate_id ORDER BY s.stargate_id), ARRAY[]::integer[]) \
         FROM resources.worlds w \
         JOIN resources.stargates s ON s.world_id = w.world_id \
         WHERE w.world = $1",
    )
    .bind(world_name)
    .fetch_one(pool.as_ref())
    .await
    {
        Ok(stargates) => stargates,
        Err(e) => {
            tracing::warn!(world = %world_name, error = %e, "Failed to query world stargates");
            vec![]
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::require_db_or_skip;
    use std::time::Duration;
    use tokio::time::timeout;

    #[tokio::test]
    async fn query_world_entry_no_db_allocates_entity_and_returns_default() {
        let mgr = Arc::new(std::sync::Mutex::new(EntityManager::new()));
        // access_level 0 → regular player → SGWPlayer class.
        let entry = query_world_entry(&None, 1, None, 1, 0, &mgr, &None).await;
        assert_ne!(
            entry.player_entity_id, NO_ENTITY_ID,
            "no-DB mode must allocate a real entity id"
        );
        assert_eq!(entry.world_name, "CombatSim");
        assert_eq!(entry.space_id, DEFAULT_SPACE_ID);
        assert_eq!(entry.pos, [0.0; 3]);
        assert_eq!(entry.class_id, SGWPLAYER_CLASS_ID);
    }

    /// A GM (access_level > 0) world entry must build the
    /// `WorldEntryInfo` with the SGWGmPlayer class id so CREATE_BASE_PLAYER
    /// emits 0x03 and the client binds the GM method table. A regular
    /// player (access_level 0) stays 0x02 — the byte-unchanged property
    /// asserted in the test above. This pins the class flip at the source
    /// where `WorldEntryInfo.class_id` is decided.
    #[tokio::test]
    async fn query_world_entry_gm_access_level_uses_gmplayer_class() {
        let mgr = Arc::new(std::sync::Mutex::new(EntityManager::new()));
        // access_level 2 = GameMaster (any non-zero level → GM class).
        let entry = query_world_entry(&None, 1, None, 1, 2, &mgr, &None).await;
        assert_eq!(
            entry.class_id, SGWGMPLAYER_CLASS_ID,
            "GM (access_level > 0) world entry must use SGWGmPlayer class id 0x03"
        );
    }

    #[test]
    fn class_id_for_access_level_only_flips_for_nonzero() {
        assert_eq!(class_id_for_access_level(0), SGWPLAYER_CLASS_ID);
        assert_eq!(class_id_for_access_level(1), SGWGMPLAYER_CLASS_ID);
        assert_eq!(class_id_for_access_level(2), SGWGMPLAYER_CLASS_ID);
        assert_eq!(class_id_for_access_level(4), SGWGMPLAYER_CLASS_ID);
    }

    #[tokio::test]
    async fn query_world_entry_no_db_with_cell_tx_round_trips_create_entity() {
        let mgr = Arc::new(std::sync::Mutex::new(EntityManager::new()));
        let (cell_tx, mut cell_rx) = mpsc::channel(4);

        let handle = tokio::spawn(async move {
            let cell_tx = Some(cell_tx);
            query_world_entry(&None, 1, None, 1, 0, &mgr, &cell_tx).await
        });

        // Drive the CreateEntity round-trip so the oneshot doesn't hang.
        let msg = timeout(Duration::from_secs(2), cell_rx.recv())
            .await
            .expect("CreateEntity receive must not hang")
            .expect("CreateEntity message expected");
        if let BaseToCellMsg::CreateEntity { reply_tx, .. } = msg {
            let _ = reply_tx.send(DEFAULT_SPACE_ID);
        } else {
            panic!("expected CreateEntity message");
        }

        let entry = timeout(Duration::from_secs(2), handle)
            .await
            .expect("query_world_entry task must not hang")
            .unwrap();
        assert_ne!(entry.player_entity_id, NO_ENTITY_ID);
        assert_eq!(entry.world_name, "CombatSim");
    }

    /// Sentinel ids for the historical-CellBlock login guards. Neighbours:
    /// 0x7000_1C00 below, 0x7000_2000 above.
    const HISTORICAL_LOGIN_BASE: i32 = 0x7000_1D00;

    async fn seed_player_in(pool: &PgPool, account_id: i32, player_id: i32, world: &str) {
        sqlx::query("DELETE FROM account WHERE account_id = $1")
            .bind(account_id)
            .execute(pool)
            .await
            .expect("pre-clean sentinel account");
        sqlx::query("INSERT INTO account (account_id, account_name, password) VALUES ($1, $2, '')")
            .bind(account_id)
            .bind(format!("historical-login-{account_id}"))
            .execute(pool)
            .await
            .expect("INSERT sentinel account");
        sqlx::query(
            "INSERT INTO sgw_player (\
                account_id, player_id, level, alignment, archetype, gender, \
                player_name, extra_name, world_location, bodyset, \
                pos_x, pos_y, pos_z, skin_color_id, naquadah\
             ) VALUES ($1, $2, 1, 0, 1, 1, $3, '', $4, 'BS_HumanMale.BS_HumanMale', \
                       -334.231, 73.472, -228.026, 0, 0)",
        )
        .bind(account_id)
        .bind(player_id)
        .bind(format!("historical-login-{player_id}"))
        .bind(world)
        .execute(pool)
        .await
        .expect("INSERT sentinel sgw_player");
    }

    /// Log in with the cell dropping the `CreateEntity` reply, the way it
    /// does when the create fails. Returns the entry and the manager.
    async fn login_with_failed_cell_create(
        pool: PgPool,
        account_id: i32,
        player_id: i32,
    ) -> (WorldEntryInfo, Arc<std::sync::Mutex<EntityManager>>) {
        let mgr = Arc::new(std::sync::Mutex::new(EntityManager::new()));
        let (cell_tx, mut cell_rx) = mpsc::channel(4);
        let task_mgr = Arc::clone(&mgr);
        let handle = tokio::spawn(async move {
            let db = Some(Arc::new(pool));
            let cell_tx = Some(cell_tx);
            query_world_entry(
                &db,
                account_id as u32,
                None,
                player_id,
                0,
                &task_mgr,
                &cell_tx,
            )
            .await
        });
        let msg = timeout(Duration::from_secs(5), cell_rx.recv())
            .await
            .expect("CreateEntity must not hang")
            .expect("CreateEntity expected");
        let BaseToCellMsg::CreateEntity { reply_tx, .. } = msg else {
            panic!("expected CreateEntity");
        };
        drop(reply_tx);
        let entry = timeout(Duration::from_secs(5), handle)
            .await
            .expect("query_world_entry must not hang")
            .unwrap();
        (entry, mgr)
    }

    /// A character saved in a historical CellBlock world whose cell create
    /// fails must not be admitted into the stock CellBlock space (the
    /// unknown-world fallback). Entry is refused and the id handed back.
    #[tokio::test]
    async fn live_db_historical_cellblock_login_fails_closed_when_the_cell_create_fails() {
        let pool = require_db_or_skip!();
        let account_id = HISTORICAL_LOGIN_BASE;
        let player_id = HISTORICAL_LOGIN_BASE + 1;
        seed_player_in(&pool, account_id, player_id, "CellBlock43").await;

        let (entry, mgr) = login_with_failed_cell_create(pool.clone(), account_id, player_id).await;

        sqlx::query("DELETE FROM account WHERE account_id = $1")
            .bind(account_id)
            .execute(&pool)
            .await
            .expect("cleanup sentinel account");
        assert_eq!(
            entry.player_entity_id, NO_ENTITY_ID,
            "entry must be refused, not placed in space {}",
            entry.space_id
        );
        assert_eq!(
            mgr.lock().unwrap().entity_count(),
            0,
            "the refused entry's entity id must be released"
        );
    }

    /// Control for the guard above: a stock world keeps its fallback space,
    /// so the refusal is scoped to the worlds that have no safe one.
    #[tokio::test]
    async fn live_db_stock_cellblock_login_keeps_its_fallback_space_when_the_cell_create_fails() {
        let pool = require_db_or_skip!();
        let account_id = HISTORICAL_LOGIN_BASE + 2;
        let player_id = HISTORICAL_LOGIN_BASE + 3;
        seed_player_in(&pool, account_id, player_id, "Castle_CellBlock").await;

        let (entry, _) = login_with_failed_cell_create(pool.clone(), account_id, player_id).await;

        sqlx::query("DELETE FROM account WHERE account_id = $1")
            .bind(account_id)
            .execute(&pool)
            .await
            .expect("cleanup sentinel account");
        assert_ne!(entry.player_entity_id, NO_ENTITY_ID);
        assert_eq!(entry.space_id, DEFAULT_SPACE_ID);
        assert_eq!(entry.world_name, "Castle_CellBlock");
    }

    /// Log in at `access_level`, answering the cell's `CreateEntity`.
    /// Returns the world the cell was asked for and the entry.
    async fn login_answered(
        pool: PgPool,
        account_id: i32,
        player_id: i32,
        access_level: u32,
    ) -> (String, WorldEntryInfo) {
        let mgr = Arc::new(std::sync::Mutex::new(EntityManager::new()));
        let (cell_tx, mut cell_rx) = mpsc::channel(4);
        let handle = tokio::spawn(async move {
            let db = Some(Arc::new(pool));
            let cell_tx = Some(cell_tx);
            query_world_entry(
                &db,
                account_id as u32,
                None,
                player_id,
                access_level,
                &mgr,
                &cell_tx,
            )
            .await
        });
        let msg = timeout(Duration::from_secs(5), cell_rx.recv())
            .await
            .expect("CreateEntity must not hang")
            .expect("CreateEntity expected");
        let BaseToCellMsg::CreateEntity {
            world_name,
            reply_tx,
            ..
        } = msg
        else {
            panic!("expected CreateEntity");
        };
        let _ = reply_tx.send(DEFAULT_SPACE_ID);
        let entry = timeout(Duration::from_secs(5), handle)
            .await
            .expect("query_world_entry must not hang")
            .unwrap();
        (world_name, entry)
    }

    /// D-DA4: a non-GM saved in the Debug Area (a relog after a GM summon, a
    /// demoted GM) logs in at the Praxis start, before the cell places them;
    /// a GM saved there logs in there. Revert proof: drop the
    /// `gm_only_redirect` block and the player's create names `DebugArea`.
    #[tokio::test]
    async fn live_db_a_non_gm_saved_in_the_debug_area_logs_in_at_the_faction_start() {
        let pool = require_db_or_skip!();
        for (offset, access_level, want) in [(4, 0, "Castle_CellBlock"), (6, 2, "DebugArea")] {
            let account_id = HISTORICAL_LOGIN_BASE + offset;
            let player_id = account_id + 1;
            seed_player_in(&pool, account_id, player_id, "DebugArea").await;
            let (created_in, entry) =
                login_answered(pool.clone(), account_id, player_id, access_level).await;
            sqlx::query("DELETE FROM account WHERE account_id = $1")
                .bind(account_id)
                .execute(&pool)
                .await
                .expect("cleanup sentinel account");
            assert_eq!(created_in, want, "access level {access_level}");
            assert_eq!(entry.world_name, want, "access level {access_level}");
            if access_level == 0 {
                assert_eq!(entry.pos, [-334.231, 73.472, -228.026]);
                assert!(
                    super::super::super::gm_only_worlds::take_redirect_line(player_id).is_some(),
                    "the arrival owes the player the reason"
                );
            }
        }
    }

    #[tokio::test]
    async fn query_world_stargates_no_db_returns_empty() {
        let result = query_world_stargates(&None, "CombatSim").await;
        assert!(
            result.is_empty(),
            "no-DB mode must return empty stargate list"
        );
    }
}
