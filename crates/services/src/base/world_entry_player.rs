//! Player data queries and persistence handlers for world entry.
//!
//! Extracted from `world_entry.rs` — DB queries for character world entry,
//! player load data, inventory, and persistence handlers for XP, missions,
//! items, and mail.

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use sqlx::PgPool;
use tokio::net::UdpSocket;
use tokio::sync::mpsc;

use cimmeria_entity::manager::EntityManager;

use crate::cell::messages::{BaseToCellMsg, MailOp};
use crate::mercury::{
    archetype_ability_tree, build_entity_method_packet,
    method_idx, PlayerLoadData, WorldEntryInfo,
    DEFAULT_SPACE_ID, SGWPLAYER_CLASS_ID,
};
use cimmeria_game::player::{MAX_LEVEL, TRAINING_POINTS_PER_LEVEL};

use super::ConnectedClientState;
use super::helpers::send_to_witness;
use super::world_entry::resolve_space_id_fallback;

// ── World entry query ───────────────────────────────────────────────────────

/// Query the character's world entry data from the database and allocate a player entity ID.
///
/// If a CellService channel is available, sends `CreateEntity` to resolve the space_id
/// dynamically. Otherwise falls back to the hardcoded space ID table.
pub(crate) async fn query_world_entry(
    db_pool: &Option<Arc<PgPool>>,
    account_id: u32,
    player_id: i32,
    entity_manager: &Arc<Mutex<EntityManager>>,
    cell_tx: &Option<mpsc::Sender<BaseToCellMsg>>,
) -> WorldEntryInfo {
    let player_eid = entity_manager.lock().unwrap().create_entity("SGWPlayer").0 as u32;

    let default_entry = || WorldEntryInfo {
        player_entity_id: player_eid,
        space_id: DEFAULT_SPACE_ID,
        pos: [0.0; 3],
        rot: [0.0; 3],
        world_name: "CombatSim".to_string(),
        class_id: SGWPLAYER_CLASS_ID,
    };

    let pool = match db_pool {
        Some(p) => p,
        None => return default_entry(),
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
            let pos = [row.pos_x, row.pos_y, row.pos_z];

            // Resolve space_id via CellService if available, else fall back
            // to the legacy hardcoded table.
            let space_id = if let Some(tx) = cell_tx {
                // Send CreateEntity with a oneshot so we can await the
                // resolved space_id before building the wire packet.
                let (reply_tx, reply_rx) = tokio::sync::oneshot::channel();
                if tx.send(BaseToCellMsg::CreateEntity {
                    entity_id: player_eid,
                    world_name: row.world_location.clone(),
                    position: pos,
                    rotation: [0.0; 3],
                    reply_tx,
                }).await.is_ok() {
                    match reply_rx.await {
                        Ok(sid) => sid,
                        Err(_) => {
                            tracing::warn!(world = %row.world_location, "CellService oneshot dropped -- using fallback");
                            resolve_space_id_fallback(&row.world_location)
                        }
                    }
                } else {
                    resolve_space_id_fallback(&row.world_location)
                }
            } else {
                resolve_space_id_fallback(&row.world_location)
            };

            WorldEntryInfo {
                player_entity_id: player_eid,
                space_id,
                pos,
                rot: [0.0; 3],
                world_name: row.world_location.clone(),
                class_id: SGWPLAYER_CLASS_ID,
            }
        }
        Ok(None) => {
            tracing::warn!(player_id, account_id, "Character not found for world entry");
            default_entry()
        }
        Err(e) => {
            tracing::error!("Failed to query world entry: {e}");
            default_entry()
        }
    }
}

// ── Player load data query ──────────────────────────────────────────────────

/// Query full player data from the database for the mapLoaded sequence.
///
/// Returns all fields needed by [`build_map_loaded`]: level, name, archetype,
/// appearance, abilities, inventory stubs, experience, etc.
pub(crate) async fn query_player_load_data(
    db_pool: &Option<Arc<PgPool>>,
    account_id: u32,
    player_id: i32,
) -> PlayerLoadData {
    let pool = match db_pool {
        Some(p) => p,
        None => return default_player_load_data(),
    };

    #[derive(sqlx::FromRow)]
    struct PlayerRow {
        level: i32,
        player_name: String,
        extra_name: String,
        alignment: i32,
        archetype: i32,
        gender: i32,
        bodyset: String,
        components: Vec<String>,
        exp: i32,
        naquadah: i32,
        known_stargates: Vec<i32>,
        abilities: Vec<i32>,
        training_points: i32,
        applied_science_points: i32,
        blueprint_ids: Vec<i32>,
        first_login: i32,
        access_level: i32,
        skin_color_id: i32,
        bandolier_slot: i32,
    }

    match sqlx::query_as::<_, PlayerRow>(
        "SELECT level, player_name, extra_name, alignment, archetype, gender, \
         bodyset, components, exp, naquadah, known_stargates, abilities, \
         training_points, applied_science_points, blueprint_ids, first_login, \
         access_level, skin_color_id, bandolier_slot \
         FROM sgw_player WHERE player_id = $1 AND account_id = $2",
    )
    .bind(player_id)
    .bind(account_id as i32)
    .fetch_optional(pool.as_ref())
    .await
    {
        Ok(Some(row)) => {
            tracing::info!(
                player_id, level = row.level, archetype = row.archetype,
                name = %row.player_name, bodyset = %row.bodyset,
                base_components = ?row.components,
                "Loaded player data for mapLoaded"
            );
            // Query inventory items for this character
            let items = query_inventory_items(pool.as_ref(), player_id).await;
            tracing::debug!(player_id, item_count = items.len(), "Loaded inventory items");

            // Merge equipped item visual components into body components
            // (matches requestCharacterVisuals / Inventory.py:462-465)
            let mut components = row.components;
            let item_visuals: Vec<String> = sqlx::query_scalar(
                "SELECT ri.visual_component \
                 FROM sgw_inventory inv \
                 JOIN resources.items ri ON ri.item_id = inv.type_id \
                 WHERE inv.character_id = $1 \
                   AND ri.visual_component IS NOT NULL \
                   AND ( \
                     (inv.container_id IN (3,4,5,6,7,8,9,10,11,12,13,14) AND inv.slot_id = 0) \
                     OR (inv.container_id = 3 AND inv.slot_id = $2) \
                   )",
            )
            .bind(player_id)
            .bind(row.bandolier_slot)
            .fetch_all(pool.as_ref())
            .await
            .unwrap_or_default();
            if !item_visuals.is_empty() {
                tracing::debug!(player_id, visuals = ?item_visuals, "Equipped item visual components");
            }
            components.extend(item_visuals);

            tracing::info!(
                player_id,
                bodyset = %row.bodyset,
                final_component_count = components.len(),
                final_components = ?components,
                "Player load data: final appearance after visual merge"
            );

            PlayerLoadData {
                player_id,
                level: row.level,
                player_name: row.player_name,
                extra_name: row.extra_name,
                alignment: row.alignment,
                archetype: row.archetype,
                gender: row.gender,
                bodyset: row.bodyset,
                components,
                exp: row.exp,
                naquadah: row.naquadah,
                known_stargates: row.known_stargates,
                abilities: row.abilities,
                training_points: row.training_points,
                applied_science_points: row.applied_science_points,
                blueprint_ids: row.blueprint_ids,
                first_login: row.first_login,
                access_level: row.access_level,
                skin_color_id: row.skin_color_id,
                ability_tree: archetype_ability_tree(row.archetype),
                items,
            }
        }
        Ok(None) => {
            tracing::warn!(player_id, account_id, "Player not found for mapLoaded");
            default_player_load_data()
        }
        Err(e) => {
            tracing::error!(player_id, "Failed to query player load data: {e}");
            default_player_load_data()
        }
    }
}

/// Query player load data using just the account_id (for gate travel where we
/// don't have the player_id readily available in ConnectedClientState).
pub(crate) async fn query_player_load_data_by_account(
    db_pool: &Option<Arc<PgPool>>,
    account_id: u32,
) -> PlayerLoadData {
    let pool = match db_pool {
        Some(p) => p,
        None => return default_player_load_data(),
    };

    #[derive(sqlx::FromRow)]
    struct PlayerRow {
        player_id: i32,
    }

    // Find the most recently used character for this account
    match sqlx::query_as::<_, PlayerRow>(
        "SELECT player_id FROM sgw_player WHERE account_id = $1 ORDER BY player_id LIMIT 1",
    )
    .bind(account_id as i32)
    .fetch_optional(pool.as_ref())
    .await
    {
        Ok(Some(row)) => query_player_load_data(db_pool, account_id, row.player_id).await,
        _ => default_player_load_data(),
    }
}

/// Query inventory items from `sgw_inventory` for a character.
///
/// Returns `InvItem` structs ready for wire serialization via `onUpdateItem`.
/// Note: `slot_id` is stored 0-indexed in DB but sent 1-indexed on the wire
/// (Python: `'slotID': self.slotId + 1`).
async fn query_inventory_items(pool: &PgPool, player_id: i32) -> Vec<cimmeria_entity::inventory::InvItem> {
    #[derive(sqlx::FromRow)]
    struct InvRow {
        item_id: i32,
        type_id: i32,
        stack_size: i32,
        slot_id: i32,
        container_id: i32,
        bound: bool,
        durability: i32,
        charges: i32,
    }

    match sqlx::query_as::<_, InvRow>(
        "SELECT item_id, type_id, stack_size, slot_id, container_id, bound, durability, charges \
         FROM sgw_inventory WHERE character_id = $1 ORDER BY container_id, slot_id",
    )
    .bind(player_id)
    .fetch_all(pool)
    .await
    {
        Ok(rows) => rows
            .into_iter()
            .map(|r| cimmeria_entity::inventory::InvItem {
                id: r.item_id,
                dbid: r.type_id,
                stack_size: r.stack_size,
                slot_id: r.slot_id + 1, // DB is 0-indexed, wire is 1-indexed
                container_id: r.container_id,
                is_bound: r.bound,
                durability: r.durability,
                ammo_types: vec![], // TODO: parse EAmmoType[] from DB
                cur_ammo_type: 0,   // TODO: parse EAmmoType enum from DB
                charges: r.charges,
            })
            .collect(),
        Err(e) => {
            tracing::error!(player_id, "Failed to query inventory items: {e}");
            vec![]
        }
    }
}

/// Default player load data when the DB is unavailable.
pub(crate) fn default_player_load_data() -> PlayerLoadData {
    PlayerLoadData {
        player_id: 0,
        level: 1,
        player_name: "Unknown".into(),
        extra_name: String::new(),
        alignment: 1,
        archetype: 1,
        gender: 1,
        bodyset: "BS_HumanMale.BS_HumanMale".into(),
        components: vec![],
        exp: 0,
        naquadah: 0,
        known_stargates: vec![],
        abilities: vec![],
        training_points: 0,
        applied_science_points: 0,
        blueprint_ids: vec![],
        first_login: 1,
        access_level: 0,
        skin_color_id: 0,
        ability_tree: archetype_ability_tree(1),
        items: vec![],
    }
}

// ── XP grant handler ────────────────────────────────────────────────────────

/// XP thresholds per level, matching `crates/game/src/player.rs` LEVEL_XP.
const LEVEL_XP: [u64; 21] = [
    0,
    100, 200, 300, 600, 1_000, 1_600, 2_500, 4_000, 6_000, 9_000,
    14_000, 18_000, 25_000, 40_000, 60_000, 90_000, 120_000, 180_000, 250_000, 400_000,
];

/// GENERICPROPERTY_TrainingPoints enum value from `entities/defs/enumerations.xml`.
const GENERICPROPERTY_TRAINING_POINTS: i32 = 1;

/// Handle XP grant from CellService -- compute level-ups and send client notifications.
///
/// Matches the Python `giveExperience()` flow from `python/cell/SGWPlayer.py:787`:
/// 1. Add XP
/// 2. Send `onExpUpdate(total_xp)`
/// 3. For each level-up: send `giveXPForLevel(level)` + `onMaxExpUpdate(threshold)`
/// 4. Send `onLevelUpdate(level)` (for HUD)
/// 5. Send `onEntityProperty(GENERICPROPERTY_TrainingPoints, tp)` (for TP UI)
pub(crate) async fn handle_grant_xp(
    entity_id: u32,
    xp_amount: u64,
    socket: &Arc<UdpSocket>,
    connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    entity_to_addr: &Arc<Mutex<HashMap<u32, SocketAddr>>>,
) {
    // Look up the player's session state via entity_id -> addr -> ConnectedClientState
    let addr = {
        let map = entity_to_addr.lock().unwrap();
        match map.get(&entity_id) {
            Some(a) => *a,
            None => {
                tracing::warn!(entity_id, "GrantXP: no address for entity");
                return;
            }
        }
    };

    // Read current XP/level from session, compute level-ups, write back
    let (total_xp, new_level, training_points, levels_gained) = {
        let mut map = connected.lock().unwrap();
        let state = match map.get_mut(&addr) {
            Some(s) => s,
            None => {
                tracing::warn!(entity_id, "GrantXP: no connected state for entity");
                return;
            }
        };

        let mut xp = state.player_xp.unwrap_or(0);
        let mut level = state.player_level.unwrap_or(1) as u32;
        let mut tp = state.player_training_points.unwrap_or(0);

        xp += xp_amount;

        let mut gained = Vec::new();
        while level < MAX_LEVEL && xp > LEVEL_XP[level as usize] {
            level += 1;
            tp += TRAINING_POINTS_PER_LEVEL;
            gained.push(level);
        }

        // Write back
        state.player_xp = Some(xp);
        state.player_level = Some(level as i32);
        state.player_training_points = Some(tp);

        (xp, level, tp, gained)
    };

    tracing::info!(
        entity_id, xp_amount, total_xp, new_level,
        levels_up = levels_gained.len(),
        "GrantXP processed"
    );

    // 1. onExpUpdate(INT32 total_xp) -- XP bar
    send_to_witness(
        socket, connected, entity_to_addr, entity_id,
        |key, seq, acks| {
            build_entity_method_packet(
                key, seq, acks, entity_id,
                method_idx::ON_EXP_UPDATE,
                &(total_xp as i32).to_le_bytes(),
            )
        },
    ).await;

    // 2. Per level-up: giveXPForLevel(INT32 level) + onMaxExpUpdate(INT32 threshold)
    for &lvl in &levels_gained {
        // giveXPForLevel -- triggers level-up VFX/sound on client
        send_to_witness(
            socket, connected, entity_to_addr, entity_id,
            |key, seq, acks| {
                build_entity_method_packet(
                    key, seq, acks, entity_id,
                    method_idx::GIVE_XP_FOR_LEVEL,
                    &(lvl as i32).to_le_bytes(),
                )
            },
        ).await;

        // onMaxExpUpdate -- update XP bar cap
        let next_threshold = if lvl >= MAX_LEVEL {
            LEVEL_XP[MAX_LEVEL as usize] as i32
        } else {
            LEVEL_XP[lvl as usize] as i32
        };
        send_to_witness(
            socket, connected, entity_to_addr, entity_id,
            |key, seq, acks| {
                build_entity_method_packet(
                    key, seq, acks, entity_id,
                    method_idx::ON_MAX_EXP_UPDATE,
                    &next_threshold.to_le_bytes(),
                )
            },
        ).await;
    }

    // 3. onLevelUpdate(INT32 level) -- update level display in HUD
    if !levels_gained.is_empty() {
        send_to_witness(
            socket, connected, entity_to_addr, entity_id,
            |key, seq, acks| {
                build_entity_method_packet(
                    key, seq, acks, entity_id,
                    method_idx::ON_LEVEL_UPDATE,
                    &(new_level as i32).to_le_bytes(),
                )
            },
        ).await;

        // 4. onEntityProperty(GENERICPROPERTY_TrainingPoints, tp) -- TP UI
        let mut tp_args = Vec::with_capacity(8);
        tp_args.extend_from_slice(&GENERICPROPERTY_TRAINING_POINTS.to_le_bytes());
        tp_args.extend_from_slice(&(training_points as i32).to_le_bytes());
        send_to_witness(
            socket, connected, entity_to_addr, entity_id,
            |key, seq, acks| {
                build_entity_method_packet(
                    key, seq, acks, entity_id,
                    method_idx::ON_ENTITY_PROPERTY,
                    &tp_args,
                )
            },
        ).await;
    }
}

// ── Mission loading ──────────────────────────────────────────────────────────

/// Query saved missions from the database for a player re-login.
///
/// Returns missions with status = active (1) so the CellService can restore
/// them before the content engine fires. Completed (2) missions are also loaded
/// so the content engine sees them and doesn't re-trigger.
pub(crate) async fn query_saved_missions(
    db_pool: &Option<Arc<PgPool>>,
    player_id: i32,
) -> Vec<crate::cell::messages::SavedMission> {
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
    }

    match sqlx::query_as::<_, MissionRow>(
        "SELECT mission_id, status, current_step_id, \
         completed_step_ids, completed_objective_ids, active_objective_ids, failed_objective_ids \
         FROM sgw_mission WHERE player_id = $1",
    )
    .bind(player_id)
    .fetch_all(pool.as_ref())
    .await
    {
        Ok(rows) => {
            let missions: Vec<_> = rows.into_iter().map(|r| {
                crate::cell::messages::SavedMission {
                    mission_id: r.mission_id,
                    status: r.status as i8,
                    current_step_id: r.current_step_id,
                    completed_step_ids: r.completed_step_ids,
                    completed_objective_ids: r.completed_objective_ids,
                    active_objective_ids: r.active_objective_ids,
                    failed_objective_ids: r.failed_objective_ids,
                }
            }).collect();
            tracing::info!(player_id, count = missions.len(), "Loaded saved missions from DB");
            missions
        }
        Err(e) => {
            tracing::error!(player_id, "Failed to query saved missions: {e}");
            vec![]
        }
    }
}

// ── Mission persistence ─────────────────────────────────────────────────────

/// Persist a mission state change to the database.
pub(crate) async fn handle_mission_update(
    player_id: i32,
    mission_id: i32,
    status: i8,
    current_step_id: Option<i32>,
    completed_step_ids: &[i32],
    completed_objective_ids: &[i32],
    active_objective_ids: &[i32],
    failed_objective_ids: &[i32],
    db_pool: &Option<Arc<PgPool>>,
) {
    let pool = match db_pool {
        Some(p) => p,
        None => {
            tracing::debug!(player_id, mission_id, "MissionUpdate: no DB pool");
            return;
        }
    };

    let result = sqlx::query(
        "INSERT INTO sgw_mission (player_id, mission_id, status, current_step_id, \
         completed_step_ids, completed_objective_ids, active_objective_ids, failed_objective_ids) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8) \
         ON CONFLICT (player_id, mission_id) DO UPDATE SET \
         status = EXCLUDED.status, \
         current_step_id = EXCLUDED.current_step_id, \
         completed_step_ids = EXCLUDED.completed_step_ids, \
         completed_objective_ids = EXCLUDED.completed_objective_ids, \
         active_objective_ids = EXCLUDED.active_objective_ids, \
         failed_objective_ids = EXCLUDED.failed_objective_ids",
    )
    .bind(player_id)
    .bind(mission_id)
    .bind(status as i32)
    .bind(current_step_id)
    .bind(completed_step_ids)
    .bind(completed_objective_ids)
    .bind(active_objective_ids)
    .bind(failed_objective_ids)
    .execute(pool.as_ref())
    .await;

    match result {
        Ok(_) => tracing::debug!(player_id, mission_id, status, "Mission state persisted"),
        Err(e) => tracing::error!(player_id, mission_id, "Failed to persist mission: {e}"),
    }
}

// ── Item persistence ────────────────────────────────────────────────────────

/// Persist a granted item to the inventory database and send visual updates if equipped.
pub(crate) async fn handle_grant_item(
    entity_id: u32,
    player_id: i32,
    item_id: i32,
    container_id: i32,
    count: i32,
    db_pool: &Option<Arc<PgPool>>,
    socket: &Arc<tokio::net::UdpSocket>,
    connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    entity_to_addr: &Arc<Mutex<HashMap<u32, SocketAddr>>>,
) {
    let pool = match db_pool {
        Some(p) => p,
        None => {
            tracing::debug!(player_id, item_id, "GrantItem: no DB pool");
            return;
        }
    };

    // Find the next available slot in this container
    let next_slot: i32 = sqlx::query_scalar(
        "SELECT COALESCE(MAX(slot_id), -1) + 1 FROM sgw_inventory \
         WHERE character_id = $1 AND container_id = $2",
    )
    .bind(player_id)
    .bind(container_id)
    .fetch_one(pool.as_ref())
    .await
    .unwrap_or(0);

    let result = sqlx::query(
        "INSERT INTO sgw_inventory (character_id, type_id, stack_size, slot_id, container_id, \
         bound, durability, charges) VALUES ($1, $2, $3, $4, $5, false, 100, 0)",
    )
    .bind(player_id)
    .bind(item_id)
    .bind(count)
    .bind(next_slot)
    .bind(container_id)
    .execute(pool.as_ref())
    .await;

    match result {
        Ok(_) => tracing::debug!(player_id, item_id, container_id, slot = next_slot, "Item persisted to inventory"),
        Err(e) => {
            tracing::error!(player_id, item_id, "Failed to persist item: {e}");
            return;
        }
    }

    // Re-query ALL inventory items and send the full list via onUpdateItem.
    // The client replaces its item list with the array contents, so we must
    // send everything — not just the newly added item.
    // Reference: python/cell/Inventory.py flushUpdates() step 3
    {
        let all_items: Vec<(i32, i32, i32, i32, bool, i32, i32)> = sqlx::query_as(
            "SELECT type_id, stack_size, slot_id, container_id, bound, durability, charges \
             FROM sgw_inventory WHERE character_id = $1 ORDER BY container_id, slot_id",
        )
        .bind(player_id)
        .fetch_all(pool.as_ref())
        .await
        .unwrap_or_default();

        let mut args = Vec::with_capacity(4 + all_items.len() * 48);
        args.extend_from_slice(&(all_items.len() as u32).to_le_bytes());
        for (i, (type_id, stack_size, slot_id, cid, bound, durability, charges)) in all_items.iter().enumerate() {
            let item = cimmeria_entity::inventory::InvItem {
                id: *cid * 100 + *slot_id + 1, // stable instance ID (1-indexed)
                dbid: *type_id,
                stack_size: *stack_size,
                slot_id: *slot_id + 1, // DB is 0-indexed, wire is 1-indexed
                container_id: *cid,
                is_bound: *bound,
                durability: *durability,
                ammo_types: vec![],
                cur_ammo_type: 0,
                charges: *charges,
            };
            item.serialize(&mut args);
        }

        send_to_witness(
            socket, connected, entity_to_addr, entity_id,
            |key, seq, acks| {
                build_entity_method_packet(
                    key, seq, acks, entity_id,
                    method_idx::ON_UPDATE_ITEM, &args,
                )
            },
        ).await;
        tracing::debug!(entity_id, player_id, item_id, total_items = all_items.len(), "Sent full onUpdateItem to client");
    }

    // If this is an equipped container (3=bandolier, 4-14=equipment), send visual updates
    let is_equipped = (3..=14).contains(&container_id);
    if !is_equipped {
        return;
    }

    // For bandolier (container_id=3), send onActiveSlotUpdate(bagId, slotId)
    if container_id == 3 {
        let mut args = Vec::with_capacity(8);
        args.extend_from_slice(&container_id.to_le_bytes()); // bagId
        args.extend_from_slice(&(next_slot + 1).to_le_bytes()); // slotId (1-indexed on wire)
        send_to_witness(
            socket, connected, entity_to_addr, entity_id,
            |key, seq, acks| {
                build_entity_method_packet(
                    key, seq, acks, entity_id,
                    method_idx::ON_ACTIVE_SLOT_UPDATE, &args,
                )
            },
        ).await;
    }

    // Look up the item's visual_component from the resources.items table
    let visual: Option<String> = sqlx::query_scalar(
        "SELECT visual_component FROM resources.items WHERE item_id = $1 AND visual_component IS NOT NULL",
    )
    .bind(item_id)
    .fetch_optional(pool.as_ref())
    .await
    .unwrap_or(None);

    if let Some(ref visual_component) = visual {
        tracing::info!(
            entity_id, player_id, item_id, container_id, %visual_component,
            "Equipped item has visual — resending BeingAppearance"
        );

        // Re-query the full appearance (bodyset + all equipped visuals + base components)
        // and resend BeingAppearance so the client updates the model immediately.
        let account_id = {
            let addr = match entity_to_addr.lock().unwrap().get(&entity_id).copied() {
                Some(a) => a,
                None => return,
            };
            let clients = connected.lock().unwrap();
            match clients.get(&addr) {
                Some(c) => c.account_id,
                None => return,
            }
        };

        let player_data = query_player_load_data(db_pool, account_id, player_id).await;
        let appearance_args = super::world_entry_appearance::build_appearance_args(
            &player_data.bodyset, &player_data.components,
        );

        // Update cached appearance for future resends (cancelMovie, etc.)
        {
            let addr = match entity_to_addr.lock().unwrap().get(&entity_id).copied() {
                Some(a) => a,
                None => return,
            };
            let mut clients = connected.lock().unwrap();
            if let Some(c) = clients.get_mut(&addr) {
                c.cached_appearance_args = Some(appearance_args.clone());
            }
        }

        send_to_witness(
            socket, connected, entity_to_addr, entity_id,
            |key, seq, acks| {
                build_entity_method_packet(
                    key, seq, acks, entity_id,
                    method_idx::BEING_APPEARANCE, &appearance_args,
                )
            },
        ).await;
    }
}

// ── Mail handling ───────────────────────────────────────────────────────────

/// Handle a mail request from CellService by querying the DB and sending results to the client.
pub(crate) async fn handle_mail_request(
    entity_id: u32,
    op: MailOp,
    socket: &Arc<UdpSocket>,
    connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    entity_to_addr: &Arc<Mutex<HashMap<u32, SocketAddr>>>,
    db_pool: &Option<Arc<PgPool>>,
) {
    use crate::cell::mail;

    let pool = match db_pool {
        Some(p) => p,
        None => {
            tracing::debug!(entity_id, "Mail request: no DB pool available");
            return;
        }
    };

    // Get the player's character_id (player_id) from their account via entity lookup
    let account_id = {
        let addr = match entity_to_addr.lock().unwrap().get(&entity_id).copied() {
            Some(a) => a,
            None => { tracing::warn!(entity_id, "Mail: no client addr"); return; }
        };
        let clients = connected.lock().unwrap();
        match clients.get(&addr) {
            Some(c) => c.account_id,
            None => { tracing::warn!(entity_id, "Mail: client not found"); return; }
        }
    };

    // Resolve player_id from account_id
    let player_id = match sqlx::query_scalar::<_, i32>(
        "SELECT player_id FROM sgw_player WHERE account_id = $1 ORDER BY player_id LIMIT 1",
    )
    .bind(account_id as i32)
    .fetch_optional(pool.as_ref())
    .await
    {
        Ok(Some(pid)) => pid,
        _ => {
            tracing::warn!(entity_id, account_id, "Mail: could not resolve player_id");
            return;
        }
    };

    // Get player name for mail read responses
    let player_name = {
        let addr = match entity_to_addr.lock().unwrap().get(&entity_id).copied() {
            Some(a) => a,
            None => { tracing::warn!(entity_id, "Mail: no addr for player name lookup"); return; }
        };
        let clients = connected.lock().unwrap();
        clients.get(&addr)
            .and_then(|c| c.player_name.clone())
            .unwrap_or_default()
    };

    match op {
        MailOp::RequestHeaders { b_archive } => {
            tracing::debug!(entity_id, player_id, b_archive, "Mail: querying headers");

            #[derive(sqlx::FromRow)]
            struct MailRow {
                mail_id: i32,
                sender_name: String,
                sender_id: Option<i32>,
                subject: String,
                cash: i64,
                sent_time: i32,
                read_time: i32,
                flags: i32,
            }

            let rows = sqlx::query_as::<_, MailRow>(
                "SELECT mail_id, sender_name, sender_id, subject, cash, sent_time, read_time, flags \
                 FROM sgw_gate_mail WHERE character_id = $1 ORDER BY mail_id DESC",
            )
            .bind(player_id)
            .fetch_all(pool.as_ref())
            .await
            .unwrap_or_default();

            let headers: Vec<mail::MailHeader> = rows.iter().map(|r| mail::MailHeader {
                id: r.mail_id,
                from_text: r.sender_name.clone(),
                from_id: r.sender_id.unwrap_or(0),
                subject_text: r.subject.clone(),
                cash: r.cash as i32,
                sent_time: r.sent_time as f32,
                read_time: r.read_time as f32,
                flags: r.flags,
            }).collect();

            tracing::debug!(entity_id, count = headers.len(), "Mail: sending headers to client");

            let args = mail::serialize_on_mail_header_info(b_archive, &headers);
            send_to_witness(
                socket, connected, entity_to_addr, entity_id,
                |key, seq, acks| {
                    build_entity_method_packet(key, seq, acks, entity_id,
                        crate::mercury::method_idx::ON_MAIL_HEADER_INFO, &args)
                },
            ).await;
        }

        MailOp::RequestBody { mail_id } => {
            tracing::debug!(entity_id, mail_id, "Mail: querying body");

            #[derive(sqlx::FromRow)]
            struct BodyRow {
                message: String,
            }

            match sqlx::query_as::<_, BodyRow>(
                "SELECT message FROM sgw_gate_mail WHERE mail_id = $1 AND character_id = $2",
            )
            .bind(mail_id)
            .bind(player_id)
            .fetch_optional(pool.as_ref())
            .await
            {
                Ok(Some(row)) => {
                    // Mark as read (set read_time if not already set)
                    let now = std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_secs() as i32;
                    let _ = sqlx::query(
                        "UPDATE sgw_gate_mail SET read_time = $1 WHERE mail_id = $2 AND read_time = 0",
                    )
                    .bind(now)
                    .bind(mail_id)
                    .execute(pool.as_ref())
                    .await;

                    let args = mail::serialize_on_mail_read(mail_id, &row.message, &player_name);
                    send_to_witness(
                        socket, connected, entity_to_addr, entity_id,
                        |key, seq, acks| {
                            build_entity_method_packet(key, seq, acks, entity_id,
                                crate::mercury::method_idx::ON_MAIL_READ, &args)
                        },
                    ).await;
                }
                _ => {
                    tracing::warn!(entity_id, mail_id, "Mail body not found");
                }
            }
        }

        MailOp::Delete { mail_id } => {
            tracing::debug!(entity_id, mail_id, "Mail: deleting");
            let _ = sqlx::query(
                "DELETE FROM sgw_gate_mail WHERE mail_id = $1 AND character_id = $2",
            )
            .bind(mail_id)
            .bind(player_id)
            .execute(pool.as_ref())
            .await;

            // Notify client to remove the header
            let args = mail::serialize_on_mail_header_remove(mail_id);
            send_to_witness(
                socket, connected, entity_to_addr, entity_id,
                |key, seq, acks| {
                    build_entity_method_packet(key, seq, acks, entity_id,
                        crate::mercury::method_idx::ON_MAIL_HEADER_REMOVE, &args)
                },
            ).await;
        }

        MailOp::Archive { mail_id } => {
            tracing::debug!(entity_id, mail_id, "Mail: archiving");
            // Set the MAIL_Archive flag (bit 0)
            let _ = sqlx::query(
                "UPDATE sgw_gate_mail SET flags = flags | 1 WHERE mail_id = $1 AND character_id = $2",
            )
            .bind(mail_id)
            .bind(player_id)
            .execute(pool.as_ref())
            .await;

            // Notify client to remove the header from inbox
            let args = mail::serialize_on_mail_header_remove(mail_id);
            send_to_witness(
                socket, connected, entity_to_addr, entity_id,
                |key, seq, acks| {
                    build_entity_method_packet(key, seq, acks, entity_id,
                        crate::mercury::method_idx::ON_MAIL_HEADER_REMOVE, &args)
                },
            ).await;
        }
    }
}
