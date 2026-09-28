//! Out-of-order kill backstops for the Cellblock controller missions
//! 681-686 (`castle_cellblock_chains.sql` chains 1181-1190).
//!
//! The 2026-09-28 colo playtest (build 5f9730c6, tester Lomiada): after a
//! respawn in the Stasis Chamber the player ran back through Hallway01, its
//! guard chased him into the Mess Hall and died 4.5 s *before*
//! MessHall_Guard1. Chain 1088 completes 682 only on the Hallway01 death
//! *event* while 682 is active, so 682 never completed, 683-686 were never
//! offered and the Straegis scene never played: a soft-lock.
//!
//! Every test here runs the full seeded engine ([`build_engine`], the one the
//! server loads) against real spawn rows, and drives kills through
//! [`fire_entity_death`] exactly as combat does, so the cascade runs through
//! the real executor: `accept_mission` → `mission_accepted` → backstop →
//! `complete_mission` → `accept_mission` → ...
//!
//! Revert proof: delete chains 1181-1190 from the seed (or the
//! `entity_tag_state` loader arm) and the playtest replay leaves 682 active.

use std::collections::HashMap;

use cimmeria_content_engine::actions::Action;
use cimmeria_content_engine::chain::{ChainEngine, ResolvedActions};
use cimmeria_entity::missions::{MISSION_ACTIVE, MISSION_COMPLETED};
use cimmeria_entity::stats::HEALTH;
use cimmeria_wire::state_field::BSF_DEAD;
use tokio::sync::mpsc;

use super::super::engine_loader::build_engine;
use super::super::executor::execute_actions;
use crate::cell::content::fire_entity_death;
use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::{spawn_npcs_from_records, SpaceManager};
use crate::cell::spawner::{load_mission_defs, load_spawns_from_db};
use crate::test_support::require_db_or_skip;

const PLAYER: u32 = 7681;
const PLAYER_ID: i32 = 68_100;

const TAGS: &[&str] = &[
    "MessHall_Guard1",
    "MessHall_Guard2",
    "Hallway01_Guard",
    "Hallway02_Guard",
    "Hallway03_Guard",
    "Hallway04_Guard",
    "Hallway05_Guard1",
    "Hallway05_Guard2",
    "Preparation_ColMarsh",
];

struct Rig {
    mgr: SpaceManager,
    engine: ChainEngine,
    tx: mpsc::Sender<CellToBaseMsg>,
    rx: mpsc::Receiver<CellToBaseMsg>,
}

/// Castle_CellBlock with the Mess Hall, hallway and Col. Marsh spawns from
/// the seed, the seeded mission defs, and one connected player.
async fn rig(pool: &sqlx::PgPool) -> Rig {
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
        "every tag this test kills is seeded exactly once"
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
    mgr.mission_defs = load_mission_defs(pool).await.expect("load mission defs");

    mgr.create_entity(PLAYER, "Castle_CellBlock", [-95.0, 34.6, -95.0], [0.0; 3])
        .unwrap();
    let p = mgr.get_entity_mut(PLAYER).unwrap();
    p.is_player = true;
    p.player_id = Some(PLAYER_ID);
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
    /// Accept `mission_id` the way chain 1073 does (an `accept_mission`
    /// action through the executor), so its `mission_accepted` fires.
    async fn accept(&mut self, mission_id: i32) {
        let resolved = ResolvedActions {
            actions: vec![(0, Action::AcceptMission { mission_id })],
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

    /// Kill the guard carrying `tag` the way combat does: the corpse carries
    /// `BSF_DEAD` and zero health before `entity_dead_tag` fires.
    async fn kill(&mut self, tag: &str) {
        let id = self
            .mgr
            .find_entity_by_tag(PLAYER, tag)
            .unwrap_or_else(|| panic!("{tag} is spawned"));
        let npc = self.mgr.get_entity_mut(id).unwrap();
        npc.state_field |= BSF_DEAD;
        if let Some(h) = npc.stats.get_mut(HEALTH) {
            h.update(0, 0, h.max);
        }
        fire_entity_death(
            PLAYER,
            PLAYER_ID,
            tag,
            &self.engine,
            &self.tx,
            &mut self.mgr,
        )
        .await;
    }

    fn status(&self, mission_id: i32) -> Option<i8> {
        self.mgr
            .get_entity(PLAYER)?
            .missions
            .get_mission(mission_id)
            .map(|m| m.status)
    }

    fn assert_status(&self, mission_id: i32, want: i8, what: &str) {
        assert_eq!(
            self.status(mission_id),
            Some(want),
            "{what}: mission {mission_id}"
        );
    }

    /// How many times each mission was persisted as newly active. The
    /// offer guard refuses a second accept, so every mission must be here
    /// at most once (the T15 "no duplicate accept" invariant).
    fn accept_counts(&mut self) -> HashMap<i32, usize> {
        let mut counts = HashMap::new();
        while let Ok(msg) = self.rx.try_recv() {
            if let CellToBaseMsg::MissionUpdate {
                mission_id, status, ..
            } = msg
            {
                if status == MISSION_ACTIVE {
                    *counts.entry(mission_id).or_insert(0) += 1;
                }
            }
        }
        counts
    }

    fn marsh_present(&self) -> bool {
        self.mgr
            .find_entity_by_tag(PLAYER, "Preparation_ColMarsh")
            .is_some()
    }
}

fn assert_each_accepted_once(counts: &HashMap<i32, usize>, missions: &[i32]) {
    for m in missions {
        assert_eq!(
            counts.get(m).copied(),
            Some(1),
            "mission {m} must be accepted exactly once; accept updates: {counts:?}"
        );
    }
}

/// The 2026-09-28 order: MessHall_Guard2, then Hallway01_Guard (while 681 is
/// still active), then MessHall_Guard1. The backstop completes 682 at once;
/// the rest of the hallways then run in order, 686 completes and chain 1161
/// (`mission_completed 686`, the Straegis scene) despawns Col. Marsh.
#[tokio::test]
async fn live_db_hallway01_killed_before_the_mess_hall_still_reaches_686() {
    let pool = require_db_or_skip!();
    let mut r = rig(&pool).await;

    r.accept(681).await;
    r.assert_status(681, MISSION_ACTIVE, "fixture");

    r.kill("MessHall_Guard2").await;
    r.kill("Hallway01_Guard").await;
    assert_eq!(r.status(682), None, "no chain may react to the early kill");
    r.kill("MessHall_Guard1").await;

    r.assert_status(681, MISSION_COMPLETED, "Mess Hall cleared");
    r.assert_status(
        682,
        MISSION_COMPLETED,
        "682 must complete on accept because Hallway01_Guard is already dead \
         (the 2026-09-28 soft-lock)",
    );
    r.assert_status(683, MISSION_ACTIVE, "the cascade stops at a living guard");

    for tag in [
        "Hallway02_Guard",
        "Hallway03_Guard",
        "Hallway04_Guard",
        "Hallway05_Guard1",
    ] {
        r.kill(tag).await;
    }
    r.assert_status(686, MISSION_ACTIVE, "one Hallway05 guard left");
    assert!(r.marsh_present(), "Marsh stays until 686 completes");
    r.kill("Hallway05_Guard2").await;

    for m in 683..=686 {
        r.assert_status(m, MISSION_COMPLETED, "hallways cleared in order");
    }
    r.assert_status(687, MISSION_ACTIVE, "686 hands off to 687");
    assert!(
        !r.marsh_present(),
        "chain 1161 on mission_completed 686 must despawn Col. Marsh (Straegis scene)"
    );
    let counts = r.accept_counts();
    assert_each_accepted_once(&counts, &[681, 682, 683, 684, 685, 686, 687]);
}

/// Every hallway guard dies while 681 is active. Clearing the Mess Hall must
/// cascade all the way: 682-685 complete on accept, 686 completes because
/// both of its guards are dead, 687 is accepted and the Straegis scene plays.
#[tokio::test]
async fn live_db_every_hallway_cleared_early_cascades_through_686() {
    let pool = require_db_or_skip!();
    let mut r = rig(&pool).await;

    r.accept(681).await;
    for tag in [
        "Hallway01_Guard",
        "Hallway02_Guard",
        "Hallway03_Guard",
        "Hallway04_Guard",
        "Hallway05_Guard1",
        "Hallway05_Guard2",
        "MessHall_Guard2",
    ] {
        r.kill(tag).await;
    }
    assert_eq!(r.status(682), None);
    r.kill("MessHall_Guard1").await;

    for m in 681..=686 {
        r.assert_status(m, MISSION_COMPLETED, "one kill cascades the chain");
    }
    r.assert_status(687, MISSION_ACTIVE, "686 hands off to 687");
    assert!(!r.marsh_present(), "the Straegis scene despawns Marsh");
    let counts = r.accept_counts();
    assert_each_accepted_once(&counts, &[681, 682, 683, 684, 685, 686, 687]);
}

/// A two-guard room with one guard dead before its mission is accepted: the
/// backstop credits the early kill to the counter, so the remaining kill
/// completes the mission. Both two-guard rooms are covered, 681
/// (`messhall_kills`, chains 1182/1183) and 686 (`hallway05_kills`, chains
/// 1189/1190).
#[tokio::test]
async fn live_db_one_early_kill_in_a_two_guard_room_still_counts() {
    let pool = require_db_or_skip!();
    let mut r = rig(&pool).await;

    // MessHall_Guard1 dies before 681 is offered (680 still in progress).
    r.kill("MessHall_Guard1").await;
    // Hallway05_Guard2 dies while 681 is active, long before 686.
    r.accept(681).await;
    r.kill("Hallway05_Guard2").await;
    r.kill("MessHall_Guard2").await;
    r.assert_status(
        681,
        MISSION_COMPLETED,
        "the early Mess Hall kill must count toward messhall_kills",
    );

    for tag in [
        "Hallway01_Guard",
        "Hallway02_Guard",
        "Hallway03_Guard",
        "Hallway04_Guard",
    ] {
        r.kill(tag).await;
    }
    r.assert_status(686, MISSION_ACTIVE, "Hallway05_Guard1 still alive");
    r.kill("Hallway05_Guard1").await;
    r.assert_status(
        686,
        MISSION_COMPLETED,
        "the early Hallway05 kill must count toward hallway05_kills",
    );
    let counts = r.accept_counts();
    assert_each_accepted_once(&counts, &[681, 682, 683, 684, 685, 686, 687]);
}

/// The normal order still works and no backstop fires early: a controller
/// accepted while its guard is alive stays active until the kill.
#[tokio::test]
async fn live_db_in_order_kills_leave_each_controller_active_until_its_kill() {
    let pool = require_db_or_skip!();
    let mut r = rig(&pool).await;

    r.accept(681).await;
    r.kill("MessHall_Guard1").await;
    r.kill("MessHall_Guard2").await;
    r.assert_status(682, MISSION_ACTIVE, "Hallway01_Guard is alive");
    r.kill("Hallway01_Guard").await;
    r.assert_status(682, MISSION_COMPLETED, "chain 1088");
    r.assert_status(683, MISSION_ACTIVE, "Hallway02_Guard is alive");
}
