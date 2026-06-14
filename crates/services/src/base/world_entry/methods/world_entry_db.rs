use std::sync::Arc;

use sqlx::PgPool;
use tokio::sync::mpsc;

use cimmeria_entity::manager::EntityManager;

use crate::cell::messages::BaseToCellMsg;
use crate::mercury::{WorldEntryInfo, DEFAULT_SPACE_ID, SGWGMPLAYER_CLASS_ID, SGWPLAYER_CLASS_ID};

use super::super::space_registry::resolve_space_id_fallback;

/// Resolve the CREATE_BASE_PLAYER `class_id` byte from the caller's access
/// level. GMs (access_level > 0) come up as SGWGmPlayer (0x03) so the
/// client binds the GM method table and the native gm* cell surface
/// (flattened indices 109+) is reachable; everyone else stays SGWPlayer
/// (0x02). Inherited indices 0-108 don't shift either way (append-at-end
/// inheritance), so the existing player wire path is byte-identical for
/// access_level 0. See `spec`/`SGWGMPLAYER_CLASS_ID` for the derivation.
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
        world_location: String,
        pos_x: f32,
        pos_y: f32,
        pos_z: f32,
    }

    match sqlx::query_as::<_, EntryRow>(
        "SELECT world_location, pos_x, pos_y, pos_z \
         FROM sgw_player WHERE player_id = $1 AND account_id = $2",
    )
    .bind(player_id)
    .bind(account_id as i32)
    .fetch_optional(pool.as_ref())
    .await
    {
        Ok(Some(row)) => {
            let player_eid = alloc_entity();
            let pos = [row.pos_x, row.pos_y, row.pos_z];

            let space_id = if let Some(tx) = cell_tx {
                let (reply_tx, reply_rx) = tokio::sync::oneshot::channel();
                match tx
                    .send(BaseToCellMsg::CreateEntity {
                        entity_id: player_eid,
                        world_name: row.world_location.clone(),
                        position: pos,
                        rotation: [0.0; 3],
                        reply_tx,
                    })
                    .await
                {
                    Ok(_) => match reply_rx.await {
                        Ok(sid) => sid,
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
    use std::time::Duration;
    use tokio::time::timeout;

    #[tokio::test]
    async fn query_world_entry_no_db_allocates_entity_and_returns_default() {
        let mgr = Arc::new(std::sync::Mutex::new(EntityManager::new()));
        // access_level 0 → regular player → SGWPlayer class.
        let entry = query_world_entry(&None, 1, 1, 0, &mgr, &None).await;
        assert_ne!(
            entry.player_entity_id, NO_ENTITY_ID,
            "no-DB mode must allocate a real entity id"
        );
        assert_eq!(entry.world_name, "CombatSim");
        assert_eq!(entry.space_id, DEFAULT_SPACE_ID);
        assert_eq!(entry.pos, [0.0; 3]);
        assert_eq!(entry.class_id, SGWPLAYER_CLASS_ID);
    }

    /// #473 / CAT-N-04: a GM (access_level > 0) world entry must build the
    /// `WorldEntryInfo` with the SGWGmPlayer class id so CREATE_BASE_PLAYER
    /// emits 0x03 and the client binds the GM method table. A regular
    /// player (access_level 0) stays 0x02 — the byte-unchanged property
    /// asserted in the test above. This pins the class flip at the source
    /// where `WorldEntryInfo.class_id` is decided.
    #[tokio::test]
    async fn query_world_entry_gm_access_level_uses_gmplayer_class() {
        let mgr = Arc::new(std::sync::Mutex::new(EntityManager::new()));
        // access_level 2 = GameMaster (any non-zero level → GM class).
        let entry = query_world_entry(&None, 1, 1, 2, &mgr, &None).await;
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
            query_world_entry(&None, 1, 1, 0, &mgr, &cell_tx).await
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

    #[tokio::test]
    async fn query_world_stargates_no_db_returns_empty() {
        let result = query_world_stargates(&None, "CombatSim").await;
        assert!(
            result.is_empty(),
            "no-DB mode must return empty stargate list"
        );
    }
}
