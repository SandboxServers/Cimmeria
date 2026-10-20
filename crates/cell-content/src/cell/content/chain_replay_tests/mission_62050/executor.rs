//! CB-N05-7 — mission 62050 "The Remaining Pods" replayed through the real
//! executor and event dispatchers (`fire_interact_tag`, `fire_player_loaded`,
//! `execute_actions`) against the seeded engine, spawn rows, mission defs,
//! dialog-set maps and world rows. `resolve.rs` proves each chain resolves;
//! this proves the executed story: the nested accept follow-up, the
//! per-player binding, the monologue wire bytes, the Marsh completion beside
//! the core chains, and the replays (abandon, relog, duplicate, skip).
//!
//! Revert proof: delete chain 11004's `remove_dialog_set` row from the seed
//! and test 1 fails on the slot-17 assertion.

use std::collections::HashMap;

use cimmeria_content_engine::actions::Action;
use cimmeria_content_engine::chain::{ChainEngine, ResolvedActions};
use cimmeria_entity::missions::{
    MissionInstance, MissionObjective, MISSION_ACTIVE, MISSION_COMPLETED, STATUS_ACTIVE,
};
use tokio::sync::mpsc;

use super::super::super::engine_loader::build_engine;
use super::super::super::executor::execute_actions;
use crate::cell::content::{fire_interact_tag, fire_player_loaded};
use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::{spawn_npcs_from_records, SpaceManager};
use crate::cell::spawner::{
    load_dialog_set_maps, load_mission_defs, load_monologue_dialog_ids, load_spawns_from_db,
    load_step_objectives, load_world_rows,
};
use crate::test_support::require_db_or_skip;

const PLAYER: u32 = 62_050;
const PLAYER_ID: i32 = 620_500;
const TAGS: &[&str] = &["Prisoner_329", "Preparation_ColMarsh"];

/// The 2795 interaction-only binding at slot 17 (dialog-set-map 2795,
/// `dialog_id` NULL, flags 1073741824).
fn binding() -> Option<Vec<(i32, Option<i32>, i64)>> {
    Some(vec![(2795, None, 1_073_741_824i64)])
}

struct Rig {
    mgr: SpaceManager,
    engine: ChainEngine,
    tx: mpsc::Sender<CellToBaseMsg>,
    rx: mpsc::Receiver<CellToBaseMsg>,
}

/// Castle_CellBlock with Prisoner 329 and Col. Marsh from the seed, the
/// seeded mission defs and dialog caches, and one connected player of
/// `archetype`. `stamp` false leaves the world rows unstamped, so every
/// `world`-gated chain fails closed.
async fn rig(pool: &sqlx::PgPool, archetype: i32, stamp: bool) -> Rig {
    let records: Vec<_> = load_spawns_from_db(pool)
        .await
        .expect("load spawnlist")
        .into_iter()
        .filter(|r| r.world_name == "Castle_CellBlock")
        .filter(|r| r.tag.as_deref().is_some_and(|t| TAGS.contains(&t)))
        .collect();
    assert_eq!(records.len(), TAGS.len(), "both NPCs are seeded once");

    let mut mgr = SpaceManager::new(1);
    mgr.parse_spaces_xml(
        r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle_CellBlock" Instanced="false" MinX="-800" MaxX="800" MinY="-800" MaxY="800" /></Spaces>"#,
    )
    .unwrap();
    mgr.create_startup_spaces(
        r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle_CellBlock" /></Spaces>"#,
    )
    .unwrap();
    assert_eq!(spawn_npcs_from_records(&records, &mut mgr), TAGS.len());
    if stamp {
        mgr.stamp_world_rows(&load_world_rows(pool).await.expect("world rows"));
    }
    mgr.mission_defs = load_mission_defs(pool).await.expect("load mission defs");
    // Without the step roster `advance_step` activates a step with no
    // objectives and `complete_objective` is a no-op.
    mgr.step_objectives = load_step_objectives(pool).await.expect("step objectives");
    mgr.dialog_set_maps = load_dialog_set_maps(pool).await.expect("dialog set maps");
    mgr.monologue_dialog_ids = load_monologue_dialog_ids(pool).await.expect("monologues");

    mgr.create_entity(PLAYER, "Castle_CellBlock", [-190.0, 54.7, -138.0], [0.0; 3])
        .unwrap();
    let p = mgr.get_entity_mut(PLAYER).unwrap();
    p.is_player = true;
    p.player_id = Some(PLAYER_ID);
    p.archetype_id = Some(archetype);
    mgr.connect_entity(PLAYER);

    let (tx, rx) = mpsc::channel(8192);
    Rig {
        mgr,
        engine: build_engine(Some(pool)).await,
        tx,
        rx,
    }
}

impl Rig {
    async fn run(&mut self, action: Action) {
        let resolved = ResolvedActions {
            actions: vec![(0, action)],
            action_delays: vec![0],
            params: HashMap::new(),
        };
        execute_actions(
            resolved,
            PLAYER,
            PLAYER_ID,
            &self.tx,
            &mut self.mgr,
            &self.engine,
        )
        .await;
    }

    async fn accept(&mut self, mission_id: i32) {
        self.run(Action::AcceptMission { mission_id }).await;
    }

    /// Stage `mission_id` as already completed (a saved-game state).
    fn stage_completed(&mut self, mission_id: i32) {
        let mut m = MissionInstance::new(mission_id, 0, Vec::new());
        m.complete();
        self.mgr
            .get_entity_mut(PLAYER)
            .unwrap()
            .missions
            .add_mission(m);
    }

    /// Stage `mission_id` active at `step_id` with one active objective.
    fn stage_active(&mut self, mission_id: i32, step_id: i32, objective_id: i32) {
        let m = MissionInstance::new(
            mission_id,
            step_id,
            vec![MissionObjective {
                objective_id,
                status: STATUS_ACTIVE,
                hidden: false,
                optional: false,
            }],
        );
        self.mgr
            .get_entity_mut(PLAYER)
            .unwrap()
            .missions
            .add_mission(m);
    }

    /// Click the NPC carrying `tag` through the real dispatcher.
    async fn click(&mut self, tag: &str) -> bool {
        let npc = self
            .mgr
            .find_entity_by_tag(PLAYER, tag)
            .unwrap_or_else(|| panic!("{tag} is spawned"));
        fire_interact_tag(
            PLAYER,
            PLAYER_ID,
            tag,
            npc,
            &self.engine,
            &self.tx,
            &mut self.mgr,
        )
        .await
    }

    async fn load(&mut self) {
        fire_player_loaded(
            PLAYER,
            PLAYER_ID,
            "Castle_CellBlock",
            &self.engine,
            &self.tx,
            &mut self.mgr,
        )
        .await;
    }

    fn drain(&mut self) -> Vec<CellToBaseMsg> {
        let mut out = Vec::new();
        while let Ok(msg) = self.rx.try_recv() {
            out.push(msg);
        }
        out
    }

    fn slot17(&self) -> Option<Vec<(i32, Option<i32>, i64)>> {
        self.mgr
            .get_entity(PLAYER)
            .unwrap()
            .available_interactions
            .get(&17)
            .cloned()
    }

    fn mission(&self, id: i32) -> Option<(i8, Option<i32>)> {
        self.mgr
            .get_entity(PLAYER)?
            .missions
            .get_mission(id)
            .map(|m| (m.status, m.current_step_id))
    }
}

/// `(speaker entity id, dialog id)` of every `onDialogDisplay` sent to the
/// player (args bytes 0..4 and 4..8, LE).
fn dialogs(msgs: &[CellToBaseMsg]) -> Vec<(i32, i32)> {
    msgs.iter()
        .filter_map(|m| match m {
            CellToBaseMsg::EntityMethodCall {
                entity_id,
                method_index,
                args,
            } if *method_index == crate::mercury::method_idx::ON_DIALOG_DISPLAY
                && *entity_id == PLAYER =>
            {
                Some((
                    i32::from_le_bytes(args[0..4].try_into().unwrap()),
                    i32::from_le_bytes(args[4..8].try_into().unwrap()),
                ))
            }
            _ => None,
        })
        .collect()
}

/// How many `MissionUpdate`s persisted `mission_id` (any status).
fn updates(msgs: &[CellToBaseMsg], mission_id: i32) -> usize {
    msgs.iter()
        .filter(|m| matches!(m, CellToBaseMsg::MissionUpdate { mission_id: id, .. } if *id == mission_id))
        .count()
}

#[tokio::test]
async fn live_db_mission_62050_replay_offer_accept_and_report() {
    let pool = require_db_or_skip!();
    let mut r = rig(&pool, 1, true).await;

    r.stage_active(638, 0, 0);
    r.run(Action::CompleteMission { mission_id: 638 }).await;
    assert_eq!(r.slot17(), binding(), "638 completing must offer 62050");
    r.drain();

    assert!(r.click("Prisoner_329").await, "the click must be handled");
    assert_eq!(
        r.mission(62050),
        Some((MISSION_ACTIVE, Some(62054))),
        "the nested accept follow-up must land on step B, not 62051"
    );
    assert!(
        !r.slot17().unwrap_or_default().iter().any(|e| e.0 == 2795),
        "the accept must remove the 2795 offer (chain 11004)"
    );
    assert_eq!(
        dialogs(&r.drain()),
        vec![(PLAYER as i32, 60150)],
        "exactly one monologue 60150 with the player as speaker"
    );
}

async fn marsh_completes(archetype: i32, dialog: i32) {
    let pool = require_db_or_skip!();
    let mut r = rig(&pool, archetype, true).await;
    r.stage_completed(638);
    r.stage_completed(640);
    assert!(r.click("Prisoner_329").await);
    assert_eq!(r.mission(62050), Some((MISSION_ACTIVE, Some(62054))));
    r.drain();

    assert!(r.click("Preparation_ColMarsh").await);
    assert_eq!(
        r.mission(62050).map(|m| m.0),
        Some(MISSION_COMPLETED),
        "Marsh must complete 62050 (archetype {archetype})"
    );
    assert_eq!(
        dialogs(&r.drain()).iter().map(|d| d.1).collect::<Vec<_>>(),
        vec![dialog],
        "exactly the core Marsh dialog beside the completion"
    );

    r.accept(641).await;
    assert_eq!(
        dialogs(&r.drain()).iter().map(|d| d.1).collect::<Vec<_>>(),
        vec![4000],
        "chain 1152 must still show 4000 on 641"
    );
}

#[tokio::test]
async fn live_db_mission_62050_replay_marsh_completes_beside_core_archetype_1() {
    marsh_completes(1, 4001).await;
}

#[tokio::test]
async fn live_db_mission_62050_replay_marsh_completes_beside_core_archetype_8() {
    marsh_completes(8, 5022).await;
}

#[tokio::test]
async fn live_db_mission_62050_replay_duplicate_and_skip() {
    let pool = require_db_or_skip!();

    // Duplicate click while active: nothing happens.
    let mut r = rig(&pool, 1, true).await;
    r.stage_completed(638);
    assert!(r.click("Prisoner_329").await);
    r.drain();
    r.click("Prisoner_329").await;
    let msgs = r.drain();
    assert_eq!(r.mission(62050), Some((MISSION_ACTIVE, Some(62054))));
    assert_eq!(updates(&msgs, 62050), 0, "no second accept");
    assert!(
        !dialogs(&msgs).iter().any(|d| d.1 == 60150),
        "no second 60150"
    );

    // Step A staged by hand: a Marsh click must not complete anything.
    let mut r = rig(&pool, 1, true).await;
    r.stage_completed(638);
    r.stage_completed(640);
    r.stage_active(62050, 62051, 62052);
    r.click("Preparation_ColMarsh").await;
    assert_eq!(
        r.mission(62050),
        Some((MISSION_ACTIVE, Some(62051))),
        "Marsh must not complete 62050 from step A"
    );

    // Skipping 62050 for 641: nothing binds or accepts it afterwards.
    let mut r = rig(&pool, 1, true).await;
    r.stage_completed(638);
    r.accept(641).await;
    r.drain();
    r.load().await;
    assert_eq!(r.slot17(), None, "641 active: no 2795 binding at login");
    r.click("Prisoner_329").await;
    assert!(
        r.mission(62050).is_none_or(|m| m.0 != MISSION_ACTIVE),
        "Prisoner must not accept 62050 once 641 is active"
    );
}

#[tokio::test]
async fn live_db_mission_62050_replay_abandon_restores_completion_does_not() {
    let pool = require_db_or_skip!();
    let mut r = rig(&pool, 1, true).await;
    r.stage_completed(638);
    r.stage_completed(640);

    assert!(r.click("Prisoner_329").await);
    assert_eq!(r.slot17(), None);
    r.run(Action::AbandonMission { mission_id: 62050 }).await;
    assert_eq!(r.slot17(), binding(), "abandon must restore the offer");

    assert!(r.click("Prisoner_329").await, "re-accept after abandon");
    assert_eq!(r.mission(62050), Some((MISSION_ACTIVE, Some(62054))));
    assert!(r.click("Preparation_ColMarsh").await);
    assert_eq!(r.mission(62050).map(|m| m.0), Some(MISSION_COMPLETED));
    r.drain();

    r.load().await;
    assert_eq!(r.slot17(), None, "completed: no offer at login");
    r.click("Prisoner_329").await;
    let msgs = r.drain();
    assert_eq!(r.mission(62050).map(|m| m.0), Some(MISSION_COMPLETED));
    assert_eq!(updates(&msgs, 62050), 0, "no re-accept");
    assert!(!dialogs(&msgs).iter().any(|d| d.1 == 60150));
}

#[tokio::test]
async fn live_db_mission_62050_replay_relog_restores_offer_once() {
    let pool = require_db_or_skip!();
    let mut r = rig(&pool, 1, true).await;
    r.stage_completed(638);
    r.load().await;
    assert_eq!(r.slot17(), binding(), "login must bind the offer");
    r.load().await;
    assert_eq!(r.slot17(), binding(), "`once` must stop a duplicate bind");

    let mut r = rig(&pool, 1, false).await;
    r.stage_completed(638);
    r.load().await;
    assert_eq!(
        r.slot17(),
        None,
        "unstamped world rows fail the gate closed"
    );
}
