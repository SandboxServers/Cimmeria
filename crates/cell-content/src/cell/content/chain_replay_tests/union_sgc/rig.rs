//! Shared rig for the Union SGC_W1 tests (US-10).
//!
//! Sealed at US-10: no later packet adds or changes a method here. A local
//! helper goes in the packet's own module. Every entry point is the real
//! dispatcher; the rig only chooses which engine a setup step runs through
//! and when deferred actions are due.

// The whole API is frozen here for the later SGC packets, so a constant or
// method no test has called yet is not dead code.
#![allow(dead_code)]

use std::collections::HashMap;
use std::time::{Duration, Instant};

use sqlx::PgPool;
use tokio::sync::mpsc;

use cimmeria_common::math::Vector3;
use cimmeria_content_engine::actions::Action;
use cimmeria_content_engine::chain::{ChainEngine, ResolvedActions};
use cimmeria_entity::missions::{MissionManager, MISSION_ACTIVE, MISSION_COMPLETED};
use cimmeria_entity::stats::HEALTH;
use cimmeria_wire::state_field::BSF_DEAD;

use crate::cell::content::{
    build_engine, execute_actions, fire_dialog_choice, fire_enter_region, fire_entity_death,
    fire_interact_tag, fire_player_loaded,
};
use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::{spawn_instance_npcs_from_records, SpaceManager};
use crate::cell::spawner::{
    load_mission_defs, load_spawn_templates, load_spawns_from_db, WorldRow,
};
use crate::mercury::method_idx::{ON_DIALOG_DISPLAY, ON_SEQUENCE};
use crate::test_support::require_db_or_skip;

pub(super) const PLAYER: u32 = 7158;
pub(super) const PLAYER_ID: i32 = 15_800;
pub(super) const WORLD: &str = "SGC_W1";
pub(super) const WORLD_ID: i32 = 58;

// `EArchetype` (entities/defs/enumerations.xml).
pub(super) const SOLDIER: i32 = 1;
pub(super) const ARCHAEOLOGIST: i32 = 4;
pub(super) const ASGARD: i32 = 5;
pub(super) const GOAULD: i32 = 6;
pub(super) const FREE_JAFFA: i32 = 7;
pub(super) const LOYALIST_JAFFA: i32 = 8;
pub(super) const HUMANS: [i32; 4] = [1, 2, 3, 4];
pub(super) const NON_HUMANS: [i32; 4] = [ASGARD, GOAULD, FREE_JAFFA, LOYALIST_JAFFA];
/// A player whose archetype was never stamped (GM-created): `Rig::new(pool, UNSTAMPED)`. It reads as -1
/// (`conditions/mod.rs:333-338`) and must fail the Union Human gate (`gte 1` and `lte 4`).
pub(super) const UNSTAMPED: Option<i32> = None;

// Interaction masks (crates/entity/src/interaction_flags.rs).
pub(super) const OFFER: i64 = 8_388_608; // "?"
pub(super) const STORY_ACTIVE: i64 = 16_777_216; // "!"
pub(super) const TURN_IN: i64 = 33_554_432;
pub(super) const MISSION_OBJECT: i64 = 1_073_741_824; // glow

pub(super) const ACTIVE: Option<i8> = Some(MISSION_ACTIVE);
pub(super) const COMPLETED: Option<i8> = Some(MISSION_COMPLETED);

/// H3 / P13, where the player stands when the rig is built.
pub(super) const ARRIVAL: [f32; 3] = [172.79, -10.9, 12.0];

/// What a chain asked the base for, reduced to what the SGC packets assert.
#[derive(Debug, Clone, PartialEq)]
pub(super) enum Sent {
    /// `add_item`: item, container, count.
    Item(i32, i32, i32),
    /// `display_dialog`: `onDialogDisplay` dialog id (args bytes 4..8).
    Dialog(i32),
    /// `play_sequence`: `onSequence` sequence id (args bytes 0..4).
    Sequence(i32),
    /// `cross_world_teleport`: `GateTravel` world and position.
    Travel(String, [f32; 3]),
}

pub(super) struct Rig {
    pub(super) mgr: SpaceManager,
    pub(super) engine: ChainEngine,
    quiet: ChainEngine,
    tx: mpsc::Sender<CellToBaseMsg>,
    rx: mpsc::Receiver<CellToBaseMsg>,
    archetype: Option<i32>,
}

/// A Vector3 as the [x, y, z] the rig and the chains speak.
fn xyz(v: Vector3) -> [f32; 3] {
    [v.x, v.y, v.z]
}

/// The `i32` at `at..at + 4` of a method's little-endian argument bytes.
fn i32_at(args: &[u8], at: usize) -> i32 {
    let bytes: [u8; 4] = args
        .get(at..at + 4)
        .and_then(|b| b.try_into().ok())
        .expect("the method call carries an i32 at this offset");
    i32::from_le_bytes(bytes)
}

impl Rig {
    /// A fresh instanced SGC_W1 with the player connected at [`ARRIVAL`].
    pub(super) async fn new(pool: &PgPool, archetype: Option<i32>) -> Rig {
        let mut mgr = SpaceManager::new(1);
        mgr.parse_spaces_xml(
            r#"<?xml version="1.0"?><Spaces><Space WorldName="SGC_W1" Instanced="true" MinX="-800" MaxX="800" MinY="-800" MaxY="800" /></Spaces>"#,
        )
        .expect("spaces xml must parse");
        mgr.stamp_world_rows(&HashMap::from([(
            WORLD.to_string(),
            WorldRow::enforcing(WORLD_ID),
        )]));
        mgr.spawn_templates = load_spawn_templates(pool)
            .await
            .expect("load spawn templates");
        mgr.mission_defs = load_mission_defs(pool).await.expect("load mission defs");
        let space_id = mgr
            .find_or_create_space(WORLD)
            .expect("SGC_W1 is a known world");
        mgr.create_entity_in_space(PLAYER, space_id, ARRIVAL, [0.0; 3])
            .expect("the player enters SGC_W1");
        {
            let p = mgr.get_entity_mut(PLAYER).expect("the player exists");
            p.is_player = true;
            p.player_id = Some(PLAYER_ID);
            p.archetype_id = archetype;
        }
        mgr.connect_entity(PLAYER);
        let records = load_spawns_from_db(pool).await.expect("load spawns");
        spawn_instance_npcs_from_records(&records, WORLD, space_id, &mut mgr);
        let (tx, rx) = mpsc::channel(8192);
        Rig {
            mgr,
            engine: build_engine(Some(pool)).await,
            quiet: ChainEngine::new(),
            tx,
            rx,
            archetype,
        }
    }

    /// One action through `engine` (chains may fire) or the empty engine
    /// (nothing fires), then the channel is drained so setup leaves no trace.
    async fn exec(&mut self, action: Action, chains: bool) {
        let resolved = ResolvedActions {
            actions: vec![(0, action)],
            action_delays: vec![0],
            params: HashMap::new(),
        };
        let engine = if chains { &self.engine } else { &self.quiet };
        execute_actions(resolved, PLAYER, PLAYER_ID, &self.tx, &mut self.mgr, engine).await;
        while self.rx.try_recv().is_ok() {}
    }

    /// Give the player `mission_id` on `step_id` quietly: no chain fires.
    pub(super) async fn accept(&mut self, mission_id: i32) {
        self.exec(Action::AcceptMission { mission_id }, false).await;
    }

    /// Move the player's mission to `step_id` quietly: no chain fires.
    pub(super) async fn advance(&mut self, mission_id: i32, step_id: i32) {
        self.exec(
            Action::AdvanceStep {
                mission_id,
                step_id,
            },
            false,
        )
        .await;
    }

    /// Complete `mission_id` quietly: no chain fires.
    pub(super) async fn complete(&mut self, mission_id: i32) {
        self.exec(Action::CompleteMission { mission_id }, false)
            .await;
    }

    /// Complete one objective of `mission_id` quietly: no chain fires.
    pub(super) async fn objective(&mut self, mission_id: i32, objective_id: i32) {
        self.exec(
            Action::CompleteObjective {
                mission_id,
                objective_id,
            },
            false,
        )
        .await;
    }

    /// Complete `mission_id` through the live engine, so `mission_completed`
    /// chains fire.
    pub(super) async fn finish(&mut self, mission_id: i32) {
        self.exec(Action::CompleteMission { mission_id }, true)
            .await;
    }

    /// Press the tagged world object or NPC the way the client does.
    pub(super) async fn press(&mut self, tag: &str) -> bool {
        let target = self
            .mgr
            .find_entity_by_tag(PLAYER, tag)
            .unwrap_or_else(|| panic!("{tag} is spawned"));
        fire_interact_tag(
            PLAYER,
            PLAYER_ID,
            tag,
            target,
            &self.engine,
            &self.tx,
            &mut self.mgr,
        )
        .await
    }

    /// Close a zero-button dialog.
    pub(super) async fn close(&mut self, dialog_id: i32) {
        self.choose(dialog_id, -1).await;
    }

    /// Pick `button` on `dialog_id`.
    pub(super) async fn choose(&mut self, dialog_id: i32, button: i32) {
        fire_dialog_choice(
            PLAYER,
            PLAYER_ID,
            dialog_id,
            button,
            &self.engine,
            &self.tx,
            &mut self.mgr,
        )
        .await;
    }

    /// The player finished loading into SGC_W1.
    pub(super) async fn loaded(&mut self) {
        fire_player_loaded(
            PLAYER,
            PLAYER_ID,
            WORLD,
            &self.engine,
            &self.tx,
            &mut self.mgr,
        )
        .await;
    }

    /// The player entered the named region.
    pub(super) async fn enter(&mut self, region: &str) {
        fire_enter_region(
            PLAYER,
            PLAYER_ID,
            region,
            &self.engine,
            &self.tx,
            &mut self.mgr,
        )
        .await;
    }

    /// Kill the tagged NPC the way combat does: the corpse carries `BSF_DEAD`
    /// and zero health before `entity_dead_tag` fires, with the player as killer.
    pub(super) async fn kill(&mut self, tag: &str) {
        let id = self
            .mgr
            .find_entity_by_tag(PLAYER, tag)
            .unwrap_or_else(|| panic!("{tag} is spawned"));
        let npc = self.mgr.get_entity_mut(id).expect("the tagged NPC exists");
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

    /// Run every deferred action due within `within_ms`, in queue order.
    pub(super) async fn run_due(&mut self, within_ms: u64) {
        let now = Instant::now() + Duration::from_millis(within_ms);
        for (entity_id, pending) in self.mgr.take_ready_content_actions(now) {
            let resolved = ResolvedActions {
                actions: vec![(pending.chain_id, pending.action)],
                action_delays: vec![0],
                params: pending.params,
            };
            execute_actions(
                resolved,
                entity_id,
                pending.player_id,
                &self.tx,
                &mut self.mgr,
                &self.engine,
            )
            .await;
        }
    }

    /// How many deferred actions are queued for the player.
    pub(super) fn pending(&self) -> usize {
        self.mgr
            .pending_content_actions
            .get(&PLAYER)
            .map_or(0, Vec::len)
    }

    /// Drain the cell-to-base channel into [`Sent`] rows, in send order.
    pub(super) fn sent(&mut self) -> Vec<Sent> {
        let mut out = Vec::new();
        while let Ok(msg) = self.rx.try_recv() {
            match msg {
                CellToBaseMsg::GrantItem {
                    item_id,
                    container_id,
                    count,
                    ..
                } => out.push(Sent::Item(item_id, container_id, count)),
                CellToBaseMsg::EntityMethodCall {
                    method_index, args, ..
                } if method_index == ON_DIALOG_DISPLAY => {
                    out.push(Sent::Dialog(i32_at(&args, 4)));
                }
                CellToBaseMsg::EntityMethodCall {
                    method_index, args, ..
                } if method_index == ON_SEQUENCE => {
                    out.push(Sent::Sequence(i32_at(&args, 0)));
                }
                CellToBaseMsg::GateTravel {
                    target_world_name,
                    position,
                    ..
                } => out.push(Sent::Travel(target_world_name, position)),
                _ => {}
            }
        }
        out
    }

    /// The player's mission status, if the player holds the mission.
    pub(super) fn status(&self, mission_id: i32) -> Option<i8> {
        self.mgr
            .get_entity(PLAYER)?
            .missions
            .get_mission(mission_id)
            .map(|m| m.status)
    }

    /// The player's current step in `mission_id`, if any.
    pub(super) fn step(&self, mission_id: i32) -> Option<i32> {
        self.mgr
            .get_entity(PLAYER)?
            .missions
            .get_mission(mission_id)
            .and_then(|m| m.current_step_id)
    }

    /// Whether `objective_id` of `mission_id` is completed.
    pub(super) fn objective_done(&self, mission_id: i32, objective_id: i32) -> bool {
        self.mgr
            .get_entity(PLAYER)
            .and_then(|p| p.missions.get_mission(mission_id))
            .is_some_and(|m| m.completed_objectives.contains(&objective_id))
    }

    /// The interaction-type flags of the tagged entity (0 if absent).
    pub(super) fn flags(&self, tag: &str) -> i64 {
        self.mgr
            .find_entity_by_tag(PLAYER, tag)
            .and_then(|id| self.mgr.get_entity(id))
            .map_or(0, |e| e.interaction_type_flags)
    }

    /// Whether the tagged entity exists and is not dead.
    pub(super) fn live(&self, tag: &str) -> bool {
        self.mgr
            .find_entity_by_tag(PLAYER, tag)
            .and_then(|id| self.mgr.get_entity(id))
            .is_some_and(|e| e.state_field & BSF_DEAD == 0)
    }

    /// The tagged entity's position.
    pub(super) fn pos(&self, tag: &str) -> [f32; 3] {
        self.mgr
            .find_entity_by_tag(PLAYER, tag)
            .and_then(|id| self.mgr.get_entity(id))
            .map(|e| xyz(e.position))
            .unwrap_or_else(|| panic!("{tag} is spawned"))
    }

    /// The player's position.
    pub(super) fn player_pos(&self) -> [f32; 3] {
        xyz(self
            .mgr
            .get_entity(PLAYER)
            .expect("the player exists")
            .position)
    }

    /// Log out and back in: the missions survive, the deferred queue does not.
    pub(super) async fn relog(mut self, pool: &PgPool) -> Rig {
        let missions = std::mem::replace(
            &mut self
                .mgr
                .get_entity_mut(PLAYER)
                .expect("the player exists")
                .missions,
            MissionManager::new(),
        );
        let archetype = self.archetype;
        drop(self);
        let mut r = Rig::new(pool, archetype).await;
        r.mgr
            .get_entity_mut(PLAYER)
            .expect("the player exists")
            .missions = missions;
        r.loaded().await;
        r
    }
}

/// **Guard: the rig builds an instanced SGC_W1 that a chain can spawn
/// into.** Revert proof: build the space with `Instanced="false"` and the
/// spawn is refused (`spawn_entity: refusing to spawn into a non-instanced
/// world`, reason `shared_world_refused`), so `live` is false.
#[tokio::test]
async fn live_db_union_sgc_rig_spawns_static_rows_and_a_chain_npc() {
    let pool = require_db_or_skip!();
    let mut rig = Rig::new(&pool, Some(SOLDIER)).await;
    for tag in [
        "SGCW1_GenHammond",
        "SGC_W1_SamCarter",
        "SGC_W1_Tealc",
        "SGC_W1_GateTerminal1",
    ] {
        assert!(rig.live(tag), "{tag} is a seeded world-58 spawn");
    }
    // One spawn_entity through the full engine: template 47 (Woolsey) at P02.
    // Same call shape as harset_spawn_entity.rs:291; chain id 0 is a probe.
    let resolved = ResolvedActions {
        action_delays: vec![0],
        params: HashMap::new(),
        actions: vec![(
            0,
            Action::SpawnEntity {
                template_id: 47,
                position: [197.5, 1.311, 43.6],
                heading: 0.0,
                tag: "SGC_W1_RigProbe".to_string(),
                is_stationary: None,
                aggression: None,
                allow_shared: None,
            },
        )],
    };
    execute_actions(
        resolved,
        PLAYER,
        PLAYER_ID,
        &rig.tx,
        &mut rig.mgr,
        &rig.engine,
    )
    .await;
    assert!(rig.live("SGC_W1_RigProbe"));
}
