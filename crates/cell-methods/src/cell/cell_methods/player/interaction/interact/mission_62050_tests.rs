//! Live-DB guards for the interaction gates in front of mission 62050 "The
//! Remaining Pods" (CB-N05-5). The accept must come only from a validated
//! Prisoner 329 click: the real entity, in the player's space, within
//! `MAX_INTERACT_DISTANCE`. A lying client must not accept or complete 62050
//! with a forged click, a forged dialog choice or a stale `initialResponse`.
//!
//! Every test runs the real seed rows through the startup spawn path and the
//! production chain engine, so the observable is the player's mission list.
//! Each negative sits beside a positive in the same rig: the refusal is only
//! evidence when the very same rig accepts a legitimate click.

use tokio::sync::mpsc;

use cimmeria_content_engine::chain::ChainEngine;
use cimmeria_entity::missions::{
    MissionInstance, MissionObjective, MISSION_ACTIVE, MISSION_COMPLETED, STATUS_ACTIVE,
};

use crate::cell::cell_methods::player::{
    dispatch, DIALOG_BUTTON_CHOICE, INITIAL_RESPONSE, INTERACT,
};
use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;
use crate::cell::spawner;
use crate::test_support::require_db_or_skip;

const PLAYER: u32 = 1;
const PLAYER_ID: i32 = 4242;
const MISSION: i32 = 62050;
const STEP_B: i32 = 62054;
const OBJECTIVE_B: i32 = 62055;
const DIALOG_REPORT: i32 = 60150;
const SET_MAP_OFFER: i32 = 2795;
/// `interact_range`'s ceiling; a click this far out plus a margin is refused.
const FAR: f32 = 6.0;
const NEAR: f32 = 1.0;

/// The startup caches the rig needs, read with the cell's own loaders. A
/// macro rather than a function because this crate has no `sqlx` dependency
/// to name the pool type with.
macro_rules! load_seed {
    ($pool:expr) => {
        Seed {
            records: spawner::load_spawns_from_db(&$pool)
                .await
                .expect("spawns must load"),
            mission_defs: spawner::load_mission_defs(&$pool)
                .await
                .expect("mission defs must load"),
            world_rows: spawner::load_world_rows(&$pool)
                .await
                .expect("world rows must load"),
        }
    };
}

struct Seed {
    records: Vec<spawner::SpawnRecord>,
    mission_defs: std::collections::HashMap<i32, spawner::MissionDefEntry>,
    world_rows: std::collections::HashMap<String, spawner::WorldRow>,
}

struct Rig {
    mgr: SpaceManager,
    engine: ChainEngine,
    prisoner: u32,
    marsh: u32,
    /// The Prisoner record again, for the cross-space copy.
    prisoner_record: spawner::SpawnRecord,
}

/// Castle_CellBlock with only Prisoner 329 and Col. Marsh spawned from their
/// real rows, the world rows stamped (without them `ctx.world_id` is `None`
/// and every chain gated on `world 12` fails closed), and the player with
/// 638 completed so the offer conditions hold.
fn stage(seed: Seed, engine: ChainEngine) -> Rig {
    let mut mgr = SpaceManager::new(1);
    let xml = r#"<?xml version="1.0"?><Spaces>
        <Space WorldName="Castle_CellBlock" Instanced="false" MinX="-800" MaxX="800" MinY="-800" MaxY="800" />
        <Space WorldName="Agnos" Instanced="false" MinX="-800" MaxX="800" MinY="-800" MaxY="800" />
    </Spaces>"#;
    let cxml = r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle_CellBlock" /><Space WorldName="Agnos" /></Spaces>"#;
    mgr.parse_spaces_xml(xml).unwrap();
    mgr.create_startup_spaces(cxml).unwrap();
    mgr.mission_defs = seed.mission_defs;
    mgr.stamp_world_rows(&seed.world_rows);

    let find = |tag: &str| {
        seed.records
            .iter()
            .find(|r| r.tag.as_deref() == Some(tag) && r.world_name == "Castle_CellBlock")
            .unwrap_or_else(|| panic!("{tag} must be seeded in Castle_CellBlock"))
            .clone()
    };
    let prisoner_record = find("Prisoner_329");
    let marsh_record = find("Preparation_ColMarsh");
    let prisoner = mgr.allocate_npc_id();
    mgr.spawn_npc_from_record(prisoner, &prisoner_record)
        .expect("Prisoner 329 must spawn from its record");
    let marsh = mgr.allocate_npc_id();
    mgr.spawn_npc_from_record(marsh, &marsh_record)
        .expect("Col. Marsh must spawn from his record");

    mgr.create_entity(
        PLAYER,
        "Castle_CellBlock",
        [-334.231, 73.472, -228.026],
        [0.0; 3],
    )
    .expect("the player must stage in Castle_CellBlock");
    let p = mgr.get_entity_mut(PLAYER).unwrap();
    p.is_player = true;
    p.player_id = Some(PLAYER_ID);
    p.archetype_id = Some(1);
    complete(&mut mgr, 638, 2116);
    mgr.connect_entity(PLAYER);
    Rig {
        mgr,
        engine,
        prisoner,
        marsh,
        prisoner_record,
    }
}

/// Give the player `mission_id` as already completed (any step id will do).
fn complete(mgr: &mut SpaceManager, mission_id: i32, step_id: i32) {
    let mut done = MissionInstance::new(mission_id, step_id, vec![]);
    done.complete();
    mgr.get_entity_mut(PLAYER)
        .unwrap()
        .missions
        .add_mission(done);
}

fn mission(rig: &Rig) -> Option<&MissionInstance> {
    rig.mgr
        .get_entity(PLAYER)
        .unwrap()
        .missions
        .get_mission(MISSION)
}

/// Move the player `dx` metres east of `npc`'s position.
fn stand_near(rig: &mut Rig, npc: u32, dx: f32) {
    let at = rig.mgr.get_entity(npc).unwrap().position;
    rig.mgr
        .update_entity_position(PLAYER, [at.x + dx, at.y, at.z], [0; 3], [0.0; 3]);
}

/// Send a wire method through the player dispatcher and collect what it
/// queued for the base.
async fn send(rig: &mut Rig, method: u16, args: &[u8]) -> Vec<CellToBaseMsg> {
    let (tx, mut rx) = mpsc::channel(64);
    assert!(dispatch(PLAYER, method, args, &tx, &mut rig.mgr, &rig.engine).await);
    let mut out = Vec::new();
    while let Ok(msg) = rx.try_recv() {
        out.push(msg);
    }
    out
}

/// Right-click `target` (a raw wire id, possibly a lie).
async fn click_id(rig: &mut Rig, target: i32) {
    send(rig, INTERACT, &target.to_le_bytes()).await;
}

/// Stand `dx` from `npc` and click it.
async fn click(rig: &mut Rig, npc: u32, dx: f32) {
    stand_near(rig, npc, dx);
    click_id(rig, npc as i32).await;
}

fn dialog_choice_args(dialog_id: i32, button_id: i32) -> Vec<u8> {
    let mut a = Vec::with_capacity(8);
    a.extend_from_slice(&dialog_id.to_le_bytes());
    a.extend_from_slice(&button_id.to_le_bytes());
    a
}

/// The positive control: a legitimate Prisoner click on this rig accepts and
/// the nested follow-up moves 62050 to step B. Called after a negative has
/// shown the mission absent, so the refusal cannot be a rig that never works.
async fn assert_real_click_accepts(rig: &mut Rig) {
    assert!(
        mission(rig).is_none(),
        "62050 must still be absent before the control click"
    );
    let prisoner = rig.prisoner;
    click(rig, prisoner, NEAR).await;
    let m = mission(rig).expect("a validated click must accept 62050 on this rig");
    assert_eq!(m.status, MISSION_ACTIVE);
    assert_eq!(
        m.current_step_id,
        Some(STEP_B),
        "the nested accept follow-up must have advanced to step B"
    );
}

#[tokio::test]
async fn live_db_mission_62050_dispatch_prisoner_click_in_range_accepts() {
    let pool = require_db_or_skip!();
    let engine = crate::cell::content::build_engine(Some(&pool)).await;
    let mut rig = stage(load_seed!(pool), engine);

    assert!(mission(&rig).is_none());
    let prisoner = rig.prisoner;
    click(&mut rig, prisoner, NEAR).await;

    let m = mission(&rig).expect("an in-range Prisoner click must accept 62050");
    assert_eq!(m.status, MISSION_ACTIVE);
    assert_eq!(
        m.current_step_id,
        Some(STEP_B),
        "the nested accept follow-up must have advanced 62051 to 62054"
    );
}

#[tokio::test]
async fn live_db_mission_62050_dispatch_far_click_refused() {
    let pool = require_db_or_skip!();
    let engine = crate::cell::content::build_engine(Some(&pool)).await;
    let mut rig = stage(load_seed!(pool), engine);

    let prisoner = rig.prisoner;
    click(&mut rig, prisoner, FAR).await;
    assert!(mission(&rig).is_none(), "a 6 m click must not accept 62050");

    assert_real_click_accepts(&mut rig).await;
}

#[tokio::test]
async fn live_db_mission_62050_dispatch_wrong_and_missing_entity_refused() {
    let pool = require_db_or_skip!();
    let engine = crate::cell::content::build_engine(Some(&pool)).await;
    let mut rig = stage(load_seed!(pool), engine);

    // Marsh has no 62050 effect while 62050 is absent (chain 11005 needs
    // step B active), even at point-blank range.
    let marsh = rig.marsh;
    click(&mut rig, marsh, NEAR).await;
    assert!(
        mission(&rig).is_none(),
        "a Marsh click must not accept 62050"
    );

    // An id nothing is allocated at, and a negative one.
    let prisoner = rig.prisoner;
    stand_near(&mut rig, prisoner, NEAR);
    click_id(&mut rig, 900_000).await;
    click_id(&mut rig, -1).await;
    assert!(
        mission(&rig).is_none(),
        "a click on no entity must not accept"
    );

    assert_real_click_accepts(&mut rig).await;
}

#[tokio::test]
async fn live_db_mission_62050_dispatch_cross_space_refused() {
    let pool = require_db_or_skip!();
    let engine = crate::cell::content::build_engine(Some(&pool)).await;
    let mut rig = stage(load_seed!(pool), engine);

    // A copy of the Prisoner record in a different space, at the same
    // coordinates: the player is numerically in range, so only the space
    // check can refuse.
    let mut copy = rig.prisoner_record.clone();
    copy.world_name = "Agnos".to_string();
    let other = rig.mgr.allocate_npc_id();
    rig.mgr
        .spawn_npc_from_record(other, &copy)
        .expect("the copy must spawn in the second space");
    assert_ne!(other, rig.prisoner);

    let prisoner = rig.prisoner;
    stand_near(&mut rig, prisoner, NEAR);
    click_id(&mut rig, other as i32).await;
    assert!(
        mission(&rig).is_none(),
        "a Prisoner in another space must not accept 62050"
    );

    assert_real_click_accepts(&mut rig).await;
}

#[tokio::test]
async fn live_db_mission_62050_dispatch_forged_dialog_choice_refused() {
    let pool = require_db_or_skip!();
    let engine = crate::cell::content::build_engine(Some(&pool)).await;
    let mut rig = stage(load_seed!(pool), engine);

    for button in [-1, 0] {
        send(
            &mut rig,
            DIALOG_BUTTON_CHOICE,
            &dialog_choice_args(DIALOG_REPORT, button),
        )
        .await;
    }
    assert!(
        mission(&rig).is_none(),
        "a dialog choice for a dialog never shown must not accept 62050"
    );

    assert_real_click_accepts(&mut rig).await;

    // The same forged choices after a real accept change nothing.
    for button in [-1, 0] {
        send(
            &mut rig,
            DIALOG_BUTTON_CHOICE,
            &dialog_choice_args(DIALOG_REPORT, button),
        )
        .await;
    }
    let m = mission(&rig).expect("62050 stays accepted");
    assert_eq!(m.status, MISSION_ACTIVE);
    assert_eq!(m.current_step_id, Some(STEP_B));
    assert!(
        !m.completed_objectives.contains(&OBJECTIVE_B),
        "a forged choice must not complete objective 62055"
    );
}

#[tokio::test]
async fn live_db_mission_62050_dispatch_stale_initial_response_refused() {
    let pool = require_db_or_skip!();
    let engine = crate::cell::content::build_engine(Some(&pool)).await;
    let mut rig = stage(load_seed!(pool), engine);

    let args = SET_MAP_OFFER.to_le_bytes();
    // No binding present at all.
    send(&mut rig, INITIAL_RESPONSE, &args).await;
    assert!(
        mission(&rig).is_none(),
        "an unbound initialResponse accepts nothing"
    );

    // The 2795 binding present in slot 17, as chain 11000/11001 leave it:
    // interaction-only (NULL dialog), so there is nothing to run.
    rig.mgr
        .get_entity_mut(PLAYER)
        .unwrap()
        .available_interactions
        .insert(17, vec![(SET_MAP_OFFER, None, 1_073_741_824i64)]);
    send(&mut rig, INITIAL_RESPONSE, &args).await;
    assert!(
        mission(&rig).is_none(),
        "initialResponse 2795 must not accept 62050; only the validated click does"
    );

    assert_real_click_accepts(&mut rig).await;
}

#[tokio::test]
async fn live_db_mission_62050_dispatch_marsh_click_far_refused() {
    let pool = require_db_or_skip!();
    let engine = crate::cell::content::build_engine(Some(&pool)).await;
    let mut rig = stage(load_seed!(pool), engine);

    // 62050 at step B with 640 completed: chain 11005's conditions hold.
    complete(&mut rig.mgr, 640, 2118);
    rig.mgr
        .get_entity_mut(PLAYER)
        .unwrap()
        .missions
        .add_mission(MissionInstance::new(
            MISSION,
            STEP_B,
            vec![MissionObjective {
                objective_id: OBJECTIVE_B,
                status: STATUS_ACTIVE,
                hidden: false,
                optional: false,
            }],
        ));

    let marsh = rig.marsh;
    click(&mut rig, marsh, FAR).await;
    let m = mission(&rig).expect("62050 stays");
    assert_eq!(
        m.status, MISSION_ACTIVE,
        "a 6 m Marsh click must not complete 62050"
    );

    click(&mut rig, marsh, NEAR).await;
    assert_eq!(
        mission(&rig).expect("62050 stays").status,
        MISSION_COMPLETED,
        "the same click at 1 m completes it"
    );
}
