//! `Action::GmAbilityBulk` (Debug Area DA-02): the ability granter and the
//! ability reset NPC.
//!
//! - A GM's grant sends the base exactly the abilities of their archetype
//!   tree they do not know (every branch, no repeats), clears their
//!   cooldowns with a clear timer each, and answers on the first click.
//! - A non-GM gets the refusal line and nothing else: no base message, no
//!   cooldown touched.
//! - A GM who knows the whole tree gets a line, and nothing goes to the
//!   base.
//! - The reset forwards `Reset` for the GM's own character.
//!
//! Revert proofs: drop the `is_gm` check in `ability_granter::run` and
//! `a_non_gm_is_refused_and_nothing_changes` fails; drop the cooldown reset
//! and `a_gm_gets_the_whole_tree_and_cleared_cooldowns` fails.

use std::time::Duration;

use cimmeria_cell_catalog::ability_tree::{AbilityTreeCatalog, TreeNode};
use cimmeria_content_engine::actions::AbilityBulkChange;

use super::*;
use crate::cell::client_methods::communicator::ON_PLAYER_COMMUNICATION;
use crate::cell::messages::{GmAbilityBulk, GmAbilityChange};
use crate::test_support::LogCapture;

const PLAYER: u32 = 30;
const PLAYER_ID: i32 = 4242;
const NPC: u32 = 100_030;
/// Commando, and its fixture tree. 641 sits in two branches.
const ARCHETYPE: i32 = 2;
const KNOWN: i32 = 597;
const COOLING: i32 = 641;
/// `account.accesslevel` of a GameMaster.
const GM_LEVEL: u32 = 2;

fn world(access_level: u32) -> SpaceManager {
    let mut mgr = make_space_mgr();
    mgr.create_entity(PLAYER, "Agnos", [0.0; 3], [0.0; 3])
        .unwrap();
    let p = mgr.get_entity_mut(PLAYER).unwrap();
    p.is_player = true;
    p.player_id = Some(PLAYER_ID);
    p.archetype_id = Some(ARCHETYPE);
    p.access_level = access_level;
    p.abilities.add_ability(KNOWN);
    p.abilities.add_ability(COOLING);
    p.abilities
        .start_ability_cooldown(COOLING, Duration::from_secs(60));
    mgr.connect_entity(PLAYER);
    mgr.spawn_npc(NPC, "Agnos", [2.0, 0.0, 0.0], [0.0; 3])
        .unwrap();
    mgr.ability_tree_catalog = AbilityTreeCatalog::from_nodes([
        TreeNode::with_defaults(ARCHETYPE, 0, KNOWN, 1, vec![]),
        TreeNode::with_defaults(ARCHETYPE, 0, COOLING, 1, vec![KNOWN]),
        TreeNode::with_defaults(ARCHETYPE, 0, 700, 1, vec![]),
        TreeNode::with_defaults(ARCHETYPE, 1, 642, 1, vec![]),
        TreeNode::with_defaults(ARCHETYPE, 1, COOLING, 1, vec![]),
        // A capstone: level 50, behind three prerequisites.
        TreeNode::with_defaults(ARCHETYPE, 2, 2826, 50, vec![642, 700, COOLING]),
        // Another archetype's tree is never granted.
        TreeNode::with_defaults(9, 0, 999, 1, vec![]),
    ]);
    mgr
}

fn resolved(change: AbilityBulkChange) -> ResolvedActions {
    let mut params = std::collections::HashMap::new();
    params.insert("target_entity_id".to_string(), serde_json::json!(NPC));
    ResolvedActions {
        action_delays: Vec::new(),
        params,
        // The seeded chains: 13000 the granter, 13001 the reset NPC.
        actions: vec![(chain_of(change), Action::GmAbilityBulk { change })],
    }
}

fn chain_of(change: AbilityBulkChange) -> i64 {
    match change {
        AbilityBulkChange::GrantAll => 13000,
        AbilityBulkChange::Reset => 13001,
    }
}

async fn click(mgr: &mut SpaceManager, change: AbilityBulkChange) -> Vec<CellToBaseMsg> {
    let (tx, mut rx) = mpsc::channel(64);
    execute_actions(
        resolved(change),
        PLAYER,
        PLAYER_ID,
        &tx,
        mgr,
        &ChainEngine::new(),
    )
    .await;
    std::iter::from_fn(|| rx.try_recv().ok()).collect()
}

fn utf16(text: &str) -> Vec<u8> {
    text.encode_utf16().flat_map(u16::to_le_bytes).collect()
}

/// The feedback lines sent to the player, as raw `onPlayerCommunication`
/// args.
fn lines(msgs: &[CellToBaseMsg]) -> Vec<&[u8]> {
    msgs.iter()
        .filter_map(|m| match m {
            CellToBaseMsg::EntityMethodCall {
                entity_id: PLAYER,
                method_index: ON_PLAYER_COMMUNICATION,
                args,
            } => Some(args.as_slice()),
            _ => None,
        })
        .collect()
}

fn says(msgs: &[CellToBaseMsg], text: &str) -> bool {
    let needle = utf16(text);
    lines(msgs)
        .iter()
        .any(|a| a.windows(needle.len()).any(|w| w == needle))
}

fn bulk(msgs: &[CellToBaseMsg]) -> Vec<&GmAbilityBulk> {
    msgs.iter()
        .filter_map(|m| match m {
            CellToBaseMsg::GmAbilityBulk(b) => Some(b),
            _ => None,
        })
        .collect()
}

#[tokio::test]
async fn a_gm_gets_the_whole_tree_and_cleared_cooldowns() {
    let capture = LogCapture::install();
    let mut mgr = world(GM_LEVEL);
    let msgs = click(&mut mgr, AbilityBulkChange::GrantAll).await;

    let sent = bulk(&msgs);
    assert_eq!(sent.len(), 1, "one base write: {msgs:#?}");
    let b = sent[0];
    assert_eq!(b.entity_id, PLAYER);
    assert_eq!(b.player_id, PLAYER_ID);
    assert_eq!(b.change, GmAbilityChange::GrantAll);
    assert_eq!(
        b.ability_ids,
        vec![700, 642, 2826],
        "every tree ability not known, tree order, once each, capstone included"
    );

    let p = mgr.get_entity(PLAYER).unwrap();
    assert!(!p.abilities.is_on_cooldown(COOLING), "cooldowns cleared");
    let clear_timer = crate::cell::abilities::clear_cooldown_timer(COOLING, PLAYER);
    assert!(
        msgs.iter().any(|m| matches!(
            m,
            CellToBaseMsg::EntityMethodCall { entity_id: PLAYER, args, .. } if *args == clear_timer
        )),
        "the client is sent the cooldown clear: {msgs:#?}"
    );
    assert!(
        says(&msgs, "granting 3 abilities of your Commando tree"),
        "first-click feedback: {msgs:#?}"
    );
    assert!(says(&msgs, "1 cooldown(s) cleared"));

    let row = capture
        .all()
        .into_iter()
        .find(|c| c.has_field("event", "ability_granter"))
        .expect("one ability_granter row");
    assert!(row.has_field("decision_outcome", "forwarded"), "{row:#?}");
    assert!(
        row.has_field("player_id", &PLAYER_ID.to_string()),
        "{row:#?}"
    );
    assert!(row.has_field("ability_count", "3"), "{row:#?}");
}

#[tokio::test]
async fn a_non_gm_is_refused_and_nothing_changes() {
    let capture = LogCapture::install();
    let mut mgr = world(0);
    let msgs = click(&mut mgr, AbilityBulkChange::GrantAll).await;

    assert!(
        bulk(&msgs).is_empty(),
        "nothing reaches the base: {msgs:#?}"
    );
    assert!(says(&msgs, "Only a GM can use this."), "{msgs:#?}");
    assert_eq!(lines(&msgs).len(), msgs.len(), "a line and nothing else");
    assert!(
        mgr.get_entity(PLAYER)
            .unwrap()
            .abilities
            .is_on_cooldown(COOLING),
        "a refused click clears no cooldown"
    );
    assert!(capture
        .all()
        .iter()
        .any(|c| c.has_field("event", "ability_granter")
            && c.has_field("decision_outcome", "refused")
            && c.has_field("reason", "not_gm")));

    let msgs = click(&mut mgr, AbilityBulkChange::Reset).await;
    assert!(bulk(&msgs).is_empty(), "the reset is gated too");
    assert!(says(&msgs, "Only a GM can use this."));
}

#[tokio::test]
async fn a_gm_who_knows_the_tree_gets_a_line_and_no_write() {
    let mut mgr = world(GM_LEVEL);
    let p = mgr.get_entity_mut(PLAYER).unwrap();
    for id in [700, 642, 2826] {
        p.abilities.add_ability(id);
    }
    let msgs = click(&mut mgr, AbilityBulkChange::GrantAll).await;
    assert!(bulk(&msgs).is_empty(), "{msgs:#?}");
    assert!(
        says(&msgs, "you already know all 6 abilities of your tree"),
        "{msgs:#?}"
    );
}

#[tokio::test]
async fn the_reset_npc_forwards_a_reset_for_the_gm() {
    let mut mgr = world(GM_LEVEL);
    let msgs = click(&mut mgr, AbilityBulkChange::Reset).await;
    let sent = bulk(&msgs);
    assert_eq!(sent.len(), 1, "{msgs:#?}");
    assert_eq!(sent[0].change, GmAbilityChange::Reset);
    assert_eq!(sent[0].player_id, PLAYER_ID);
    assert!(
        sent[0].ability_ids.is_empty(),
        "the base reads the starters"
    );
    assert!(says(&msgs, "back to your starter abilities"));
}

/// Review F3: a second click on the same NPC inside a second is dropped, so
/// a scripted client cannot queue a locked base write per packet. Another
/// NPC is its own window. Revert proof: drop the `chain_debounce` check and
/// the second click forwards a second write.
#[tokio::test]
async fn a_repeat_click_inside_the_debounce_window_writes_nothing() {
    let mut mgr = world(GM_LEVEL);
    assert_eq!(
        bulk(&click(&mut mgr, AbilityBulkChange::Reset).await).len(),
        1
    );
    let again = click(&mut mgr, AbilityBulkChange::Reset).await;
    assert!(again.is_empty(), "the repeat sends nothing: {again:#?}");
    assert_eq!(
        bulk(&click(&mut mgr, AbilityBulkChange::GrantAll).await).len(),
        1,
        "the granter is another chain"
    );
}

/// Review F4: the granter's request tells the base it came from the NPC.
#[tokio::test]
async fn the_granter_marks_its_request_as_an_npc_grant() {
    let mut mgr = world(GM_LEVEL);
    let msgs = click(&mut mgr, AbilityBulkChange::GrantAll).await;
    assert_eq!(
        bulk(&msgs)[0].source,
        crate::cell::messages::GmAbilitySource::NpcGranter
    );
}
