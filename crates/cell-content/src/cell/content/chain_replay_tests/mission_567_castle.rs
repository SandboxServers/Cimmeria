//! Mission 567 "Romney's Files", the Castle leg (CS-08b1): the chains
//! `castle_567_chains.sql` seeds as 1394-1397, 1400 and 10240-10243.
//!
//! The offer binds Copplemann (1000018 at slot 48) once 701 and 703 are done.
//! The accept (1396) unbinds the offer and accepts 567. The cabinet click at
//! step 2000 (1397) grants item 2698, advances to 2012 and shows monologue
//! 10733; every other click (1400) shows 10734. The Harset crossing at 2012
//! (10241) advances to 4039. Every event goes through the real dispatchers
//! (`fire_dialog_choice`, `fire_interact_tag`, `fire_stargate_crossed`), so a
//! resolve-only check cannot pass a broken executor arm.
//!
//! Chains 6503 and 6504 (the Harset turn-in) stay with the sibling
//! `mission_567` module.

use cimmeria_content_engine::chain::ChainEngine;
use cimmeria_entity::missions::{
    MissionInstance, MissionObjective, MISSION_ACTIVE, MISSION_COMPLETED, STATUS_ACTIVE,
};
use sqlx::PgPool;
use tokio::sync::mpsc;

use super::super::engine_loader::load_single_chain_for_test;
use super::super::event_dispatch::{fire_dialog_choice, fire_stargate_crossed};
use super::super::{fire_interact_tag, fire_player_loaded};
use crate::cell::messages::CellToBaseMsg;
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
/// The offer: dialog-set map 1000018 bound at Copplemann's slot 48.
const OFFER_SET: i32 = 1000018;
const OFFER_SLOT: i32 = 48;
const FOUND_DIALOG: i32 = 10733;
const EMPTY_DIALOG: i32 = 10734;
/// Wire index of the client method onDialogDisplay.
const ON_DIALOG_DISPLAY: u16 = 105;
const ALL_CHAINS: [i32; 9] = [1394, 1395, 1396, 1397, 1400, 10240, 10241, 10242, 10243];

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

/// `world` with player `ENTITY` holding `missions` and the cabinet actor,
/// with the dialog-set and monologue caches loaded from the seed.
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

fn step_of(mgr: &SpaceManager) -> Option<i32> {
    let player = mgr.get_entity(ENTITY).unwrap();
    player
        .missions
        .get_mission(MISSION)
        .and_then(|m| m.current_step_id)
}

/// The dialog-set-map ids bound on the player at `slot`, ascending.
fn bound_at(mgr: &SpaceManager, slot: i32) -> Vec<i32> {
    let player = mgr.get_entity(ENTITY).unwrap();
    let mut ids: Vec<i32> = player
        .available_interactions
        .get(&slot)
        .map(|v| v.iter().map(|&(id, _, _)| id).collect())
        .unwrap_or_default();
    ids.sort_unstable();
    ids
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
            CellToBaseMsg::RemoveInventoryItemByType { type_id, .. } => format!("remove {type_id}"),
            _ => continue,
        };
        if trace.last() != Some(&label) {
            trace.push(label);
        }
    }
    trace
}

/// Click the cabinet; returns whether a chain handled it and the trace.
async fn click_cabinet(engine: &ChainEngine, mgr: &mut SpaceManager) -> (bool, Vec<String>) {
    let mut handled = false;
    let trace = traffic(async |tx| {
        handled = fire_interact_tag(
            ENTITY,
            PLAYER,
            "Castle_FileCabinet",
            CABINET,
            engine,
            tx,
            mgr,
        )
        .await;
    })
    .await;
    (handled, trace)
}

/// Cross the gate to `destination`; returns the trace.
async fn cross(engine: &ChainEngine, mgr: &mut SpaceManager, destination: &str) -> Vec<String> {
    traffic(async |tx| fire_stargate_crossed(ENTITY, PLAYER, destination, engine, tx, mgr).await)
        .await
}

// ── Seed shape ───────────────────────────────────────────────────────────

/// Debug text of the sealed `world = 8` condition.
const WORLD_8: &str = "World { operator: Eq, world_id: 8 }";
/// Debug text of the offer bind and unbind actions.
const BIND: &str = "AddDialogSet { dialog_set_id: 1000018, slot: 48, mission_id: None }";
const UNBIND: &str = "RemoveDialogSet { dialog_set_id: 1000018, slot: 48 }";

/// Debug text of a `mission_status eq <status>` condition.
fn mission_is(mission_id: i32, status: &str) -> String {
    format!("MissionStatus {{ mission_id: {mission_id}, operator: Eq, expected_status: {status} }}")
}

/// Debug text of a `step_status <op> active` condition on 567.
fn step_is(step_id: i32, op: &str) -> String {
    format!("StepStatus {{ mission_id: 567, step_id: {step_id}, operator: {op}, expected_status: Active }}")
}

/// **Chains 1394-1397, 1400 and 10240-10243 load with the sealed trigger,
/// every condition in order and the exact action list.** Each chain is
/// compared whole, plus priority 0, not `once` and no delays. The shipped
/// enabled flags are pinned in `mission_567`.
/// Revert proof: delete a chain, a trigger row or an action row, drop
/// 10240's `not_active` condition or swap 1396's action order, and this
/// comparison fails.
#[tokio::test]
async fn live_db_mission_567_castle_chains_load() {
    let pool = require_db_or_skip!();
    let gate = || {
        vec![
            WORLD_8.to_string(),
            mission_is(701, "Completed"),
            mission_is(703, "Completed"),
            mission_is(MISSION, "NotActive"),
        ]
    };
    let active = |step| {
        vec![
            WORLD_8.to_string(),
            mission_is(MISSION, "Active"),
            step_is(step, "Eq"),
        ]
    };
    let accept = "AcceptMission { mission_id: 567 }";
    let grant = "GrantItem { item_id: 2698, count: 1, container_id: Some(2) }";
    let advance = |step| format!("AdvanceStep {{ mission_id: 567, step_id: {step} }}");
    let show = |id| format!("DisplayDialog {{ dialog_id: {id} }}");
    let cabinet = "OnInteractTag { entity_tag: \"Castle_FileCabinet\" }";
    let abandoned = "OnMissionAbandoned { mission_id: 567 }";
    let remove_turn_in = "RemoveDialogSet { dialog_set_id: 2817, slot: 48 }";
    let remove_item = "RemoveItem { item_id: 2698, count: 1 }";
    // (chain, trigger Debug, conditions in order, actions in order)
    let sealed: [(i32, &str, Vec<String>, Vec<String>); 9] = [
        (
            1394,
            "OnMissionCompleted { mission_id: 703 }",
            gate(),
            vec![BIND.into()],
        ),
        (
            1395,
            "OnPlayerLoaded { world_name: Some(\"Castle\") }",
            gate(),
            vec![BIND.into()],
        ),
        (
            1396,
            "OnDialogChoice { dialog_id: 2043 }",
            gate(),
            vec![UNBIND.into(), accept.into()],
        ),
        (
            1397,
            cabinet,
            active(STEP_FETCH),
            vec![grant.into(), advance(STEP_EXIT), show(FOUND_DIALOG)],
        ),
        (
            1400,
            cabinet,
            vec![WORLD_8.to_string(), step_is(STEP_FETCH, "Neq")],
            vec![show(EMPTY_DIALOG)],
        ),
        (
            10240,
            "OnStargateCrossed { destination_world: None }",
            vec![WORLD_8.to_string(), mission_is(MISSION, "NotActive")],
            vec![UNBIND.into()],
        ),
        (
            10241,
            "OnStargateCrossed { destination_world: Some(\"Harset\") }",
            active(STEP_EXIT),
            vec![advance(STEP_TURN_IN)],
        ),
        (
            10242,
            abandoned,
            vec![mission_is(MISSION, "NotActive")],
            vec![remove_turn_in.into(), remove_item.into()],
        ),
        (10243, abandoned, gate(), vec![BIND.into()]),
    ];
    for (id, trigger, conditions, actions) in sealed {
        let chain = load_single_chain_for_test(&pool, id)
            .await
            .unwrap_or_else(|e| panic!("DB query for chain {id} must succeed: {e}"))
            .unwrap_or_else(|| panic!("chain {id} must exist in seeded content_chains"));
        assert_eq!(chain.priority, 0, "chain {id} priority");
        assert!(!chain.once, "chain {id} must not be once");
        assert_eq!(
            format!("{:?}", chain.trigger),
            trigger,
            "chain {id} trigger"
        );
        let got: Vec<String> = chain.conditions.iter().map(|c| format!("{c:?}")).collect();
        assert_eq!(got, conditions, "chain {id} conditions in sort order");
        let got: Vec<String> = chain.actions.iter().map(|a| format!("{a:?}")).collect();
        assert_eq!(got, actions, "chain {id} actions in sort order");
        assert!(
            chain.action_delays.iter().all(|&d| d == 0),
            "chain {id} must have no delays"
        );
    }
}

// ── The offer and the accept ─────────────────────────────────────────────

/// **The login binds the offer, and Accept on dialog 2043 unbinds it and
/// accepts 567 at step 2000.** 701 and 703 done, 567 not_active: 1395 binds
/// map 1000018 on slot 48; `dialog_choice 2043` runs 1396. Revert proof:
/// delete 1395's action and the bind assertion fails; delete 1396's accept
/// and 567 is never at step 2000.
#[tokio::test]
async fn live_db_mission_567_castle_offer_binds_and_accept_removes_it() {
    let pool = require_db_or_skip!();
    let engine = engine_with(&pool, &ALL_CHAINS).await;
    let mut mgr = staged(&pool, "Castle", vec![completed(701), completed(703)]).await;
    assert!(
        bound_at(&mgr, OFFER_SLOT).is_empty(),
        "nothing bound before login"
    );
    let (tx, _rx) = mpsc::channel(256);
    fire_player_loaded(ENTITY, PLAYER, "Castle", &engine, &tx, &mut mgr).await;
    assert_eq!(
        bound_at(&mgr, OFFER_SLOT),
        vec![OFFER_SET],
        "the login binds the offer"
    );

    let out = traffic(async |tx| {
        fire_dialog_choice(ENTITY, PLAYER, 2043, 1796, &engine, tx, &mut mgr).await;
    })
    .await;

    assert_eq!(out, ["step 2000"], "the accept persists 567 at step 2000");
    assert_eq!(step_of(&mgr), Some(STEP_FETCH), "567 must be at step 2000");
    let player = mgr.get_entity(ENTITY).unwrap();
    assert_eq!(
        player.missions.get_mission(MISSION).unwrap().status,
        MISSION_ACTIVE
    );
    assert!(
        bound_at(&mgr, OFFER_SLOT).is_empty(),
        "the offer is unbound after the accept"
    );
}

// ── The cabinet ──────────────────────────────────────────────────────────

/// **The cabinet click at step 2000 grants 2698, advances to 2012 and shows
/// 10733, in that order; the next click grants nothing.** The whole trace is
/// compared, so order and exact content are pinned. The second click
/// resolves with 2000 no longer active, so only 1400 answers. Revert proof:
/// delete the `add_item` row from 1397 and the grant is missing; swap 1397's
/// actions and the order differs; drop 1397's `step_status 2000` condition
/// and the second click grants again.
#[tokio::test]
async fn live_db_mission_567_castle_cabinet_grants_then_advances() {
    let pool = require_db_or_skip!();
    let engine = engine_with(&pool, &ALL_CHAINS).await;
    let mut mgr = staged(&pool, "Castle", vec![mission_567(STEP_FETCH, 2338)]).await;

    let (handled, out) = click_cabinet(&engine, &mut mgr).await;

    assert!(handled, "chain 1397 must handle the click at step 2000");
    assert_eq!(
        out,
        ["grant 2698 x1 container 2", "step 2012", "dialog 10733"]
    );
    assert_eq!(step_of(&mgr), Some(STEP_EXIT), "567 must be at step 2012");

    let (handled, again) = click_cabinet(&engine, &mut mgr).await;

    assert!(handled, "chain 1400 must answer the second click");
    assert_eq!(again, ["dialog 10734"], "no grant, no step change");
    assert_eq!(step_of(&mgr), Some(STEP_EXIT), "the step must stay at 2012");
}

/// **A cabinet click at step 2012 grants nothing and shows the sealed
/// feedback.** Chain 1400 is loaded as shipped, with no enabled override.
/// Revert proof: set 1400 `enabled = false`, delete it, or flip its `neq` to
/// `eq` and the click shows no dialog; drop 1397's `step_status` condition and
/// the click grants a second file.
#[tokio::test]
async fn live_db_mission_567_castle_repeat_click_at_2012_grants_nothing() {
    let pool = require_db_or_skip!();
    let engine = engine_with(&pool, &ALL_CHAINS).await;
    let mut mgr = staged(&pool, "Castle", vec![mission_567(STEP_EXIT, 2350)]).await;

    let (handled, out) = click_cabinet(&engine, &mut mgr).await;

    assert!(handled, "chain 1400 must answer the click at 2012");
    assert_eq!(
        out,
        ["dialog 10734"],
        "monologue only: no grant, no step change"
    );
    assert_eq!(step_of(&mgr), Some(STEP_EXIT), "the step must stay at 2012");
}

/// **Exactly one of 1397 and 1400 answers a cabinet click in every state.**
/// Step 2000 active gets 10733 only; step 2012, no mission and a completed
/// 567 get 10734 only. Revert proof: change 1400's `neq` to `eq` and both
/// fire on the grant click.
#[tokio::test]
async fn live_db_mission_567_castle_cabinet_chains_are_disjoint() {
    let pool = require_db_or_skip!();
    let engine = engine_with(&pool, &ALL_CHAINS).await;
    let cases: [(&str, Vec<MissionInstance>, i32); 4] = [
        (
            "step 2000",
            vec![mission_567(STEP_FETCH, 2338)],
            FOUND_DIALOG,
        ),
        (
            "step 2012",
            vec![mission_567(STEP_EXIT, 2350)],
            EMPTY_DIALOG,
        ),
        ("no mission", Vec::new(), EMPTY_DIALOG),
        ("completed", vec![completed(MISSION)], EMPTY_DIALOG),
    ];
    for (label, missions, expected) in cases {
        let mut mgr = staged(&pool, "Castle", missions).await;
        let (handled, out) = click_cabinet(&engine, &mut mgr).await;
        assert!(handled, "{label}: one chain must answer");
        let shown: Vec<_> = out.iter().filter(|l| l.starts_with("dialog")).collect();
        assert_eq!(
            shown,
            [&format!("dialog {expected}")],
            "{label}: exactly one monologue"
        );
    }
}

// ── The crossing ─────────────────────────────────────────────────────────

/// **Crossing to Harset from Castle at step 2012 advances 567 to 4039 and
/// touches no item.** From Harset (world 57), at step 2000, to another
/// destination, or a second time after the hop, nothing fires. Revert proof:
/// remove 10241's world-8 condition and the world-57 case advances; remove
/// its `step_status 2012` condition and the step-2000 case advances.
#[tokio::test]
async fn live_db_mission_567_castle_crossing_advances_to_4039() {
    let pool = require_db_or_skip!();
    let engine = engine_with(&pool, &ALL_CHAINS).await;
    let mut mgr = staged(&pool, "Castle", vec![mission_567(STEP_EXIT, 2350)]).await;

    let out = cross(&engine, &mut mgr, "Harset").await;

    assert_eq!(
        out,
        ["step 4039"],
        "the hop persists 4039 and touches no item"
    );
    assert_eq!(step_of(&mgr), Some(STEP_TURN_IN), "567 must be at 4039");
    let again = cross(&engine, &mut mgr, "Harset").await;
    assert!(
        again.is_empty(),
        "a second crossing at 4039 fires nothing; got {again:?}"
    );

    let negatives = [
        ("from Harset", "Harset", STEP_EXIT, "Harset"),
        ("at step 2000", "Castle", STEP_FETCH, "Harset"),
        ("other destination", "Castle", STEP_EXIT, "Abydos"),
    ];
    for (label, world, step, destination) in negatives {
        let objective = if step == STEP_FETCH { 2338 } else { 2350 };
        let mut mgr = staged(&pool, world, vec![mission_567(step, objective)]).await;
        let out = cross(&engine, &mut mgr, destination).await;
        assert!(out.is_empty(), "{label}: must fire nothing; got {out:?}");
        assert_eq!(step_of(&mgr), Some(step), "{label}: step must not move");
    }
}
