//! Class Start v6, CS-05: the SGC_W1 start
//! (`db/resources/Content/Seed/sgc_w1_chains.sql`; ledger
//! `docs/analysis/class-start-v6/README.md`).
//!
//! - [`starter_gates`]: chains 3001, 3017 and 3018 keep a Free Jaffa out
//!   (`archetype neq 7`, OD-CS09).
//! - [`firearm_body`]: chain 3008 gives a Human the pistol, the core
//!   abilities and tutorial 5882 in that order; chain 3029 leaves every
//!   other archetype (the Asgard holding state) with the pre-CS-05 pickup;
//!   chain 7101 then shows 5883 to the Human only.
//! - [`carter_smg`]: chains 3030-3037 finish M1562 for a Human: Carter, then
//!   the SMG on her desk (item 21), with her lab doors re-opened.
//! - [`lab_placement`]: the desk SMG's spawn row and Carter's heading against
//!   the numbers read from the cooked map.
//! - [`ordinance_rewards`]: chains 3041-3044 deliver the M1569 class gear
//!   and signature on `mission_completed 1569`.
//!
//! Every test loads its chains from the database and drives them through
//! the real dispatchers, so the `archetype` param comes from the player
//! entity as it does in play, and the assertions are on what the cell sends
//! the base. None of them asserts on provenance rows or on what a new
//! character already knows: those depend on whether the universal spawn kit
//! still exists (CS-02), and these chains must be right either way.

mod carter_smg;
mod firearm_body;
mod lab_placement;
mod ordinance_rewards;
mod starter_gates;

use std::collections::HashMap;

use sqlx::PgPool;
use tokio::sync::mpsc;

use cimmeria_content_engine::actions::AbilityGrantKind;
use cimmeria_content_engine::chain::ChainEngine;
use cimmeria_entity::missions::{
    MissionInstance, MissionObjective, MISSION_ACTIVE, MISSION_COMPLETED, STATUS_ACTIVE,
};

use super::super::engine_loader::{build_engine, load_single_chain_for_test};
use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;
use crate::cell::spawner::WorldRow;
use crate::test_support::require_db_or_skip;

const PLAYER_EID: u32 = 7101;
const PLAYER_ID: i32 = 42;
/// `resources.worlds.world_id` of SGC_W1.
const SGC_W1_WORLD_ID: i32 = 58;

// `EArchetype` (`entities/defs/enumerations.xml`).
const SOLDIER: i32 = 1;
const COMMANDO: i32 = 2;
const SCIENTIST: i32 = 3;
const ARCHAEOLOGIST: i32 = 4;
const ASGARD: i32 = 5;
const GOAULD: i32 = 6;
/// `ARCHETYPE_Sholva`, the SGU Free Jaffa (char_defs 8 and 18).
const FREE_JAFFA: i32 = 7;
/// `ARCHETYPE_Jaffa`, the Praxis Loyalist Jaffa.
const LOYALIST_JAFFA: i32 = 8;

/// The four SGU Human classes.
const HUMANS: [i32; 4] = [SOLDIER, COMMANDO, SCIENTIST, ARCHAEOLOGIST];
/// Every archetype that is not one of them.
const NON_HUMANS: [i32; 4] = [ASGARD, GOAULD, FREE_JAFFA, LOYALIST_JAFFA];

/// `INT_AStoryMissionActive`, the "!" the SGC chains mark a contact with.
const STORY_ACTIVE: i64 = 16_777_216;
/// `INT_MissionWorldObject`, the glow that makes a quest prop pressable.
const MISSION_OBJECT: i64 = 1_073_741_824;

/// An engine holding exactly `chain_ids`, each loaded from the seeded
/// database through the cell's own loader.
async fn engine_with(pool: &PgPool, chain_ids: &[i32]) -> ChainEngine {
    let mut engine = ChainEngine::new();
    for &chain_id in chain_ids {
        let chain = load_single_chain_for_test(pool, chain_id)
            .await
            .unwrap_or_else(|e| panic!("DB query for chain {chain_id} must succeed: {e}"))
            .unwrap_or_else(|| panic!("chain {chain_id} must be seeded and must load"));
        engine.register_chain(chain);
    }
    engine
}

/// An SGC_W1 space holding one connected player of `archetype` (`None`: a
/// player with no archetype).
fn sgc_mgr(archetype: Option<i32>) -> SpaceManager {
    let mut mgr = SpaceManager::new(1);
    mgr.parse_spaces_xml(
        r#"<?xml version="1.0"?><Spaces><Space WorldName="SGC_W1" Instanced="false" MinX="-800" MaxX="800" MinY="-800" MaxY="800" /></Spaces>"#,
    )
    .expect("spaces xml must parse");
    mgr.create_startup_spaces(
        r#"<?xml version="1.0"?><Spaces><Space WorldName="SGC_W1" /></Spaces>"#,
    )
    .expect("the SGC_W1 startup space must be created");
    mgr.stamp_world_rows(&HashMap::from([(
        "SGC_W1".to_string(),
        WorldRow::enforcing(SGC_W1_WORLD_ID),
    )]));
    mgr.create_entity(PLAYER_EID, "SGC_W1", [0.0; 3], [0.0; 3])
        .expect("SGC_W1 must accept the player entity");
    let p = mgr
        .get_entity_mut(PLAYER_EID)
        .expect("the player exists right after create_entity");
    p.is_player = true;
    p.player_id = Some(PLAYER_ID);
    p.archetype_id = archetype;
    mgr.connect_entity(PLAYER_EID);
    mgr
}

/// Put a tagged entity (an NPC or a world object) next to the player.
fn spawn_tagged(mgr: &mut SpaceManager, entity_id: u32, tag: &str) {
    mgr.create_entity(entity_id, "SGC_W1", [1.0, 0.0, 1.0], [0.0; 3])
        .expect("SGC_W1 must accept the tagged entity");
    mgr.get_entity_mut(entity_id)
        .expect("the tagged entity exists right after create_entity")
        .tag = Some(tag.to_string());
}

/// Give the player `mission_id`, active on `step_id`.
fn give_mission(mgr: &mut SpaceManager, mission_id: i32, step_id: i32) {
    mgr.get_entity_mut(PLAYER_EID)
        .expect("the player exists")
        .missions
        .add_mission(MissionInstance::new(
            mission_id,
            step_id,
            vec![MissionObjective {
                objective_id: step_id,
                status: STATUS_ACTIVE,
                hidden: false,
                optional: false,
            }],
        ));
}

fn mission_status(mgr: &SpaceManager, mission_id: i32) -> Option<i8> {
    mgr.get_entity(PLAYER_EID)
        .and_then(|p| p.missions.get_mission(mission_id))
        .map(|m| m.status)
}

fn mission_step(mgr: &SpaceManager, mission_id: i32) -> Option<i32> {
    mgr.get_entity(PLAYER_EID)
        .and_then(|p| p.missions.get_mission(mission_id))
        .and_then(|m| m.current_step_id)
}

fn interaction_flags(mgr: &SpaceManager, entity_id: u32) -> i64 {
    mgr.get_entity(entity_id)
        .expect("the tagged entity exists")
        .interaction_type_flags
}

/// What a chain asked the base for, reduced to the parts CS-05 authors.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Sent {
    /// `add_item`: item, container, count.
    Item(i32, i32, i32),
    /// `grant_ability`: ids, kind, source id, archetypes.
    Abilities(Vec<i32>, AbilityGrantKind, Option<i32>, Vec<i32>),
    /// `show_tutorial`: the tutorial the cell asked the base to record.
    Tutorial(i32),
    /// `display_dialog`: the dialog id of an `onDialogDisplay`.
    Dialog(i32),
}

/// Drain the cell-to-base channel into the [`Sent`] rows, in send order.
/// Everything else (mission updates, interaction-type broadcasts, journal
/// writes) is dropped.
fn drain(rx: &mut mpsc::Receiver<CellToBaseMsg>) -> Vec<Sent> {
    let mut out = Vec::new();
    while let Ok(msg) = rx.try_recv() {
        match msg {
            CellToBaseMsg::GrantItem {
                item_id,
                container_id,
                count,
                ..
            } => out.push(Sent::Item(item_id, container_id, count)),
            CellToBaseMsg::ContentGrantAbilities(g) => out.push(Sent::Abilities(
                g.ability_ids,
                g.source_kind,
                g.source_id,
                g.archetypes,
            )),
            CellToBaseMsg::RecordTutorialShown(r) => out.push(Sent::Tutorial(r.tutorial_id)),
            CellToBaseMsg::EntityMethodCall {
                method_index, args, ..
            } if method_index == crate::mercury::method_idx::ON_DIALOG_DISPLAY => {
                // onDialogDisplay(EntityId, DialogID, ...): the id is bytes 4..8.
                let id = args
                    .get(4..8)
                    .and_then(|b| b.try_into().ok())
                    .map(i32::from_le_bytes)
                    .expect("onDialogDisplay carries a dialog id");
                out.push(Sent::Dialog(id));
            }
            _ => {}
        }
    }
    out
}

/// A label for an archetype in an assertion message.
fn label(archetype: Option<i32>) -> String {
    match archetype {
        Some(a) => format!("archetype {a}"),
        None => "a player with no archetype".to_string(),
    }
}

/// Completed and active as the tests read them.
const COMPLETED: Option<i8> = Some(MISSION_COMPLETED);
const ACTIVE: Option<i8> = Some(MISSION_ACTIVE);

/// Every chain CS-05 adds or edits, with its action count.
const CS05_CHAINS: [(i64, usize); 17] = [
    (3001, 2),
    (3008, 5),
    (3017, 1),
    (3018, 2),
    (3029, 3),
    (3030, 2),
    (3031, 1),
    (3032, 2),
    (3033, 3),
    (3034, 1),
    (3035, 4),
    (3036, 1),
    (3037, 1),
    (3041, 3),
    (3042, 7),
    (3043, 3),
    (3044, 3),
];

/// **Guard: every CS-05 chain survives the cell's real engine build.** The
/// per-chain loader the other tests use skips the two whole-chain refusals
/// (`refuse_chains_with_unknown_abilities`, `..._unknown_tutorials`), and a
/// refused chain is gone with all its actions: a typo in a granted ability
/// id would take the pistol and the mission completion with it. Each chain
/// must also keep every action row (a malformed row is dropped, not
/// refused).
#[tokio::test]
async fn live_db_every_cs05_chain_survives_the_full_engine_load() {
    let pool = require_db_or_skip!();
    let engine = build_engine(Some(&pool)).await;

    for (chain_id, actions) in CS05_CHAINS {
        assert_eq!(
            engine.get_chain_actions(chain_id).len(),
            actions,
            "chain {chain_id} must load with all {actions} of its actions",
        );
    }
}
