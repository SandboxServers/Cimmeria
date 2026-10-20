//! Executor rig for mission 568 on Tollana (world 19).
//!
//! Drives the real executor (`fire_interact_tag`, `fire_dialog_choice`,
//! `fire_player_loaded`, `execute_actions`) against the seeded engine and
//! mission defs, so the tests assert real per-player mission state and
//! dialog binds (D-TW11). Modelled on `mission_681_686_backstop.rs`.

use std::collections::HashMap;

use cimmeria_content_engine::actions::Action;
use cimmeria_content_engine::chain::{ChainEngine, ResolvedActions};
use cimmeria_entity::missions::MISSION_ACTIVE;
use tokio::sync::mpsc;

use super::super::super::engine_loader::build_engine;
use super::super::super::executor::execute_actions;
use crate::cell::content::{fire_dialog_choice, fire_interact_tag, fire_player_loaded};
use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;
use crate::test_support::require_db_or_skip;

pub(super) const PLAYER: u32 = 7568;
pub(super) const PLAYER_ID: i32 = 56_800;
pub(super) const PLAYER_B: u32 = 7569;
pub(super) const PLAYER_B_ID: i32 = 56_900;
/// The dialog-set slot every 568 bind uses.
pub(super) const SLOT: i32 = 420;

const MISSION: i32 = 568;
const SPAWN: [f32; 3] = [206.0, -0.58, -543.0];

pub(super) struct Rig {
    pub mgr: SpaceManager,
    pub engine: ChainEngine,
    tx: mpsc::Sender<CellToBaseMsg>,
    rx: mpsc::Receiver<CellToBaseMsg>,
}

/// Tollana with world id 19, the seeded mission, step and dialog-set
/// tables, and two connected players at the Copplemann spawn.
pub(super) async fn rig(pool: &sqlx::PgPool) -> Rig {
    let mut mgr = SpaceManager::new(1);
    mgr.parse_spaces_xml(
        r#"<?xml version="1.0"?><Spaces><Space WorldName="Tollana" Instanced="false" MinX="-2000" MaxX="2000" MinY="-2000" MaxY="2000" /></Spaces>"#,
    )
    .unwrap();
    mgr.create_startup_spaces(
        r#"<?xml version="1.0"?><Spaces><Space WorldName="Tollana" /></Spaces>"#,
    )
    .unwrap();
    mgr.worlds.get_mut("Tollana").expect("parsed").world_id = Some(19);
    // All three loaders are required: without `step_objectives` the second
    // `advance_step` loads an empty roster, without `dialog_set_maps` the
    // `add_dialog_set` cache misses.
    mgr.mission_defs = crate::cell::spawner::load_mission_defs(pool)
        .await
        .expect("load mission defs");
    mgr.step_objectives = crate::cell::spawner::load_step_objectives(pool)
        .await
        .expect("load step objectives");
    mgr.dialog_set_maps = crate::cell::spawner::load_dialog_set_maps(pool)
        .await
        .expect("load dialog set maps");

    for (id, player_id) in [(PLAYER, PLAYER_ID), (PLAYER_B, PLAYER_B_ID)] {
        mgr.create_entity(id, "Tollana", SPAWN, [0.0; 3]).unwrap();
        let p = mgr.get_entity_mut(id).unwrap();
        p.is_player = true;
        p.player_id = Some(player_id);
        mgr.connect_entity(id);
    }

    let (tx, rx) = mpsc::channel(8192);
    Rig {
        mgr,
        engine: build_engine(Some(pool)).await,
        tx,
        rx,
    }
}

/// The player id the executor sees for `who`.
fn pid(who: u32) -> i32 {
    if who == PLAYER {
        PLAYER_ID
    } else {
        PLAYER_B_ID
    }
}

impl Rig {
    /// Fire an interaction on `tag` for `who`. Returns whether a chain fired.
    pub(super) async fn click(&mut self, who: u32, tag: &str) -> bool {
        let fired =
            fire_interact_tag(who, pid(who), tag, 0, &self.engine, &self.tx, &mut self.mgr).await;
        self.drain();
        fired
    }

    /// Close dialog `dialog_id` for `who` with the `-1` close button.
    pub(super) async fn close(&mut self, who: u32, dialog_id: i32) {
        fire_dialog_choice(
            who,
            pid(who),
            dialog_id,
            -1,
            &self.engine,
            &self.tx,
            &mut self.mgr,
        )
        .await;
        self.drain();
    }

    /// Load `who` into Tollana. The binds are rebuilt empty on entry, as
    /// the base does on a fresh login.
    pub(super) async fn enter(&mut self, who: u32) {
        if let Some(e) = self.mgr.get_entity_mut(who) {
            e.available_interactions.clear();
        }
        fire_player_loaded(
            who,
            pid(who),
            "Tollana",
            &self.engine,
            &self.tx,
            &mut self.mgr,
        )
        .await;
        self.drain();
    }

    /// Abandon mission 568 for `who` through the executor, so
    /// `mission_abandoned` fires and chain 9010 runs. The T-03a3 walk and
    /// the T-03a4 relog tests call it.
    pub(super) async fn abandon(&mut self, who: u32) {
        let resolved = ResolvedActions {
            actions: vec![(
                0,
                Action::AbandonMission {
                    mission_id: MISSION,
                },
            )],
            action_delays: vec![0],
            params: HashMap::new(),
        };
        execute_actions(
            resolved,
            who,
            pid(who),
            &self.tx,
            &mut self.mgr,
            &self.engine,
        )
        .await;
        self.drain();
    }

    /// Dialog-set-map ids bound on slot 420 for `who`.
    pub(super) fn binds(&self, who: u32) -> Vec<i32> {
        self.mgr
            .get_entity(who)
            .and_then(|e| e.available_interactions.get(&SLOT))
            .map(|v| v.iter().map(|b| b.0).collect())
            .unwrap_or_default()
    }

    /// Mission 568 status for `who`, or `None` if the mission is absent.
    pub(super) fn status(&self, who: u32) -> Option<i8> {
        self.mgr
            .get_entity(who)?
            .missions
            .get_mission(MISSION)
            .map(|m| m.status)
    }

    /// Current step of mission 568 for `who`.
    pub(super) fn step(&self, who: u32) -> Option<i32> {
        self.mgr
            .get_entity(who)?
            .missions
            .get_mission(MISSION)?
            .current_step_id
    }

    /// Completed objectives of mission 568 for `who`.
    pub(super) fn completed(&self, who: u32) -> Vec<i32> {
        self.mgr
            .get_entity(who)
            .and_then(|e| e.missions.get_mission(MISSION))
            .map(|m| m.completed_objectives.clone())
            .unwrap_or_default()
    }

    /// Discard the client messages an action queued.
    fn drain(&mut self) {
        while self.rx.try_recv().is_ok() {}
    }
}

#[tokio::test]
async fn live_db_568_rig_accept_on_2153_close() {
    let pool = require_db_or_skip!();
    let mut r = rig(&pool).await;
    r.enter(PLAYER).await;
    assert_eq!(r.binds(PLAYER), vec![2656], "entry offers the accept bind");
    assert!(r.click(PLAYER, "Tollana_Copplemann").await);
    r.close(PLAYER, 2153).await;
    assert_eq!(r.status(PLAYER), Some(MISSION_ACTIVE), "2153 accepts 568");
    assert_eq!(r.step(PLAYER), Some(1964));
    let done = r.completed(PLAYER);
    assert!(done.contains(&2309), "objective 2309 is done: {done:?}");
    assert!(!done.contains(&2310), "objective 2310 is open: {done:?}");
    assert_eq!(r.binds(PLAYER), vec![4959], "progress bind after accept");
}
