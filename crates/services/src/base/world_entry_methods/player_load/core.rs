use std::sync::Arc;

use sqlx::PgPool;

use crate::mercury::PlayerLoadData;
use super::meta::{default_player_load_data, query_archetype_ability_tree, query_active_weapon_stats};

const INVENTORY_ITEM_SELECT: &str = r#"
SELECT inv.item_id, inv.type_id, inv.stack_size, inv.slot_id, inv.container_id,
       inv.bound, inv.durability, inv.charges,
       COALESCE((
           SELECT array_agg(array_position(enum_range(NULL::resources."EAmmoType"), ammo) - 1 ORDER BY ord)
           FROM unnest(ri.ammo_types) WITH ORDINALITY AS ammo_values(ammo, ord)
       ), ARRAY[]::integer[]) AS ammo_type_ids,
       CASE WHEN ri.default_ammo_type IS NULL THEN 0
            ELSE array_position(enum_range(NULL::resources."EAmmoType"), ri.default_ammo_type) - 1
       END AS cur_ammo_type_id
FROM sgw_inventory inv
LEFT JOIN resources.items ri ON ri.item_id = inv.type_id
WHERE inv.character_id = $1
ORDER BY inv.container_id, inv.slot_id
"#;

/// Query full player data from the database for the mapLoaded sequence.
///
/// Returns all fields needed by [`build_map_loaded`]: level, name, archetype,
/// appearance, abilities, inventory stubs, experience, etc.
pub async fn query_player_load_data(
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
            let items = query_inventory_items(pool.as_ref(), player_id).await;
            tracing::debug!(
                player_id,
                item_count = items.len(),
                "Loaded inventory items"
            );

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

            let ability_tree = query_archetype_ability_tree(pool.as_ref(), row.archetype)
                .await
                .unwrap_or_else(|| {
                    tracing::warn!(
                        player_id,
                        archetype = row.archetype,
                        "Using fallback ability tree for player load"
                    );
                    crate::mercury::archetype_ability_tree(row.archetype)
                });
            let (active_weapon_clip_size, active_ammo_type) =
                query_active_weapon_stats(pool.as_ref(), player_id, row.bandolier_slot).await;

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
                active_bandolier_slot: row.bandolier_slot,
                active_weapon_clip_size,
                active_ammo_type,
                ability_tree,
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
pub async fn query_player_load_data_by_account(
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
/// Note: `slot_id` is stored 0-indexed in DB but sent 1-indexed on the wire.
pub async fn query_inventory_items(
    pool: &PgPool,
    player_id: i32,
) -> Vec<cimmeria_entity::inventory::InvItem> {
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
        ammo_type_ids: Vec<i32>,
        cur_ammo_type_id: i32,
    }

    match sqlx::query_as::<_, InvRow>(INVENTORY_ITEM_SELECT)
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
                slot_id: r.slot_id + 1,
                container_id: r.container_id,
                is_bound: r.bound,
                durability: r.durability,
                ammo_types: r.ammo_type_ids,
                cur_ammo_type: r.cur_ammo_type_id,
                charges: r.charges,
            })
            .collect(),
        Err(e) => {
            tracing::error!(player_id, "Failed to query inventory items: {e}");
            vec![]
        }
    }
}