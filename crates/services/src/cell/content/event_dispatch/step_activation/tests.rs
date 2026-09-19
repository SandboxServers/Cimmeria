//! Guards for the H52 step-activation region replay.
//!
//! Where the behaviour under test is "the replay happens at all", the test
//! fires through the **executor arm** rather than calling
//! [`fire_step_activation_regions`] directly: the hook inside that arm is the
//! thing that regresses, and a direct call would keep passing after it is
//! deleted.

use std::collections::HashMap;

use tokio::sync::mpsc;

use cimmeria_content_engine::actions::Action;
use cimmeria_content_engine::chain::{Chain, ChainEngine, ResolvedActions};
use cimmeria_content_engine::conditions::{ComparisonOp, Condition, StepStatusValue};
use cimmeria_content_engine::triggers::Trigger;
use cimmeria_entity::missions::{MissionInstance, MissionObjective, STATUS_ACTIVE};

use super::*;
use crate::cell::ring_transport::RingRegion;
use crate::cell::space_manager::{RegionData, REGION_FLAG_STARGATE};
use crate::cell::spawner::{MissionDefEntry, MissionObjectiveDef, WorldRow};

const PLAYER_EID: u32 = 1;
const PLAYER_ID: i32 = 100;
const HARSET: i32 = 57;
const HARSET_CMD_CENTER: i32 = 68;

/// Chain ids sit in the reserved `0x7000_xxxx` test range so they can never
/// collide with a seed row; `0x7005_2xxx` is this packet's block.
const FIRING_CHAIN: i64 = 0x7005_2000;
const CHAIN_INSIDE: i64 = 0x7005_2001;
const CHAIN_UNGATED: i64 = 0x7005_2002;
const CHAIN_LEG_A: i64 = 0x7005_2003;
const CHAIN_LEG_B: i64 = 0x7005_2004;
const CHAIN_RING: i64 = 0x7005_2005;
const CHAIN_INVISIBLE: i64 = 0x7005_2006;

const MISSION: i32 = 1343;
const STEP_ONE: i32 = 4401;
const STEP_TWO: i32 = 4402;
const STEP_THREE: i32 = 4403;
const OBJ: i32 = 5401;

const JAFFA_ZONE: &str = "Harset.JaffaZone";
const STORAGE: &str = "Harset.Storage";
const RING_PAD: &str = "Harset.RingLeftBottom";
const INVISIBLE: &str = "Harset.Invisible";

/// The ring region and point set the [`RING_PAD`] volume belongs to. The
/// point set id must match `region(3, ..).db_set_id`, which is how
/// `ring_transport::handle_region_trigger` finds the transporter.
const RING_REGION_ID: i32 = 4;
const RING_POINT_SET_ID: i32 = 2003;

/// A 20x20 volume centred on `(cx, cz)` with the loader's asymmetric `py + h`
/// fourth corner — the exact shape `load_regions_from_db` produces when it
/// expands a cylinder.
fn square(cx: f32, cz: f32) -> Vec<[f32; 3]> {
    vec![
        [cx - 10.0, 0.0, cz - 10.0],
        [cx - 10.0, 0.0, cz + 10.0],
        [cx + 10.0, 0.0, cz + 10.0],
        [cx + 10.0, 4.0, cz - 10.0],
    ]
}

fn region(runtime_id: u32, tag: &str, cx: f32, cz: f32, flags: i32) -> RegionData {
    RegionData {
        runtime_id,
        db_set_id: runtime_id as i32 + 2000,
        tag: tag.to_string(),
        world_name: "Harset".to_string(),
        height: 4.0,
        radius: 10.0,
        flags,
        points: square(cx, cz),
    }
}

/// Harset plus its Command Center, four registered volumes, and one connected
/// player at `pos`.
///
/// The Jaffa Zone and the ring pad both sit on the origin (so one position is
/// inside two volumes, which the re-entrancy test needs); Storage is 40 units
/// east, far enough outside the 10-unit half-width plus the 1.5-unit slop to
/// be a real negative.
fn make_mgr(pos: [f32; 3]) -> SpaceManager {
    let mut mgr = SpaceManager::new(1);
    mgr.parse_spaces_xml(
        r#"<?xml version="1.0"?><Spaces>
        <Space WorldName="Harset" Instanced="false" MinX="-800" MaxX="800" MinY="-800" MaxY="800" />
        <Space WorldName="Harset_CmdCenter" Instanced="false" MinX="-800" MaxX="800" MinY="-800" MaxY="800" />
    </Spaces>"#,
    )
    .unwrap();
    mgr.create_startup_spaces(
        r#"<?xml version="1.0"?><Spaces>
        <Space WorldName="Harset" />
        <Space WorldName="Harset_CmdCenter" />
    </Spaces>"#,
    )
    .unwrap();
    mgr.stamp_world_rows(&HashMap::from([
        ("Harset".to_string(), WorldRow::enforcing(HARSET)),
        (
            "Harset_CmdCenter".to_string(),
            WorldRow::enforcing(HARSET_CMD_CENTER),
        ),
    ]));

    mgr.regions.insert(
        1,
        region(1, JAFFA_ZONE, 0.0, 0.0, REGION_FLAG_CLIENT_HINTED),
    );
    mgr.regions
        .insert(2, region(2, STORAGE, 40.0, 0.0, REGION_FLAG_CLIENT_HINTED));
    // Both a ring pad and a stargate volume, so one fixture covers both halves
    // of the "content chains only" hazard.
    mgr.regions.insert(
        3,
        region(
            3,
            RING_PAD,
            0.0,
            0.0,
            REGION_FLAG_CLIENT_HINTED | REGION_FLAG_STARGATE,
        ),
    );
    // Registered server-side but never handed to the client.
    mgr.regions.insert(4, region(4, INVISIBLE, 0.0, 0.0, 0));
    mgr.next_region_id = 5;

    mgr.create_entity(PLAYER_EID, "Harset", pos, [0.0; 3])
        .unwrap();
    if let Some(e) = mgr.get_entity_mut(PLAYER_EID) {
        e.is_player = true;
        e.player_id = Some(PLAYER_ID);
    }
    mgr.connect_entity(PLAYER_EID);
    mgr
}

/// Make [`RING_PAD`]'s point set a live ring transporter, so an assertion
/// that the ring FSM stayed untouched is not vacuous.
fn arm_ring(mgr: &mut SpaceManager) {
    let region = RingRegion {
        region_id: RING_REGION_ID,
        world_id: HARSET,
        world_name: "Harset".to_string(),
        x: 0.0,
        y: 0.0,
        z: 0.0,
        tag: RING_PAD.to_string(),
        height: 4.0,
        radius: 10.0,
        event_set_id: 100,
        display_name_id: 7508,
        destination_ids: vec![5],
        point_set_id: RING_POINT_SET_ID,
        required_mission_id: None,
    };
    let regions = HashMap::from([(RING_REGION_ID, region)]);
    mgr.ring_transporters.load(&regions);
    mgr.ring_point_set_to_region
        .insert(RING_POINT_SET_ID, RING_REGION_ID);
    mgr.ring_regions = regions;
}

fn ring_occupants(mgr: &SpaceManager) -> usize {
    mgr.ring_transporters
        .get(RING_REGION_ID)
        .expect("the ring fixture must be loaded")
        .players
        .len()
}

/// Register `MISSION` so the executor's accept arm can resolve its first step.
fn seed_mission_def(mgr: &mut SpaceManager, step_id: i32) {
    mgr.mission_defs.insert(
        MISSION,
        MissionDefEntry {
            step_id,
            objectives: vec![MissionObjectiveDef {
                objective_id: OBJ,
                is_hidden: false,
                is_optional: false,
            }],
            is_hidden: false,
            num_repeats: 0,
            can_repeat_on_fail: false,
        },
    );
}

/// Put the mission on the player, already active at `step_id` — the state
/// `advance_step` needs in order to have something to move.
fn give_mission(mgr: &mut SpaceManager, step_id: i32) {
    let e = mgr.get_entity_mut(PLAYER_EID).unwrap();
    e.missions.add_mission(MissionInstance::new(
        MISSION,
        step_id,
        vec![MissionObjective {
            objective_id: OBJ,
            status: STATUS_ACTIVE,
            hidden: false,
            optional: false,
        }],
    ));
}

fn chain(id: i64, region_key: &str, conditions: Vec<Condition>, actions: Vec<Action>) -> Chain {
    Chain {
        id,
        name: format!("test: replay {id}"),
        enabled: true,
        trigger: Trigger::OnRegionEnter {
            region_key: region_key.to_string(),
        },
        conditions,
        actions,
        action_delays: Vec::new(),
        priority: 0,
    }
}

fn step_gate(step_id: i32) -> Condition {
    Condition::StepStatus {
        mission_id: MISSION,
        step_id,
        operator: ComparisonOp::Eq,
        expected_status: StepStatusValue::Active,
    }
}

fn bump(name: &str) -> Action {
    Action::IncrementCounter {
        counter_name: name.to_string(),
        amount: 1,
    }
}

fn advance(step_id: i32) -> Action {
    Action::AdvanceStep {
        mission_id: MISSION,
        step_id,
    }
}

fn counter(mgr: &SpaceManager, name: &str) -> i32 {
    mgr.get_entity(PLAYER_EID)
        .and_then(|e| e.counters.get(name).copied())
        .unwrap_or(0)
}

/// Run one action through the real executor — the production hook lives in
/// its `AdvanceStep` / `AcceptMission` arms, so deleting the hook fails every
/// test that goes through here.
///
/// The receiver is held for the whole call: an unread `mpsc` of this size
/// never fills, and dropping it early would turn every `MissionUpdate` send
/// into an error path the tests are not about.
async fn run_action(mgr: &mut SpaceManager, engine: &ChainEngine, action: Action) {
    let (tx, _rx) = mpsc::channel::<CellToBaseMsg>(8192);
    let resolved = ResolvedActions {
        actions: vec![(FIRING_CHAIN, action)],
        action_delays: vec![0],
        params: HashMap::new(),
    };
    executor::execute_actions(resolved, PLAYER_EID, PLAYER_ID, &tx, mgr, engine).await;
}

/// The client's own `triggerClientHintedGenericRegion`, minus the wire layer.
async fn client_hint(mgr: &mut SpaceManager, engine: &ChainEngine, tag: &str) {
    let (tx, _rx) = mpsc::channel::<CellToBaseMsg>(8192);
    crate::cell::content::fire_enter_region(PLAYER_EID, PLAYER_ID, tag, engine, &tx, mgr).await;
}

// ── The headline behaviour ────────────────────────────────────────────────

/// A step-gated `enter_region` chain resolves when the step activates with the
/// player already standing in the volume.
///
/// Bug shape (playtest finding H9): the client's real hint arrived *before*
/// the step was active, the chain's `step_status` gate failed, and the edge
/// was spent for good. Reverting the replay hook in
/// `executor::mission::advance_step` leaves the counter at 0.
#[tokio::test]
async fn step_activation_replays_a_region_the_player_is_standing_in() {
    let mut mgr = make_mgr([0.0, 1.0, 0.0]);
    give_mission(&mut mgr, STEP_ONE);

    let mut engine = ChainEngine::new();
    engine.register_chain(chain(
        CHAIN_INSIDE,
        JAFFA_ZONE,
        vec![step_gate(STEP_TWO)],
        vec![bump("replayed")],
    ));

    run_action(&mut mgr, &engine, advance(STEP_TWO)).await;

    assert_eq!(
        counter(&mgr, "replayed"),
        1,
        "a step-gated enter_region chain must fire when its step activates with \
         the player inside the volume; 0 means the replay hook is gone"
    );
    assert!(
        mgr.step_region_replay.is_idle(),
        "the re-entrancy guard must be balanced after the replay"
    );
}

/// The control for the test above: the same chain must not fire for a player
/// standing outside the volume. Without this, a replay that fired every region
/// registered for the world would also pass.
#[tokio::test]
async fn step_activation_does_not_replay_a_region_the_player_is_outside() {
    let mut mgr = make_mgr([40.0, 1.0, 0.0]);
    give_mission(&mut mgr, STEP_ONE);

    let mut engine = ChainEngine::new();
    engine.register_chain(chain(
        CHAIN_INSIDE,
        JAFFA_ZONE,
        vec![step_gate(STEP_TWO)],
        vec![bump("replayed")],
    ));

    run_action(&mut mgr, &engine, advance(STEP_TWO)).await;

    assert_eq!(
        counter(&mgr, "replayed"),
        0,
        "the replay must test server-known containment, not fire every region \
         registered for the world"
    );
}

/// Accepting a mission activates its first step, so the accept path replays
/// too — 742's offer, 1326's offer and the 1343 patrol's first leg all hand
/// out a step inside the volume the player is already standing in.
#[tokio::test]
async fn accepting_a_mission_replays_the_first_steps_regions() {
    let mut mgr = make_mgr([0.0, 1.0, 0.0]);
    seed_mission_def(&mut mgr, STEP_ONE);

    let mut engine = ChainEngine::new();
    engine.register_chain(chain(
        CHAIN_INSIDE,
        JAFFA_ZONE,
        vec![step_gate(STEP_ONE)],
        vec![bump("accept_replayed")],
    ));

    run_action(
        &mut mgr,
        &engine,
        Action::AcceptMission {
            mission_id: MISSION,
        },
    )
    .await;

    assert_eq!(
        counter(&mgr, "accept_replayed"),
        1,
        "accept_mission activates step 1, so its enter_region chains must be replayed"
    );
}

// ── Hazard (b): the double fire ───────────────────────────────────────────

/// A mission-gated chain is idempotent under a double delivery: the replay
/// moves the state its own gate reads, so the client's real hint arriving a
/// moment later finds the gate closed. This is the property that makes the
/// replay safe, and the reason the filter admits only such chains.
#[tokio::test]
async fn a_step_gated_chain_does_not_double_fire_when_the_client_hint_arrives() {
    let mut mgr = make_mgr([0.0, 1.0, 0.0]);
    give_mission(&mut mgr, STEP_ONE);

    let mut engine = ChainEngine::new();
    engine.register_chain(chain(
        CHAIN_INSIDE,
        JAFFA_ZONE,
        vec![step_gate(STEP_TWO)],
        vec![bump("gated"), advance(STEP_THREE)],
    ));

    run_action(&mut mgr, &engine, advance(STEP_TWO)).await;
    assert_eq!(counter(&mgr, "gated"), 1, "the replay fires it once");

    client_hint(&mut mgr, &engine, JAFFA_ZONE).await;

    assert_eq!(
        counter(&mgr, "gated"),
        1,
        "the client's real hint must find the gate closed — a second bump means \
         the replay is not idempotent and the double fire is real"
    );
}

/// A chain with **no** mission gate is refused by the replay, because a double
/// delivery would run it twice — a bare `enter_region` → `display_dialog`
/// would show the dialog twice. Ordinary dispatch is unaffected.
#[tokio::test]
async fn an_ungated_chain_is_refused_by_the_replay_but_not_by_the_client_hint() {
    let mut mgr = make_mgr([0.0, 1.0, 0.0]);
    give_mission(&mut mgr, STEP_ONE);

    let mut engine = ChainEngine::new();
    engine.register_chain(chain(
        CHAIN_UNGATED,
        JAFFA_ZONE,
        // `world` looks mission-adjacent and is deliberately NOT a mission
        // gate: its value does not change when the chain runs, so it makes a
        // re-fire no safer.
        vec![Condition::World {
            operator: ComparisonOp::Eq,
            world_id: HARSET,
        }],
        vec![bump("ungated")],
    ));

    run_action(&mut mgr, &engine, advance(STEP_TWO)).await;
    assert_eq!(
        counter(&mgr, "ungated"),
        0,
        "an ungated chain must not be replayed — running it twice is exactly the \
         hazard the mission-gate filter exists to avoid"
    );

    client_hint(&mut mgr, &engine, JAFFA_ZONE).await;
    assert_eq!(
        counter(&mgr, "ungated"),
        1,
        "ordinary dispatch must be unaffected by the replay's filter"
    );
}

// ── Hazard (a): re-entrancy ───────────────────────────────────────────────

/// Two overlapping volumes and two steps, each chain advancing into the
/// other's step. Unbounded this recurses until the stack dies; bounded it
/// terminates and leaves the guard idle.
///
/// The player stands on the origin, inside both the Jaffa Zone and the ring
/// pad volume, so both chains are candidates on every activation.
#[tokio::test]
async fn a_two_region_two_step_ping_pong_terminates() {
    let mut mgr = make_mgr([0.0, 1.0, 0.0]);
    give_mission(&mut mgr, STEP_ONE);

    let mut engine = ChainEngine::new();
    // Step TWO + standing in the Jaffa Zone → back to step ONE.
    engine.register_chain(chain(
        CHAIN_LEG_A,
        JAFFA_ZONE,
        vec![step_gate(STEP_TWO)],
        vec![bump("leg_a"), advance(STEP_ONE)],
    ));
    // Step ONE + standing on the ring pad → forward to step TWO.
    engine.register_chain(chain(
        CHAIN_LEG_B,
        RING_PAD,
        vec![step_gate(STEP_ONE)],
        vec![bump("leg_b"), advance(STEP_TWO)],
    ));

    run_action(&mut mgr, &engine, advance(STEP_TWO)).await;

    // The exact counts follow from MAX_REPLAY_DEPTH and are not the contract.
    // Termination and a balanced guard are.
    assert!(
        counter(&mgr, "leg_a") >= 1,
        "the first leg must run at least once, or the fixture proves nothing"
    );
    assert!(
        counter(&mgr, "leg_a") + counter(&mgr, "leg_b") <= 2 * MAX_REPLAY_DEPTH as i32,
        "the ping-pong must be bounded by the depth guard; got a={} b={}",
        counter(&mgr, "leg_a"),
        counter(&mgr, "leg_b"),
    );
    assert!(
        mgr.step_region_replay.is_idle(),
        "every refused `enter` must still leave the guard balanced, or the next \
         genuine activation is silently swallowed"
    );
}

/// A later, genuine activation of the same step replays again: the visited set
/// is per-activation, not permanent. Without the `visited.clear()` in `exit`,
/// the third advance below is refused as `step_already_replayed`.
#[tokio::test]
async fn the_visited_set_does_not_leak_across_activations() {
    let mut mgr = make_mgr([0.0, 1.0, 0.0]);
    give_mission(&mut mgr, STEP_ONE);

    let mut engine = ChainEngine::new();
    engine.register_chain(chain(
        CHAIN_INSIDE,
        JAFFA_ZONE,
        vec![step_gate(STEP_TWO)],
        vec![bump("replayed")],
    ));

    run_action(&mut mgr, &engine, advance(STEP_TWO)).await;
    run_action(&mut mgr, &engine, advance(STEP_ONE)).await;
    run_action(&mut mgr, &engine, advance(STEP_TWO)).await;

    assert_eq!(
        counter(&mgr, "replayed"),
        2,
        "the second genuine activation of the step must replay again"
    );
}

// ── Hazard (c): rings and stargates ───────────────────────────────────────

/// The replay reaches content chains only. A volume that is both a live ring
/// pad and a `REGION_FLAG_STARGATE` region still resolves its content chain,
/// and still leaves the ring FSM unoccupied and the player in their own world.
///
/// Structural, not incidental: ring forwarding and stargate passage are
/// sequenced by the dispatch arm in `cell_methods::player::world`, *after*
/// `fire_enter_region`. `the_ring_fixture_is_live` is the companion that
/// proves this assertion is not vacuous.
#[tokio::test]
async fn replay_never_touches_rings_or_gates() {
    let mut mgr = make_mgr([0.0, 1.0, 0.0]);
    arm_ring(&mut mgr);
    give_mission(&mut mgr, STEP_ONE);

    let mut engine = ChainEngine::new();
    engine.register_chain(chain(
        CHAIN_RING,
        RING_PAD,
        vec![step_gate(STEP_TWO)],
        vec![bump("ring_pad_chain")],
    ));

    let world_before = mgr.get_entity_world_name(PLAYER_EID);
    run_action(&mut mgr, &engine, advance(STEP_TWO)).await;

    assert_eq!(
        counter(&mgr, "ring_pad_chain"),
        1,
        "a content chain keyed on a ring/stargate volume still replays"
    );
    assert_eq!(
        ring_occupants(&mgr),
        0,
        "the replay must not push the player onto the ring pad's occupant list"
    );
    assert_eq!(
        mgr.get_entity_world_name(PLAYER_EID),
        world_before,
        "the replay must not carry the player through a stargate"
    );
    assert!(
        mgr.pending_gate_dials.is_empty(),
        "the replay must not arm a gate dial"
    );
}

/// Anti-vacuity companion: the same fixture, driven through the ring
/// transporter's own region entry point, *does* register the player. Without
/// this, `replay_never_touches_rings_or_gates` would pass against a fixture
/// whose ring was never loaded.
#[tokio::test]
async fn the_ring_fixture_is_live() {
    let mut mgr = make_mgr([0.0, 1.0, 0.0]);
    arm_ring(&mut mgr);

    let engine = ChainEngine::new();
    let (tx, _rx) = mpsc::channel::<CellToBaseMsg>(8192);
    crate::cell::ring_transport::handle_region_trigger(
        RING_POINT_SET_ID,
        true,
        PLAYER_EID,
        &tx,
        &mut mgr,
        &engine,
    )
    .await;

    assert_eq!(
        ring_occupants(&mgr),
        1,
        "the ring fixture must actually route this point set, or the negative \
         assertion in replay_never_touches_rings_or_gates is vacuous"
    );
}

/// A region the client was never handed (no `REGION_FLAG_CLIENT_HINTED`) is
/// not replayed: the replay stands in for a hint that would never have
/// arrived, so replaying a server-only volume would invent an event.
#[tokio::test]
async fn a_non_client_hinted_region_is_not_replayed() {
    let mut mgr = make_mgr([0.0, 1.0, 0.0]);
    give_mission(&mut mgr, STEP_ONE);

    let mut engine = ChainEngine::new();
    engine.register_chain(chain(
        CHAIN_INVISIBLE,
        INVISIBLE,
        vec![step_gate(STEP_TWO)],
        vec![bump("invisible")],
    ));

    run_action(&mut mgr, &engine, advance(STEP_TWO)).await;

    assert_eq!(
        counter(&mgr, "invisible"),
        0,
        "only client-hinted volumes are replayable"
    );
}

/// `advance_step` against an untracked mission activates nothing, so the
/// replay must not run. Pins the `activated` gate in the executor arm: the
/// chain below would fire if the replay ran at all.
#[tokio::test]
async fn a_failed_advance_does_not_replay() {
    let mut mgr = make_mgr([0.0, 1.0, 0.0]);
    // Deliberately no `give_mission` — the player does not hold 1343.

    let mut engine = ChainEngine::new();
    engine.register_chain(chain(
        CHAIN_INSIDE,
        JAFFA_ZONE,
        vec![Condition::StepStatus {
            mission_id: MISSION,
            step_id: STEP_TWO,
            operator: ComparisonOp::Neq,
            expected_status: StepStatusValue::Active,
        }],
        vec![bump("should_not_run")],
    ));

    run_action(&mut mgr, &engine, advance(STEP_TWO)).await;

    assert_eq!(
        counter(&mgr, "should_not_run"),
        0,
        "no step activated, so nothing may be replayed"
    );
}

// ── The guard in isolation ────────────────────────────────────────────────

#[test]
fn the_guard_caps_depth_and_clears_on_unwind() {
    let mut g = StepRegionReplayGuard::default();
    for step in 0..MAX_REPLAY_DEPTH as i32 {
        assert_eq!(g.enter(PLAYER_EID, MISSION, step), None, "step {step}");
    }
    assert_eq!(
        g.enter(PLAYER_EID, MISSION, 99),
        Some("replay_depth_exceeded"),
    );
    for _ in 0..MAX_REPLAY_DEPTH {
        g.exit();
    }
    assert!(g.is_idle());
}

#[test]
fn the_guard_refuses_a_repeat_of_the_same_step_within_one_activation() {
    let mut g = StepRegionReplayGuard::default();
    assert_eq!(g.enter(PLAYER_EID, MISSION, STEP_ONE), None);
    assert_eq!(
        g.enter(PLAYER_EID, MISSION, STEP_ONE),
        Some("step_already_replayed"),
    );
    // A different player's identical step is its own slot.
    assert_eq!(g.enter(PLAYER_EID + 1, MISSION, STEP_ONE), None);
}
