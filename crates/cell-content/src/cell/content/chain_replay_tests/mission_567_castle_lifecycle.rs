//! Mission 567 "Romney's Files", the Castle leg after the accept (CS-08b2):
//! relogs, refusals, the abandon, and the hand-off to the Harset turn-in
//! (chains 6503 and 6504). The chains under test are the ones
//! `castle_567_chains.sql` seeds (1394-1397, 1400, 10240-10243); the happy
//! path itself is pinned by [`super::mission_567_castle`].
//!
//! Every event goes through the real dispatchers, so the executor arms run.
//! The harness is copied from that module rather than shared, so a change
//! to one file cannot move the other's assertions.
//!
//! Chains 6503/6504 and the nine Castle chains ship enabled (FZ-12); `mission_567` pins the flags. This file loads them as shipped.

use cimmeria_content_engine::chain::ChainEngine;
use cimmeria_entity::missions::{
    MissionInstance, MissionObjective, MISSION_COMPLETED, STATUS_ACTIVE,
};
use sqlx::PgPool;
use tokio::sync::mpsc;

use super::super::engine_loader::load_single_chain_for_test;
use super::super::event_dispatch::{fire_dialog_choice, fire_stargate_crossed};
use super::super::{fire_interact_tag, fire_player_loaded};
use crate::cell::content::fire_mission_abandoned;
use crate::cell::messages::CellToBaseMsg;
use crate::cell::missions::abandon_mission;
use crate::cell::space_manager::SpaceManager;
use crate::cell::spawner::{
    load_dialog_set_maps, load_item_containers, load_mission_defs, load_monologue_dialog_ids,
    load_step_objectives, load_world_rows,
};
use crate::test_support::require_db_or_skip;

const ENTITY: u32 = 1;
const PLAYER: i32 = 42;
const CABINET: u32 = 3;
const MISSION: i32 = 567;
const STEP_FETCH: i32 = 2000;
const STEP_EXIT: i32 = 2012;
const STEP_TURN_IN: i32 = 4039;
const CASTLE: &str = "Castle";
const CMD_CENTER: &str = "Harset_CmdCenter";
/// The offer: dialog-set map 1000018 bound at Copplemann's slot 48.
const OFFER_SET: i32 = 1000018;
/// The Harset turn-in map: dialog-set map 2817 on the same slot.
const TURN_IN_SET: i32 = 2817;
const SLOT: i32 = 48;
/// Wire index of the client method onDialogDisplay.
const ON_DIALOG_DISPLAY: u16 = 105;
const ALL_CHAINS: [i32; 9] = [1394, 1395, 1396, 1397, 1400, 10240, 10241, 10242, 10243];
/// The Harset turn-in chains.
const HARSET_CHAINS: [i32; 2] = [6503, 6504];

// ── Harness ──────────────────────────────────────────────────────────────

/// An engine holding `ids`, loaded as shipped.
async fn engine_with(pool: &PgPool, ids: &[i32]) -> ChainEngine {
    let mut engine = ChainEngine::new();
    for &id in ids {
        let chain = load_single_chain_for_test(pool, id)
            .await
            .unwrap_or_else(|e| panic!("DB query for chain {id} must succeed: {e}"))
            .unwrap_or_else(|| panic!("chain {id} must exist in seeded content_chains"));
        engine.register_chain(chain);
    }
    engine
}

/// `world` with player `ENTITY` holding `missions`, with the dialog-set and
/// monologue caches loaded from the seed.
async fn staged(pool: &PgPool, world: &str, missions: Vec<MissionInstance>) -> SpaceManager {
    let mut mgr = SpaceManager::new(1);
    let space = format!(
        r#"<Space WorldName="{world}" Instanced="false" MinX="-800" MaxX="800" MinY="-800" MaxY="800" />"#
    );
    mgr.parse_spaces_xml(&format!("<Spaces>{space}</Spaces>"))
        .unwrap();
    mgr.create_startup_spaces(&format!(
        r#"<Spaces><Space WorldName="{world}" /></Spaces>"#
    ))
    .unwrap();
    mgr.stamp_world_rows(&load_world_rows(pool).await.expect("world cache"));
    mgr.mission_defs = load_mission_defs(pool).await.expect("mission cache");
    mgr.step_objectives = load_step_objectives(pool).await.expect("objective cache");
    mgr.item_containers = load_item_containers(pool).await.expect("item cache");
    mgr.dialog_set_maps = load_dialog_set_maps(pool).await.expect("dialog set maps");
    mgr.monologue_dialog_ids = load_monologue_dialog_ids(pool)
        .await
        .expect("monologue ids");
    mgr.create_entity(CABINET, world, [3.0; 3], [0.0; 3])
        .unwrap();
    mgr.create_entity(ENTITY, world, [0.0; 3], [0.0; 3])
        .unwrap();
    let entity = mgr.get_entity_mut(ENTITY).unwrap();
    entity.is_player = true;
    entity.player_id = Some(PLAYER);
    entity.archetype_id = Some(1);
    for mission in missions {
        entity.missions.add_mission(mission);
    }
    mgr
}

/// A completed mission (the offer prerequisites 701 and 703, or 567 itself).
fn completed(mission_id: i32) -> MissionInstance {
    let mut mission = MissionInstance::new(mission_id, 0, Vec::new());
    mission.status = MISSION_COMPLETED;
    mission.current_step_id = None;
    mission
}

/// 567 active at `step` with `objective` active.
fn mission_567(step: i32, objective: i32) -> MissionInstance {
    let objective = MissionObjective {
        objective_id: objective,
        status: STATUS_ACTIVE,
        hidden: false,
        optional: false,
    };
    let mut mission = MissionInstance::new(MISSION, step, vec![objective]);
    mission.current_step_id = Some(step);
    mission
}

/// The prerequisites plus 567 at `step`: a player part-way through the leg.
fn on_the_leg(step: i32, objective: i32) -> Vec<MissionInstance> {
    vec![completed(701), completed(703), mission_567(step, objective)]
}

fn mission_of(mgr: &SpaceManager) -> Option<&MissionInstance> {
    mgr.get_entity(ENTITY)
        .unwrap()
        .missions
        .get_mission(MISSION)
}

fn step_of(mgr: &SpaceManager) -> Option<i32> {
    mission_of(mgr).and_then(|m| m.current_step_id)
}

/// Every bind on the player as `(slot, dialog-set-map ids)`, ascending.
fn all_binds(mgr: &SpaceManager) -> Vec<(i32, Vec<i32>)> {
    let player = mgr.get_entity(ENTITY).unwrap();
    let mut binds: Vec<(i32, Vec<i32>)> = player
        .available_interactions
        .iter()
        .filter(|(_, v)| !v.is_empty())
        .map(|(&slot, v)| {
            let mut ids: Vec<i32> = v.iter().map(|&(id, _, _)| id).collect();
            ids.sort_unstable();
            (slot, ids)
        })
        .collect();
    binds.sort_unstable();
    binds
}

/// The dialog-set-map ids bound on Copplemann's slot 48, ascending.
fn bound_at_copplemann(mgr: &SpaceManager) -> Vec<i32> {
    all_binds(mgr)
        .into_iter()
        .find(|(slot, _)| *slot == SLOT)
        .map(|(_, ids)| ids)
        .unwrap_or_default()
}

/// The player's state that a no-op event must leave alone.
fn snapshot(mgr: &SpaceManager) -> (Vec<(i32, Vec<i32>)>, Option<i32>, Option<i8>) {
    (
        all_binds(mgr),
        step_of(mgr),
        mission_of(mgr).map(|m| m.status),
    )
}

/// The player as the destination cell hydrates it after a gate crossing: the
/// saved missions come across, the cell-side binds do not (chain 6504 exists
/// to rebuild them).
async fn arrive(pool: &PgPool, from: &SpaceManager, world: &str) -> SpaceManager {
    let missions = &from.get_entity(ENTITY).unwrap().missions;
    let carried: Vec<MissionInstance> = [701, 703, MISSION]
        .iter()
        .filter_map(|&id| missions.get_mission(id).cloned())
        .collect();
    staged(pool, world, carried).await
}

/// Fire one event against the stage and return everything sent to the
/// cell-to-base channel, as a compact trace in send order.
async fn traffic(fire: impl AsyncFnOnce(&mpsc::Sender<CellToBaseMsg>)) -> Vec<String> {
    let (tx, mut rx) = mpsc::channel(256);
    fire(&tx).await;
    let mut trace: Vec<String> = Vec::new();
    while let Ok(msg) = rx.try_recv() {
        let label = match msg {
            CellToBaseMsg::GrantItem {
                item_id,
                container_id,
                count,
                ..
            } => {
                format!("grant {item_id} x{count} container {container_id}")
            }
            CellToBaseMsg::MissionUpdate {
                mission_id: MISSION,
                current_step_id: Some(step),
                ..
            } => {
                format!("step {step}")
            }
            CellToBaseMsg::EntityMethodCall {
                entity_id: ENTITY,
                method_index: ON_DIALOG_DISPLAY,
                args,
            } => {
                format!(
                    "dialog {}",
                    i32::from_le_bytes(args[4..8].try_into().unwrap())
                )
            }
            CellToBaseMsg::RemoveInventoryItemByType { type_id, count, .. } => {
                format!("remove {type_id} x{count}")
            }
            _ => continue,
        };
        // Only the step echo may repeat (one advance can update twice); a
        // doubled grant, dialog or removal must stay visible in the trace.
        if label.starts_with("step ") && trace.last() == Some(&label) {
            continue;
        }
        trace.push(label);
    }
    trace
}

async fn log_in(engine: &ChainEngine, mgr: &mut SpaceManager, world: &str) -> Vec<String> {
    traffic(async |tx| fire_player_loaded(ENTITY, PLAYER, world, engine, tx, mgr).await).await
}

async fn accept(engine: &ChainEngine, mgr: &mut SpaceManager) -> Vec<String> {
    traffic(async |tx| fire_dialog_choice(ENTITY, PLAYER, 2043, 1796, engine, tx, mgr).await).await
}

async fn click(engine: &ChainEngine, mgr: &mut SpaceManager, tag: &str) -> (bool, Vec<String>) {
    let mut handled = false;
    let trace = traffic(async |tx| {
        handled = fire_interact_tag(ENTITY, PLAYER, tag, CABINET, engine, tx, mgr).await;
    })
    .await;
    (handled, trace)
}

async fn cross(engine: &ChainEngine, mgr: &mut SpaceManager, destination: &str) -> Vec<String> {
    traffic(async |tx| fire_stargate_crossed(ENTITY, PLAYER, destination, engine, tx, mgr).await)
        .await
}

/// The real abandon (removes 567), then the `mission_abandoned` chains.
async fn abandon(engine: &ChainEngine, mgr: &mut SpaceManager) -> Vec<String> {
    let (tx, _rx) = mpsc::channel(256);
    assert!(
        abandon_mission(ENTITY, MISSION, &tx, mgr).await,
        "the abandon must remove 567"
    );
    traffic(async |tx| fire_mission_abandoned(ENTITY, PLAYER, MISSION, engine, tx, mgr).await).await
}

// ── Relog ────────────────────────────────────────────────────────────────

/// **A relog in Castle at step 2000 or 2012 restores nothing.** The cabinet is
/// always interactable and 1397/1400 branch on step state, so neither step
/// needs a bind; the offer chain 1395 must stay quiet because 567 is held.
/// Revert proof: drop 1395's `mission_status 567 not_active` condition and the
/// login rebinds the offer on a running mission.
#[tokio::test]
async fn live_db_mission_567_castle_relog_at_2000_and_2012_restores_nothing() {
    let pool = require_db_or_skip!();
    let engine = engine_with(&pool, &ALL_CHAINS).await;
    for (step, objective) in [(STEP_FETCH, 2338), (STEP_EXIT, 2350)] {
        let mut mgr = staged(&pool, CASTLE, on_the_leg(step, objective)).await;
        let before = snapshot(&mgr);

        let out = log_in(&engine, &mut mgr, CASTLE).await;

        assert!(
            out.is_empty(),
            "step {step}: a relog sends nothing; got {out:?}"
        );
        assert_eq!(
            snapshot(&mgr),
            before,
            "step {step}: a relog changes nothing"
        );
        assert!(
            bound_at_copplemann(&mgr).is_empty(),
            "step {step}: no offer on a held mission"
        );
    }
}

/// **A relog in Castle at step 4039 binds nothing, on purpose.** Dialog 2044
/// must not be offered in world 8: chain 6503 (world 68 only) would not
/// complete the mission from there. Chain 6504 is in the engine, so a wrong
/// firing there would show. Revert proof: key 6504 to `Castle` and drop its
/// world condition, and the login binds 2817 in Castle.
#[tokio::test]
async fn live_db_mission_567_castle_relog_at_4039_binds_nothing_in_castle() {
    let pool = require_db_or_skip!();
    let mut ids = ALL_CHAINS.to_vec();
    ids.extend(HARSET_CHAINS);
    let engine = engine_with(&pool, &ids).await;
    let mut mgr = staged(&pool, CASTLE, on_the_leg(STEP_TURN_IN, 4652)).await;
    let before = snapshot(&mgr);

    let out = log_in(&engine, &mut mgr, CASTLE).await;

    assert!(out.is_empty(), "a relog sends nothing; got {out:?}");
    assert_eq!(snapshot(&mgr), before, "a relog changes nothing");
    assert!(
        bound_at_copplemann(&mgr).is_empty(),
        "no turn-in bind in world 8; got {:?}",
        bound_at_copplemann(&mgr)
    );
}

/// **The offer is restored only for a player who has earned and not taken it,
/// and only in Castle.** 701 alone, 703 alone, a completed 567 and a login
/// whose world is 68 each bind nothing. Revert proof: drop 703 from 1395's
/// conditions and the 701-only case binds; drop its world condition and the
/// world-68 case binds.
#[tokio::test]
async fn live_db_mission_567_castle_relog_refuses_without_the_offer_gate() {
    let pool = require_db_or_skip!();
    let engine = engine_with(&pool, &ALL_CHAINS).await;
    let cases: [(&str, &str, Vec<MissionInstance>); 4] = [
        ("701 only", CASTLE, vec![completed(701)]),
        ("703 only", CASTLE, vec![completed(703)]),
        (
            "567 completed",
            CASTLE,
            vec![completed(701), completed(703), completed(MISSION)],
        ),
        ("world 68", CMD_CENTER, vec![completed(701), completed(703)]),
    ];
    for (label, world, missions) in cases {
        let mut mgr = staged(&pool, world, missions).await;
        // The trigger key is Castle in every case; only the gates differ.
        log_in(&engine, &mut mgr, CASTLE).await;
        assert!(
            bound_at_copplemann(&mgr).is_empty(),
            "{label}: the login must not offer 567; got {:?}",
            bound_at_copplemann(&mgr)
        );
    }
}

// ── Refusals ─────────────────────────────────────────────────────────────

/// **A cabinet click in the wrong world, or with no 567 to advance, grants
/// nothing and moves nothing.** In world 68 no chain answers at all; with 567
/// not active or completed only the sealed feedback (10734) shows. Revert
/// proof: drop 1397's world condition and the world-68 click grants; drop
/// its `step_status` condition and the not-active click grants.
#[tokio::test]
async fn live_db_mission_567_castle_cabinet_refuses_wrong_world_and_state() {
    let pool = require_db_or_skip!();
    let engine = engine_with(&pool, &ALL_CHAINS).await;

    let mut wrong_world = staged(&pool, CMD_CENTER, on_the_leg(STEP_FETCH, 2338)).await;
    let (handled, out) = click(&engine, &mut wrong_world, "Castle_FileCabinet").await;
    assert!(!handled, "no chain may answer the cabinet in world 68");
    assert!(out.is_empty(), "world 68 must send nothing; got {out:?}");
    assert_eq!(
        step_of(&wrong_world),
        Some(STEP_FETCH),
        "the step must hold"
    );

    let cases = [
        ("not active", vec![completed(701), completed(703)]),
        ("completed", vec![completed(MISSION)]),
    ];
    for (label, missions) in cases {
        let mut mgr = staged(&pool, CASTLE, missions).await;
        let before = snapshot(&mgr);
        let (handled, out) = click(&engine, &mut mgr, "Castle_FileCabinet").await;
        assert!(handled, "{label}: the feedback chain must answer");
        assert_eq!(out, ["dialog 10734"], "{label}: feedback only, no grant");
        assert_eq!(snapshot(&mgr), before, "{label}: nothing may move");
    }
}

/// **Dialog 2043 refuses an accept it should not take.** A second accept
/// while 567 runs, an accept after completion, an accept without 703, and an
/// accept in world 68 each send nothing and leave 567 where it was. Revert
/// proof: drop 1396's `mission_status 567 not_active` condition and the
/// running case is accepted again; drop its 703 condition and the 701-only
/// case accepts.
#[tokio::test]
async fn live_db_mission_567_castle_accept_refuses_outside_the_offer() {
    let pool = require_db_or_skip!();
    let engine = engine_with(&pool, &ALL_CHAINS).await;
    let cases: [(&str, &str, Vec<MissionInstance>); 4] = [
        ("567 running", CASTLE, on_the_leg(STEP_EXIT, 2350)),
        (
            "567 completed",
            CASTLE,
            vec![completed(701), completed(703), completed(MISSION)],
        ),
        ("701 only", CASTLE, vec![completed(701)]),
        ("world 68", CMD_CENTER, vec![completed(701), completed(703)]),
    ];
    for (label, world, missions) in cases {
        let mut mgr = staged(&pool, world, missions).await;
        let before = snapshot(&mgr);
        let out = accept(&engine, &mut mgr).await;
        assert!(out.is_empty(), "{label}: must send nothing; got {out:?}");
        assert_eq!(snapshot(&mgr), before, "{label}: 567 must not move");
    }
}

/// **The Harset crossing refuses every state but step 2012 in Castle.** With
/// 567 not active it only closes the offer (10240); at 2000 and at 4039 it
/// sends nothing. Revert proof: drop 10241's `step_status 2012` condition and
/// the 2000 case advances.
#[tokio::test]
async fn live_db_mission_567_castle_crossing_refuses_the_wrong_step() {
    let pool = require_db_or_skip!();
    let engine = engine_with(&pool, &ALL_CHAINS).await;
    let cases = [
        ("step 2000", on_the_leg(STEP_FETCH, 2338)),
        ("step 4039", on_the_leg(STEP_TURN_IN, 4652)),
        ("not active", vec![completed(701), completed(703)]),
    ];
    for (label, missions) in cases {
        let mut mgr = staged(&pool, CASTLE, missions).await;
        let before = snapshot(&mgr);
        let out = cross(&engine, &mut mgr, "Harset").await;
        assert!(out.is_empty(), "{label}: must send nothing; got {out:?}");
        assert_eq!(snapshot(&mgr), before, "{label}: nothing may move");
    }
}

mod harset;
mod offer_and_abandon;
