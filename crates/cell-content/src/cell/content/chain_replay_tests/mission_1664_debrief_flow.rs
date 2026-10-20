//! Mission 1664's debrief on the whole seeded engine (FZ-27).
//!
//! Every other seeded chain on `player_loaded 'Harset_CmdCenter'`,
//! `interact_tag 'CmdCenter_Marsh'` / `'CmdCenter_Mohkatan'`,
//! `mission_completed` and `mission_abandoned` is loaded, so a co-firing
//! chain shows up as an extra trace label or an extra bind. The Praxis offer
//! (6511/6512) must stand down while 1664 is on step 5226, and come back the
//! moment the debrief ends in the Command Center.
//!
//! The harness is copied from `mission_1664_flow` rather than shared, so a
//! change to one file cannot move another file's assertions.

use cimmeria_content_engine::chain::ChainEngine;
use cimmeria_entity::missions::{
    MissionInstance, MissionObjective, MISSION_COMPLETED, STATUS_ACTIVE,
};
use sqlx::PgPool;
use tokio::sync::mpsc;

use super::super::engine_loader::build_engine;
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
const CMD_CENTER: &str = "Harset_CmdCenter";
const MARSH: &str = "CmdCenter_Marsh";
const MOHKATAN: &str = "CmdCenter_Mohkatan";
/// Wire index of the client method onDialogDisplay.
const ON_DIALOG_DISPLAY: u16 = 105;
/// Wire index of the client method onPlayerCommunication.
const ON_PLAYER_COMMUNICATION: u16 = 28;
/// The template slots Col. Marsh and Moh'katan sit in.
const MARSH_SLOT: i32 = 10;
const MOHKATAN_SLOT: i32 = 54;

// ── Harness ──────────────────────────────────────────────────────────────

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

/// 1664 on step 5226, the Harset debrief: both objectives active.
fn debrief() -> MissionInstance {
    let objectives = [6061, 6062]
        .into_iter()
        .map(|objective_id| MissionObjective {
            objective_id,
            status: STATUS_ACTIVE,
            hidden: false,
            optional: false,
        })
        .collect();
    let mut mission = MissionInstance::new(MISSION, 5226, objectives);
    mission.current_step_id = Some(5226);
    mission
}

fn mission_of(mgr: &SpaceManager) -> Option<&MissionInstance> {
    mgr.get_entity(ENTITY)
        .unwrap()
        .missions
        .get_mission(MISSION)
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

/// The dialog-set-map ids bound in `slot`, ascending.
fn slot_ids(mgr: &SpaceManager, slot: i32) -> Vec<i32> {
    all_binds(mgr)
        .into_iter()
        .filter(|(s, _)| *s == slot)
        .flat_map(|(_, ids)| ids)
        .collect()
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

/// The text of dialog screen `id`, which the tests compare against.
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
/// cell-to-base channel, as a compact trace in send order. Labels are
/// `dialog N`, `bark TEXT` and `travel W`; a doubled one stays visible.
async fn traffic(fire: impl AsyncFnOnce(&mpsc::Sender<CellToBaseMsg>)) -> Vec<String> {
    let (tx, mut rx) = mpsc::channel(256);
    fire(&tx).await;
    let mut trace: Vec<String> = Vec::new();
    while let Ok(msg) = rx.try_recv() {
        let label = match msg {
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

/// Remove 1664 from the player (the real abandon), then fire the
/// `mission_abandoned` chains.
async fn abandon(engine: &ChainEngine, mgr: &mut SpaceManager) -> Vec<String> {
    let (tx, _rx) = mpsc::channel(256);
    assert!(
        abandon_mission(ENTITY, MISSION, &tx, mgr).await,
        "the abandon must remove 1664"
    );
    traffic(async |tx| fire_mission_abandoned(ENTITY, PLAYER, MISSION, engine, tx, mgr).await).await
}

// ── Debrief ──────────────────────────────────────────────────────────────

/// **The Praxis offer stands down during the 1664 debrief and returns the
/// moment it ends (H9).** Logging into the Command Center on 5226 binds the
/// report to Marsh and nothing else; the Moh'katan click completes 1664 and
/// hands the offer back in that same click, so the next Marsh click shows
/// 4457.
///
/// Revert proof: drop the 5226 row from 6511 and 6512, and the first Marsh
/// trace gains `dialog 4457`. Drop 6511's `mission_completed '1664'` row and
/// slot 10 is empty after the Moh'katan click.
#[tokio::test]
async fn live_db_mission_1664_debrief_hands_the_praxis_offer_back() {
    let pool = require_db_or_skip!();
    let engine = build_engine(Some(&pool)).await;
    let marsh = screen(&pool, 200180).await;
    let mohkatan = screen(&pool, 200181).await;
    let mut mgr = staged(
        &pool,
        CMD_CENTER,
        vec![completed(708), completed(1360), debrief()],
    )
    .await;

    log_in(&engine, &mut mgr, CMD_CENTER).await;
    assert_eq!(
        slot_ids(&mgr, MARSH_SLOT),
        vec![120302],
        "on 5226 slot 10 holds the report only: 6511 stands down"
    );
    assert_eq!(slot_ids(&mgr, MOHKATAN_SLOT), vec![120303]);

    let (handled, trace) = click(&engine, &mut mgr, MARSH).await;
    assert!(handled, "Col. Marsh must take the click");
    assert_eq!(
        trace,
        vec![format!("bark {marsh}")],
        "the Marsh click on 5226 is the report alone, not the Praxis offer"
    );
    assert!(slot_ids(&mgr, MARSH_SLOT).is_empty());

    let (handled, trace) = click(&engine, &mut mgr, MOHKATAN).await;
    assert!(handled, "Moh'katan must take the click");
    assert_eq!(trace, vec![format!("bark {mohkatan}")]);
    assert_eq!(
        mission_of(&mgr).unwrap().status,
        MISSION_COMPLETED,
        "the Moh'katan click completes 1664"
    );
    assert_eq!(
        slot_ids(&mgr, MARSH_SLOT),
        vec![5254],
        "the Praxis offer is back in the same click the debrief ends"
    );
    assert!(slot_ids(&mgr, MOHKATAN_SLOT).is_empty());

    let (handled, trace) = click(&engine, &mut mgr, MARSH).await;
    assert!(handled, "Col. Marsh must take the click");
    assert_eq!(
        trace,
        vec!["dialog 4457".to_string()],
        "the Praxis briefing is now reachable from Marsh"
    );
}

/// **Abandoning 1664 on 5226 hands the Praxis offer back too.** The abandon
/// removes 1664 without a completion, so only the `mission_abandoned` row on
/// 6511 can bind slot 10 again.
///
/// Revert proof: drop 6511's `mission_abandoned '1664'` row and slot 10 is
/// empty after the abandon.
#[tokio::test]
async fn live_db_mission_1664_abandon_at_5226_hands_the_praxis_offer_back() {
    let pool = require_db_or_skip!();
    let engine = build_engine(Some(&pool)).await;
    let mut mgr = staged(
        &pool,
        CMD_CENTER,
        vec![completed(708), completed(1360), debrief()],
    )
    .await;

    log_in(&engine, &mut mgr, CMD_CENTER).await;
    abandon(&engine, &mut mgr).await;
    assert_eq!(
        slot_ids(&mgr, MARSH_SLOT),
        vec![5254],
        "abandoning 1664 on 5226 must hand the Praxis offer back"
    );
    assert!(slot_ids(&mgr, MOHKATAN_SLOT).is_empty());
}
