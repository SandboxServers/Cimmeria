//! `Action::SpawnSet` (Debug Area DA-10): the Lineup attendants.
//!
//! - A GM's click shows the group for the whole world and answers on the
//!   first press; the next group's click switches the first off.
//! - A non-GM gets the refusal line and nothing spawns.
//! - "Clear lineup" switches off whatever group is on, and says so when
//!   nothing is.
//!
//! Revert proof: drop the `is_gm` check in `spawn_set::run` and
//! `a_non_gm_is_refused_and_nothing_spawns` fails on the actor count.

use cimmeria_cell_world::test_fixtures::{
    install_lineup_sets, LINEUP_KIND, LINEUP_SET_A, LINEUP_SET_A_NAME, LINEUP_SET_A_SIZE,
    LINEUP_SET_B, LINEUP_SET_B_SIZE, LINEUP_WORLD_ID,
};
use cimmeria_content_engine::actions::SpawnSetOp;

use super::*;
use crate::cell::client_methods::communicator::ON_PLAYER_COMMUNICATION;
use crate::test_support::LogCapture;

const PLAYER: u32 = 31;
const PLAYER_ID: i32 = 4343;
/// `account.accesslevel` of a GameMaster.
const GM_LEVEL: u32 = 2;

fn world(access_level: u32) -> SpaceManager {
    let mut mgr = make_space_mgr();
    mgr.create_entity(PLAYER, "Agnos", [0.0; 3], [0.0; 3])
        .unwrap();
    let p = mgr.get_entity_mut(PLAYER).unwrap();
    p.is_player = true;
    p.player_id = Some(PLAYER_ID);
    p.access_level = access_level;
    mgr.connect_entity(PLAYER);
    install_lineup_sets(&mut mgr, "Agnos");
    mgr
}

/// Each op its own chain, as seeded (one attendant per button), so the
/// debounce never swallows a different button.
async fn click(mgr: &mut SpaceManager, op: SpawnSetOp) -> Vec<CellToBaseMsg> {
    let chain_id = match &op {
        SpawnSetOp::Show { set_id } | SpawnSetOp::Hide { set_id } => i64::from(*set_id),
        SpawnSetOp::Clear { .. } => 1,
    };
    let (tx, mut rx) = mpsc::channel(64);
    execute_actions(
        ResolvedActions {
            action_delays: Vec::new(),
            params: std::collections::HashMap::new(),
            actions: vec![(chain_id, Action::SpawnSet(op))],
        },
        PLAYER,
        PLAYER_ID,
        &tx,
        mgr,
        &ChainEngine::new(),
    )
    .await;
    std::iter::from_fn(|| rx.try_recv().ok()).collect()
}

fn says(msgs: &[CellToBaseMsg], text: &str) -> bool {
    let needle: Vec<u8> = text.encode_utf16().flat_map(u16::to_le_bytes).collect();
    msgs.iter().any(|m| match m {
        CellToBaseMsg::EntityMethodCall {
            entity_id: PLAYER,
            method_index: ON_PLAYER_COMMUNICATION,
            args,
        } => args.windows(needle.len()).any(|w| w == needle.as_slice()),
        _ => false,
    })
}

fn npc_count(mgr: &SpaceManager) -> usize {
    mgr.spaces
        .values()
        .flat_map(|s| s.entities.values())
        .filter(|e| !e.is_player)
        .count()
}

#[tokio::test]
async fn a_gm_click_shows_the_group_and_the_next_switches_it_off() {
    let capture = LogCapture::install();
    let mut mgr = world(GM_LEVEL);
    let msgs = click(
        &mut mgr,
        SpawnSetOp::Show {
            set_id: LINEUP_SET_A,
        },
    )
    .await;
    assert_eq!(npc_count(&mgr), LINEUP_SET_A_SIZE);
    assert!(
        says(
            &msgs,
            &format!("{LINEUP_SET_A_NAME}: showing {LINEUP_SET_A_SIZE} actors")
        ),
        "first-click line: {msgs:#?}"
    );
    assert!(says(&msgs, "one group at a time"));

    let msgs = click(
        &mut mgr,
        SpawnSetOp::Show {
            set_id: LINEUP_SET_B,
        },
    )
    .await;
    assert_eq!(npc_count(&mgr), LINEUP_SET_B_SIZE, "A went off first");
    assert!(says(&msgs, "Cleared first"), "{msgs:#?}");

    let row = capture
        .all()
        .into_iter()
        .find(|c| c.has_field("event", "spawn_set.switched"))
        .expect("a spawn_set.switched row");
    assert!(row.has_field("door", "attendant"), "{row:#?}");
    assert!(row.has_field("decision_outcome", "shown"), "{row:#?}");
    assert!(
        row.has_field("player_id", &PLAYER_ID.to_string()),
        "{row:#?}"
    );
}

#[tokio::test]
async fn a_non_gm_is_refused_and_nothing_spawns() {
    let capture = LogCapture::install();
    let mut mgr = world(0);
    let msgs = click(
        &mut mgr,
        SpawnSetOp::Show {
            set_id: LINEUP_SET_A,
        },
    )
    .await;
    assert_eq!(npc_count(&mgr), 0, "nothing spawned for a player");
    assert!(says(&msgs, "Only a GM can switch the lineup"), "{msgs:#?}");
    let row = capture
        .all()
        .into_iter()
        .find(|c| c.has_field("event", "spawn_set.switched"))
        .expect("the refusal is logged");
    assert!(row.has_field("decision_outcome", "refused"), "{row:#?}");
    assert!(row.has_field("reason", "not_gm"), "{row:#?}");
}

#[tokio::test]
async fn clear_lineup_switches_off_the_group_that_is_on() {
    let mut mgr = world(GM_LEVEL);
    let clear = || SpawnSetOp::Clear {
        kind: LINEUP_KIND.to_string(),
        world_id: LINEUP_WORLD_ID,
    };
    let msgs = click(&mut mgr, clear()).await;
    assert!(says(&msgs, "Nothing to clear"), "{msgs:#?}");

    click(
        &mut mgr,
        SpawnSetOp::Show {
            set_id: LINEUP_SET_B,
        },
    )
    .await;
    // A second chain id, so the debounce of the first clear does not apply.
    let (tx, mut rx) = mpsc::channel(64);
    execute_actions(
        ResolvedActions {
            action_delays: Vec::new(),
            params: std::collections::HashMap::new(),
            actions: vec![(2, Action::SpawnSet(clear()))],
        },
        PLAYER,
        PLAYER_ID,
        &tx,
        &mut mgr,
        &ChainEngine::new(),
    )
    .await;
    let msgs: Vec<_> = std::iter::from_fn(|| rx.try_recv().ok()).collect();
    assert_eq!(npc_count(&mgr), 0);
    assert!(says(&msgs, "Cleared"), "{msgs:#?}");
}
