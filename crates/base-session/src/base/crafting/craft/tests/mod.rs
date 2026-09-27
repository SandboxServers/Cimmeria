//! `craft` tests: the pure rules, then the verb end to end against the
//! seeded catalog and a real inventory, through the induction engine on a
//! manual clock.

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use cimmeria_mercury::transport::Transport;
use sqlx::PgPool;

use super::*;
use crate::base::crafting::request::CraftCtx;
use crate::base::crafting::rng::ScriptedRng;
use crate::base::crafting::session::{CraftingSessions, ManualScheduler};
use crate::base::crafting::test_packets::{decode_all, feedback_text, MethodCall};
use crate::base::crafting::test_players::OneSession;
use crate::base::ConnectedClientState;
use crate::test_support::{test_default_connected_client_state, TestTransport};

mod concurrency;
mod live;
mod refusals;
mod routing;
mod rules_tests;
mod telemetry;

/// Crafting's `0x7000_C100..=0x7000_C1FF` block, reserved for these
/// tests. Each test takes a 4-id slot (account, player, entity).
const TEST_BASE: i32 = 0x7000_C100;

/// Biomedical, the discipline of blueprints 412, 159, 21 and the alloy 42.
const DISCIPLINE: i32 = 21;
/// Titanium Plating: set 1 is 14 Steel Cores; set 2 is one Steel Core and
/// five Titanium Cores. Makes one 5401.
const BLUEPRINT: i32 = 412;
const STEEL_CORE: i32 = 5254;
const TITANIUM_CORE: i32 = 5256;
const TITANIUM_PLATING: i32 = 5401;
/// Ambernol Vial: a blueprint with no component set.
const NO_COMPONENTS: i32 = 21;
/// An alloy blueprint of discipline 21.
const ALLOY: i32 = 42;

const INV_MAIN: i32 = 1;
const INV_CRAFTING: i32 = 15;
const INV_BANK: i32 = 17;

struct Fixture {
    pool: PgPool,
    db_pool: Option<Arc<PgPool>>,
    account_id: i32,
    player_id: i32,
    entity_id: u32,
    typed: Arc<TestTransport>,
    transport: Arc<dyn Transport>,
    connected: Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    entity_to_addr: Arc<Mutex<HashMap<u32, SocketAddr>>>,
    addr: SocketAddr,
    sessions: Arc<CraftingSessions>,
    scheduler: Arc<ManualScheduler>,
}

impl Fixture {
    /// A fresh connected player in sentinel slot `slot` (0..64).
    async fn new(pool: &PgPool, slot: i32) -> Self {
        assert!((0..64).contains(&slot), "slot {slot} outside the block");
        let account_id = TEST_BASE + slot * 4;
        let player_id = account_id + 1;
        let entity_id = (account_id + 2) as u32;
        let addr: SocketAddr = format!("127.0.0.1:{}", 56100 + slot).parse().unwrap();
        let typed = Arc::new(TestTransport::new());
        let transport: Arc<dyn Transport> = typed.clone();
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
            db_pool: Some(Arc::new(pool.clone())),
            account_id,
            player_id,
            entity_id,
            typed,
            transport,
            connected: Arc::new(Mutex::new(HashMap::from([(addr, state)]))),
            entity_to_addr: Arc::new(Mutex::new(HashMap::from([(entity_id, addr)]))),
            addr,
            sessions,
            scheduler,
        };
        f.cleanup().await;
        sqlx::query("INSERT INTO account (account_id, account_name, password) VALUES ($1, $2, '')")
            .bind(account_id)
            .bind(format!("craft-cr07-{account_id}"))
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
        .bind(format!("craft-cr07-{player_id}"))
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

    /// Know `disciplines` (`(id, expertise)`) and `blueprints`.
    async fn know(&self, disciplines: &[(i32, i32)], blueprints: &[i32]) {
        let ids: Vec<i32> = disciplines.iter().map(|&(id, _)| id).collect();
        sqlx::query(
            "UPDATE sgw_player SET discipline_ids = $2, blueprint_ids = $3 WHERE player_id = $1",
        )
        .bind(self.player_id)
        .bind(&ids)
        .bind(blueprints)
        .execute(&self.pool)
        .await
        .expect("seed crafting columns");
        for &(id, expertise) in disciplines {
            sqlx::query(
                "INSERT INTO sgw_player_discipline_expertise (player_id, discipline_id, expertise) \
                 VALUES ($1, $2, $3)",
            )
            .bind(self.player_id)
            .bind(id)
            .bind(expertise)
            .execute(&self.pool)
            .await
            .expect("seed expertise");
        }
    }

    /// Insert a stack; returns its instance id.
    async fn stack(&self, type_id: i32, container_id: i32, slot_id: i32, size: i32) -> i32 {
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

    /// `n` single stacks of `type_id` in `container_id` from `first_slot`;
    /// returns the instance ids.
    async fn stacks(&self, type_id: i32, container_id: i32, first_slot: i32, n: i32) -> Vec<i32> {
        let mut ids = Vec::new();
        for slot in first_slot..first_slot + n {
            ids.push(self.stack(type_id, container_id, slot, 1).await);
        }
        ids
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

    /// Units of `type_id` the player holds anywhere.
    async fn units(&self, type_id: i32) -> i64 {
        self.stacks_of(type_id)
            .await
            .iter()
            .map(|s| i64::from(s.1))
            .sum()
    }

    async fn expertise(&self, discipline_id: i32) -> Option<i32> {
        sqlx::query_scalar(
            "SELECT expertise FROM sgw_player_discipline_expertise \
             WHERE player_id = $1 AND discipline_id = $2",
        )
        .bind(self.player_id)
        .bind(discipline_id)
        .fetch_optional(&self.pool)
        .await
        .expect("read expertise")
    }

    fn ctx(&self) -> CraftCtx<'_> {
        CraftCtx {
            db_pool: &self.db_pool,
            cell_tx: &None,
            transport: &self.transport,
            connected: &self.connected,
            entity_to_addr: &self.entity_to_addr,
        }
    }

    /// Send one `craft` request through the verb.
    async fn craft(&self, blueprint_id: i32, items: &[i32], quantity: i32) {
        handle_craft_with(
            &self.sessions,
            self.entity_id,
            self.player_id,
            blueprint_id,
            items,
            quantity,
            &self.ctx(),
        )
        .await;
    }

    /// Run every induction to its end, in order, as the clock would.
    async fn run_inductions(&self) -> usize {
        let env = InductionEnv::from_ctx(&self.ctx());
        let mut ran = 0;
        loop {
            let due = self.scheduler.take();
            if due.is_empty() {
                return ran;
            }
            for d in due {
                self.sessions
                    .expire_at(d.entity_id, d.job_id, d.deadline, &env)
                    .await;
                ran += 1;
            }
        }
    }

    fn calls(&self) -> Vec<MethodCall> {
        decode_all(&self.typed.filter_to(self.addr))
    }

    /// Every `CHAN_FEEDBACK` line sent so far.
    fn lines(&self) -> Vec<String> {
        self.calls()
            .iter()
            .filter(|c| c.method == method_idx::ON_PLAYER_COMMUNICATION)
            .map(feedback_text)
            .collect()
    }
}

/// A session with no database, for the tests that never reach one.
pub(super) struct Offline {
    entity_id: u32,
    typed: Arc<TestTransport>,
    transport: Arc<dyn Transport>,
    connected: Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    entity_to_addr: Arc<Mutex<HashMap<u32, SocketAddr>>>,
    addr: SocketAddr,
    sessions: Arc<CraftingSessions>,
    db_pool: Option<Arc<PgPool>>,
}

pub(super) fn offline(entity_id: u32) -> Offline {
    let s = OneSession::new(entity_id, 56300 + (entity_id % 100) as u16);
    Offline {
        entity_id,
        typed: s.typed,
        transport: s.transport,
        connected: s.connected,
        entity_to_addr: s.entity_to_addr,
        addr: s.addr,
        sessions: Arc::new(CraftingSessions::new(
            Box::new(ManualScheduler::default()),
            Box::new(|| Box::new(ScriptedRng::new(vec![0.5]))),
        )),
        db_pool: None,
    }
}

impl Offline {
    fn ctx(&self) -> CraftCtx<'_> {
        CraftCtx {
            db_pool: &self.db_pool,
            cell_tx: &None,
            transport: &self.transport,
            connected: &self.connected,
            entity_to_addr: &self.entity_to_addr,
        }
    }

    fn lines(&self) -> Vec<String> {
        decode_all(&self.typed.filter_to(self.addr))
            .iter()
            .filter(|c| c.method == method_idx::ON_PLAYER_COMMUNICATION)
            .map(feedback_text)
            .collect()
    }
}
