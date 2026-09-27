//! Pets campaign PT-07: the non-GM refusal at the chat gate, and
//! `.giveability`'s cell half (`console/give.rs`).
//!
//! Filter prefix: `pt07_`.
//!
//! Bug shapes: a non-GM's `.pet` / `.giveability` broadcast to everyone
//! nearby, or silently ignored; a non-GM reaching a handler at all; a grant
//! aimed at an NPC or at a player in another space that lands on that
//! entity; an unknown or already-known ability forwarded to the base.

use cimmeria_content_engine::chain::ChainEngine;
use cimmeria_entity::abilities::AbilityDef;
use tokio::sync::mpsc;

use super::decode_feedback;
use crate::cell::console::chat::handle_chat_message;
use crate::cell::console::handle_console_command;
use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;
use crate::mercury::method_idx::ON_PLAYER_COMMUNICATION;

const CALLER: u32 = 1;
const WITNESS: u32 = 2;
const FAR: u32 = 3;
const CHAN_SAY: u8 = 0;

fn ability_def(id: i32) -> AbilityDef {
    AbilityDef {
        ability_id: id,
        name: format!("Ability{id}"),
        cooldown: 0.0,
        warmup: 0.0,
        flags: 0,
        is_ranged: false,
        min_range: 0,
        max_range: 0,
        target_type_id: 0,
        effect_ids: vec![],
        moniker_ids: vec![],
        required_ammo: 0,
        event_set_id: None,
        velocity: 0.0,
    }
}

fn add_player(mgr: &mut SpaceManager, id: u32, world: &str) {
    mgr.create_entity(id, world, [10.0 + id as f32, 0.0, 10.0], [0.0; 3])
        .unwrap();
    mgr.connect_entity(id);
    let e = mgr.get_entity_mut(id).unwrap();
    e.is_player = true;
    e.player_id = Some(70 + id as i32);
}

/// Caller (1, `access_level` as given) and a witness (2) in Agnos, a third
/// player (3) in Harset, an NPC in Agnos; ability 2826 defined.
fn world(access_level: u32) -> (SpaceManager, u32) {
    let mut mgr = SpaceManager::new(1);
    let xml = r#"<?xml version="1.0"?><Spaces><Space WorldName="Agnos" Instanced="false" MinX="0" MaxX="100" MinY="0" MaxY="100" /><Space WorldName="Harset" Instanced="false" MinX="0" MaxX="100" MinY="0" MaxY="100" /></Spaces>"#;
    let cxml = r#"<?xml version="1.0"?><Spaces><Space WorldName="Agnos" /><Space WorldName="Harset" /></Spaces>"#;
    mgr.parse_spaces_xml(xml).unwrap();
    mgr.create_startup_spaces(cxml).unwrap();
    add_player(&mut mgr, CALLER, "Agnos");
    add_player(&mut mgr, WITNESS, "Agnos");
    add_player(&mut mgr, FAR, "Harset");
    let npc = mgr.allocate_npc_id();
    mgr.spawn_npc(npc, "Agnos", [12.0, 0.0, 12.0], [0.0; 3])
        .unwrap();
    let caller = mgr.get_entity_mut(CALLER).unwrap();
    caller.access_level = access_level;
    caller
        .witnesses
        .insert(cimmeria_common::EntityId(WITNESS as _));
    mgr.ability_defs.insert(2826, ability_def(2826));
    (mgr, npc)
}

async fn say(mgr: &mut SpaceManager, text: &str) -> Vec<CellToBaseMsg> {
    let (tx, mut rx) = mpsc::channel(64);
    handle_chat_message(
        CALLER,
        "Tester",
        0,
        CHAN_SAY,
        text,
        &tx,
        mgr,
        &ChainEngine::new(),
    )
    .await;
    std::iter::from_fn(|| rx.try_recv().ok()).collect()
}

/// Run a console line as the (GM) caller with `target` selected.
async fn console(mgr: &mut SpaceManager, target: Option<u32>, text: &str) -> Vec<CellToBaseMsg> {
    mgr.get_entity_mut(CALLER).unwrap().current_target_id = target.map(|t| t as i32);
    let (tx, mut rx) = mpsc::channel(64);
    handle_console_command(CALLER, text, &tx, mgr, &ChainEngine::new()).await;
    std::iter::from_fn(|| rx.try_recv().ok()).collect()
}

fn chat_recipients(msgs: &[CellToBaseMsg]) -> Vec<u32> {
    msgs.iter()
        .filter_map(|m| match m {
            CellToBaseMsg::EntityMethodCall {
                entity_id,
                method_index,
                ..
            } if *method_index == ON_PLAYER_COMMUNICATION => Some(*entity_id),
            _ => None,
        })
        .collect()
}

fn grants(msgs: &[CellToBaseMsg]) -> Vec<(u32, i32, i32, u32, i32)> {
    msgs.iter()
        .filter_map(|m| match m {
            CellToBaseMsg::GmGrantAbility {
                entity_id,
                player_id,
                ability_id,
                gm_entity_id,
                gm_player_id,
            } => Some((
                *entity_id,
                *player_id,
                *ability_id,
                *gm_entity_id,
                *gm_player_id,
            )),
            _ => None,
        })
        .collect()
}

/// A player (0) or a trial GM below GameMaster (1) typing a registered
/// command gets one feedback line, and nothing reaches the witness or the
/// handler (no pet, no grant).
#[tokio::test]
async fn pt07_non_gm_pet_and_giveability_are_refused_with_feedback_not_broadcast() {
    for access_level in [0, 1] {
        for line in [".pet summon 2826", ".giveability 2826", ".pet"] {
            let (mut mgr, _npc) = world(access_level);
            let msgs = say(&mut mgr, line).await;

            assert_eq!(
                chat_recipients(&msgs),
                vec![CALLER],
                "level {access_level} {line:?}: one line, to the sender only"
            );
            let fb: Vec<String> = msgs.iter().filter_map(decode_feedback).collect();
            let cmd = line[1..].split_whitespace().next().unwrap();
            assert_eq!(
                fb,
                vec![format!(".{cmd} is a GM command; you do not have GM rights")],
                "no arguments echoed"
            );
            assert!(grants(&msgs).is_empty(), "no grant reaches the base");
            assert!(mgr.pets.is_empty(), "no pet is spawned");
        }
    }
}

/// `.`-text that names no command stays ordinary chat for a non-GM.
#[tokio::test]
async fn pt07_non_gm_dot_text_that_is_not_a_command_still_broadcasts() {
    let (mut mgr, _npc) = world(0);
    let msgs = say(&mut mgr, ".petting the dog").await;
    assert!(chat_recipients(&msgs).contains(&WITNESS), "{msgs:?}");
}

#[tokio::test]
async fn pt07_giveability_with_no_target_grants_the_caller() {
    let (mut mgr, _npc) = world(2);
    let msgs = console(&mut mgr, None, ".giveability 2826").await;
    assert_eq!(grants(&msgs), vec![(CALLER, 71, 2826, CALLER, 71)]);
    assert!(
        msgs.iter().filter_map(decode_feedback).next().is_none(),
        "no optimistic line: the base answers after the UPDATE"
    );
}

#[tokio::test]
async fn pt07_giveability_grants_a_selected_player() {
    let (mut mgr, _npc) = world(2);
    let msgs = console(&mut mgr, Some(WITNESS), ".giveability 2826").await;
    assert_eq!(grants(&msgs), vec![(WITNESS, 72, 2826, CALLER, 71)]);
}

/// An NPC selection falls back to the caller, and says so.
#[tokio::test]
async fn pt07_giveability_with_an_npc_selected_grants_the_caller_and_says_so() {
    let (mut mgr, npc) = world(2);
    let msgs = console(&mut mgr, Some(npc), ".giveability 2826").await;
    assert_eq!(grants(&msgs), vec![(CALLER, 71, 2826, CALLER, 71)]);
    let fb: Vec<String> = msgs.iter().filter_map(decode_feedback).collect();
    assert!(
        fb.iter()
            .any(|l| l.contains(&format!("target {npc} is not a player"))),
        "{fb:?}"
    );
}

/// A player in another space is never the subject.
#[tokio::test]
async fn pt07_giveability_never_grants_a_player_in_another_space() {
    let (mut mgr, _npc) = world(2);
    let msgs = console(&mut mgr, Some(FAR), ".giveability 2826").await;
    let subjects: Vec<u32> = grants(&msgs).iter().map(|g| g.0).collect();
    assert_eq!(subjects, vec![CALLER]);
}

#[tokio::test]
async fn pt07_giveability_refuses_unknown_and_already_known_abilities() {
    let (mut mgr, _npc) = world(2);
    let unknown = console(&mut mgr, None, ".giveability 99999").await;
    mgr.get_entity_mut(CALLER)
        .unwrap()
        .abilities
        .add_ability(2826);
    let known = console(&mut mgr, None, ".giveability 2826").await;
    let not_a_number = console(&mut mgr, None, ".giveability x").await;

    for (label, msgs) in [
        ("unknown", unknown),
        ("known", known),
        ("not a number", not_a_number),
    ] {
        assert!(grants(&msgs).is_empty(), "{label}: nothing to the base");
        assert_eq!(
            msgs.iter().filter_map(decode_feedback).count(),
            1,
            "{label}: one refusal line"
        );
    }
}
