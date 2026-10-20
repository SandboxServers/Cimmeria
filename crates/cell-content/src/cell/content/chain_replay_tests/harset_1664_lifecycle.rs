//! Mission 1664 "Debriefing" in Harset (FZ-21): the player's walk through the
//! mission in order. Kaerlia moves the cue to the door, the door credits and
//! hands off, a relog after the door binds nothing, and an abandon at either
//! cue is final. `harset_1664.rs` (FZ-20) pins each click on its own; this file
//! pins the sequence. The harness is copied from `harset_1664.rs` rather than
//! shared, so a change to one file cannot move the other's assertions.

use cimmeria_content_engine::chain::ChainEngine;
use cimmeria_entity::missions::{
    MissionInstance, MissionObjective, MISSION_COMPLETED, STATUS_ACTIVE,
};
use sqlx::PgPool;
use tokio::sync::mpsc;

use super::super::engine_loader::load_single_chain_for_test;
use super::super::{fire_interact_tag, fire_player_loaded};
use crate::cell::content::fire_mission_abandoned;
use crate::cell::messages::CellToBaseMsg;
use crate::cell::missions::abandon_mission;
use crate::cell::space_manager::SpaceManager;
use crate::cell::spawner::{
    load_dialog_screen_text, load_dialog_set_maps, load_item_containers, load_mission_defs,
    load_monologue_dialog_ids, load_step_objectives, load_world_rows,
};
use crate::test_support::require_db_or_skip;

const ENTITY: u32 = 1;
const PLAYER: i32 = 42;
const TARGET: u32 = 3;
const MISSION: i32 = 1664;
const HARSET: &str = "Harset";
const CMD_CENTER: &str = "Harset_CmdCenter";
const KAERLIA: &str = "Harset_Kaerlia";
const DOOR: &str = "Harset_CmdDoor";
/// Wire index of the client method onDialogDisplay.
const ON_DIALOG_DISPLAY: u16 = 105;
/// Wire index of the client method onPlayerCommunication (barks).
const ON_PLAYER_COMMUNICATION: u16 = 28;
const CHAINS: [i32; 9] = [6008, 6009, 6010, 6011, 6012, 6013, 6014, 6015, 6016];

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

/// `world` with player `ENTITY` holding `missions`, with the dialog-set,
/// monologue and dialog-screen caches loaded from the seed.
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
    mgr.dialog_screen_text = load_dialog_screen_text(pool)
        .await
        .expect("dialog screen text");
    mgr.monologue_dialog_ids = load_monologue_dialog_ids(pool)
        .await
        .expect("monologue ids");
    mgr.create_entity(TARGET, world, [3.0; 3], [0.0; 3])
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

/// 1664 active at `step` with `objective` active.
fn mission_1664(step: i32, objective: i32) -> MissionInstance {
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

/// The live 1664 on the player, if any.
fn mission_of(mgr: &SpaceManager) -> Option<&MissionInstance> {
    mgr.get_entity(ENTITY)?.missions.get_mission(MISSION)
}

/// The current step of the live 1664, if any.
fn step(mgr: &SpaceManager) -> Option<i32> {
    mission_of(mgr)?.current_step_id
}

/// Read a WSTRING at `offset`: a `u32` UTF-16 code-unit count followed by
/// that many little-endian `u16`s. Returns the decoded string and the
/// code-unit count, as in `npc_bark.rs`.
fn read_wstring(args: &[u8], offset: usize) -> (String, usize) {
    let units = u32::from_le_bytes(args[offset..offset + 4].try_into().unwrap()) as usize;
    let (pairs, _) = args[offset + 4..offset + 4 + units * 2].as_chunks::<2>();
    let s = char::decode_utf16(pairs.iter().copied().map(u16::from_le_bytes))
        .map(|r| r.expect("wire text must be valid UTF-16"))
        .collect();
    (s, units)
}

/// The text of an `onPlayerCommunication` payload: speaker WSTRING, SpeakerFlags
/// byte, Channel byte, then the text WSTRING.
fn decode_bark_text(args: &[u8]) -> String {
    let (_, speaker_units) = read_wstring(args, 0);
    let flags_off = 4 + speaker_units * 2;
    let (text, _) = read_wstring(args, flags_off + 2);
    text
}

/// The text of dialog screen `id`, which the bark tests compare against.
async fn screen(pool: &PgPool, id: i32) -> String {
    let text: String =
        sqlx::query_scalar("SELECT text FROM resources.dialog_screens WHERE screen_id = $1")
            .bind(id)
            .fetch_one(pool)
            .await
            .expect("dialog screen row");
    assert!(!text.is_empty(), "screen {id} must carry text");
    text
}

/// Fire one event against the stage and return everything sent to the
/// cell-to-base channel, as a compact trace in send order. The completion arm
/// comes before the step arm, so a completion never hides behind a step label.
async fn traffic(fire: impl AsyncFnOnce(&mpsc::Sender<CellToBaseMsg>)) -> Vec<String> {
    let (tx, mut rx) = mpsc::channel(256);
    fire(&tx).await;
    let mut trace: Vec<String> = Vec::new();
    while let Ok(msg) = rx.try_recv() {
        let label = match msg {
            CellToBaseMsg::MissionUpdate {
                mission_id: MISSION,
                status,
                ..
            } if status == MISSION_COMPLETED => "completed".to_string(),
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
            CellToBaseMsg::EntityMethodCall {
                method_index: ON_PLAYER_COMMUNICATION,
                args,
                ..
            } => {
                format!("bark {}", decode_bark_text(&args))
            }
            CellToBaseMsg::GateTravel {
                target_world_name, ..
            } => {
                format!("travel {target_world_name}")
            }
            _ => continue,
        };
        // Only the step echo may repeat (one advance can update twice); a
        // doubled dialog, bark or travel must stay visible in the trace.
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

async fn click(engine: &ChainEngine, mgr: &mut SpaceManager, tag: &str) -> (bool, Vec<String>) {
    let mut handled = false;
    let trace = traffic(async |tx| {
        handled = fire_interact_tag(ENTITY, PLAYER, tag, TARGET, engine, tx, mgr).await;
    })
    .await;
    (handled, trace)
}

/// The real abandon (removes 1664), then the `mission_abandoned` chains.
async fn abandon(engine: &ChainEngine, mgr: &mut SpaceManager) -> Vec<String> {
    let (tx, _rx) = mpsc::channel(256);
    assert!(
        abandon_mission(ENTITY, MISSION, &tx, mgr).await,
        "the abandon must remove 1664"
    );
    traffic(async |tx| fire_mission_abandoned(ENTITY, PLAYER, MISSION, engine, tx, mgr).await).await
}

// ── Kaerlia: the cue moves to the door ───────────────────────────────────

/// **Kaerlia at 5224 moves the cue to the door, and a second click is quiet.**
/// The first click advances to 5225 and shows dialog 6169, in that order; the
/// Kaerlia cue (slot 260) is removed and the door cue (slot 261) is bound. A
/// second click answers with the Command Center reminder only: the step stays
/// 5225 and the binds do not change. Revert proof: drop 6008's
/// `remove_dialog_set` row and slot 260 keeps 7016.
#[tokio::test]
async fn live_db_mission_1664_kaerlia_moves_the_cue_to_the_door() {
    let pool = require_db_or_skip!();
    let engine = engine_with(&pool, &CHAINS).await;
    let report = screen(&pool, 117162).await;
    let mut mgr = staged(&pool, HARSET, vec![mission_1664(5224, 6063)]).await;

    log_in(&engine, &mut mgr, HARSET).await;
    assert_eq!(
        all_binds(&mgr),
        vec![(260, vec![7016])],
        "5224 holds the Kaerlia cue before the first click"
    );

    let (handled, trace) = click(&engine, &mut mgr, KAERLIA).await;
    assert!(handled, "the first Kaerlia click must be answered");
    assert_eq!(
        trace,
        vec!["step 5225".to_string(), "dialog 6169".to_string()],
        "the first click: advance, then the dialog"
    );
    assert_eq!(step(&mgr), Some(5225), "the first click leaves 5225 open");
    assert_eq!(
        all_binds(&mgr),
        vec![(261, vec![120301])],
        "the first click moves the cue: 260 empty, 261 bound"
    );

    let (handled, trace) = click(&engine, &mut mgr, KAERLIA).await;
    assert!(handled, "the second Kaerlia click must be answered");
    assert_eq!(
        trace,
        vec![format!("bark {report}")],
        "the second click is the Command Center reminder, once"
    );
    assert_eq!(step(&mgr), Some(5225), "the second click keeps 5225");
    assert_eq!(
        all_binds(&mgr),
        vec![(261, vec![120301])],
        "the second click leaves the binds unchanged"
    );
}

// ── Door: credit, then hand off ──────────────────────────────────────────

/// **The door at 5225 credits, then hands the player off, and the step stays
/// open.** The trace is the step to 5226 then the travel to Harset_CmdCenter,
/// with no `completed` label (D-PU3). The teleport removes the cell entity.
/// Revert proof: swap 6012's `advance_step` and `cross_world_teleport` sort
/// orders and the step label never appears.
#[tokio::test]
async fn live_db_mission_1664_door_credits_then_hands_off() {
    let pool = require_db_or_skip!();
    let engine = engine_with(&pool, &CHAINS).await;
    let mut mgr = staged(&pool, HARSET, vec![mission_1664(5225, 6060)]).await;

    log_in(&engine, &mut mgr, HARSET).await;
    assert_eq!(
        all_binds(&mgr),
        vec![(261, vec![120301])],
        "5225 holds the door cue before the click"
    );

    let (handled, trace) = click(&engine, &mut mgr, DOOR).await;
    assert!(handled, "the door click must be answered");
    assert_eq!(
        trace,
        vec!["step 5226".to_string(), format!("travel {CMD_CENTER}")],
        "the door: credit to 5226, then travel, and no completion"
    );
    assert!(
        mgr.get_entity(ENTITY).is_none(),
        "the teleport must hand the player off"
    );
}

// ── Relog after the door ─────────────────────────────────────────────────

/// **A relog at 5226 binds nothing, in Harset or in the Command Center.** In
/// Harset at 5226 the login binds no cue, and a Kaerlia click answers with the
/// Command Center reminder once. In Harset_CmdCenter at 5226 none of chains
/// 6008-6016 binds anything; the debrief cues there come from chains 6033/6034,
/// which this file does not load, and are pinned in `harset_1664_debrief`.
/// Revert proof: widen 6015's
/// `step_status` row to `neq` 5224 and the 5226 login binds the door cue.
#[tokio::test]
async fn live_db_mission_1664_relog_after_the_door_binds_nothing() {
    let pool = require_db_or_skip!();
    let engine = engine_with(&pool, &CHAINS).await;
    let report = screen(&pool, 117162).await;

    let mut harset = staged(&pool, HARSET, vec![mission_1664(5226, 6061)]).await;
    log_in(&engine, &mut harset, HARSET).await;
    assert!(
        all_binds(&harset).is_empty(),
        "a Harset login at 5226 must bind nothing; got {:?}",
        all_binds(&harset)
    );
    let (handled, trace) = click(&engine, &mut harset, KAERLIA).await;
    assert!(handled, "the Kaerlia click must be answered at 5226");
    assert_eq!(
        trace,
        vec![format!("bark {report}")],
        "a Kaerlia click at 5226 is the reminder, once"
    );

    let mut cmd_center = staged(&pool, CMD_CENTER, vec![mission_1664(5226, 6061)]).await;
    log_in(&engine, &mut cmd_center, CMD_CENTER).await;
    assert!(
        all_binds(&cmd_center).is_empty(),
        "chains 6008-6016 must bind nothing on a Command Center login at 5226; got {:?}",
        all_binds(&cmd_center)
    );
}

// ── Abandon: final at either cue ─────────────────────────────────────────

/// **An abandon at 5224 drops the Kaerlia cue and never re-grants.** After the
/// abandon no bind remains and the mission is absent. A Kaerlia click then
/// answers with the stock line only (no step label, so no re-grant); a relog in
/// Harset binds nothing and the mission stays absent. Revert proof: delete
/// chain 6016's first action and slot 260 keeps 7016.
/// Pins option P0 (O18-02a, pending the owner): an abandon is final. If the owner picks P3, this test changes in the P3 packet.
#[tokio::test]
async fn live_db_mission_1664_abandon_at_5224_is_final() {
    let pool = require_db_or_skip!();
    let engine = engine_with(&pool, &CHAINS).await;
    let keep = screen(&pool, 117158).await;
    let mut mgr = staged(&pool, HARSET, vec![mission_1664(5224, 6063)]).await;

    log_in(&engine, &mut mgr, HARSET).await;
    assert_eq!(
        all_binds(&mgr),
        vec![(260, vec![7016])],
        "5224 holds the Kaerlia cue before the abandon"
    );
    abandon(&engine, &mut mgr).await;
    assert!(mission_of(&mgr).is_none(), "the abandon must remove 1664");
    assert!(
        all_binds(&mgr).is_empty(),
        "the abandon must drop the Kaerlia cue; got {:?}",
        all_binds(&mgr)
    );

    let (handled, trace) = click(&engine, &mut mgr, KAERLIA).await;
    assert!(
        handled,
        "the stock Kaerlia line must answer after the abandon"
    );
    assert_eq!(
        trace,
        vec![format!("bark {keep}")],
        "after the abandon a Kaerlia click is the stock line, once, with no step"
    );

    log_in(&engine, &mut mgr, HARSET).await;
    assert!(
        all_binds(&mgr).is_empty(),
        "a relog after the abandon must bind nothing; got {:?}",
        all_binds(&mgr)
    );
    assert!(mission_of(&mgr).is_none(), "1664 must stay absent");
}

/// **An abandon at 5225 drops the door cue, and the door is then a plain
/// door.** After the abandon no bind remains. A door click answers with the
/// plain travel (6013) only, and the mission stays absent. Revert proof: delete
/// chain 6016's second action and slot 261 keeps 120301.
/// Pins option P0 (O18-02a, pending the owner): an abandon is final. If the owner picks P3, this test changes in the P3 packet.
#[tokio::test]
async fn live_db_mission_1664_abandon_at_5225_is_final() {
    let pool = require_db_or_skip!();
    let engine = engine_with(&pool, &CHAINS).await;
    let mut mgr = staged(&pool, HARSET, vec![mission_1664(5225, 6060)]).await;

    log_in(&engine, &mut mgr, HARSET).await;
    assert_eq!(
        all_binds(&mgr),
        vec![(261, vec![120301])],
        "5225 holds the door cue before the abandon"
    );
    abandon(&engine, &mut mgr).await;
    assert!(mission_of(&mgr).is_none(), "the abandon must remove 1664");
    assert!(
        all_binds(&mgr).is_empty(),
        "the abandon must drop the door cue; got {:?}",
        all_binds(&mgr)
    );

    let (handled, trace) = click(&engine, &mut mgr, DOOR).await;
    assert!(handled, "the plain door must answer after the abandon");
    assert_eq!(
        trace,
        vec![format!("travel {CMD_CENTER}")],
        "after the abandon the door is the plain travel only"
    );
    assert!(mission_of(&mgr).is_none(), "1664 must stay absent");
}
