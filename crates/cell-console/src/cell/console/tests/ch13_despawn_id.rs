//! CH-13 (D-CH9): `.despawn <entityId>` names the NPC by id; with no argument
//! it still acts on the selection. The lab runner's `burst` removes the NPCs
//! it spawned this way, by the ids `.spawnrandom` reported.
//!
//! Each refusal is its own test so a dropped check shows by name.

use cimmeria_content_engine::chain::ChainEngine;
use cimmeria_entity::cell_entity::PlayerIdentity;
use tokio::sync::mpsc;

use super::{decode_feedback, setup};
use crate::cell::console::handle_console_command;
use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;

/// Run one console line as `gm` and return the feedback lines.
async fn run(mgr: &mut SpaceManager, gm: u32, line: &str) -> Vec<String> {
    let (tx, mut rx) = mpsc::channel::<CellToBaseMsg>(64);
    handle_console_command(gm, line, &tx, mgr, &ChainEngine::new()).await;
    std::iter::from_fn(|| rx.try_recv().ok())
        .filter_map(|m| decode_feedback(&m))
        .collect()
}

/// A second NPC in the GM's space, not selected and not in its view.
fn second_npc(mgr: &mut SpaceManager) -> u32 {
    let id = mgr.allocate_npc_id();
    mgr.spawn_npc(id, "Agnos", [60.0, 0.0, 60.0], [0.0; 3])
        .unwrap();
    id
}

/// The id form despawns the named NPC even with nothing selected and the NPC
/// out of view. Revert proof: drop the `Target::MobOrId` argument branch in
/// `resolve_target` and this is refused for want of a target.
#[tokio::test]
async fn despawn_by_id_removes_the_named_npc_without_a_selection() {
    let (mut mgr, gm, _npc) = setup();
    let other = second_npc(&mut mgr);
    mgr.get_entity_mut(gm).unwrap().current_target_id = None;

    let fb = run(&mut mgr, gm, &format!(".despawn {other}")).await;

    assert!(mgr.get_entity(other).is_none(), "the named NPC is gone");
    assert!(
        fb.iter()
            .any(|f| f.starts_with(&format!("despawn [{other}]: removed"))),
        "the GM is told what was removed; got {fb:?}"
    );
}

/// With an id, the selection is ignored: the named NPC goes, the selected
/// one stays.
#[tokio::test]
async fn despawn_by_id_ignores_the_selection() {
    let (mut mgr, gm, npc) = setup();
    let other = second_npc(&mut mgr);

    run(&mut mgr, gm, &format!(".despawn {other}")).await;

    assert!(mgr.get_entity(other).is_none(), "the named NPC is gone");
    assert!(
        mgr.get_entity(npc).is_some(),
        "the selected NPC is untouched"
    );
}

/// No argument: today's behaviour, the selected NPC goes.
#[tokio::test]
async fn despawn_without_an_id_still_removes_the_selection() {
    let (mut mgr, gm, npc) = setup();
    let other = second_npc(&mut mgr);

    run(&mut mgr, gm, ".despawn").await;

    assert!(mgr.get_entity(npc).is_none(), "the selected NPC is gone");
    assert!(mgr.get_entity(other).is_some(), "nothing else is touched");
}

#[tokio::test]
async fn despawn_by_id_refuses_a_player() {
    let (mut mgr, gm, _npc) = setup();
    let victim = 2u32;
    mgr.create_entity(victim, "Agnos", [11.0, 0.0, 11.0], [0.0; 3])
        .unwrap();
    mgr.get_entity_mut(victim).unwrap().is_player = true;

    let fb = run(&mut mgr, gm, &format!(".despawn {victim}")).await;

    assert!(
        mgr.get_entity(victim).is_some(),
        "a player is never despawned"
    );
    assert!(
        fb.iter().any(|f| f.contains("is a player")),
        "the refusal names why; got {fb:?}"
    );
}

/// The GM's own id is a player too.
#[tokio::test]
async fn despawn_by_id_refuses_the_caller() {
    let (mut mgr, gm, _npc) = setup();

    let fb = run(&mut mgr, gm, &format!(".despawn {gm}")).await;

    assert!(mgr.get_entity(gm).is_some());
    assert!(fb.iter().any(|f| f.contains("is a player")), "got {fb:?}");
}

#[tokio::test]
async fn despawn_by_id_refuses_an_unknown_id() {
    let (mut mgr, gm, npc) = setup();

    let fb = run(&mut mgr, gm, ".despawn 987654").await;

    assert!(
        mgr.get_entity(npc).is_some(),
        "the selection is not a fallback"
    );
    assert!(
        fb.iter().any(|f| f.contains("no entity has id 987654")),
        "got {fb:?}"
    );
}

#[tokio::test]
async fn despawn_by_id_refuses_a_non_number() {
    let (mut mgr, gm, npc) = setup();

    let fb = run(&mut mgr, gm, ".despawn abc").await;

    assert!(
        mgr.get_entity(npc).is_some(),
        "the selection is not a fallback"
    );
    assert!(
        fb.iter().any(|f| f.contains("entityId must be")),
        "got {fb:?}"
    );
}

/// A player's pet is an NPC-class entity, but it belongs to a player.
#[tokio::test]
async fn despawn_by_id_refuses_a_players_pet() {
    let (mut mgr, gm, _npc) = setup();
    let pet = second_npc(&mut mgr);
    mgr.pets.register(gm, pet, PlayerIdentity::UNKNOWN);

    let fb = run(&mut mgr, gm, &format!(".despawn {pet}")).await;

    assert!(mgr.get_entity(pet).is_some(), "a pet is never despawned");
    assert!(fb.iter().any(|f| f.contains("is a pet")), "got {fb:?}");
}

/// A spawnable prop (class 0) is not an NPC.
#[tokio::test]
async fn despawn_by_id_refuses_a_non_npc_entity() {
    let (mut mgr, gm, _npc) = setup();
    let prop = second_npc(&mut mgr);
    mgr.get_entity_mut(prop).unwrap().class_id = 0x00;

    let fb = run(&mut mgr, gm, &format!(".despawn {prop}")).await;

    assert!(mgr.get_entity(prop).is_some(), "a prop is never despawned");
    assert!(fb.iter().any(|f| f.contains("is not an NPC")), "got {fb:?}");
}

#[tokio::test]
async fn despawn_by_id_refuses_an_npc_in_another_space() {
    let (mut mgr, gm, _npc) = setup();
    mgr.parse_spaces_xml(
        r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle" Instanced="false" MinX="0" MaxX="100" MinY="0" MaxY="100" /></Spaces>"#,
    )
    .unwrap();
    mgr.create_startup_spaces(
        r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle" /></Spaces>"#,
    )
    .unwrap();
    let far = mgr.allocate_npc_id();
    mgr.spawn_npc(far, "Castle", [5.0, 0.0, 5.0], [0.0; 3])
        .unwrap();

    let fb = run(&mut mgr, gm, &format!(".despawn {far}")).await;

    assert!(
        mgr.get_entity(far).is_some(),
        "another map's NPC is untouched"
    );
    assert!(
        fb.iter().any(|f| f.contains("is in another space")),
        "got {fb:?}"
    );
}
