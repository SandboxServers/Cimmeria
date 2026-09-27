//! NA13 console: the GM's `.aggro on|off` proximity-aggro switch (D-NA02)
//! and `.aggression`'s `EMobAggressionLevel` override.
//!
//! Filter prefix: `na13_console_`.

use cimmeria_content_engine::chain::ChainEngine;
use cimmeria_entity::cell_entity::MobAggression;
use tokio::sync::mpsc;

use super::super::handle_console_command;
use super::{decode_feedback, setup};
use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;

const CHARACTER: i32 = 500;

async fn run(mgr: &mut SpaceManager, gm: u32, text: &str) -> Vec<String> {
    let (tx, mut rx) = mpsc::channel::<CellToBaseMsg>(64);
    handle_console_command(gm, text, &tx, mgr, &ChainEngine::new()).await;
    let mut lines = Vec::new();
    while let Ok(msg) = rx.try_recv() {
        if let Some(t) = decode_feedback(&msg) {
            lines.push(t);
        }
    }
    lines
}

fn gm_setup() -> (SpaceManager, u32, u32) {
    let (mut mgr, gm, npc) = setup();
    mgr.get_entity_mut(gm).unwrap().player_id = Some(CHARACTER);
    (mgr, gm, npc)
}

/// `.aggro off` / `.aggro on` flip the character-keyed switch, and every use
/// (including a bare `.aggro` query) answers on the feedback channel.
#[tokio::test]
async fn na13_console_aggro_toggles_and_always_answers() {
    let (mut mgr, gm, _) = gm_setup();

    let lines = run(&mut mgr, gm, ".aggro").await;
    assert_eq!(lines.len(), 1, "{lines:?}");
    assert!(lines[0].contains("ON"), "{lines:?}");
    assert!(!mgr.gm_aggro_off.contains(&CHARACTER));

    let lines = run(&mut mgr, gm, ".aggro off").await;
    assert!(
        mgr.gm_aggro_off.contains(&CHARACTER),
        "switch set by character id"
    );
    assert!(
        lines[0].starts_with("aggro set") && lines[0].contains("OFF"),
        "{lines:?}"
    );

    let lines = run(&mut mgr, gm, ".aggro off").await;
    assert!(lines[0].starts_with("aggro unchanged"), "{lines:?}");

    let lines = run(&mut mgr, gm, ".aggro ON").await;
    assert!(!mgr.gm_aggro_off.contains(&CHARACTER));
    assert!(lines[0].contains("ON"), "{lines:?}");
}

/// A bad argument changes nothing and still gets a visible answer.
#[tokio::test]
async fn na13_console_aggro_rejects_a_bad_argument_visibly() {
    let (mut mgr, gm, _) = gm_setup();
    let lines = run(&mut mgr, gm, ".aggro maybe").await;
    assert_eq!(lines.len(), 1, "{lines:?}");
    assert!(lines[0].contains("expected 'on' or 'off'"), "{lines:?}");
    assert!(mgr.gm_aggro_off.is_empty());
}

/// `.aggression` takes `EMobAggressionLevel` values and `clear`, reports the
/// effective level, and refuses out-of-range levels with feedback.
#[tokio::test]
async fn na13_console_aggression_sets_and_clears_the_override() {
    let (mut mgr, gm, npc) = gm_setup();
    mgr.get_entity_mut(npc).unwrap().faction = crate::cell::combat::HOSTILE_FACTION;

    let lines = run(&mut mgr, gm, ".aggression 3").await;
    assert_eq!(
        mgr.get_entity(npc).unwrap().aggro.override_level,
        Some(MobAggression::Neutral)
    );
    assert!(lines[0].contains("neutral (override)"), "{lines:?}");

    let lines = run(&mut mgr, gm, ".aggression clear").await;
    assert_eq!(mgr.get_entity(npc).unwrap().aggro.override_level, None);
    assert!(lines[0].contains("hostile (faction)"), "{lines:?}");

    let lines = run(&mut mgr, gm, ".aggression 9").await;
    assert_eq!(mgr.get_entity(npc).unwrap().aggro.override_level, None);
    assert!(lines[0].contains("not 0-5"), "{lines:?}");
}

/// NA33: `.aggression <level>` broadcasts `onAggressionOverrideUpdate` and
/// `.aggression clear` broadcasts `onAggressionOverrideCleared` to every
/// witness of the target — the visible half of the command, not just the
/// server-side field write `na13_console_aggression_sets_and_clears_the_override`
/// already pins.
#[tokio::test]
async fn na13_console_aggression_broadcasts_update_then_cleared() {
    use crate::mercury::method_idx::{
        ON_AGGRESSION_OVERRIDE_CLEARED, ON_AGGRESSION_OVERRIDE_UPDATE,
    };
    use cimmeria_common::EntityId;

    let (mut mgr, gm, npc) = gm_setup();
    // A witness distinct from the GM issuing the command, so the fan-out
    // isn't accidentally validated against the caller's own client.
    const WITNESS: u32 = 77;
    mgr.create_entity(WITNESS, "Agnos", [0.0; 3], [0.0; 3])
        .unwrap();
    let w = mgr.get_entity_mut(WITNESS).unwrap();
    w.is_player = true;
    w.player_id = Some(900);
    w.witnesses.insert(EntityId(npc as i32));
    mgr.connect_entity(WITNESS);

    let (tx, mut rx) = mpsc::channel::<CellToBaseMsg>(64);
    handle_console_command(gm, ".aggression 1", &tx, &mut mgr, &ChainEngine::new()).await;

    let mut wire = Vec::new();
    while let Ok(msg) = rx.try_recv() {
        if let CellToBaseMsg::WitnessEntityMethod {
            witness_id,
            entity_id,
            method_index,
            args,
            ..
        } = msg
        {
            wire.push((witness_id, entity_id, method_index, args));
        }
    }
    assert_eq!(
        wire,
        vec![(
            WITNESS,
            npc,
            ON_AGGRESSION_OVERRIDE_UPDATE,
            vec![MobAggression::Hostile.level()]
        )],
        "level=1 must broadcast onAggressionOverrideUpdate(HOSTILE) to the witness"
    );

    handle_console_command(gm, ".aggression clear", &tx, &mut mgr, &ChainEngine::new()).await;
    let mut wire = Vec::new();
    while let Ok(msg) = rx.try_recv() {
        if let CellToBaseMsg::WitnessEntityMethod {
            witness_id,
            entity_id,
            method_index,
            args,
            ..
        } = msg
        {
            wire.push((witness_id, entity_id, method_index, args));
        }
    }
    assert_eq!(
        wire,
        vec![(WITNESS, npc, ON_AGGRESSION_OVERRIDE_CLEARED, Vec::new())],
        "clear must broadcast onAggressionOverrideCleared with an empty payload"
    );
}
