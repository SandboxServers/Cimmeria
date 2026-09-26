use std::sync::Arc;

use sqlx::PgPool;

use cimmeria_entity::abilities::AbilityTreeData;

use crate::mercury::PlayerLoadData;

/// Default player load data when the DB is unavailable.
///
/// The ability tree is empty: with no database there is no catalog, and the
/// tree is never fabricated in Rust (AT-02).
pub fn default_player_load_data() -> PlayerLoadData {
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
        weapon_visual: None,
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
        active_bandolier_slot: 0,
        bandolier_items: vec![],
        ability_tree: AbilityTreeData::default(),
        items: vec![],
        // Defaults from SystemOptions.xml when the row can't be loaded —
        // keeps the in-memory `SystemOptions::default()` value and the
        // wire defaults in agreement.
        auto_reload: true,
        reload_on_activate: false,
    }
}

#[derive(sqlx::FromRow)]
struct BandolierItemRow {
    slot_id: i32,
    /// `sgw_inventory.item_id` — the per-row instance id used as the ammo-persist
    /// TOCTOU guard (distinct from the design `item_id` below).
    instance_id: i32,
    item_id: i32,
    clip_size: i32,
    default_ammo_type_id: i32,
    /// Per-slot remaining ammo from `sgw_inventory.ammo`. Populated by Stage A
    /// but not yet read by fire/reload — Stage C wires the consumers.
    current_ammo: i32,
    /// Per-slot selected ammo subtype from `sgw_inventory.cur_ammo_type`. Zero
    /// means "no explicit choice" — Rust-side defaulting falls back to the
    /// item's default_ammo_type below.
    cur_ammo_type: i32,
}

const BANDOLIER_ITEMS_QUERY: &str = r#"
SELECT inv.slot_id, inv.item_id AS instance_id, inv.type_id AS item_id, COALESCE(ri.clip_size, 0) AS clip_size,
       CASE WHEN ri.default_ammo_type IS NULL THEN 0
            ELSE array_position(enum_range(NULL::resources."EAmmoType"), ri.default_ammo_type) - 1
       END AS default_ammo_type_id,
       inv.ammo AS current_ammo,
       inv.cur_ammo_type
FROM sgw_inventory inv
JOIN resources.items ri ON ri.item_id = inv.type_id
WHERE inv.character_id = $1 AND inv.container_id = 3
ORDER BY inv.slot_id
"#;

fn map_bandolier_rows(
    rows: Vec<BandolierItemRow>,
) -> Vec<(i32, cimmeria_entity::cell_entity::BandolierItem)> {
    rows.into_iter()
        .map(|row| {
            // Treat 0 as "no explicit choice" and fall back to the item's
            // default ammo type — matches the legacy Account.py behavior where
            // a slot with no override picks default_ammo_type at load time.
            let cur_ammo_type = if row.cur_ammo_type == 0 {
                row.default_ammo_type_id
            } else {
                row.cur_ammo_type
            };
            (
                row.slot_id,
                cimmeria_entity::cell_entity::BandolierItem {
                    instance_id: row.instance_id,
                    item_id: row.item_id,
                    clip_size: row.clip_size,
                    default_ammo_type: row.default_ammo_type_id,
                    current_ammo: row.current_ammo,
                    cur_ammo_type,
                },
            )
        })
        .collect()
}

/// Query bandolier items from the database for a player.
///
/// Returns tuples of (slot_id, BandolierItem) containing equipped weapon info.
pub async fn query_bandolier_items(
    db_pool: &Option<Arc<PgPool>>,
    player_id: i32,
) -> Vec<(i32, cimmeria_entity::cell_entity::BandolierItem)> {
    let pool = match db_pool {
        Some(p) => p,
        None => return vec![],
    };

    match sqlx::query_as::<_, BandolierItemRow>(BANDOLIER_ITEMS_QUERY)
        .bind(player_id)
        .fetch_all(pool.as_ref())
        .await
    {
        Ok(rows) => map_bandolier_rows(rows),
        Err(e) => {
            tracing::error!(player_id, "query_bandolier_items failed: {e}");
            vec![]
        }
    }
}

/// Transaction-aware variant — runs the bandolier read inside an existing tx
/// so callers that have already acquired locks (e.g. `FOR UPDATE` on
/// `sgw_player`) see a consistent snapshot.
pub async fn query_bandolier_items_tx(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    player_id: i32,
) -> Result<Vec<(i32, cimmeria_entity::cell_entity::BandolierItem)>, sqlx::Error> {
    let rows = sqlx::query_as::<_, BandolierItemRow>(BANDOLIER_ITEMS_QUERY)
        .bind(player_id)
        .fetch_all(&mut **tx)
        .await?;
    Ok(map_bandolier_rows(rows))
}

/// The player's `onAbilityTreeInfo` payload, built from the process's
/// shared [`crate::ability_tree::AbilityTreeCatalog`], the one the cell's
/// trainer and purchase gate read. No per-player query and no fallback
/// tree: an archetype with no rows gets an empty tree and one WARN (see
/// [`crate::ability_tree::tree_info`]), and a catalog that fails to load
/// gets an empty tree and one ERROR.
pub async fn player_ability_tree(
    pool: &PgPool,
    archetype_id: i32,
    player_id: i32,
) -> AbilityTreeData {
    match crate::ability_tree::shared_catalog(pool).await {
        Ok(catalog) => crate::ability_tree::tree_info(&catalog, archetype_id, player_id),
        Err(e) => {
            tracing::error!(
                target: "abilities",
                event = "tree_catalog_load_failed",
                reason = "catalog_load_failed",
                player_id,
                archetype_id,
                "Ability-tree catalog failed to load; sending an empty onAbilityTreeInfo: {e}"
            );
            AbilityTreeData::default()
        }
    }
}

#[cfg(test)]
mod tests {
    //! Live-DB integration tests for the player_load metadata loaders.
    //!
    //! Skip cleanly when DATABASE_URL is unset; against the bundled
    //! local Postgres they exercise:
    //!
    //! - `query_bandolier_items` happy path + cur_ammo_type=0 fallback
    //!   to default_ammo_type (the legacy Account.py behavior).
    //! - `query_bandolier_items` empty-bandolier path.
    //! - `query_bandolier_items` no-pool short-circuit.
    //! - `player_ability_tree` against every `EArchetype` value: the
    //!   catalog-built tree equals the table read directly.

    use super::*;
    use crate::test_support::require_db_or_skip;

    /// Sentinel base for player_load/meta tests, stepped past prior
    /// live-DB sentinels reserved elsewhere in the crate.
    const TEST_BASE: i32 = 0x7000_0D00;

    const INV_BANDOLIER: i32 = 3;

    /// Picked at runtime in each test rather than hard-coded. The
    /// behavior under test is the `cur_ammo_type=0 -> default` fallback
    /// in `map_bandolier_rows`, which doesn't care which specific
    /// weapon design is used — it only needs *a* bandolier-allowed
    /// design with a non-NULL `default_ammo_type` and `clip_size > 0`.
    /// Querying lets the test follow seed-data renumbers automatically.
    /// Returns `(weapon_type_id, expected_clip_size, expected_default_ammo_index)`.
    async fn pick_bandolier_weapon_with_stats(pool: &PgPool) -> (i32, i32, i32) {
        sqlx::query_as::<_, (i32, i32, i32)>(
            "SELECT ri.item_id, ri.clip_size, \
                    array_position(enum_range(NULL::resources.\"EAmmoType\"), \
                                   ri.default_ammo_type) - 1 \
             FROM resources.items ri \
             WHERE (ri.container_sets IS NULL OR 3 = ANY(ri.container_sets)) \
               AND ri.default_ammo_type IS NOT NULL \
               AND ri.clip_size > 0 \
             ORDER BY ri.item_id LIMIT 1",
        )
        .fetch_one(pool)
        .await
        .expect(
            "seed must provide a bandolier-allowed weapon with default_ammo_type \
             and clip_size > 0",
        )
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

    async fn insert_account_and_player(pool: &PgPool, account_id: i32, player_id: i32) {
        sqlx::query(
            "INSERT INTO account (account_id, account_name, password) \
             VALUES ($1, $2, '')",
        )
        .bind(account_id)
        .bind(format!("meta-test-{account_id}"))
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
    }

    async fn insert_bandolier_row(
        pool: &PgPool,
        player_id: i32,
        weapon_type_id: i32,
        slot_id: i32,
        cur_ammo_type: i32,
        ammo: i32,
    ) {
        sqlx::query(
            "INSERT INTO sgw_inventory \
                (character_id, type_id, stack_size, slot_id, container_id, \
                 bound, durability, charges, ammo, cur_ammo_type) \
             VALUES ($1, $2, 1, $3, $4, false, 100, 0, $5, $6)",
        )
        .bind(player_id)
        .bind(weapon_type_id)
        .bind(slot_id)
        .bind(INV_BANDOLIER)
        .bind(ammo)
        .bind(cur_ammo_type)
        .execute(pool)
        .await
        .expect("insert bandolier row");
    }

    /// Happy path + the cur_ammo_type=0 fallback: when the inventory
    /// row's cur_ammo_type is 0 ("no explicit choice"), the loader
    /// substitutes the item's default_ammo_type. Locks the legacy
    /// Account.py-compat behavior in.
    #[tokio::test]
    async fn bandolier_zero_cur_ammo_type_falls_back_to_default_ammo() {
        let pool = require_db_or_skip!();
        let (weapon_type, expected_clip_size, expected_default_ammo_index) =
            pick_bandolier_weapon_with_stats(&pool).await;
        let account_id = TEST_BASE;
        let player_id = TEST_BASE + 1;
        cleanup(&pool, account_id, player_id).await;
        insert_account_and_player(&pool, account_id, player_id).await;
        // Slot 1: cur_ammo_type=0 — must fall back to weapon's default.
        insert_bandolier_row(&pool, player_id, weapon_type, 1, 0, 7).await;
        // Slot 2: cur_ammo_type set to a value distinct from the default
        // so the verbatim-passthrough assertion can't coincidentally
        // match the fallback.
        let explicit_cur_ammo_type = expected_default_ammo_index + 1;
        insert_bandolier_row(&pool, player_id, weapon_type, 2, explicit_cur_ammo_type, 12).await;

        let db_pool = Some(Arc::new(pool.clone()));
        let items = query_bandolier_items(&db_pool, player_id).await;

        assert_eq!(items.len(), 2);
        let by_slot: std::collections::HashMap<i32, _> = items.into_iter().collect();

        let slot1 = by_slot.get(&1).expect("slot 1 must be present");
        assert_eq!(slot1.item_id, weapon_type);
        assert_eq!(slot1.clip_size, expected_clip_size);
        assert_eq!(slot1.default_ammo_type, expected_default_ammo_index);
        assert_eq!(
            slot1.current_ammo, 7,
            "current_ammo must round-trip from inv.ammo"
        );
        // The behavior under test is the fallback itself — assert
        // structurally rather than against a specific enum index, so
        // the test follows seed renumbers automatically.
        assert_eq!(
            slot1.cur_ammo_type, slot1.default_ammo_type,
            "cur_ammo_type=0 must fall back to default_ammo_type",
        );

        let slot2 = by_slot.get(&2).expect("slot 2 must be present");
        assert_eq!(
            slot2.cur_ammo_type, explicit_cur_ammo_type,
            "non-zero cur_ammo_type must be returned verbatim",
        );

        cleanup(&pool, account_id, player_id).await;
    }

    /// Empty-bandolier path: a player with no rows in container 3
    /// gets an empty Vec — must NOT error or return a sentinel item.
    #[tokio::test]
    async fn bandolier_empty_returns_empty_vec() {
        let pool = require_db_or_skip!();
        let account_id = TEST_BASE + 100;
        let player_id = TEST_BASE + 101;
        cleanup(&pool, account_id, player_id).await;
        insert_account_and_player(&pool, account_id, player_id).await;
        // Deliberately do NOT insert any bandolier rows.

        let db_pool = Some(Arc::new(pool.clone()));
        let items = query_bandolier_items(&db_pool, player_id).await;

        assert!(items.is_empty());

        cleanup(&pool, account_id, player_id).await;
    }

    /// No-pool short-circuit: `db_pool: None` returns empty Vec
    /// without hitting the DB. Important because the offline-mode
    /// path (no DB) relies on this returning empty rather than
    /// erroring.
    #[tokio::test]
    async fn bandolier_no_pool_returns_empty_vec() {
        // No `require_db_or_skip!()` here — this test deliberately
        // exercises the None branch and runs even when DATABASE_URL
        // is unset.
        let items = query_bandolier_items(&None, 1).await;
        assert!(items.is_empty());
    }

    /// `player_ability_tree` for every `EArchetype` value, and for an id
    /// outside it, equals the table grouped by `tree_index` in
    /// `ability_index` order. That is exactly what the retired per-player
    /// query sent, so AT-02 leaves the seeded archetypes' `onAbilityTreeInfo`
    /// bytes unchanged, and an archetype with no rows gets an empty tree
    /// instead of the old hand-copied Rust fallback.
    #[tokio::test]
    async fn player_ability_tree_matches_table_for_every_archetype() {
        let pool = require_db_or_skip!();
        let enum_size: i32 =
            sqlx::query_scalar("SELECT cardinality(enum_range(NULL::resources.\"EArchetype\"))")
                .fetch_one(&pool)
                .await
                .expect("read enum cardinality");

        let mut seeded = 0;
        for archetype_id in -1..=enum_size {
            let mut expected: [Vec<i32>; 3] = Default::default();
            let rows: Vec<(i32, i32)> = sqlx::query_as(
                "SELECT tree_index, ability_id                  FROM resources.archetype_ability_tree                  WHERE archetype = (enum_range(NULL::resources.\"EArchetype\"))[$1 + 1]                  ORDER BY tree_index, ability_index",
            )
            .bind(archetype_id)
            .fetch_all(&pool)
            .await
            .expect("read tree rows");
            for (tree_index, ability_id) in rows {
                expected[tree_index as usize].push(ability_id);
            }
            if expected.iter().any(|t| !t.is_empty()) {
                seeded += 1;
            }

            let tree = player_ability_tree(&pool, archetype_id, TEST_BASE).await;
            assert_eq!(tree.trees, expected, "archetype_id={archetype_id}");
        }
        assert!(
            seeded >= 2,
            "the seed must carry at least Soldier and Commando"
        );
    }

    /// Cardinality tripwire: pin the `cimmeria_entity::stats::ARCHETYPE_COUNT`
    /// constant against the actual size of `resources."EArchetype"`.
    ///
    /// Failure here is intentional — adding a value to the SQL enum
    /// without bumping `ARCHETYPE_COUNT` (and walking the checklist
    /// next to it) leaves downstream consumers (CHECK constraint,
    /// chardef, ability-tree seed rows) potentially stale. Failing CI
    /// loud forces the dev to revisit each one.
    #[tokio::test]
    async fn archetype_count_matches_earchetype_enum_cardinality() {
        let pool = require_db_or_skip!();
        let enum_size: i32 =
            sqlx::query_scalar("SELECT cardinality(enum_range(NULL::resources.\"EArchetype\"))")
                .fetch_one(&pool)
                .await
                .expect("read enum cardinality");
        assert_eq!(
            usize::try_from(enum_size).expect("enum_size must be non-negative"),
            cimmeria_entity::stats::ARCHETYPE_COUNT,
            "EArchetype enum size ({enum_size}) drifted from ARCHETYPE_COUNT \
             ({}); see the checklist next to ARCHETYPE_COUNT in \
             crates/entity/src/stats/archetype.rs and walk every item before \
             bumping the constant",
            cimmeria_entity::stats::ARCHETYPE_COUNT,
        );
    }
}
