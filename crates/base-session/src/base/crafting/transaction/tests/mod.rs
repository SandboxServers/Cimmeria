//! Live-DB tests of the crafting transaction and of an induction that runs
//! it. Each test builds its own player from the `0x7000_CF40` sentinel
//! block, runs against real `resources.items` rows, asserts the database
//! and the packets sent to the player, and cleans up by exact id.

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use cimmeria_mercury::transport::Transport;
use sqlx::PgPool;

use super::*;
use crate::base::crafting::telemetry::JobIds;
use crate::base::crafting::test_packets::{decode_all, MethodCall};
use crate::test_support::{test_default_connected_client_state, TestTransport};

mod concurrency;
mod consumption;
mod induction;
mod named;
mod notify;
mod refusals;

/// Inside crafting's `0x7000_Cxxx` block, clear of the persistence
/// (`0x7000_C000`..), GM-grant (`0x7000_CC00`..), login-sync, spend and
/// station (`0x7000_CD00`..`0x7000_CE1F`) and world-entry (`0x7000_CF00`,
/// `0x7000_CF01`) tests. Each test takes a 4-id slot (account, player,
/// entity), 24 slots in all: `0x7000_CF40..=0x7000_CF9F`.
const TEST_BASE: i32 = 0x7000_CF40;

/// Dross Kit: Tier 2. A crafting component: `{17,15}`, stacks to 2.
const COMPONENT: i32 = 8891;
/// T1 Cell (Bio-Medical). `{17,15}`, does not stack; used to fill bags.
const FILLER: i32 = 5189;
/// Protein Complex (Bio-Medical). `{17,15}`, does not stack: a product
/// whose first listed container is the bank.
const BANK_FIRST_PRODUCT: i32 = 5188;
/// Health Slappack TC1. `{1,17}`, stacks to 10.
const STACKABLE_PRODUCT: i32 = 2893;
/// Ambernol Vial. `{2}`: mission bag only.
const MISSION_ONLY: i32 = 19;

const INV_MAIN: i32 = 1;
const INV_CRAFTING: i32 = 15;
const INV_BANK: i32 = 17;

struct Fixture {
    pool: PgPool,
    account_id: i32,
    player_id: i32,
    entity_id: u32,
    env: InductionEnv,
    transport: Arc<TestTransport>,
    addr: SocketAddr,
}

impl Fixture {
    /// A fresh player in sentinel slot `slot`, connected to a test
    /// transport so every packet sent to it can be decoded.
    async fn new(pool: &PgPool, slot: i32) -> Self {
        let account_id = TEST_BASE + slot * 4;
        let player_id = account_id + 1;
        let entity_id = (account_id + 2) as u32;
        let transport = Arc::new(TestTransport::new());
        let dyn_transport: Arc<dyn Transport> = transport.clone();
        let addr: SocketAddr = format!("127.0.0.1:{}", 56000 + slot).parse().unwrap();
        let mut state = test_default_connected_client_state();
        state.player_entity_id = Some(entity_id);
        state.active_player_id = Some(player_id);
        state.account_id = account_id as u32;
        state.world_name = Some("CombatSim".to_string());
        let env = InductionEnv {
            db_pool: Some(Arc::new(pool.clone())),
            cell_tx: None,
            transport: dyn_transport,
            connected: Arc::new(Mutex::new(HashMap::from([(addr, state)]))),
            entity_to_addr: Arc::new(Mutex::new(HashMap::from([(entity_id, addr)]))),
        };
        let f = Self {
            pool: pool.clone(),
            account_id,
            player_id,
            entity_id,
            env,
            transport,
            addr,
        };
        f.cleanup().await;
        sqlx::query("INSERT INTO account (account_id, account_name, password) VALUES ($1, $2, '')")
            .bind(account_id)
            .bind(format!("craft-tx-{account_id}"))
            .execute(pool)
            .await
            .expect("insert account");
        sqlx::query(
            "INSERT INTO sgw_player (\
                account_id, player_id, level, alignment, archetype, gender, \
                player_name, extra_name, world_location, bodyset, \
                pos_x, pos_y, pos_z, skin_color_id\
             ) VALUES ($1, $2, 1, 0, 1, 1, $3, '', 'CombatSim', 'BS_HumanMale.BS_HumanMale', \
                       0.0, 0.0, 0.0, 0)",
        )
        .bind(account_id)
        .bind(player_id)
        .bind(format!("craft-tx-{player_id}"))
        .execute(pool)
        .await
        .expect("insert player");
        f
    }

    async fn cleanup(&self) {
        let _ = sqlx::query("DELETE FROM cell_event_outbox WHERE entity_id = $1")
            .bind(self.entity_id as i32)
            .execute(&self.pool)
            .await;
        for sql in [
            "DELETE FROM sgw_inventory WHERE character_id = $1",
            "DELETE FROM sgw_player_discipline_expertise WHERE player_id = $1",
        ] {
            let _ = sqlx::query(sql)
                .bind(self.player_id)
                .execute(&self.pool)
                .await;
        }
        let _ = sqlx::query("DELETE FROM sgw_player WHERE player_id = $1")
            .bind(self.player_id)
            .execute(&self.pool)
            .await;
        let _ = sqlx::query("DELETE FROM account WHERE account_id = $1")
            .bind(self.account_id)
            .execute(&self.pool)
            .await;
    }

    /// Insert a stack; returns its instance id.
    async fn stack(&self, type_id: i32, container_id: i32, slot_id: i32, size: i32) -> i32 {
        insert_stack(
            &self.pool,
            self.player_id,
            type_id,
            container_id,
            slot_id,
            size,
        )
        .await
    }

    /// `(stack_size, container_id)` of an instance, `None` once deleted.
    async fn row(&self, item_id: i32) -> Option<(i32, i32)> {
        sqlx::query_as("SELECT stack_size, container_id FROM sgw_inventory WHERE item_id = $1")
            .bind(item_id)
            .fetch_optional(&self.pool)
            .await
            .expect("read row")
    }

    /// `(item_id, stack_size, container_id, slot_id)` of every stack of a
    /// type the player holds.
    async fn stacks_of(&self, type_id: i32) -> Vec<(i32, i32, i32, i32)> {
        sqlx::query_as(
            "SELECT item_id, stack_size, container_id, slot_id FROM sgw_inventory \
             WHERE character_id = $1 AND type_id = $2 ORDER BY container_id, slot_id",
        )
        .bind(self.player_id)
        .bind(type_id)
        .fetch_all(&self.pool)
        .await
        .expect("read stacks")
    }

    async fn outbox_rows(&self) -> i64 {
        sqlx::query_scalar("SELECT count(*) FROM cell_event_outbox WHERE entity_id = $1")
            .bind(self.entity_id as i32)
            .fetch_one(&self.pool)
            .await
            .expect("count outbox")
    }

    async fn apply(&self, plan: &CraftTransaction) -> Result<CraftApplied, CraftReject> {
        apply_craft_transaction(&self.env, &self.ids(), plan).await
    }

    /// The identity the transaction logs with; job 0 outside an induction.
    fn ids(&self) -> JobIds {
        JobIds {
            job_id: 0,
            verb: "test_plan",
            account_id: self.account_id as u32,
            player_id: self.player_id,
            entity_id: self.entity_id,
            gm_entity_id: None,
        }
    }

    fn calls(&self) -> Vec<MethodCall> {
        decode_all(&self.transport.filter_to(self.addr))
    }
}

async fn insert_stack(
    pool: &PgPool,
    player_id: i32,
    type_id: i32,
    container_id: i32,
    slot_id: i32,
    size: i32,
) -> i32 {
    sqlx::query_scalar(
        "INSERT INTO sgw_inventory \
            (character_id, type_id, stack_size, slot_id, container_id, bound, durability, charges) \
         VALUES ($1, $2, $3, $4, $5, false, 100, 0) RETURNING item_id",
    )
    .bind(player_id)
    .bind(type_id)
    .bind(size)
    .bind(slot_id)
    .bind(container_id)
    .fetch_one(pool)
    .await
    .expect("insert stack")
}

/// The tests lean on these seed rows; if one changes, fail here with a
/// clear message rather than with a confusing placement.
async fn assert_seed_shape(pool: &PgPool) {
    let expect = [
        (COMPONENT, vec![17, 15], 2),
        (FILLER, vec![17, 15], 1),
        (BANK_FIRST_PRODUCT, vec![17, 15], 1),
        (STACKABLE_PRODUCT, vec![1, 17], 10),
        (MISSION_ONLY, vec![2], 1),
    ];
    for (item_id, sets, max_stack) in expect {
        let row: (Vec<i32>, i32) = sqlx::query_as(
            "SELECT container_sets, max_stack_size FROM resources.items WHERE item_id = $1",
        )
        .bind(item_id)
        .fetch_one(pool)
        .await
        .unwrap_or_else(|e| panic!("seed item {item_id} missing: {e}"));
        assert_eq!(row, (sets, max_stack), "seed shape of item {item_id}");
    }
}
