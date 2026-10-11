use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use cimmeria_mercury::transport::Transport;
use sqlx::PgPool;

use super::super::super::super::ConnectedClientState;
/// `rechargeItems` (cell method 82): recharge the items at the vendor the
/// player has open, for payment.
///
/// `vendor_template_id` is always the server's (the cell takes it from the
/// player's vendor session). There is no free path: until 2026-10-10 a
/// request without a template set `charges` to full with no price, no debit
/// and no vendor check (security audit CAT-E-01).
pub async fn handle_recharge_inventory_items(
    entity_id: u32,
    player_id: i32,
    item_ids: Vec<i32>,
    vendor_template_id: i32,
    db_pool: &Option<Arc<PgPool>>,
    transport: &Arc<dyn Transport>,
    connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    entity_to_addr: &Arc<Mutex<HashMap<u32, SocketAddr>>>,
) {
    super::paid_recharge::handle_paid_recharge_inventory_items(
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
}
#[cfg(test)]
mod refusal_tests {
    //! Live-DB guard that `rechargeItems` has no free branch (CAT-E-01).
    //!
    //! Skip cleanly when DATABASE_URL is unset.
    //!
    //! The seeded `resources.items` has zero rows with `charges > 0`,
    //! so a meaningful happy-path test has to insert dedicated fixture
    //! data into `resources.items` keyed at a sentinel `item_id` far
    //! outside the seeded sequence. The fixture row is inserted
    //! idempotently (`ON CONFLICT (item_id) DO NOTHING`) and is NOT
    //! deleted by per-test cleanup — concurrent tests would otherwise
    //! race on PK-conflict / FK-violation between insert and cleanup.
    //!
    //! The paid branch is covered by `paid_recharge::tests`.

    use super::*;
    use crate::test_support::require_db_or_skip;
    use crate::test_support::TestTransport;

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
        // Per-test rows only. The synthetic `resources.items` row at
        // `SYNTH_RECHARGEABLE_TYPE_ID` is intentionally NOT deleted:
        // `cargo test` runs tests in parallel within a binary, and
        // deleting a shared FK target between tests' insert/handler
        // calls causes spurious failures. The sentinel id sits well
        // outside any production data range, so leaving it in place
        // is safe.
        let _ = sqlx::query("DELETE FROM sgw_inventory WHERE character_id = $1")
            .bind(player_id)
            .execute(pool)
            .await;
        let _ = sqlx::query("DELETE FROM account WHERE account_id = $1")
            .bind(account_id)
            .execute(pool)
            .await;
    }

    /// Insert a synthetic resources.items row keyed by
    /// `SYNTH_RECHARGEABLE_TYPE_ID` with `charges > 0` so the
    /// `WHERE ri.charges > 0 AND inv.charges < ri.charges` filter
    /// matches inventory rows we hang off it. Idempotent so
    /// concurrent tests don't fight over the PK.
    async fn insert_synthetic_rechargeable_design(pool: &PgPool) {
        sqlx::query(
            "INSERT INTO resources.items (\
                item_id, description, name, quality_id, tech_comp, tier, \
                max_stack_size, charges \
             ) VALUES ($1, '', $2, 'ITEM_QUALITY_Normal', 0, 1, 1, $3) \
             ON CONFLICT (item_id) DO NOTHING",
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

    async fn naquadah_of(pool: &PgPool, player_id: i32) -> i32 {
        sqlx::query_scalar("SELECT naquadah FROM sgw_player WHERE player_id = $1")
            .bind(player_id)
            .fetch_one(pool)
            .await
            .expect("naquadah_of")
    }

    /// A recharge the vendor cannot price (a vendor with no recharge list)
    /// leaves charges and cash alone. Not the CAT-E-01 guard (it passes a
    /// template, so the old code took the paid path too); see the note on
    /// `repair::tests::live_db_repair_at_a_vendor_without_a_repair_list_is_refused`.
    #[tokio::test]
    async fn live_db_recharge_at_a_vendor_without_a_recharge_list_is_refused() {
        let pool = require_db_or_skip!();
        let account_id = TEST_BASE;
        let player_id = TEST_BASE + 1;
        cleanup(&pool, account_id, player_id).await;
        insert_synthetic_rechargeable_design(&pool).await;
        insert_account_and_player(&pool, account_id, player_id).await;
        sqlx::query("UPDATE sgw_player SET naquadah = 1000 WHERE player_id = $1")
            .bind(player_id)
            .execute(&pool)
            .await
            .expect("fund player");
        let item = insert_inventory_row(&pool, player_id, SYNTH_RECHARGEABLE_TYPE_ID, 0, 0).await;
        let template: i32 = sqlx::query_scalar(
            "SELECT template_id FROM resources.entity_templates \
             WHERE recharge_item_list IS NULL ORDER BY template_id LIMIT 1",
        )
        .fetch_one(&pool)
        .await
        .expect("a template without a recharge list");

        let (transport, e2a, conn) = make_state(0x7000_1301);
        let db_pool = Some(Arc::new(pool.clone()));

        handle_recharge_inventory_items(
            0x7000_1301,
            player_id,
            vec![item],
            template,
            &db_pool,
            &transport,
            &conn,
            &e2a,
        )
        .await;

        assert_eq!(
            charges_of(&pool, item).await,
            0,
            "a recharge no vendor priced must not restore charges"
        );
        assert_eq!(naquadah_of(&pool, player_id).await, 1000, "no debit");

        cleanup(&pool, account_id, player_id).await;
    }
}
