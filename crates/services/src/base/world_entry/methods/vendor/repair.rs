use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use cimmeria_mercury::transport::Transport;
use sqlx::PgPool;

use super::super::super::super::ConnectedClientState;
use super::super::inventory::core::send_full_inventory_update;
use super::super::inventory::grant::normalize_item_ids;

/// Containers that can be operated on by the vendor stack — main bag, bandolier,
/// equipment slots, and quick bars. Bank, mail attachments, and loot are excluded.
use super::VENDOR_FILTER_BAGS;

pub async fn handle_repair_inventory_item(
    entity_id: u32,
    player_id: i32,
    item_id: i32,
    repair_ratio: f32,
    db_pool: &Option<Arc<PgPool>>,
    transport: &Arc<dyn Transport>,
    connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    entity_to_addr: &Arc<Mutex<HashMap<u32, SocketAddr>>>,
) {
    let pool = match db_pool {
        Some(p) => p,
        None => {
            tracing::debug!(player_id, item_id, "RepairInventoryItem: no DB pool");
            return;
        }
    };

    if item_id <= 0 || !repair_ratio.is_finite() || repair_ratio <= 0.0 {
        tracing::warn!(
            player_id,
            item_id,
            repair_ratio,
            "RepairInventoryItem: invalid item or ratio"
        );
        return;
    }

    // Tiny ratios (<0.005) round down to 0 points, which then matches no rows
    // in the WHERE clause and silently reports "no repairable item changed."
    // Floor-clamp so any non-zero ratio repairs at least one durability point.
    let repair_points = ((repair_ratio.clamp(0.0, 1.0) * 100.0).round() as i32).max(1);
    let result = sqlx::query(
        "UPDATE sgw_inventory \
         SET durability = LEAST(100, GREATEST(0, durability) + $1) \
         WHERE character_id = $2 AND item_id = $3 \
           AND container_id = ANY($4) \
           AND durability >= 0 AND durability < 100",
    )
    .bind(repair_points)
    .bind(player_id)
    .bind(item_id)
    .bind(VENDOR_FILTER_BAGS.as_slice())
    .execute(pool.as_ref())
    .await;

    match result {
        Ok(r) if r.rows_affected() == 1 => {
            let total_items = send_full_inventory_update(
                entity_id,
                player_id,
                pool,
                transport,
                connected,
                entity_to_addr,
            )
            .await;
            tracing::debug!(
                entity_id,
                player_id,
                item_id,
                repair_points,
                total_items,
                "Inventory item repaired"
            );
        }
        Ok(_) => {
            tracing::debug!(
                player_id,
                item_id,
                repair_points,
                "RepairInventoryItem: no repairable item changed"
            );
        }
        Err(e) => {
            tracing::error!(
                player_id,
                item_id,
                "RepairInventoryItem: update failed: {e}"
            );
        }
    }
}

pub async fn handle_repair_inventory_items(
    entity_id: u32,
    player_id: i32,
    item_ids: Vec<i32>,
    vendor_template_id: Option<i32>,
    db_pool: &Option<Arc<PgPool>>,
    transport: &Arc<dyn Transport>,
    connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    entity_to_addr: &Arc<Mutex<HashMap<u32, SocketAddr>>>,
) {
    if let Some(vendor_template_id) = vendor_template_id {
        super::paid_repair::handle_paid_repair_inventory_items(
            entity_id,
            player_id,
            item_ids,
            vendor_template_id,
            db_pool,
            transport,
            connected,
            entity_to_addr,
        )
        .await;
        return;
    }

    let pool = match db_pool {
        Some(p) => p,
        None => {
            tracing::debug!(player_id, "RepairInventoryItems: no DB pool");
            return;
        }
    };

    let item_ids = normalize_item_ids(item_ids);
    if item_ids.is_empty() {
        tracing::debug!(
            entity_id,
            player_id,
            "RepairInventoryItems: empty item list"
        );
        return;
    }

    let result = sqlx::query(
        "UPDATE sgw_inventory \
         SET durability = 100 \
         WHERE character_id = $1 \
           AND item_id = ANY($2) \
           AND container_id = ANY($3) \
           AND stack_size = 1 \
           AND durability >= 0 \
           AND durability < 100",
    )
    .bind(player_id)
    .bind(&item_ids)
    .bind(VENDOR_FILTER_BAGS.as_slice())
    .execute(pool.as_ref())
    .await;

    match result {
        Ok(r) if r.rows_affected() > 0 => {
            let total_items = send_full_inventory_update(
                entity_id,
                player_id,
                pool,
                transport,
                connected,
                entity_to_addr,
            )
            .await;
            tracing::debug!(
                entity_id,
                player_id,
                item_count = item_ids.len(),
                repaired = r.rows_affected(),
                total_items,
                "Inventory items repaired"
            );
        }
        Ok(_) => tracing::debug!(
            entity_id,
            player_id,
            item_count = item_ids.len(),
            "RepairInventoryItems: no repairable items changed"
        ),
        Err(e) => tracing::error!(
            entity_id,
            player_id,
            item_count = item_ids.len(),
            "RepairInventoryItems: update failed: {e}"
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::require_db_or_skip;
    use crate::test_support::TestTransport;

    /// Sentinel base for vendor repair tests. Distinct from grant_cash (+0x100),
    /// move_inventory (+0x200), grant_item (+0x300), missions (+0x400),
    /// mail (+0x500).
    const TEST_BASE: i32 = 0x7000_0600;

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
        .bind(format!("repair-test-{account_id}"))
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

    async fn pick_main_bag_type_id(pool: &PgPool) -> i32 {
        sqlx::query_scalar::<_, i32>(
            "SELECT item_id FROM resources.items \
             WHERE container_sets IS NULL OR 1 = ANY(container_sets) \
             ORDER BY item_id LIMIT 1",
        )
        .fetch_one(pool)
        .await
        .expect("pick item_id")
    }

    /// Insert a sgw_inventory row with the given durability. Returns the
    /// auto-generated item_id.
    async fn insert_item_with_durability(
        pool: &PgPool,
        player_id: i32,
        type_id: i32,
        slot_id: i32,
        durability: i32,
    ) -> i32 {
        sqlx::query_scalar(
            "INSERT INTO sgw_inventory \
                (character_id, type_id, stack_size, slot_id, container_id, \
                 bound, durability, charges) \
             VALUES ($1, $2, 1, $3, 1, false, $4, 0) RETURNING item_id",
        )
        .bind(player_id)
        .bind(type_id)
        .bind(slot_id)
        .bind(durability)
        .fetch_one(pool)
        .await
        .expect("insert inventory row")
    }

    async fn durability_of(pool: &PgPool, player_id: i32, item_id: i32) -> i32 {
        sqlx::query_scalar(
            "SELECT durability FROM sgw_inventory \
             WHERE character_id = $1 AND item_id = $2",
        )
        .bind(player_id)
        .bind(item_id)
        .fetch_one(pool)
        .await
        .unwrap()
    }

    fn make_state(
        entity_id: u32,
    ) -> (
        Arc<dyn Transport>,
        Arc<Mutex<HashMap<u32, SocketAddr>>>,
        Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    ) {
        let transport: Arc<dyn Transport> = Arc::new(TestTransport::new());
        let fake_addr: SocketAddr = "127.0.0.1:65535".parse().unwrap();
        let entity_to_addr = Arc::new(Mutex::new({
            let mut m = HashMap::new();
            m.insert(entity_id, fake_addr);
            m
        }));
        let connected = Arc::new(Mutex::new(HashMap::new()));
        (transport, entity_to_addr, connected)
    }

    /// Single-item repair adds repair_points = round(ratio * 100) to durability.
    /// Pins the math: 50 + 30 = 80.
    #[tokio::test]
    async fn single_item_partial_repair_increases_durability_by_ratio() {
        let pool = require_db_or_skip!();
        let account_id = TEST_BASE;
        let player_id = TEST_BASE + 1;
        cleanup(&pool, account_id, player_id).await;
        insert_account_and_player(&pool, account_id, player_id).await;
        let type_id = pick_main_bag_type_id(&pool).await;
        let item = insert_item_with_durability(&pool, player_id, type_id, 0, 50).await;

        let (transport, e2a, conn) = make_state(0x7000_0601);
        let db_pool = Some(Arc::new(pool.clone()));

        handle_repair_inventory_item(
            0x7000_0601,
            player_id,
            item,
            0.30,
            &db_pool,
            &transport,
            &conn,
            &e2a,
        )
        .await;

        assert_eq!(
            durability_of(&pool, player_id, item).await,
            80,
            "50 + round(0.30 * 100) = 80"
        );

        cleanup(&pool, account_id, player_id).await;
    }

    /// Regression guard: the LEAST(100, ...) clamp must prevent over-repair.
    /// A near-full item (95) repaired with ratio 1.0 (would add 100) ends
    /// at exactly 100 — never above.
    #[tokio::test]
    async fn single_item_repair_clamps_durability_at_one_hundred() {
        let pool = require_db_or_skip!();
        let account_id = TEST_BASE + 100;
        let player_id = TEST_BASE + 101;
        cleanup(&pool, account_id, player_id).await;
        insert_account_and_player(&pool, account_id, player_id).await;
        let type_id = pick_main_bag_type_id(&pool).await;
        let item = insert_item_with_durability(&pool, player_id, type_id, 0, 95).await;

        let (transport, e2a, conn) = make_state(0x7000_0602);
        let db_pool = Some(Arc::new(pool.clone()));

        handle_repair_inventory_item(
            0x7000_0602,
            player_id,
            item,
            1.0,
            &db_pool,
            &transport,
            &conn,
            &e2a,
        )
        .await;

        assert_eq!(
            durability_of(&pool, player_id, item).await,
            100,
            "95 + 100 must clamp to 100, not pass through to 195"
        );

        cleanup(&pool, account_id, player_id).await;
    }

    /// Free-repair (vendor_template_id = None) sets durability to exactly 100
    /// for damaged items, but the WHERE clause's `durability < 100` filter
    /// must skip already-full items so a stack of full + damaged repairs
    /// only the damaged one. Tests both invariants in a single call.
    #[tokio::test]
    async fn free_repair_sets_damaged_to_full_and_skips_already_full() {
        let pool = require_db_or_skip!();
        let account_id = TEST_BASE + 200;
        let player_id = TEST_BASE + 201;
        cleanup(&pool, account_id, player_id).await;
        insert_account_and_player(&pool, account_id, player_id).await;
        let type_id = pick_main_bag_type_id(&pool).await;
        let damaged = insert_item_with_durability(&pool, player_id, type_id, 0, 30).await;
        let full = insert_item_with_durability(&pool, player_id, type_id, 1, 100).await;

        let (transport, e2a, conn) = make_state(0x7000_0603);
        let db_pool = Some(Arc::new(pool.clone()));

        handle_repair_inventory_items(
            0x7000_0603,
            player_id,
            vec![damaged, full],
            None, // free repair
            &db_pool,
            &transport,
            &conn,
            &e2a,
        )
        .await;

        assert_eq!(
            durability_of(&pool, player_id, damaged).await,
            100,
            "free repair sets damaged item to 100"
        );
        assert_eq!(
            durability_of(&pool, player_id, full).await,
            100,
            "already-full item must stay at 100 (the < 100 WHERE filter skips it)"
        );

        cleanup(&pool, account_id, player_id).await;
    }
}
