//! Live-DB fixture for the alloy verb: a player from crafting's alloy
//! sentinel block, connected to a test transport, with its own induction
//! engine driven by hand.

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use cimmeria_mercury::transport::Transport;
use sqlx::PgPool;
use tokio::sync::mpsc;

use super::super::{handle_alloy_in, AlloyRequest};
use crate::base::crafting::request::CraftCtx;
use crate::base::crafting::rng::ScriptedRng;
use crate::base::crafting::session::{CraftingSessions, InductionEnv, ManualScheduler};
use crate::base::crafting::test_packets::{decode_all, MethodCall};
use crate::base::ConnectedClientState;
use crate::cell::messages::BaseToCellMsg;
use crate::test_support::{test_default_connected_client_state, TestTransport};

/// The alloy tests' sentinel block, `0x7000_C300..=0x7000_C3FF`: one
/// 4-id slot (account, player, entity) per test.
pub(super) const TEST_BASE: i32 = 0x7000_C300;

pub(super) const INV_MAIN: i32 = 1;
pub(super) const INV_CRAFTING: i32 = 15;
pub(super) const INV_BANK: i32 = 17;

/// Blueprint 42 (Biomedical Engineering, discipline 21): 1x 5192 plus
/// tier-1 elementary components make 2x 5191.
pub(super) const ALLOY: i32 = 42;
/// Another alloy blueprint of discipline 21, left unlearned.
pub(super) const OTHER_ALLOY: i32 = 43;
/// Blueprint 25 makes Steel Plating: not an alloy.
pub(super) const CRAFT: i32 = 25;
pub(super) const DISCIPLINE: i32 = 21;

/// Cell (Bio-Medical): tier 2, blueprint 42's component.
pub(super) const COMPONENT: i32 = 5192;
/// Blend (Bio-Medical Alloy): tier 2, blueprint 42's product.
pub(super) const PRODUCT: i32 = 5191;
/// Tier-1 elementary components of each quality.
pub(super) const NORMAL: i32 = 5188;
pub(super) const GOOD: i32 = 5189;
pub(super) const GREAT: i32 = 5395;
pub(super) const FANTASTIC: i32 = 2891;
pub(super) const POOR: i32 = 2492;
/// Drug (Bio-Medical): tier 2, one tier too high to be elementary.
pub(super) const TIER_TWO: i32 = 5193;

pub(super) struct Fixture {
    pub(super) pool: PgPool,
    pub(super) account_id: i32,
    pub(super) player_id: i32,
    pub(super) entity_id: u32,
    db_pool: Option<Arc<PgPool>>,
    cell_tx: Option<mpsc::Sender<BaseToCellMsg>>,
    pub(super) transport: Arc<TestTransport>,
    dyn_transport: Arc<dyn Transport>,
    connected: Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    entity_to_addr: Arc<Mutex<HashMap<u32, SocketAddr>>>,
    addr: SocketAddr,
    pub(super) sessions: Arc<CraftingSessions>,
    scheduler: Arc<ManualScheduler>,
}

impl Fixture {
    /// A fresh player in slot `slot` who knows discipline 21 (expertise 1)
    /// and blueprints 42 and 25.
    pub(super) async fn new(pool: &PgPool, slot: i32) -> Self {
        assert!((0..64).contains(&slot), "slot {slot} outside the block");
        let account_id = TEST_BASE + slot * 4;
        let player_id = account_id + 1;
        let entity_id = (account_id + 2) as u32;
        let transport = Arc::new(TestTransport::new());
        let dyn_transport: Arc<dyn Transport> = transport.clone();
        let addr: SocketAddr = format!("127.0.0.1:{}", 56300 + slot).parse().unwrap();
        let mut state = test_default_connected_client_state();
        state.player_entity_id = Some(entity_id);
        state.active_player_id = Some(player_id);
        state.account_id = account_id as u32;
        state.world_name = Some("CombatSim".to_string());
        let scheduler = Arc::new(ManualScheduler::default());
        let sessions = Arc::new(CraftingSessions::new(
            Box::new(scheduler.clone()),
            Box::new(|| Box::new(ScriptedRng::new(vec![0.5]))),
        ));
        let f = Self {
            pool: pool.clone(),
            account_id,
            player_id,
            entity_id,
            db_pool: Some(Arc::new(pool.clone())),
            cell_tx: None,
            transport,
            dyn_transport,
            connected: Arc::new(Mutex::new(HashMap::from([(addr, state)]))),
            entity_to_addr: Arc::new(Mutex::new(HashMap::from([(entity_id, addr)]))),
            addr,
            sessions,
            scheduler,
        };
        f.cleanup().await;
        sqlx::query("INSERT INTO account (account_id, account_name, password) VALUES ($1, $2, '')")
            .bind(account_id)
            .bind(format!("craft-alloy-{account_id}"))
            .execute(pool)
            .await
            .expect("insert account");
        sqlx::query(
            "INSERT INTO sgw_player (\
                account_id, player_id, level, alignment, archetype, gender, \
                player_name, extra_name, world_location, bodyset, \
                pos_x, pos_y, pos_z, skin_color_id, discipline_ids, blueprint_ids\
             ) VALUES ($1, $2, 1, 0, 1, 1, $3, '', 'CombatSim', 'BS_HumanMale.BS_HumanMale', \
                       0.0, 0.0, 0.0, 0, $4, $5)",
        )
        .bind(account_id)
        .bind(player_id)
        .bind(format!("craft-alloy-{player_id}"))
        .bind(vec![DISCIPLINE])
        .bind(vec![ALLOY, CRAFT])
        .execute(pool)
        .await
        .expect("insert player");
        sqlx::query(
            "INSERT INTO sgw_player_discipline_expertise (player_id, discipline_id, expertise) \
             VALUES ($1, $2, 1)",
        )
        .bind(player_id)
        .bind(DISCIPLINE)
        .execute(pool)
        .await
        .expect("insert expertise");
        f
    }

    pub(super) async fn cleanup(&self) {
        let _ = sqlx::query("DELETE FROM cell_event_outbox WHERE entity_id = $1")
            .bind(self.entity_id as i32)
            .execute(&self.pool)
            .await;
        for sql in [
            "DELETE FROM sgw_inventory WHERE character_id = $1",
            "DELETE FROM sgw_player_discipline_expertise WHERE player_id = $1",
            "DELETE FROM sgw_player WHERE player_id = $1",
        ] {
            let _ = sqlx::query(sql)
                .bind(self.player_id)
                .execute(&self.pool)
                .await;
        }
        let _ = sqlx::query("DELETE FROM account WHERE account_id = $1")
            .bind(self.account_id)
            .execute(&self.pool)
            .await;
    }

    /// The dispatcher context the verb runs with.
    pub(super) fn ctx(&self) -> CraftCtx<'_> {
        CraftCtx {
            db_pool: &self.db_pool,
            cell_tx: &self.cell_tx,
            transport: &self.dyn_transport,
            connected: &self.connected,
            entity_to_addr: &self.entity_to_addr,
        }
    }

    fn env(&self) -> InductionEnv {
        InductionEnv::from_ctx(&self.ctx())
    }

    /// Insert a stack; returns its instance id.
    pub(super) async fn stack(
        &self,
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
        .bind(self.player_id)
        .bind(type_id)
        .bind(size)
        .bind(slot_id)
        .bind(container_id)
        .fetch_one(&self.pool)
        .await
        .expect("insert stack")
    }

    /// `n` one-item stacks of `type_id` in the main bag from slot `first`.
    pub(super) async fn singles(&self, type_id: i32, first: i32, n: i32) -> Vec<i32> {
        let mut ids = Vec::new();
        for k in 0..n {
            ids.push(self.stack(type_id, INV_MAIN, first + k, 1).await);
        }
        ids
    }

    /// Every `(item_id, type_id, stack_size, container_id)` the player
    /// holds, by instance id: the whole inventory, to prove a refusal
    /// changed nothing.
    pub(super) async fn inventory(&self) -> Vec<(i32, i32, i32, i32)> {
        sqlx::query_as(
            "SELECT item_id, type_id, stack_size, container_id FROM sgw_inventory \
             WHERE character_id = $1 ORDER BY item_id",
        )
        .bind(self.player_id)
        .fetch_all(&self.pool)
        .await
        .expect("read inventory")
    }

    pub(super) async fn expertise(&self) -> i32 {
        sqlx::query_scalar(
            "SELECT expertise FROM sgw_player_discipline_expertise \
             WHERE player_id = $1 AND discipline_id = $2",
        )
        .bind(self.player_id)
        .bind(DISCIPLINE)
        .fetch_one(&self.pool)
        .await
        .expect("read expertise")
    }

    /// Send an `alloying` request through the verb.
    pub(super) async fn alloy(&self, blueprint_id: i32, current: i32, lower: &[i32]) {
        let request = AlloyRequest {
            blueprint_id,
            current_tier_item_id: current,
            lower_tier_items: lower,
        };
        handle_alloy_in(
            &self.sessions,
            self.entity_id,
            self.player_id,
            request,
            &self.ctx(),
        )
        .await;
    }

    /// Run every induction whose wake-up is due; returns how many ran.
    pub(super) async fn finish_inductions(&self) -> usize {
        let env = self.env();
        let due = self.scheduler.take();
        for d in &due {
            self.sessions
                .expire_at(d.entity_id, d.job_id, d.deadline, &env)
                .await;
        }
        due.len()
    }

    pub(super) fn calls(&self) -> Vec<MethodCall> {
        decode_all(&self.transport.filter_to(self.addr))
    }
}

/// The tests lean on these seed rows; if one changes, fail here rather
/// than with a confusing refusal.
pub(super) async fn assert_seed_shape(pool: &PgPool) {
    let expect = [
        (COMPONENT, 2, "ITEM_QUALITY_Good", vec![17, 15]),
        (PRODUCT, 2, "ITEM_QUALITY_Great", vec![17, 15]),
        (NORMAL, 1, "ITEM_QUALITY_Normal", vec![17, 15]),
        (GOOD, 1, "ITEM_QUALITY_Good", vec![17, 15]),
        (GREAT, 1, "ITEM_QUALITY_Great", vec![17, 15]),
        (FANTASTIC, 1, "ITEM_QUALITY_Fantastic", vec![1, 7, 17]),
        (POOR, 1, "ITEM_QUALITY_Poor", vec![1, 17]),
        (TIER_TWO, 2, "ITEM_QUALITY_Good", vec![17, 15]),
    ];
    for (item_id, tier, quality, sets) in expect {
        let row: (i32, String, Vec<i32>) = sqlx::query_as(
            "SELECT tier, quality_id::text, container_sets FROM resources.items WHERE item_id = $1",
        )
        .bind(item_id)
        .fetch_one(pool)
        .await
        .unwrap_or_else(|e| panic!("seed item {item_id} missing: {e}"));
        assert_eq!(
            row,
            (tier, quality.to_string(), sets),
            "seed item {item_id}"
        );
    }
    let blueprint: (i32, bool, i32, i32, bool) = sqlx::query_as(
        "SELECT discipline_id, is_alloy, product_id, quantity, requires_elementary_components \
         FROM resources.blueprints WHERE blueprint_id = $1",
    )
    .bind(ALLOY)
    .fetch_one(pool)
    .await
    .expect("blueprint 42");
    assert_eq!(blueprint, (DISCIPLINE, true, PRODUCT, 2, true));
    let components: Vec<(i32, i32)> = sqlx::query_as(
        "SELECT item_id, quantity FROM resources.blueprints_components WHERE blueprint_id = $1",
    )
    .bind(ALLOY)
    .fetch_all(pool)
    .await
    .expect("blueprint 42 components");
    assert_eq!(components, vec![(COMPONENT, 1)]);
}
