//! The `query_player_load_data` mapLoaded loader.
//!
//! Extracted from `player_load/core.rs` (issue #529). Loads the full
//! `PlayerLoadData` for the mapLoaded sequence: base row, inventory stubs,
//! equipment/weapon visual merge, ability tree, and bandolier items. Pure
//! code movement — the function body is byte-identical to the original.

use std::sync::Arc;

use cimmeria_entity::known_names;
use sqlx::PgPool;

use super::super::meta::{default_player_load_data, player_ability_tree, query_bandolier_items};
use super::inventory_items::query_inventory_items;
use super::{CONTAINER_BANDOLIER, EQUIPMENT_CONTAINERS};
use crate::mercury::PlayerLoadData;

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
        None => {
            // Offline mode: expected, so debug rather than error.
            tracing::debug!(
                event = "player_load_degraded",
                reason = "no_db_pool",
                player_id,
                player_name = known_names::player_name(player_id),
                account_id,
                account_name = known_names::account_name(account_id),
                "Player load skipped: no database pool; loading default data"
            );
            return default_player_load_data();
        }
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
        // Server-synced client options — see SystemOptions docs.
        auto_reload: bool,
        reload_on_activate: bool,
        /// `smallint` in the schema.
        bank_slots: i16,
    }

    match sqlx::query_as::<_, PlayerRow>(
        "SELECT level, player_name, extra_name, alignment, archetype, gender, \
         bodyset, components, exp, naquadah, known_stargates, abilities, \
         training_points, applied_science_points, blueprint_ids, first_login, \
         access_level, skin_color_id, bandolier_slot, \
         auto_reload, reload_on_activate, bank_slots \
         FROM sgw_player WHERE player_id = $1 AND account_id = $2",
    )
    .bind(player_id)
    .bind(account_id as i32)
    .fetch_optional(pool.as_ref())
    .await
    {
        Ok(Some(row)) => {
            tracing::info!(
                player_id, player_name = %row.player_name, level = row.level,
                archetype = row.archetype,
                archetype_name = cimmeria_names::archetype_name(row.archetype),
                bodyset = %row.bodyset,
                base_components = ?row.components,
                "Loaded player data for mapLoaded"
            );
            let items = query_inventory_items(pool.as_ref(), player_id).await;
            tracing::debug!(
                player_id,
                player_name = %row.player_name,
                item_count = items.len(),
                "Loaded inventory items"
            );

            let mut components = row.components;

            // Query equipment visuals (head/torso/armor/etc — container ≠
            // bandolier). These always go on the wire regardless of
            // holster state.
            let equipment_visuals: Vec<String> = match sqlx::query_scalar(
                "SELECT ri.visual_component \
                 FROM sgw_inventory inv \
                 JOIN resources.items ri ON ri.item_id = inv.type_id \
                 WHERE inv.container_id = ANY($1) \
                   AND inv.character_id = $2 \
                   AND ri.visual_component IS NOT NULL \
                   AND inv.slot_id = 0",
            )
            .bind(EQUIPMENT_CONTAINERS)
            .bind(player_id)
            .fetch_all(pool.as_ref())
            .await
            {
                Ok(v) => v,
                Err(e) => {
                    // The base body components from `row.components` are
                    // still applied — only the equipment-slot visuals
                    // (head/torso/armor/etc.) are skipped on this fallback.
                    // The active bandolier weapon visual is queried
                    // separately below and is not affected here.
                    tracing::error!(
                        event = "player_load_degraded",
                        reason = "equipment_visuals_query_failed",
                        player_id,
                        player_name = %row.player_name,
                        account_id,
                        account_name = known_names::account_name(account_id),
                        error = %e,
                        "Equipment visuals query failed when loading player data; equipment-slot \
                         visuals (helmet/armor/etc.) are skipped, base body components still apply"
                    );
                    Vec::new()
                }
            };

            // Query the active bandolier slot's weapon visual separately
            // — it's filtered out of `BeingAppearance.ComponentList` when
            // the player is holstered. The client's appearance compositor
            // keys the holster-vs-armed animation pose off whether a
            // weapon-shaped entry is present in the list, so omitting
            // this string is what renders the weapon-down stance. See
            // `CellEntity::appearance_components` and the Ghidra evidence
            // at `ghidra://SGW.exe@0x00ec0840`.
            let weapon_visual: Option<String> = match sqlx::query_scalar(
                "SELECT ri.visual_component \
                 FROM sgw_inventory inv \
                 JOIN resources.items ri ON ri.item_id = inv.type_id \
                 WHERE inv.container_id = $1 \
                   AND inv.character_id = $2 \
                   AND inv.slot_id = $3 \
                   AND ri.visual_component IS NOT NULL \
                 LIMIT 1",
            )
            .bind(CONTAINER_BANDOLIER)
            .bind(player_id)
            .bind(row.bandolier_slot)
            .fetch_optional(pool.as_ref())
            .await
            {
                Ok(v) => v,
                Err(e) => {
                    tracing::error!(
                        event = "player_load_degraded",
                        reason = "weapon_visual_query_failed",
                        player_id,
                        player_name = %row.player_name,
                        account_id,
                        account_name = known_names::account_name(account_id),
                        error = %e,
                        "Active bandolier weapon visual query failed; the player loads with no weapon visual"
                    );
                    None
                }
            };

            if !equipment_visuals.is_empty() {
                tracing::debug!(player_id, player_name = %row.player_name, visuals = ?equipment_visuals, "Equipment visual components");
            }
            if let Some(ref w) = weapon_visual {
                tracing::debug!(player_id, player_name = %row.player_name, weapon_visual = %w, active_slot = row.bandolier_slot, "Active bandolier weapon visual");
            }

            components.extend(equipment_visuals);
            if let Some(ref w) = weapon_visual {
                components.push(w.clone());
            }

            tracing::info!(
                player_id,
                player_name = %row.player_name,
                bodyset = %row.bodyset,
                final_component_count = components.len(),
                final_components = ?components,
                weapon_visual = ?weapon_visual,
                "Player load data: final appearance after visual merge"
            );

            let ability_tree = player_ability_tree(pool.as_ref(), row.archetype, player_id).await;
            // Stage C: bandolier_items now carries clip_size + cur_ammo_type
            // for every populated slot, so the old `query_active_weapon_stats`
            // (which only fetched the active slot's clip + default ammo) is
            // redundant. `map_loaded.rs` reads the active item directly.
            let bandolier_items = query_bandolier_items(db_pool, player_id).await;

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
                weapon_visual,
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
                bandolier_items,
                ability_tree,
                items,
                auto_reload: row.auto_reload,
                reload_on_activate: row.reload_on_activate,
                bank_slots: i32::from(row.bank_slots),
            }
        }
        Ok(None) => {
            tracing::error!(
                event = "player_load_degraded",
                reason = "player_row_missing",
                player_id,
                player_name = known_names::player_name(player_id),
                account_id,
                account_name = known_names::account_name(account_id),
                "Player row not found when loading player data; loading default data"
            );
            default_player_load_data()
        }
        Err(e) => {
            tracing::error!(
                event = "player_load_degraded",
                reason = "player_query_failed",
                player_id,
                player_name = known_names::player_name(player_id),
                account_id,
                account_name = known_names::account_name(account_id),
                error = %e,
                "Player load query failed when loading player data; loading default data"
            );
            default_player_load_data()
        }
    }
}

#[cfg(test)]
mod degraded_tests {
    //! Negative-log guard (TESTING.md type 12) for the player-load fallbacks
    //! that run without a live row: with the database unreachable, each one
    //! logs `event = player_load_degraded` at ERROR with its `reason`.
    //! `player_row_missing` needs a live DB and is covered there by
    //! `live_db_query_player_load_data_account_mismatch_returns_default`
    //! (core/mod.rs). The equipment and weapon visual reasons need fault
    //! injection and are untested.

    use super::*;
    use crate::base::world_entry::methods::missions::query_saved_missions;
    use crate::test_support::LogCapture;
    use std::time::Duration;
    use tracing::Level;

    /// A pool whose every acquire fails fast: nothing listens on port 1.
    fn unreachable_pool() -> Arc<PgPool> {
        Arc::new(
            sqlx::postgres::PgPoolOptions::new()
                .max_connections(1)
                .acquire_timeout(Duration::from_millis(50))
                .connect_lazy("postgres://nobody:nobody@127.0.0.1:1/none")
                .expect("connect_lazy must succeed for any well-formed URL"),
        )
    }

    #[tokio::test]
    async fn player_load_fallbacks_log_degraded_reason() {
        const PLAYER_ID: i32 = 0x7000_B1C8;
        let pool = unreachable_pool();
        let capture = LogCapture::install();

        let data = query_player_load_data(&Some(pool.clone()), 0x7000_B1C6, PLAYER_ID).await;
        assert_eq!(
            data.player_id, 0,
            "a failed player query returns the default sentinel"
        );
        let items = query_inventory_items(pool.as_ref(), PLAYER_ID).await;
        assert!(
            items.is_empty(),
            "a failed inventory query returns no items"
        );
        let missions = query_saved_missions(&Some(pool.clone()), PLAYER_ID).await;
        assert!(
            missions.is_empty(),
            "a failed missions query returns no missions"
        );

        for reason in [
            "player_query_failed",
            "inventory_query_failed",
            "missions_query_failed",
        ] {
            let row = capture
                .find_event(Level::ERROR, "", reason)
                .unwrap_or_else(|| panic!("no ERROR row with reason={reason}"));
            assert!(
                row.has_field("event", "player_load_degraded"),
                "reason={reason} must carry event=player_load_degraded; got {:?}",
                row.fields,
            );
            assert!(
                row.fields.contains_key("error"),
                "reason={reason} must carry error"
            );
        }
    }
}
