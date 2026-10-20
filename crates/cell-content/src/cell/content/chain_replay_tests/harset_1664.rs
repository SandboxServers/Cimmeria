//! Mission 1664 "Debriefing" in Harset (FZ-20): Sgt. Kaerlia, the Command Center
//! door, the relog restores and the abandon tidy-up. The chains under test are
//! the ones `harset_1664_chains.sql` seeds (6008-6019 and 6033-6034); the grant itself is
//! pinned by `castle_1664`.
//!
//! Every event goes through the real dispatchers, so the executor arms run.
//! The harness is copied from `mission_567_castle_lifecycle` rather than shared,
//! so a change to one file cannot move the other's assertions. The chains load
//! as shipped (no force-enable), so `enabled = false` on any of them fails here.

use cimmeria_content_engine::chain::ChainEngine;
use cimmeria_entity::missions::{
    MissionInstance, MissionObjective, MISSION_COMPLETED, STATUS_ACTIVE,
};
use sqlx::PgPool;
use tokio::sync::mpsc;

use super::super::engine_loader::load_single_chain_for_test;
use super::super::{fire_enter_region, fire_interact_tag, fire_player_loaded};
use crate::cell::messages::CellToBaseMsg;
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
/// The walk-in path: region chain 6006 (teleport) and the credit chain 6017.
const WALK_IN: [i32; 2] = [6006, 6017];
/// The region key of the walk-in door, as chain 6006 and 6017 trigger on it.
const WALK_IN_REGION: &str = "Harset.CommandCenterTransition";

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

/// A completed mission.
fn completed(mission_id: i32) -> MissionInstance {
    let mut mission = MissionInstance::new(mission_id, 0, Vec::new());
    mission.status = MISSION_COMPLETED;
    mission.current_step_id = None;
    mission
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

/// The five states the chains split on: no 1664, then steps 5224, 5225,
/// 5226 and a completed 1664.
fn states() -> [(&'static str, Vec<MissionInstance>); 5] {
    [
        ("no 1664", vec![]),
        ("5224", vec![mission_1664(5224, 6063)]),
        ("5225", vec![mission_1664(5225, 6060)]),
        ("5226", vec![mission_1664(5226, 6061)]),
        ("completed", vec![completed(MISSION)]),
    ]
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
/// cell-to-base channel, as a compact trace in send order.
async fn traffic(fire: impl AsyncFnOnce(&mpsc::Sender<CellToBaseMsg>)) -> Vec<String> {
    let (tx, mut rx) = mpsc::channel(256);
    fire(&tx).await;
    let mut trace: Vec<String> = Vec::new();
    while let Ok(msg) = rx.try_recv() {
        let label = match msg {
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

// ── Kaerlia ──────────────────────────────────────────────────────────────

/// **Kaerlia answers exactly once in every state.** No 1664 and completed get
/// the stock line (117158); 5224 advances to 5225 and then shows dialog 6169,
/// in that order (D-FZ9); 5225 and 5226 get the Command Center reminder
/// (117162). A state with two entries means two K chains matched. The binds
/// move too: at 5224 the Kaerlia cue (260) is replaced by the door cue (261),
/// and a Kaerlia click binds nothing in any other state. Revert proof: drop
/// 6009's `neq` row and the 5224 case gains a bark; drop 6008's
/// `add_dialog_set` row and the 5224 bind check fails.
#[tokio::test]
async fn live_db_mission_1664_kaerlia_answers_once_per_state() {
    let pool = require_db_or_skip!();
    let engine = engine_with(&pool, &CHAINS).await;
    let keep = screen(&pool, 117158).await;
    let report = screen(&pool, 117162).await;
    for (label, missions) in states() {
        let mut mgr = staged(&pool, HARSET, missions).await;
        if label == "5224" {
            log_in(&engine, &mut mgr, HARSET).await;
            assert_eq!(
                all_binds(&mgr),
                vec![(260, vec![7016])],
                "5224: the Kaerlia cue is live before the click"
            );
        }

        let (handled, trace) = click(&engine, &mut mgr, KAERLIA).await;

        assert!(handled, "{label}: a K chain must answer Kaerlia");
        let expected = match label {
            "no 1664" | "completed" => vec![format!("bark {keep}")],
            "5224" => vec!["step 5225".to_string(), "dialog 6169".to_string()],
            _ => vec![format!("bark {report}")],
        };
        assert_eq!(trace, expected, "{label}: the Kaerlia trace");
        let binds = all_binds(&mgr);
        if label == "5224" {
            assert_eq!(
                binds,
                vec![(261, vec![120301])],
                "5224: the cue moves from Kaerlia to the door"
            );
        } else {
            assert!(
                binds.is_empty(),
                "{label}: a Kaerlia click binds nothing; got {binds:?}"
            );
        }
    }
}

// ── Door ─────────────────────────────────────────────────────────────────

/// **The door travels in every state, and only 5225 also credits.** At 5225
/// the trace is the step to 5226 then the travel; every other state is the
/// travel alone (D2). The teleport hands the player off, so the cell entity is
/// gone after each click. Revert proof: change 6013's operator from `neq` to
/// `eq` and the 5225 case travels twice while the others travel not at all.
#[tokio::test]
async fn live_db_mission_1664_door_answers_once_per_state() {
    let pool = require_db_or_skip!();
    let engine = engine_with(&pool, &CHAINS).await;
    let travel = format!("travel {CMD_CENTER}");
    for (label, missions) in states() {
        let mut mgr = staged(&pool, HARSET, missions).await;

        let (handled, trace) = click(&engine, &mut mgr, DOOR).await;

        assert!(handled, "{label}: a door chain must answer");
        let expected = if label == "5225" {
            vec!["step 5226".to_string(), travel.clone()]
        } else {
            vec![travel.clone()]
        };
        assert_eq!(trace, expected, "{label}: the door trace");
        assert!(
            mgr.get_entity(ENTITY).is_none(),
            "{label}: the teleport must hand the player off"
        );
    }
}

// ── Walk-in ──────────────────────────────────────────────────────────────

/// Walk through the walk-in door region with 6006 and 6017 engaged.
async fn walk_in(engine: &ChainEngine, mgr: &mut SpaceManager) -> Vec<String> {
    traffic(async |tx| fire_enter_region(ENTITY, PLAYER, WALK_IN_REGION, engine, tx, mgr).await)
        .await
}

/// **Walking through the walk-in door credits 5225 once, before the travel.**
/// 6017 runs at priority 10, ahead of 6006's teleport, so at 5225 the trace is
/// the step to 5226 and then the travel, and the player is handed off. Every
/// other state (5224, 5226, no 1664, completed) is the travel alone. Revert
/// proof: set 6017's priority to 0 and the step label follows the travel, or
/// delete 6017 and the 5225 case loses its step.
#[tokio::test]
async fn live_db_mission_1664_walk_in_door_credits_5225_once() {
    let pool = require_db_or_skip!();
    let engine = engine_with(&pool, &WALK_IN).await;
    let travel = format!("travel {CMD_CENTER}");
    for (label, missions) in states() {
        let mut mgr = staged(&pool, HARSET, missions).await;

        let trace = walk_in(&engine, &mut mgr).await;

        let expected = if label == "5225" {
            vec!["step 5226".to_string(), travel.clone()]
        } else {
            vec![travel.clone()]
        };
        assert_eq!(trace, expected, "{label}: the walk-in trace");
        assert!(
            mgr.get_entity(ENTITY).is_none(),
            "{label}: the teleport must hand the player off"
        );
    }
}

// ── Refusals ─────────────────────────────────────────────────────────────

/// **Both clicks are refused outside Harset.** In Harset_CmdCenter the Kaerlia
/// click at 5224 and the door click at 5225 each answer nothing and send
/// nothing. Revert proof: drop 6008's world row and the first click credits.
#[tokio::test]
async fn live_db_mission_1664_clicks_are_refused_outside_harset() {
    let pool = require_db_or_skip!();
    let engine = engine_with(&pool, &CHAINS).await;

    let mut kaerlia = staged(&pool, CMD_CENTER, vec![mission_1664(5224, 6063)]).await;
    let (handled, out) = click(&engine, &mut kaerlia, KAERLIA).await;
    assert!(!handled, "no K chain may answer Kaerlia in world 68");
    assert!(out.is_empty(), "world 68 must send nothing; got {out:?}");

    let mut door = staged(&pool, CMD_CENTER, vec![mission_1664(5225, 6060)]).await;
    let (handled, out) = click(&engine, &mut door, DOOR).await;
    assert!(!handled, "no door chain may answer in world 68");
    assert!(out.is_empty(), "world 68 must send nothing; got {out:?}");
}

// ── Relog ────────────────────────────────────────────────────────────────

/// **A login in Harset restores only the cue that is live.** At 5224 the
/// Kaerlia cue (slot 260, dialog set 7016) comes back; at 5225 the door cue
/// (slot 261, dsm 120301) comes back; every other state binds nothing. A login
/// in Harset_CmdCenter at 5224 binds nothing, because 6014 is keyed to Harset.
/// Revert proof: delete 6015's rows and the 5225 case binds nothing.
#[tokio::test]
async fn live_db_mission_1664_relog_restores_only_the_live_cue() {
    let pool = require_db_or_skip!();
    let engine = engine_with(&pool, &CHAINS).await;
    for (label, missions) in states() {
        let expected = match label {
            "5224" => vec![(260, vec![7016])],
            "5225" => vec![(261, vec![120301])],
            _ => vec![],
        };
        let mut mgr = staged(&pool, HARSET, missions).await;

        log_in(&engine, &mut mgr, HARSET).await;

        assert_eq!(all_binds(&mgr), expected, "{label}: the login binds");
    }

    let mut mgr = staged(&pool, CMD_CENTER, vec![mission_1664(5224, 6063)]).await;
    log_in(&engine, &mut mgr, CMD_CENTER).await;
    assert!(
        all_binds(&mgr).is_empty(),
        "a login in Harset_CmdCenter must bind nothing; got {:?}",
        all_binds(&mgr)
    );
}

// ── Seed pin ─────────────────────────────────────────────────────────────

/// **The seed never grants, never completes a mission directly and never delays;
/// its only `complete_objective` rows are the two 5226 reports (6018/6061,
/// 6019/6062).** It holds chains 6008-6019 (6017 is the walk-in credit, FZ-25),
/// and the debrief restores 6033-6034. The door cue row 120301 must be the
/// bind-only map (no dialog, INT_MissionWorldObject), and the two debrief cues
/// 120302/120303 must be bind-only too. Revert proof: add a `complete_objective`
/// on 6008, or set a `delay_ms` on 6012, and the action scan fails; set
/// 120302's `dialog_id` and the map check fails.
#[tokio::test]
async fn live_db_mission_1664_seed_never_completes_or_regrants() {
    let pool = require_db_or_skip!();

    let actions: Vec<(i32, String, Option<String>, i32)> = sqlx::query_as(
        "SELECT chain_id, action_type, target_key, delay_ms FROM resources.content_actions \
         WHERE chain_id BETWEEN 6008 AND 6020 OR chain_id IN (6033, 6034)",
    )
    .fetch_all(&pool)
    .await
    .expect("actions query");
    let mut objectives: Vec<(i32, Option<String>)> = Vec::new();
    for (chain, action, key, delay) in &actions {
        assert!(
            !matches!(action.as_str(), "complete_mission" | "accept_mission"),
            "1664 chains must never complete or grant; got {action} on {chain}"
        );
        if action.as_str() == "complete_objective" {
            objectives.push((*chain, key.clone()));
        }
        assert_eq!(
            *delay, 0,
            "1664 chains must not delay; got {action} on {chain} at {delay}"
        );
    }
    objectives.sort();
    assert_eq!(
        objectives,
        vec![
            (6018, Some("6061".to_string())),
            (6019, Some("6062".to_string()))
        ],
        "the only complete_objective rows are the two 5226 reports"
    );

    let ids: Vec<(i32,)> = sqlx::query_as(
        "SELECT DISTINCT chain_id FROM resources.content_chains \
         WHERE chain_id BETWEEN 6008 AND 6020 OR chain_id IN (6033, 6034) ORDER BY chain_id",
    )
    .fetch_all(&pool)
    .await
    .expect("chain id query");
    let ids: Vec<i32> = ids.into_iter().map(|(id,)| id).collect();
    let expected: Vec<i32> = (6008..=6019).chain([6033, 6034]).collect();
    assert_eq!(
        ids, expected,
        "chains 6008-6019 and 6033-6034 and nothing else in 6008-6020"
    );

    for (id, flags) in [
        (120301_i32, 1073741824_i64),
        (120302, 536870912),
        (120303, 536870912),
    ] {
        let map: (i32, Option<i32>, i64) = sqlx::query_as(
            "SELECT dialog_set_id, dialog_id, interaction_flags \
             FROM resources.dialog_set_maps WHERE dialog_set_map_id = $1",
        )
        .bind(id)
        .fetch_one(&pool)
        .await
        .expect("dialog set map");
        assert_eq!(
            map,
            (1992, None, flags),
            "{id} is a bind-only map: set 1992, no dialog"
        );
    }
}

// ── Abandon ──────────────────────────────────────────────────────────────

/// Remove `mission` from the player and fire its abandon through the real
/// dispatcher, returning the trace the abandon sends.
async fn abandon(engine: &ChainEngine, mgr: &mut SpaceManager, mission: i32) -> Vec<String> {
    traffic(async |tx| {
        super::super::event_dispatch::fire_mission_abandoned(
            ENTITY, PLAYER, mission, engine, tx, mgr,
        )
        .await
    })
    .await
}

/// **Abandoning 1664 drops both cues, and only for 1664.** Abandon is final:
/// the 5224 Kaerlia cue (260) and the 5225 door cue (261) are each dropped by
/// 6016, and an abandon of mission 1665 leaves the Kaerlia cue in place.
/// Revert proof: change 6016's trigger key and the binds survive the abandon.
#[tokio::test]
async fn live_db_mission_1664_abandon_drops_both_cues() {
    let pool = require_db_or_skip!();
    let engine = engine_with(&pool, &CHAINS).await;

    let mut kaerlia = staged(&pool, HARSET, vec![mission_1664(5224, 6063)]).await;
    log_in(&engine, &mut kaerlia, HARSET).await;
    assert_eq!(
        all_binds(&kaerlia),
        vec![(260, vec![7016])],
        "5224 holds the Kaerlia cue before the abandon"
    );
    kaerlia
        .get_entity_mut(ENTITY)
        .unwrap()
        .missions
        .abandon_mission(MISSION);
    abandon(&engine, &mut kaerlia, MISSION).await;
    assert!(
        all_binds(&kaerlia).is_empty(),
        "the abandon must drop the Kaerlia cue; got {:?}",
        all_binds(&kaerlia)
    );

    let mut door = staged(&pool, HARSET, vec![mission_1664(5225, 6060)]).await;
    log_in(&engine, &mut door, HARSET).await;
    assert_eq!(
        all_binds(&door),
        vec![(261, vec![120301])],
        "5225 holds the door cue before the abandon"
    );
    door.get_entity_mut(ENTITY)
        .unwrap()
        .missions
        .abandon_mission(MISSION);
    abandon(&engine, &mut door, MISSION).await;
    assert!(
        all_binds(&door).is_empty(),
        "the abandon must drop the door cue; got {:?}",
        all_binds(&door)
    );

    let mut other = staged(&pool, HARSET, vec![mission_1664(5224, 6063)]).await;
    log_in(&engine, &mut other, HARSET).await;
    abandon(&engine, &mut other, 1665).await;
    assert_eq!(
        all_binds(&other),
        vec![(260, vec![7016])],
        "an abandon of 1665 must keep the Kaerlia cue"
    );
}
