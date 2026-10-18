//! CB-N06-5 — mission 62060 "Before the Patrol Returns" replayed through the
//! real executor and event dispatchers (`fire_interact_tag`,
//! `fire_player_loaded`, `execute_actions`) against the seeded engine, spawn
//! rows, mission defs, dialog-set maps and world rows. `resolve.rs` proves
//! each chain resolves; this proves the executed story: the nested accept,
//! the per-player binds (slot 176 on the prisoners, slot 22 on the panel),
//! the 60151 monologue, exactly one slappack grant per player, and the
//! replays (abandon, 641, relog, repeat clicks). Literals: w5-frozen.md F4/F5.
//!
//! Revert proof: delete chain 11038's `step_status` condition row from the
//! seed and `..._sabotage_grants_once` fails, because the second panel click
//! grants again.

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

const PLAYER: u32 = 62_060;
const PLAYER_ID: i32 = 620_600;
const P1: &str = "Cellblock_Ambient_Prisoner1";
const P2: &str = "Cellblock_Ambient_Prisoner2";
const PANEL: &str = "329_CellDoorButton";
const TAGS: &[&str] = &[P1, P2, PANEL];
const MISSION: i32 = 62060;
const STEP: i32 = 62061;
const OBJECTIVE: i32 = 62062;
const SLAPPACK: i64 = 2893;

type Binding = Vec<(i32, Option<i32>, i64)>;

/// The 3862 offer at slot 176 (dialog-set-map 3862, `dialog_id` NULL,
/// interaction-only flags 1073741824).
fn offer() -> Binding {
    vec![(3862, None, 1_073_741_824i64)]
}

/// The 7509 panel choice at slot 22.
fn panel_choice() -> Binding {
    vec![(7509, None, 1_073_741_824i64)]
}

struct Rig {
    mgr: SpaceManager,
    engine: ChainEngine,
    tx: mpsc::Sender<CellToBaseMsg>,
    rx: mpsc::Receiver<CellToBaseMsg>,
}

/// Castle_CellBlock with both prisoners and the cell-door panel from the
/// seed, the seeded mission defs and dialog caches, and one connected player
/// of `archetype`. `stamp` false leaves the world rows unstamped, so every
/// `world`-gated chain fails closed.
async fn rig(pool: &sqlx::PgPool, archetype: i32, stamp: bool) -> Rig {
    let records: Vec<_> = load_spawns_from_db(pool)
        .await
        .expect("load spawnlist")
        .into_iter()
        .filter(|r| r.world_name == "Castle_CellBlock")
        .filter(|r| r.tag.as_deref().is_some_and(|t| TAGS.contains(&t)))
        .collect();
    assert_eq!(
        records.len(),
        TAGS.len(),
        "each tagged spawn is seeded once"
    );

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

    mgr.create_entity(
        PLAYER,
        "Castle_CellBlock",
        [-265.0, 65.45, -135.0],
        [0.0; 3],
    )
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

    /// The state after stock 638: completed through the executor, so chain
    /// 11030 offers 62060 the way the live path does. Drains the messages.
    async fn complete_638(&mut self) {
        self.stage_active(638, 0, 0);
        self.run(Action::CompleteMission { mission_id: 638 }).await;
        self.drain();
    }

    /// Click the entity carrying `tag` through the real dispatcher.
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

    /// The bind at `slot`; unbound reads as an empty list.
    fn slot(&self, slot: i32) -> Binding {
        self.mgr
            .get_entity(PLAYER)
            .unwrap()
            .available_interactions
            .get(&slot)
            .cloned()
            .unwrap_or_default()
    }

    fn mission(&self, id: i32) -> Option<(i8, Option<i32>)> {
        self.mgr
            .get_entity(PLAYER)?
            .missions
            .get_mission(id)
            .map(|m| (m.status, m.current_step_id))
    }

    /// Offer taken and 62060 sitting at its only step.
    async fn accepted(pool: &sqlx::PgPool, archetype: i32) -> Rig {
        let mut r = rig(pool, archetype, true).await;
        r.complete_638().await;
        assert!(r.click(P1).await, "the accept click must be handled");
        assert_eq!(r.mission(MISSION), Some((MISSION_ACTIVE, Some(STEP))));
        r
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

/// `(item_id, count)` of every `GrantItem` sent for the player.
fn grants(msgs: &[CellToBaseMsg]) -> Vec<(i64, i64)> {
    msgs.iter()
        .filter_map(|m| match m {
            CellToBaseMsg::GrantItem {
                entity_id,
                item_id,
                count,
                ..
            } if *entity_id == PLAYER => Some((*item_id as i64, *count as i64)),
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

fn assert_finished(r: &Rig) {
    assert_eq!(r.mission(MISSION).map(|m| m.0), Some(MISSION_COMPLETED));
    assert!(r.slot(176).is_empty(), "the prisoner offer must be gone");
    assert!(r.slot(22).is_empty(), "the panel choice must be gone");
}

/// Nothing a finished 62060 can still react to: three clicks, no grant, no
/// accept, still completed.
async fn assert_inert(r: &mut Rig) {
    r.drain();
    for tag in [P1, P2, PANEL] {
        r.click(tag).await;
    }
    let msgs = r.drain();
    assert!(grants(&msgs).is_empty(), "no second grant");
    assert_eq!(updates(&msgs, MISSION), 0, "no second accept or update");
    assert_finished(r);
}

async fn offer_accept_shows_the_choice(archetype: i32) {
    let pool = require_db_or_skip!();
    let mut r = rig(&pool, archetype, true).await;
    r.stage_active(638, 0, 0);
    r.run(Action::CompleteMission { mission_id: 638 }).await;
    assert_eq!(r.slot(176), offer(), "638 completing must offer 62060");
    r.drain();

    assert!(r.click(P1).await, "the click must be handled");
    assert_eq!(r.mission(MISSION), Some((MISSION_ACTIVE, Some(STEP))));
    assert_eq!(r.slot(22), panel_choice(), "the panel choice binds");
    assert_eq!(r.slot(176), offer(), "the offer stays until the choice");
    let msgs = r.drain();
    assert_eq!(
        dialogs(&msgs),
        vec![(PLAYER as i32, 60151)],
        "exactly one monologue 60151 with the player as speaker"
    );
    assert!(grants(&msgs).is_empty(), "the accept click grants nothing");
}

#[tokio::test]
async fn live_db_mission_62060_replay_offer_accept_shows_the_choice_archetype_1() {
    offer_accept_shows_the_choice(1).await;
}

#[tokio::test]
async fn live_db_mission_62060_replay_offer_accept_shows_the_choice_archetype_8() {
    offer_accept_shows_the_choice(8).await;
}

#[tokio::test]
async fn live_db_mission_62060_replay_aid_grants_once() {
    let pool = require_db_or_skip!();
    let mut r = Rig::accepted(&pool, 1).await;
    r.drain();

    assert!(r.click(P2).await);
    assert_eq!(grants(&r.drain()), vec![(SLAPPACK, 2)], "one grant of 2");
    assert_finished(&r);
    assert_inert(&mut r).await;
}

#[tokio::test]
async fn live_db_mission_62060_replay_sabotage_grants_once() {
    let pool = require_db_or_skip!();
    let mut r = Rig::accepted(&pool, 1).await;
    r.drain();

    assert!(r.click(PANEL).await);
    assert_eq!(grants(&r.drain()), vec![(SLAPPACK, 2)], "one grant of 2");
    assert_finished(&r);

    // A prisoner click and a second panel click: nothing.
    r.click(P1).await;
    r.click(PANEL).await;
    let msgs = r.drain();
    assert!(grants(&msgs).is_empty(), "no second grant");
    assert_eq!(updates(&msgs, MISSION), 0);
    assert_finished(&r);
}

#[tokio::test]
async fn live_db_mission_62060_replay_abandon_and_641() {
    let pool = require_db_or_skip!();

    // Abandon: only 11039 fires, so the prisoner offer stays.
    let mut r = Rig::accepted(&pool, 1).await;
    r.run(Action::AbandonMission {
        mission_id: MISSION,
    })
    .await;
    assert!(r.slot(22).is_empty(), "abandon must unbind the panel");
    assert_eq!(r.slot(176), offer(), "abandon must keep the prisoner offer");
    r.drain();
    assert!(r.click(P2).await, "re-accept after abandon");
    assert_eq!(r.mission(MISSION), Some((MISSION_ACTIVE, Some(STEP))));
    assert_eq!(r.slot(22), panel_choice());
    assert!(grants(&r.drain()).is_empty());

    // Never taken, then 641: the offer is withdrawn, 4000 still shows, and a
    // prisoner click accepts nothing.
    let mut r = rig(&pool, 1, true).await;
    r.complete_638().await;
    r.run(Action::AcceptMission { mission_id: 641 }).await;
    assert!(
        r.slot(176).is_empty(),
        "641 must withdraw the offer (11040)"
    );
    assert_eq!(
        dialogs(&r.drain()).iter().map(|d| d.1).collect::<Vec<_>>(),
        vec![4000],
        "chain 1152 must still show 4000 on 641"
    );
    r.click(P1).await;
    assert!(r.mission(MISSION).is_none(), "no accept once 641 is active");
    assert!(r.slot(22).is_empty());

    // Taken, then 641, then abandoned: 11041 withdraws the offer.
    let mut r = Rig::accepted(&pool, 1).await;
    r.run(Action::AcceptMission { mission_id: 641 }).await;
    assert_eq!(r.slot(176), offer(), "11040 is gated on 62060 not_active");
    r.run(Action::AbandonMission {
        mission_id: MISSION,
    })
    .await;
    assert!(r.slot(176).is_empty(), "11041 must withdraw the offer");
    assert!(r.slot(22).is_empty());
}

#[tokio::test]
async fn live_db_mission_62060_replay_relog_keeps_the_outcome() {
    let pool = require_db_or_skip!();

    // 638 completed: login binds the offer once.
    let mut r = rig(&pool, 1, true).await;
    r.stage_completed(638);
    r.load().await;
    assert_eq!(r.slot(176), offer(), "login must bind the offer");
    assert!(r.slot(22).is_empty());
    r.load().await;
    assert_eq!(r.slot(176), offer(), "`once` must stop a duplicate bind");

    // 62060 at its step: login restores both choices.
    let mut r = rig(&pool, 1, true).await;
    r.stage_completed(638);
    r.stage_active(MISSION, STEP, OBJECTIVE);
    r.load().await;
    assert_eq!(r.slot(176), offer());
    assert_eq!(r.slot(22), panel_choice());

    // 62060 completed: nothing binds and nothing accepts it again.
    let mut r = rig(&pool, 1, true).await;
    r.stage_completed(638);
    r.stage_completed(MISSION);
    r.load().await;
    assert!(r.slot(176).is_empty() && r.slot(22).is_empty());
    assert_inert(&mut r).await;

    // Unstamped world rows fail every gate closed.
    let mut r = rig(&pool, 1, false).await;
    r.stage_completed(638);
    r.load().await;
    assert!(r.slot(176).is_empty() && r.slot(22).is_empty());
    r.stage_active(MISSION, STEP, OBJECTIVE);
    r.load().await;
    assert!(r.slot(176).is_empty() && r.slot(22).is_empty());
}
