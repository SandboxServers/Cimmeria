use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use sqlx::PgPool;
use tokio::net::UdpSocket;

use crate::base::ConnectedClientState;
use super::super::inventory::core::send_full_inventory_update;
use super::super::inventory::grant::normalize_item_ids;
use super::store::send_store_update_to_client;
use super::helpers::send_cash_changed_to_client;
use super::purchase_helpers::load_vendor_template_lists;
use super::serializers::StoreItemCostUpdate;

use super::VENDOR_FILTER_BAGS;

#[derive(sqlx::FromRow)]
struct StoreItemCostRow {
    cost: i32,
    item_id: i32,
}

/// Transactionally repair items for payment and refresh inventory/cash.
pub async fn handle_paid_repair_inventory_items(
    entity_id: u32,
    player_id: i32,
    item_ids: Vec<i32>,
    vendor_template_id: i32,
    db_pool: &Option<Arc<PgPool>>,
    socket: &Arc<UdpSocket>,
    connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    entity_to_addr: &Arc<Mutex<HashMap<u32, SocketAddr>>>,
) {
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

    let Some(template) =
        load_vendor_template_lists(pool, vendor_template_id, "RepairInventoryItems").await
    else {
        return;
    };
    let Some(repair_item_list) = template.repair_item_list else {
        tracing::warn!(
            entity_id,
            player_id,
            vendor_template_id,
            "RepairInventoryItems: vendor has no repair list — client request dropped"
        );
        return;
    };

    let mut tx = match pool.begin().await {
        Ok(tx) => tx,
        Err(e) => {
            tracing::error!(
                entity_id,
                player_id,
                "RepairInventoryItems: begin failed: {e}"
            );
            return;
        }
    };

    // Lock acquisition order: `sgw_inventory` rows (the FOR UPDATE OF inv below)
    // are acquired BEFORE `sgw_player.naquadah` (the FOR UPDATE balance read
    // later). All vendor-stack handlers (paid_repair, paid_recharge, purchase,
    // sell, buyback) follow this same order to avoid lock-cycle deadlocks
    // between repair and concurrent grant/move operations that touch both
    // tables.
    let rows = match sqlx::query_as::<_, StoreItemCostRow>(
        "SELECT GREATEST((ili.naquadah::BIGINT * (100 - inv.durability)::BIGINT) / 100, 1)::INT AS cost, \
                inv.item_id \
         FROM resources.item_list_items ili \
         JOIN sgw_inventory inv ON inv.type_id = ili.design_id \
         WHERE ili.item_list_id = $1 \
           AND inv.character_id = $2 \
           AND inv.item_id = ANY($3) \
           AND inv.container_id = ANY($4) \
           AND inv.stack_size = 1 \
           AND inv.durability >= 0 \
           AND inv.durability < 100 \
         FOR UPDATE OF inv",
    )
    .bind(repair_item_list)
    .bind(player_id)
    .bind(&item_ids)
    .bind(VENDOR_FILTER_BAGS.as_slice())
    .fetch_all(&mut *tx)
    .await
    {
        Ok(rows) => rows,
        Err(e) => {
            let _ = tx.rollback().await;
            tracing::error!(
                entity_id,
                player_id,
                vendor_template_id,
                "RepairInventoryItems: repair query failed: {e}"
            );
            return;
        }
    };

    let rows_by_id: HashMap<i32, StoreItemCostRow> =
        rows.into_iter().map(|row| (row.item_id, row)).collect();
    let mut total_cost = 0i32;
    for item_id in &item_ids {
        let Some(row) = rows_by_id.get(item_id) else {
            let _ = tx.rollback().await;
            tracing::warn!(
                entity_id,
                player_id,
                item_id,
                "RepairInventoryItems: item is not repairable at this vendor"
            );
            return;
        };

        total_cost = match total_cost.checked_add(row.cost) {
            Some(total) => total,
            None => {
                let _ = tx.rollback().await;
                tracing::warn!(entity_id, player_id, "RepairInventoryItems: cost overflow");
                return;
            }
        };
    }

    let balance: Option<i32> =
        match sqlx::query_scalar("SELECT naquadah FROM sgw_player WHERE player_id = $1 FOR UPDATE")
            .bind(player_id)
            .fetch_optional(&mut *tx)
            .await
        {
            Ok(balance) => balance,
            Err(e) => {
                let _ = tx.rollback().await;
                tracing::error!(
                    entity_id,
                    player_id,
                    "RepairInventoryItems: balance query failed: {e}"
                );
                return;
            }
        };

    let Some(balance) = balance else {
        let _ = tx.rollback().await;
        tracing::warn!(
            entity_id,
            player_id,
            "RepairInventoryItems: player not found"
        );
        return;
    };

    if balance < total_cost {
        let _ = tx.rollback().await;
        tracing::warn!(
            entity_id,
            player_id,
            balance,
            total_cost,
            "RepairInventoryItems: insufficient naquadah"
        );
        return;
    }

    let new_cash_total = match sqlx::query_scalar::<_, i32>(
        "UPDATE sgw_player SET naquadah = naquadah - $1 \
         WHERE player_id = $2 RETURNING naquadah",
    )
    .bind(total_cost)
    .bind(player_id)
    .fetch_optional(&mut *tx)
    .await
    {
        Ok(Some(total)) => total,
        Ok(None) => {
            let _ = tx.rollback().await;
            tracing::warn!(
                entity_id,
                player_id,
                "RepairInventoryItems: player disappeared before cash update"
            );
            return;
        }
        Err(e) => {
            let _ = tx.rollback().await;
            tracing::error!(
                entity_id,
                player_id,
                "RepairInventoryItems: cash update failed: {e}"
            );
            return;
        }
    };

    let result = sqlx::query(
        "UPDATE sgw_inventory SET durability = 100 \
         WHERE character_id = $1 AND item_id = ANY($2) \
           AND container_id = ANY($3) \
           AND stack_size = 1 \
           AND durability >= 0 AND durability < 100",
    )
    .bind(player_id)
    .bind(&item_ids)
    .bind(VENDOR_FILTER_BAGS.as_slice())
    .execute(&mut *tx)
    .await;

    match result {
        Ok(r) if r.rows_affected() == item_ids.len() as u64 => {}
        Ok(r) => {
            let _ = tx.rollback().await;
            tracing::warn!(
                entity_id,
                player_id,
                expected = item_ids.len(),
                updated = r.rows_affected(),
                "RepairInventoryItems: unexpected repair update count"
            );
            return;
        }
        Err(e) => {
            let _ = tx.rollback().await;
            tracing::error!(
                entity_id,
                player_id,
                "RepairInventoryItems: update failed: {e}"
            );
            return;
        }
    }

    if let Err(e) = tx.commit().await {
        tracing::error!(
            entity_id,
            player_id,
            "RepairInventoryItems: commit failed: {e}"
        );
        return;
    }

    send_cash_changed_to_client(entity_id, new_cash_total, socket, connected, entity_to_addr).await;
    let total_items = send_full_inventory_update(
        entity_id,
        player_id,
        pool,
        socket,
        connected,
        entity_to_addr,
    )
    .await;
    let store_updates: Vec<StoreItemCostUpdate> = item_ids
        .iter()
        .map(|item_id| StoreItemCostUpdate {
            item_id: *item_id,
            sell_price: 0,
            repair_price: 0,
            recharge_price: 0,
        })
        .collect();
    send_store_update_to_client(entity_id, &store_updates, socket, connected, entity_to_addr).await;

    tracing::debug!(
        entity_id,
        player_id,
        vendor_template_id,
        item_count = item_ids.len(),
        total_cost,
        total_items,
        "Vendor repair completed"
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::require_db_or_skip;

    /// Sentinel base for paid-repair tests. Distinct from prior live-DB sentinels:
    /// outbox (0x7000_0000), grant_cash (+0x100), move (+0x200), grant_item (+0x300),
    /// missions (+0x400), mail (+0x500), vendor/repair free path (+0x600).
    const TEST_BASE: i32 = 0x7000_0700;

    /// Vendor template seeded in resources.entity_templates with a populated
    /// `repair_item_list` (verified via `SELECT template_id FROM
    /// resources.entity_templates WHERE repair_item_list IS NOT NULL`).
    /// design_id=21 is one of three repairable items keyed off this template;
    /// its row in resources.item_list_items has naquadah=1000, so a
    /// 50%-durability item costs `(1000 * (100 - 50)) / 100 = 500`.
    const SEEDED_REPAIR_VENDOR_TEMPLATE_ID: i32 = 25;
    const REPAIRABLE_TYPE_ID: i32 = 21;
    const REPAIRABLE_TYPE_PRICE: i32 = 1_000;

    async fn cleanup(pool: &PgPool, account_id: i32, player_id: i32) {
        let _ = sqlx::query("DELETE FROM sgw_inventory WHERE character_id = $1")
            .bind(player_id).execute(pool).await;
        let _ = sqlx::query("DELETE FROM account WHERE account_id = $1")
            .bind(account_id).execute(pool).await;
    }

    async fn insert_account_and_player(
        pool: &PgPool, account_id: i32, player_id: i32, naquadah: i32,
    ) {
        sqlx::query(
            "INSERT INTO account (account_id, account_name, password) \
             VALUES ($1, $2, '')",
        )
        .bind(account_id).bind(format!("paid-repair-{account_id}"))
        .execute(pool).await.expect("insert account");

        sqlx::query(
            "INSERT INTO sgw_player (\
                account_id, player_id, level, alignment, archetype, gender, \
                player_name, extra_name, world_location, bodyset, \
                pos_x, pos_y, pos_z, skin_color_id, naquadah\
             ) VALUES ($1, $2, 1, 0, 1, 1, $3, '', 'CombatSim', 'BS_HumanMale.BS_HumanMale', \
                       0.0, 0.0, 0.0, 0, $4)",
        )
        .bind(account_id).bind(player_id).bind(format!("test-{player_id}")).bind(naquadah)
        .execute(pool).await.expect("insert player");
    }

    async fn insert_damaged_item(
        pool: &PgPool, player_id: i32, type_id: i32, slot_id: i32, durability: i32,
    ) -> i32 {
        sqlx::query_scalar(
            "INSERT INTO sgw_inventory \
                (character_id, type_id, stack_size, slot_id, container_id, \
                 bound, durability, charges) \
             VALUES ($1, $2, 1, $3, 1, false, $4, 0) RETURNING item_id",
        )
        .bind(player_id).bind(type_id).bind(slot_id).bind(durability)
        .fetch_one(pool).await.expect("insert inventory row")
    }

    async fn read_durability_and_naquadah(
        pool: &PgPool, player_id: i32, item_id: i32,
    ) -> (i32, i32) {
        let durability: i32 = sqlx::query_scalar(
            "SELECT durability FROM sgw_inventory \
             WHERE character_id = $1 AND item_id = $2",
        )
        .bind(player_id).bind(item_id).fetch_one(pool).await.unwrap();
        let naquadah: i32 = sqlx::query_scalar(
            "SELECT naquadah FROM sgw_player WHERE player_id = $1",
        )
        .bind(player_id).fetch_one(pool).await.unwrap();
        (durability, naquadah)
    }

    fn make_state(entity_id: u32) -> (
        Arc<UdpSocket>,
        Arc<Mutex<HashMap<u32, SocketAddr>>>,
        Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    ) {
        let std_sock = std::net::UdpSocket::bind("127.0.0.1:0").expect("bind UDP");
        std_sock.set_nonblocking(true).unwrap();
        let socket = Arc::new(UdpSocket::from_std(std_sock).expect("from_std"));
        let fake_addr: SocketAddr = "127.0.0.1:65535".parse().unwrap();
        let entity_to_addr = Arc::new(Mutex::new({
            let mut m = HashMap::new();
            m.insert(entity_id, fake_addr);
            m
        }));
        let connected = Arc::new(Mutex::new(HashMap::new()));
        (socket, entity_to_addr, connected)
    }

    /// Happy path: vendor charges naquadah and restores durability to 100.
    /// Pins the cost formula: a 50%-durability item with naquadah=1000 costs
    /// (1000 * (100 - 50)) / 100 = 500.
    #[tokio::test]
    async fn happy_path_charges_balance_and_restores_durability() {
        let pool = require_db_or_skip!();
        let account_id = TEST_BASE;
        let player_id = TEST_BASE + 1;
        cleanup(&pool, account_id, player_id).await;
        insert_account_and_player(&pool, account_id, player_id, 2_000).await;
        let item = insert_damaged_item(&pool, player_id, REPAIRABLE_TYPE_ID, 0, 50).await;

        let (socket, e2a, conn) = make_state(0x7000_0701);
        let db_pool = Some(Arc::new(pool.clone()));

        handle_paid_repair_inventory_items(
            0x7000_0701, player_id, vec![item], SEEDED_REPAIR_VENDOR_TEMPLATE_ID,
            &db_pool, &socket, &conn, &e2a,
        ).await;

        let expected_cost = REPAIRABLE_TYPE_PRICE * (100 - 50) / 100;
        let (durability, naquadah) = read_durability_and_naquadah(&pool, player_id, item).await;
        assert_eq!(durability, 100, "repair must restore durability to 100");
        assert_eq!(
            naquadah, 2_000 - expected_cost,
            "balance must drop by exactly the computed cost ({expected_cost})",
        );

        cleanup(&pool, account_id, player_id).await;
    }

    /// Insufficient naquadah path: tx must roll back so neither durability
    /// nor balance change. Pre-fix bug would have left durability at 100
    /// while the cash UPDATE silently failed (or vice versa).
    #[tokio::test]
    async fn insufficient_naquadah_rolls_back_durability_and_balance() {
        let pool = require_db_or_skip!();
        let account_id = TEST_BASE + 100;
        let player_id = TEST_BASE + 101;
        cleanup(&pool, account_id, player_id).await;
        // 100 naquadah but a 50%-durability item costs 500 — insufficient.
        insert_account_and_player(&pool, account_id, player_id, 100).await;
        let item = insert_damaged_item(&pool, player_id, REPAIRABLE_TYPE_ID, 0, 50).await;

        let (socket, e2a, conn) = make_state(0x7000_0702);
        let db_pool = Some(Arc::new(pool.clone()));

        handle_paid_repair_inventory_items(
            0x7000_0702, player_id, vec![item], SEEDED_REPAIR_VENDOR_TEMPLATE_ID,
            &db_pool, &socket, &conn, &e2a,
        ).await;

        let (durability, naquadah) = read_durability_and_naquadah(&pool, player_id, item).await;
        assert_eq!(durability, 50, "durability must stay at 50 — repair was rejected");
        assert_eq!(naquadah, 100, "balance must stay at 100 — no cash drain");

        cleanup(&pool, account_id, player_id).await;
    }

    /// Unknown vendor_template_id: load_vendor_template_lists returns None
    /// and the function bails before any DB writes. Verifies via the
    /// invariant "no DB changes" rather than log inspection.
    #[tokio::test]
    async fn unknown_vendor_template_drops_request() {
        let pool = require_db_or_skip!();
        let account_id = TEST_BASE + 200;
        let player_id = TEST_BASE + 201;
        cleanup(&pool, account_id, player_id).await;
        insert_account_and_player(&pool, account_id, player_id, 2_000).await;
        let item = insert_damaged_item(&pool, player_id, REPAIRABLE_TYPE_ID, 0, 50).await;

        let (socket, e2a, conn) = make_state(0x7000_0703);
        let db_pool = Some(Arc::new(pool.clone()));

        // template_id = 999_999_999 is not in resources.entity_templates.
        handle_paid_repair_inventory_items(
            0x7000_0703, player_id, vec![item], 999_999_999,
            &db_pool, &socket, &conn, &e2a,
        ).await;

        let (durability, naquadah) = read_durability_and_naquadah(&pool, player_id, item).await;
        assert_eq!(durability, 50, "no repair when vendor template doesn't exist");
        assert_eq!(naquadah, 2_000, "no charge when vendor template doesn't exist");

        cleanup(&pool, account_id, player_id).await;
    }
}