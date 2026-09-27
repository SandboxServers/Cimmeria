//! Live-DB fixture for the induction verb tests: a connected player built
//! from the `0x7000_C240` sentinel block, an engine that runs with a
//! scripted RNG and hand-fired wake-ups, and readers for the database and
//! the packets the player received.

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use cimmeria_mercury::transport::Transport;
use sqlx::PgPool;

use super::request::{handle_craft_request, CraftCtx};
use super::rng::ScriptedRng;
use super::session::{
    CraftingSessions, InductionEnv, InductionJob, ManualScheduler, SubmitOutcome,
};
use super::test_packets::{decode_all, feedback_text, MethodCall};
use crate::cell::messages::{CraftRequest, CraftVerb};
use crate::mercury::method_idx;
use crate::test_support::{test_default_connected_client_state, TestTransport};

/// Inside crafting's `0x7000_Cxxx` block: `0x7000_C240..=0x7000_C2FF`, 48
/// slots of four ids (account, player, entity). `0x7000_C201` is an
/// in-memory login test's account id, so the block starts past it.
const TEST_BASE: i32 = 0x7000_C240;

/// Every craft-type bit: the cell's station check granted all four verbs.
pub(crate) const ALL_STATIONS: u8 = 0x0F;

pub(crate) struct VerbFixture {
    pub(crate) pool: PgPool,
    pub(crate) account_id: i32,
    pub(crate) player_id: i32,
    pub(crate) entity_id: u32,
    pub(crate) env: InductionEnv,
    pub(crate) transport: Arc<TestTransport>,
    pub(crate) addr: SocketAddr,
}

impl VerbFixture {
    /// A fresh player in sentinel slot `slot` (0..48), connected in world
    /// `CombatSim` to a test transport.
    pub(crate) async fn new(pool: &PgPool, slot: i32) -> Self {
        assert!((0..48).contains(&slot), "slot {slot} outside the block");
        let account_id = TEST_BASE + slot * 4;
        let player_id = account_id + 1;
        let entity_id = (account_id + 2) as u32;
        let transport = Arc::new(TestTransport::new());
        let dyn_transport: Arc<dyn Transport> = transport.clone();
        let addr: SocketAddr = format!("127.0.0.1:{}", 57200 + slot).parse().unwrap();
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
            .bind(format!("craft-verb-{account_id}"))
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
        .bind(format!("craft-verb-{player_id}"))
        .execute(pool)
        .await
        .expect("insert player");
        f
    }

    pub(crate) async fn cleanup(&self) {
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

    /// The request context the base dispatcher would build.
    pub(crate) fn ctx(&self) -> CraftCtx<'_> {
        CraftCtx {
            db_pool: &self.env.db_pool,
            cell_tx: &self.env.cell_tx,
            transport: &self.env.transport,
            connected: &self.env.connected,
            entity_to_addr: &self.env.entity_to_addr,
        }
    }

    /// Send `verb` through the base's request entry point, with every
    /// station bit granted.
    pub(crate) async fn request(&self, verb: CraftVerb) {
        let request = CraftRequest {
            entity_id: self.entity_id,
            player_id: self.player_id,
            verb,
            allowed: ALL_STATIONS,
        };
        handle_craft_request(request, &self.ctx()).await;
    }

    /// Insert a stack; returns its instance id.
    pub(crate) async fn stack(&self, type_id: i32, container_id: i32, slot_id: i32) -> i32 {
        sqlx::query_scalar(
            "INSERT INTO sgw_inventory \
                (character_id, type_id, stack_size, slot_id, container_id, bound, durability, charges) \
             VALUES ($1, $2, 1, $3, $4, false, 100, 0) RETURNING item_id",
        )
        .bind(self.player_id)
        .bind(type_id)
        .bind(slot_id)
        .bind(container_id)
        .fetch_one(&self.pool)
        .await
        .expect("insert stack")
    }

    /// Make the player know `discipline_id` at `expertise`.
    pub(crate) async fn know(&self, discipline_id: i32, expertise: i32) {
        sqlx::query(
            "UPDATE sgw_player SET discipline_ids = array_append(discipline_ids, $2) \
             WHERE player_id = $1",
        )
        .bind(self.player_id)
        .bind(discipline_id)
        .execute(&self.pool)
        .await
        .expect("learn discipline");
        sqlx::query(
            "INSERT INTO sgw_player_discipline_expertise (player_id, discipline_id, expertise) \
             VALUES ($1, $2, $3)",
        )
        .bind(self.player_id)
        .bind(discipline_id)
        .bind(expertise)
        .execute(&self.pool)
        .await
        .expect("seed expertise");
    }

    /// Set the player's known blueprints.
    pub(crate) async fn set_blueprints(&self, ids: &[i32]) {
        sqlx::query("UPDATE sgw_player SET blueprint_ids = $2 WHERE player_id = $1")
            .bind(self.player_id)
            .bind(ids)
            .execute(&self.pool)
            .await
            .expect("set blueprints");
    }

    pub(crate) async fn blueprints(&self) -> Vec<i32> {
        sqlx::query_scalar("SELECT blueprint_ids FROM sgw_player WHERE player_id = $1")
            .bind(self.player_id)
            .fetch_one(&self.pool)
            .await
            .expect("read blueprints")
    }

    pub(crate) async fn expertise(&self, discipline_id: i32) -> Option<i32> {
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

    /// Whether the instance still exists.
    pub(crate) async fn holds(&self, item_id: i32) -> bool {
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM sgw_inventory WHERE item_id = $1")
            .bind(item_id)
            .fetch_one(&self.pool)
            .await
            .expect("read row")
            == 1
    }

    /// Total units of `type_id` the player holds, and the bags they sit in.
    pub(crate) async fn units_of(&self, type_id: i32) -> (i64, Vec<i32>) {
        let rows: Vec<(i32, i32)> = sqlx::query_as(
            "SELECT stack_size, container_id FROM sgw_inventory \
             WHERE character_id = $1 AND type_id = $2",
        )
        .bind(self.player_id)
        .bind(type_id)
        .fetch_all(&self.pool)
        .await
        .expect("read stacks");
        let mut bags: Vec<i32> = rows.iter().map(|r| r.1).collect();
        bags.dedup();
        (rows.iter().map(|r| i64::from(r.0)).sum(), bags)
    }

    /// Every item row the player has, as `(item_id, type_id)`.
    pub(crate) async fn inventory(&self) -> Vec<(i32, i32)> {
        sqlx::query_as(
            "SELECT item_id, type_id FROM sgw_inventory WHERE character_id = $1 ORDER BY item_id",
        )
        .bind(self.player_id)
        .fetch_all(&self.pool)
        .await
        .expect("read inventory")
    }

    pub(crate) fn calls(&self) -> Vec<MethodCall> {
        decode_all(&self.transport.filter_to(self.addr))
    }

    /// The text of every feedback line the player received, in order.
    pub(crate) fn lines(&self) -> Vec<String> {
        self.calls()
            .iter()
            .filter(|c| c.method == method_idx::ON_PLAYER_COMMUNICATION)
            .map(feedback_text)
            .collect()
    }
}

/// An engine whose jobs roll `samples` (cycling) and whose wake-ups are
/// fired by [`run_all`].
pub(crate) fn engine(samples: Vec<f64>) -> (Arc<CraftingSessions>, Arc<ManualScheduler>) {
    let scheduler = Arc::new(ManualScheduler::default());
    let sessions = Arc::new(CraftingSessions::new(
        Box::new(scheduler.clone()),
        Box::new(move || Box::new(ScriptedRng::new(samples.clone()))),
    ));
    (sessions, scheduler)
}

/// Queue `job` for the fixture's player on `sessions`.
pub(crate) async fn submit(
    sessions: &Arc<CraftingSessions>,
    f: &VerbFixture,
    job: Box<dyn InductionJob>,
) -> SubmitOutcome {
    sessions.submit(f.entity_id, f.player_id, job, &f.env).await
}

/// Fire every wake-up at its deadline until nothing is scheduled, so each
/// queued job runs in turn. Returns how many wake-ups fired.
pub(crate) async fn run_all(
    sessions: &Arc<CraftingSessions>,
    scheduler: &ManualScheduler,
    env: &InductionEnv,
) -> usize {
    let mut fired = 0;
    loop {
        let due = scheduler.take();
        if due.is_empty() {
            return fired;
        }
        for d in due {
            sessions
                .expire_at(d.entity_id, d.job_id, d.deadline, env)
                .await;
            fired += 1;
        }
    }
}
