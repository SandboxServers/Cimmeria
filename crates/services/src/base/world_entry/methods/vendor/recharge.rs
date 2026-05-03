use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use sqlx::PgPool;
use tokio::net::UdpSocket;

use super::super::super::super::ConnectedClientState;
use super::super::inventory::core::send_full_inventory_update;
use super::super::inventory::grant::normalize_item_ids;

/// Containers that can be operated on by the vendor stack — main bag, bandolier,
/// equipment slots, and quick bars. Bank, mail attachments, and loot are excluded.
use super::VENDOR_FILTER_BAGS;

pub async fn handle_recharge_inventory_items(
    entity_id: u32,
    player_id: i32,
    item_ids: Vec<i32>,
    vendor_template_id: Option<i32>,
    db_pool: &Option<Arc<PgPool>>,
    socket: &Arc<UdpSocket>,
    connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    entity_to_addr: &Arc<Mutex<HashMap<u32, SocketAddr>>>,
) {
    if let Some(vendor_template_id) = vendor_template_id {
        super::paid_recharge::handle_paid_recharge_inventory_items(
            entity_id,
            player_id,
            item_ids,
            vendor_template_id,
            db_pool,
            socket,
            connected,
            entity_to_addr,
        )
        .await;
        return;
    }

    let pool = match db_pool {
        Some(p) => p,
        None => {
            tracing::debug!(player_id, "RechargeInventoryItems: no DB pool");
            return;
        }
    };

    let item_ids = normalize_item_ids(item_ids);
    if item_ids.is_empty() {
        tracing::debug!(
            entity_id,
            player_id,
            "RechargeInventoryItems: empty item list"
        );
        return;
    }

    // Wrap in a transaction so the matching rows can be locked before the
    // UPDATE — matches the explicit FOR UPDATE used by the paid recharge path
    // and prevents racing with concurrent sell/move operations on the same
    // items.
    let mut tx = match pool.begin().await {
        Ok(tx) => tx,
        Err(e) => {
            tracing::error!(
                entity_id,
                player_id,
                "RechargeInventoryItems: begin tx failed: {e}"
            );
            return;
        }
    };

    // Lock candidate rows up-front so the UPDATE below sees a stable snapshot.
    if let Err(e) = sqlx::query(
        "SELECT 1 FROM sgw_inventory inv \
         JOIN resources.items ri ON ri.item_id = inv.type_id \
         WHERE inv.character_id = $1 \
           AND inv.item_id = ANY($2) \
           AND inv.container_id = ANY($3) \
           AND inv.stack_size = 1 \
           AND ri.charges > 0 \
           AND inv.charges < ri.charges \
         FOR UPDATE OF inv",
    )
    .bind(player_id)
    .bind(&item_ids)
    .bind(VENDOR_FILTER_BAGS.as_slice())
    .execute(&mut *tx)
    .await
    {
        let _ = tx.rollback().await;
        tracing::error!(
            entity_id,
            player_id,
            "RechargeInventoryItems: lock query failed: {e}"
        );
        return;
    }

    let result = sqlx::query(
        "UPDATE sgw_inventory inv \
         SET charges = ri.charges \
         FROM resources.items ri \
         WHERE inv.character_id = $1 \
           AND inv.item_id = ANY($2) \
           AND inv.type_id = ri.item_id \
           AND inv.container_id = ANY($3) \
           AND inv.stack_size = 1 \
           AND ri.charges > 0 \
           AND inv.charges < ri.charges",
    )
    .bind(player_id)
    .bind(&item_ids)
    .bind(VENDOR_FILTER_BAGS.as_slice())
    .execute(&mut *tx)
    .await;

    let recharged = match result {
        Ok(r) => r.rows_affected(),
        Err(e) => {
            let _ = tx.rollback().await;
            tracing::error!(
                entity_id,
                player_id,
                item_count = item_ids.len(),
                "RechargeInventoryItems: update failed: {e}"
            );
            return;
        }
    };

    if let Err(e) = tx.commit().await {
        tracing::error!(
            entity_id,
            player_id,
            "RechargeInventoryItems: commit failed: {e}"
        );
        return;
    }

    if recharged > 0 {
        let total_items = send_full_inventory_update(
            entity_id,
            player_id,
            pool,
            socket,
            connected,
            entity_to_addr,
        )
        .await;
        tracing::debug!(
            entity_id,
            player_id,
            item_count = item_ids.len(),
            recharged,
            total_items,
            "Inventory items recharged"
        );
    } else {
        tracing::debug!(
            entity_id,
            player_id,
            item_count = item_ids.len(),
            "RechargeInventoryItems: no rechargeable items changed"
        );
    }
}

#[cfg(test)]
mod free_recharge_tests {
    //! Live-DB integration tests for the free-recharge branch
    //! (`vendor_template_id = None`) of handle_recharge_inventory_items.
    //!
    //! Skip cleanly when DATABASE_URL is unset.
    //!
    //! The seeded `resources.items` has zero rows with `charges > 0`,
    //! so a meaningful happy-path test has to insert dedicated test
    //! fixture data into `resources.items` (issue #79 explicitly
    //! sanctions this option). Each test inserts a synthetic row with
    //! a sentinel `item_id`, runs the handler, and cleans up — order
    //! matters because `sgw_inventory.type_id` FKs into
    //! `resources.items` with ON DELETE RESTRICT.
    //!
    //! The paid branch is covered by `paid_recharge::recharge_tests`.

    use super::*;
    use crate::test_support::require_db_or_skip;

    /// Sentinel base. Steps past the highest live-DB sentinel reserved
    /// elsewhere in the crate.
    const TEST_BASE: i32 = 0x7000_1300;

    /// Synthetic resources.items.item_id used by these tests. Picked
    /// far above the seed sequence's high water mark and outside any
    /// live test sentinel range so it can't collide.
    const SYNTH_RECHARGEABLE_TYPE_ID: i32 = 0x7FFF_AAAA;
    /// Base charges of the synthetic item. Inventory rows start at
    /// `charges = 0` so the recharge UPDATE has work to do.
    const SYNTH_BASE_CHARGES: i32 = 10;

    const INV_MAIN: i32 = 1;

    async fn cleanup(pool: &PgPool, account_id: i32, player_id: i32) {
        let _ = sqlx::query("DELETE FROM sgw_inventory WHERE character_id = $1")
            .bind(player_id)
            .execute(pool)
            .await;
        let _ = sqlx::query("DELETE FROM account WHERE account_id = $1")
            .bind(account_id)
            .execute(pool)
            .await;
        // resources.items must come AFTER any referencing sgw_inventory
        // rows; ON DELETE RESTRICT enforces that order.
        let _ = sqlx::query("DELETE FROM resources.items WHERE item_id = $1")
            .bind(SYNTH_RECHARGEABLE_TYPE_ID)
            .execute(pool)
            .await;
    }

    /// Insert a synthetic resources.items row keyed by
    /// `SYNTH_RECHARGEABLE_TYPE_ID` with `charges > 0` so the
    /// `WHERE ri.charges > 0 AND inv.charges < ri.charges` filter
    /// matches inventory rows we hang off it.
    async fn insert_synthetic_rechargeable_design(pool: &PgPool) {
        sqlx::query(
            "INSERT INTO resources.items (\
                item_id, description, name, quality_id, tech_comp, tier, \
                max_stack_size, charges \
             ) VALUES ($1, '', $2, 'ITEM_QUALITY_Normal', 0, 1, 1, $3)",
        )
        .bind(SYNTH_RECHARGEABLE_TYPE_ID)
        .bind(format!("synth-rechargeable-{SYNTH_RECHARGEABLE_TYPE_ID}"))
        .bind(SYNTH_BASE_CHARGES)
        .execute(pool)
        .await
        .expect("insert synthetic rechargeable design");
    }

    async fn insert_account_and_player(pool: &PgPool, account_id: i32, player_id: i32) {
        sqlx::query(
            "INSERT INTO account (account_id, account_name, password) \
             VALUES ($1, $2, '')",
        )
        .bind(account_id)
        .bind(format!("recharge-{account_id}"))
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

    async fn insert_inventory_row(
        pool: &PgPool,
        player_id: i32,
        type_id: i32,
        slot_id: i32,
        charges: i32,
    ) -> i32 {
        sqlx::query_scalar(
            "INSERT INTO sgw_inventory \
                (character_id, type_id, stack_size, slot_id, container_id, \
                 bound, durability, charges) \
             VALUES ($1, $2, 1, $3, $4, false, 100, $5) \
             RETURNING item_id",
        )
        .bind(player_id)
        .bind(type_id)
        .bind(slot_id)
        .bind(INV_MAIN)
        .bind(charges)
        .fetch_one(pool)
        .await
        .expect("insert inventory row")
    }

    async fn charges_of(pool: &PgPool, item_id: i32) -> i32 {
        sqlx::query_scalar("SELECT charges FROM sgw_inventory WHERE item_id = $1")
            .bind(item_id)
            .fetch_one(pool)
            .await
            .expect("charges_of")
    }

    fn make_state(
        entity_id: u32,
    ) -> (
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

    /// Free-recharge happy path: a depleted item (charges=0) of a
    /// design with `ri.charges > 0` gets restored to `ri.charges`.
    /// The synthetic resources.items row is what makes this branch
    /// reachable at all — the seeded set has nothing rechargeable.
    #[tokio::test]
    async fn free_recharge_restores_charges_to_full() {
        let pool = require_db_or_skip!();
        let account_id = TEST_BASE;
        let player_id = TEST_BASE + 1;
        cleanup(&pool, account_id, player_id).await;
        insert_synthetic_rechargeable_design(&pool).await;
        insert_account_and_player(&pool, account_id, player_id).await;
        let item = insert_inventory_row(&pool, player_id, SYNTH_RECHARGEABLE_TYPE_ID, 0, 0).await;

        let (socket, e2a, conn) = make_state(0x7000_1301);
        let db_pool = Some(Arc::new(pool.clone()));

        handle_recharge_inventory_items(
            0x7000_1301,
            player_id,
            vec![item],
            None, // free-recharge branch
            &db_pool,
            &socket,
            &conn,
            &e2a,
        )
        .await;

        assert_eq!(
            charges_of(&pool, item).await,
            SYNTH_BASE_CHARGES,
            "free recharge must restore charges to ri.charges",
        );

        cleanup(&pool, account_id, player_id).await;
    }

    /// Already-full items (`inv.charges = ri.charges`) are excluded
    /// by the `inv.charges < ri.charges` predicate. The function must
    /// NOT emit `ON_UPDATE_ITEM` for them — the `if recharged > 0`
    /// short-circuit at the bottom of the handler depends on the
    /// UPDATE returning `rows_affected = 0`.
    #[tokio::test]
    async fn free_recharge_skips_already_full_items() {
        let pool = require_db_or_skip!();
        let account_id = TEST_BASE + 100;
        let player_id = TEST_BASE + 101;
        cleanup(&pool, account_id, player_id).await;
        insert_synthetic_rechargeable_design(&pool).await;
        insert_account_and_player(&pool, account_id, player_id).await;
        // Insert at the synthetic design's full charge value — must
        // be excluded by `inv.charges < ri.charges`.
        let item = insert_inventory_row(
            &pool,
            player_id,
            SYNTH_RECHARGEABLE_TYPE_ID,
            0,
            SYNTH_BASE_CHARGES,
        )
        .await;

        let (socket, e2a, conn) = make_state(0x7000_1311);
        let db_pool = Some(Arc::new(pool.clone()));

        handle_recharge_inventory_items(
            0x7000_1311,
            player_id,
            vec![item],
            None,
            &db_pool,
            &socket,
            &conn,
            &e2a,
        )
        .await;

        assert_eq!(
            charges_of(&pool, item).await,
            SYNTH_BASE_CHARGES,
            "already-full item must not be touched (UPDATE excluded by < ri.charges)",
        );

        cleanup(&pool, account_id, player_id).await;
    }
}
