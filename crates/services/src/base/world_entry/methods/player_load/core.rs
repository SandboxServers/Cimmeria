use std::sync::Arc;

use sqlx::PgPool;

use super::meta::{default_player_load_data, query_archetype_ability_tree, query_bandolier_items};
use crate::mercury::PlayerLoadData;

/// Container IDs used in equipment-visual queries. Mirrors the DB schema:
/// 3 = bandolier, 4..=14 = the eleven equipment slots (head, torso, etc.).
pub const CONTAINER_BANDOLIER: i32 = 3;
pub const EQUIPMENT_CONTAINERS: &[i32] = &[4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14];

// NOTE: this query is duplicated in `inventory/core.rs` (the live-update path).
// If you change the column list, ammo_position expression, or row shape here,
// update the other site too — the two paths must produce identical row layouts.
// The SQL drift-guard test in `mod tests` below pins the two copies
// byte-for-byte to catch silent divergence.
pub(crate) const INVENTORY_ITEM_SELECT: &str = r#"
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
            let item_visuals: Vec<String> = match sqlx::query_scalar(
                "SELECT ri.visual_component \
                 FROM sgw_inventory inv \
                 JOIN resources.items ri ON ri.item_id = inv.type_id \
                 WHERE inv.container_id = ANY($1) \
                   AND inv.character_id = $2 \
                   AND ri.visual_component IS NOT NULL \
                   AND ( \
                     (inv.container_id <> $3 AND inv.slot_id = 0) \
                     OR (inv.container_id = $3 AND inv.slot_id = $4) \
                   )",
            )
            .bind({
                let mut all: Vec<i32> = EQUIPMENT_CONTAINERS.to_vec();
                all.push(CONTAINER_BANDOLIER);
                all
            })
            .bind(player_id)
            .bind(CONTAINER_BANDOLIER)
            .bind(row.bandolier_slot)
            .fetch_all(pool.as_ref())
            .await
            {
                Ok(v) => v,
                Err(e) => {
                    tracing::error!(player_id, "Failed to query equipped item visuals — keeping naked appearance for this load: {e}");
                    Vec::new()
                }
            };
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

#[cfg(test)]
mod tests {
    //! Regression guards for player_load/core:
    //!
    //! - `inventory_item_select_matches_player_load_copy_byte_for_byte`:
    //!   pure-Rust unit test pinning the two copies of
    //!   `INVENTORY_ITEM_SELECT` (here and in `inventory/core.rs`)
    //!   byte-for-byte.
    //! - `equipped_item_at_inactive_bandolier_slot_is_excluded_from_visuals`:
    //!   live-DB pin on the `(container <> $3 AND slot = 0) OR
    //!   (container = $3 AND slot = $4)` clause that filters which
    //!   bandolier slot's visual_component bleeds into the avatar.

    use super::*;
    use crate::base::world_entry::methods::inventory::core as inventory_core;
    use crate::test_support::require_db_or_skip;

    /// Sentinel base for the player_load_core IN-list tests, stepped
    /// past the highest live-DB sentinel reserved elsewhere in this
    /// crate. The sibling sentinels are listed alongside their owning
    /// test files; the +0x100 step here just reserves a fresh slot
    /// for these tests so concurrent live-DB runs don't collide on
    /// account/player ids.
    const TEST_BASE: i32 = 0x7000_1100;

    /// Bandolier-allowed weapon — same one used elsewhere in the
    /// player_load tests. The exact type doesn't matter for the IN-list
    /// guard; we only need a row whose `visual_component` is non-NULL
    /// (verified via the assertion in the test itself).
    const WEAPON_TYPE_ID: i32 = 3241;

    async fn cleanup(pool: &PgPool, account_id: i32, player_id: i32) {
        let _ = sqlx::query("DELETE FROM sgw_inventory WHERE character_id = $1")
            .bind(player_id)
            .execute(pool)
            .await;
        let _ = sqlx::query("DELETE FROM account WHERE account_id = $1")
            .bind(account_id)
            .execute(pool)
            .await;
    }

    async fn insert_account_and_player(
        pool: &PgPool,
        account_id: i32,
        player_id: i32,
        bandolier_slot: i32,
    ) {
        sqlx::query(
            "INSERT INTO account (account_id, account_name, password) \
             VALUES ($1, $2, '')",
        )
        .bind(account_id)
        .bind(format!("plcore-in-{account_id}"))
        .execute(pool)
        .await
        .expect("insert account");

        sqlx::query(
            "INSERT INTO sgw_player (\
                account_id, player_id, level, alignment, archetype, gender, \
                player_name, extra_name, world_location, bodyset, \
                pos_x, pos_y, pos_z, skin_color_id, naquadah, bandolier_slot\
             ) VALUES ($1, $2, 1, 0, 1, 1, $3, '', 'CombatSim', 'BS_HumanMale.BS_HumanMale', \
                       0.0, 0.0, 0.0, 0, 0, $4)",
        )
        .bind(account_id)
        .bind(player_id)
        .bind(format!("test-{player_id}"))
        .bind(bandolier_slot)
        .execute(pool)
        .await
        .expect("insert player");
    }

    /// SQL drift-guard: the `INVENTORY_ITEM_SELECT` constant is
    /// intentionally duplicated in `inventory/core.rs` (the
    /// live-update path) and `player_load/core.rs` (the login-load
    /// path). Both must produce identical row layouts so downstream
    /// `InvItem` consumers don't silently disagree. This test fails
    /// the build the moment one drifts.
    #[test]
    fn inventory_item_select_matches_player_load_copy_byte_for_byte() {
        assert_eq!(
            INVENTORY_ITEM_SELECT,
            inventory_core::INVENTORY_ITEM_SELECT,
            "INVENTORY_ITEM_SELECT must stay byte-identical between \
             player_load/core.rs and inventory/core.rs — see the NOTE \
             comment above each constant for why",
        );
    }

    /// IN-list double-count regression guard. The visual-merge query
    /// in `query_player_load_data` uses:
    ///
    ///   (container_id <> $3 AND slot_id = 0)
    ///     OR (container_id = $3 AND slot_id = $4)
    ///
    /// where $3 = CONTAINER_BANDOLIER (3) and $4 = bandolier_slot.
    /// Bug shape this catches: a refactor that simplifies the OR to
    /// just `slot_id = 0` (or that adds container 3 to the IN list
    /// without the slot-disambiguation OR) would surface bandolier
    /// slot 0's visual when the active slot is 1, OR double-count
    /// bandolier slot 0 when bandolier_slot is 0.
    ///
    /// Setup: bandolier_slot=2; insert items at bandolier slots 0
    /// AND 2. Only the slot-2 item's visual must merge into
    /// `components`.
    #[tokio::test]
    async fn equipped_item_at_inactive_bandolier_slot_is_excluded_from_visuals() {
        let pool = require_db_or_skip!();
        let account_id = TEST_BASE;
        let player_id = TEST_BASE + 1;
        cleanup(&pool, account_id, player_id).await;
        // bandolier_slot=2 is the active slot we expect the visual
        // merge to pick.
        insert_account_and_player(&pool, account_id, player_id, 2).await;

        // Look up the visual_component the seeded weapon design
        // contributes. If it's NULL, the test premise is invalid —
        // skip with a panic that points at fixture data rather than
        // the function under test.
        let expected_visual: Option<String> =
            sqlx::query_scalar("SELECT visual_component FROM resources.items WHERE item_id = $1")
                .bind(WEAPON_TYPE_ID)
                .fetch_one(&pool)
                .await
                .expect("look up visual_component");
        let expected_visual = expected_visual.expect(
            "WEAPON_TYPE_ID must have a non-NULL visual_component — pick a different design \
             if the seed data ever changes",
        );

        // Insert at bandolier slots 0 (NOT active) and 2 (active).
        // CONTAINER_BANDOLIER == 3.
        sqlx::query(
            "INSERT INTO sgw_inventory \
                (character_id, type_id, stack_size, slot_id, container_id, \
                 bound, durability, charges) \
             VALUES ($1, $2, 1, 0, $3, false, 100, 0), \
                    ($1, $2, 1, 2, $3, false, 100, 0)",
        )
        .bind(player_id)
        .bind(WEAPON_TYPE_ID)
        .bind(CONTAINER_BANDOLIER)
        .execute(&pool)
        .await
        .expect("insert two bandolier rows");

        let db_pool = Some(Arc::new(pool.clone()));
        let data = query_player_load_data(&db_pool, account_id as u32, player_id).await;

        // Count how many times the weapon's visual appears in the
        // merged components list. The bug shape allows two distinct
        // failure modes:
        //   - 0 occurrences: the OR branch was dropped and the
        //     active slot's item is no longer surfaced at all.
        //   - 2 occurrences: the OR widened to also match
        //     non-bandolier branch for container 3, double-counting.
        // The single correct value is 1.
        let occurrences = data
            .components
            .iter()
            .filter(|c| **c == expected_visual)
            .count();
        assert_eq!(
            occurrences, 1,
            "expected exactly one occurrence of the active bandolier slot's \
             visual_component in components; got {occurrences} (components = {:?})",
            data.components,
        );

        cleanup(&pool, account_id, player_id).await;
    }
}

#[cfg(test)]
mod inventory_loader_tests {
    //! Live-DB integration tests for the player_load core loaders:
    //!
    //! - `query_inventory_items` slot_id+1 wire conversion (DB stores
    //!   0-indexed; wire is 1-indexed).
    //! - `query_player_load_data` no-pool short-circuit returns the
    //!   default sentinel.
    //! - `query_player_load_data` player-not-found returns the default
    //!   sentinel.
    //! - `query_player_load_data` happy path round-trips key fields.
    //!
    //! The visual-merge SQL in `query_player_load_data` (the
    //! `(container_id <> $3 AND slot_id = 0) OR (container_id = $3
    //! AND slot_id = $4)` clause) is NOT exercised by these tests —
    //! the seed items used here have NULL `visual_component` and the
    //! inventory rows go in container 1, not equipment slots
    //! (4..=14) or the bandolier (3). That clause is covered
    //! separately by the IN-list double-count regression guard in
    //! the sibling `tests` module.

    use super::*;
    use crate::test_support::require_db_or_skip;

    /// Sentinel base, stepped past the IN-list test sentinel
    /// (0x7000_1100) and other sibling reservations elsewhere in the
    /// crate so concurrent live-DB runs don't collide on
    /// account/player ids.
    const TEST_BASE: i32 = 0x7000_1500;

    /// Picked at runtime rather than hard-coded. The behavior under
    /// test (slot_id+1 wire conversion, account-isolation,
    /// round-trip of basic fields) is type-agnostic — querying
    /// `resources.items` for *any* main-bag-allowed item lets the
    /// test follow seed-data renumbers automatically. Also satisfies
    /// the `sgw_inventory.type_id` FK without coupling to a specific
    /// design id.
    async fn pick_main_bag_type_id(pool: &PgPool) -> i32 {
        sqlx::query_scalar::<_, i32>(
            "SELECT item_id FROM resources.items \
             WHERE container_sets IS NULL OR 1 = ANY(container_sets) \
             ORDER BY item_id LIMIT 1",
        )
        .fetch_one(pool)
        .await
        .expect("seed must provide at least one main-bag-allowed item")
    }

    async fn cleanup(pool: &PgPool, account_id: i32, player_id: i32) {
        let _ = sqlx::query("DELETE FROM sgw_inventory WHERE character_id = $1")
            .bind(player_id)
            .execute(pool)
            .await;
        let _ = sqlx::query("DELETE FROM account WHERE account_id = $1")
            .bind(account_id)
            .execute(pool)
            .await;
    }

    async fn insert_account_and_player(
        pool: &PgPool,
        account_id: i32,
        player_id: i32,
        archetype: i32,
        player_name: &str,
    ) {
        sqlx::query(
            "INSERT INTO account (account_id, account_name, password) \
             VALUES ($1, $2, '')",
        )
        .bind(account_id)
        .bind(format!("core-test-{account_id}"))
        .execute(pool)
        .await
        .expect("insert account");

        sqlx::query(
            "INSERT INTO sgw_player (\
                account_id, player_id, level, alignment, archetype, gender, \
                player_name, extra_name, world_location, bodyset, \
                pos_x, pos_y, pos_z, skin_color_id, naquadah, bandolier_slot\
             ) VALUES ($1, $2, 7, 1, $3, 1, $4, '', 'CombatSim', 'BS_HumanMale.BS_HumanMale', \
                       0.0, 0.0, 0.0, 0, 1234, 0)",
        )
        .bind(account_id)
        .bind(player_id)
        .bind(archetype)
        .bind(player_name)
        .execute(pool)
        .await
        .expect("insert player");
    }

    async fn insert_inventory_row(
        pool: &PgPool,
        player_id: i32,
        type_id: i32,
        container_id: i32,
        slot_id: i32,
    ) -> i32 {
        sqlx::query_scalar(
            "INSERT INTO sgw_inventory \
                (character_id, type_id, stack_size, slot_id, container_id, \
                 bound, durability, charges) \
             VALUES ($1, $2, 1, $3, $4, false, 100, 0) \
             RETURNING item_id",
        )
        .bind(player_id)
        .bind(type_id)
        .bind(slot_id)
        .bind(container_id)
        .fetch_one(pool)
        .await
        .expect("insert inventory row")
    }

    /// `query_inventory_items` converts the 0-indexed DB slot_id to
    /// the 1-indexed wire slot_id. This is documented at the bottom
    /// of core.rs ("slot_id is stored 0-indexed in DB but sent
    /// 1-indexed on the wire") and a regression flips item placement
    /// in every UI panel for every player. Pin it.
    #[tokio::test]
    async fn query_inventory_items_wire_slot_is_one_based() {
        let pool = require_db_or_skip!();
        let type_id = pick_main_bag_type_id(&pool).await;
        let account_id = TEST_BASE;
        let player_id = TEST_BASE + 1;
        cleanup(&pool, account_id, player_id).await;
        insert_account_and_player(&pool, account_id, player_id, 1, "wire-slot").await;
        // Slot 0 in DB.
        let _ = insert_inventory_row(&pool, player_id, type_id, 1, 0).await;
        // Slot 5 in DB.
        let _ = insert_inventory_row(&pool, player_id, type_id, 1, 5).await;

        let items = query_inventory_items(&pool, player_id).await;
        assert_eq!(items.len(), 2);

        let wire_slots: Vec<i32> = items.iter().map(|i| i.slot_id).collect();
        assert!(
            wire_slots.contains(&1),
            "DB slot 0 must be reported as wire slot 1; got {wire_slots:?}",
        );
        assert!(
            wire_slots.contains(&6),
            "DB slot 5 must be reported as wire slot 6; got {wire_slots:?}",
        );

        cleanup(&pool, account_id, player_id).await;
    }

    /// No DB pool short-circuits to `default_player_load_data()` —
    /// the offline-mode sentinel callers rely on. The default's
    /// `player_id == 0` and `player_name == "Unknown"` are stable
    /// markers we can assert on without coupling to every default
    /// field.
    #[tokio::test]
    async fn query_player_load_data_no_pool_returns_default_sentinel() {
        // No `require_db_or_skip!()` — this branch deliberately
        // exercises the None pool path and runs even when
        // DATABASE_URL is unset.
        let data = query_player_load_data(&None, 0, 0).await;

        assert_eq!(data.player_id, 0, "default sentinel uses player_id=0");
        assert_eq!(
            data.player_name, "Unknown",
            "default sentinel uses 'Unknown' name",
        );
    }

    /// Player-not-found (mismatched account_id / player_id) returns
    /// the same default sentinel as the no-pool path. Bug shape this
    /// catches: a refactor that swaps the WHERE clause's binding
    /// order would let any player_id match any account_id, which is
    /// a session-confusion vector.
    #[tokio::test]
    async fn query_player_load_data_account_mismatch_returns_default() {
        let pool = require_db_or_skip!();
        let account_id = TEST_BASE + 100;
        let player_id = TEST_BASE + 101;
        cleanup(&pool, account_id, player_id).await;
        insert_account_and_player(&pool, account_id, player_id, 1, "real").await;

        let db_pool = Some(Arc::new(pool.clone()));
        // Wrong account_id (1) for this player_id — must NOT load.
        let data = query_player_load_data(&db_pool, 1, player_id).await;

        assert_eq!(
            data.player_id, 0,
            "account/player mismatch must return the default sentinel, NOT \
             a leaked load of the real player",
        );
        assert_eq!(data.player_name, "Unknown");

        cleanup(&pool, account_id, player_id).await;
    }

    /// Happy path: a real account+player+inventory loads through with
    /// fields round-tripped from the DB. Asserts the load surface
    /// most likely to silently regress: level (i32), name (string),
    /// archetype (drives ability tree), and items[] (the inventory
    /// query joining on resources.items).
    #[tokio::test]
    async fn query_player_load_data_round_trips_basic_fields() {
        let pool = require_db_or_skip!();
        let type_id = pick_main_bag_type_id(&pool).await;
        let account_id = TEST_BASE + 200;
        let player_id = TEST_BASE + 201;
        cleanup(&pool, account_id, player_id).await;
        insert_account_and_player(&pool, account_id, player_id, 1, "happy-path").await;
        let _ = insert_inventory_row(&pool, player_id, type_id, 1, 0).await;

        let db_pool = Some(Arc::new(pool.clone()));
        let data = query_player_load_data(&db_pool, account_id as u32, player_id).await;

        assert_eq!(data.player_id, player_id);
        assert_eq!(data.level, 7, "level must round-trip from sgw_player");
        assert_eq!(data.player_name, "happy-path");
        assert_eq!(data.archetype, 1, "archetype must round-trip");
        assert_eq!(data.naquadah, 1234, "naquadah must round-trip");
        assert_eq!(
            data.items.len(),
            1,
            "inventory query must surface the seeded item",
        );
        assert_eq!(
            data.items[0].slot_id, 1,
            "wire slot_id must be DB slot+1 (0 -> 1)",
        );
        assert_eq!(data.items[0].dbid, type_id);

        cleanup(&pool, account_id, player_id).await;
    }
}
