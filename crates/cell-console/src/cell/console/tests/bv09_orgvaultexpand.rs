//! BV-09 console: `.orgvaultexpand [team|command] [from_slots]` hands a
//! quote or a purchase of the GM's Team vault to the base, arguments it
//! cannot parse are refused with a usage line, and a non-GM's
//! `.orgvaultexpand` is refused with a visible line instead of being said
//! aloud. The purchase itself is pinned in `cimmeria-base-methods`
//! `inventory/org_vault/tests/expand/`.
//!
//! Filter prefix: `bv09_console_`.

use cimmeria_common::EntityId;
use cimmeria_content_engine::chain::ChainEngine;
use cimmeria_entity::cell_entity::VaultScope;
use tokio::sync::mpsc;

use super::{decode_feedback, setup};
use crate::cell::console::bank::{parse_org_expand_args, ORG_EXPAND_USAGE};
use crate::cell::console::chat::{handle_chat_message, CHAN_SAY};
use crate::cell::messages::{BankCellToBase, CellToBaseMsg};
use crate::cell::space_manager::SpaceManager;

/// Say `text` as `speaker`; the bank messages, the recipients of every
/// entity method, and the feedback lines.
async fn say(
    mgr: &mut SpaceManager,
    speaker: u32,
    text: &str,
) -> (Vec<BankCellToBase>, Vec<u32>, Vec<String>) {
    let (tx, mut rx) = mpsc::channel::<CellToBaseMsg>(64);
    handle_chat_message(
        speaker,
        "Tester",
        0,
        CHAN_SAY,
        text,
        &tx,
        mgr,
        &ChainEngine::new(),
    )
    .await;
    let (mut bank, mut to, mut lines) = (Vec::new(), Vec::new(), Vec::new());
    while let Ok(msg) = rx.try_recv() {
        if let Some(t) = decode_feedback(&msg) {
            lines.push(t);
        }
        match msg {
            CellToBaseMsg::Bank(b) => bank.push(b),
            CellToBaseMsg::EntityMethodCall { entity_id, .. } => to.push(entity_id),
            _ => {}
        }
    }
    (bank, to, lines)
}

/// Each form reaches the base as one `OrgVaultExpand` with the GM's own
/// character, the scope (Team by default) and the size (none for a quote).
/// No vault session is needed. Fails if the registry row or the dispatch
/// arm is removed (the console answers "Unknown command" and sends
/// nothing).
#[tokio::test]
async fn bv09_console_orgvaultexpand_sends_the_scope_and_the_size() {
    let (mut mgr, gm, _npc) = setup();
    mgr.get_entity_mut(gm).unwrap().player_id = Some(77);
    for (text, scope, from) in [
        (".orgvaultexpand", VaultScope::Team, None),
        (".orgvaultexpand 40", VaultScope::Team, Some(40)),
        (".orgvaultexpand team 60", VaultScope::Team, Some(60)),
        (".orgvaultexpand command", VaultScope::Command, None),
    ] {
        let (bank, _, _) = say(&mut mgr, gm, text).await;
        let [BankCellToBase::OrgVaultExpand {
            entity_id,
            player_id,
            scope: s,
            from_slots,
            ..
        }] = bank.as_slice()
        else {
            panic!("{text}: one OrgVaultExpand: {bank:?}");
        };
        assert_eq!((*entity_id, *player_id), (gm, 77), "{text}");
        assert_eq!((*s, *from_slots), (scope, from), "{text}");
    }
}

/// Arguments that are not `[team|command] [from_slots]` send nothing and
/// get the usage line, with WARN `expand_rejected reason=bad_args`.
#[tokio::test]
async fn bv09_console_orgvaultexpand_bad_args_are_refused() {
    assert_eq!(parse_org_expand_args(&["squad"]), None);
    assert_eq!(parse_org_expand_args(&["40", "50"]), None);
    assert_eq!(parse_org_expand_args(&["team", "command"]), None);
    assert_eq!(
        parse_org_expand_args(&["50", "Command"]),
        Some((VaultScope::Command, Some(50)))
    );

    let (mut mgr, gm, _npc) = setup();
    mgr.get_entity_mut(gm).unwrap().player_id = Some(77);
    let capture = crate::test_support::LogCapture::install();
    let (bank, _, lines) = say(&mut mgr, gm, ".orgvaultexpand guild").await;
    assert!(bank.is_empty(), "{bank:?}");
    assert_eq!(lines, vec![ORG_EXPAND_USAGE.to_string()]);
    let rejected: Vec<_> = capture
        .all()
        .into_iter()
        .filter(|c| c.target == "bank" && c.has_field("event", "expand_rejected"))
        .collect();
    assert_eq!(rejected.len(), 1, "{rejected:#?}");
    assert!(rejected[0].has_field("reason", "bad_args"));
    assert!(rejected[0].has_field("player_id", "77"));
}

/// A non-GM's `.orgvaultexpand` reaches neither the base nor a witness,
/// and tells the player why: WARN `expand_rejected reason=not_gm
/// scope=team`. Fails if the chat interceptor's refusal is removed (the
/// generic GM-command line replaces this one, with no `bank` row).
#[tokio::test]
async fn bv09_console_orgvaultexpand_is_refused_for_a_non_gm() {
    let (mut mgr, player, _npc) = setup();
    mgr.create_entity(2, "Agnos", [11.0, 0.0, 11.0], [0.0; 3])
        .unwrap();
    mgr.connect_entity(2);
    {
        let p = mgr.get_entity_mut(player).unwrap();
        p.access_level = 0;
        p.player_id = Some(78);
        p.witnesses.insert(EntityId(2));
    }
    let capture = crate::test_support::LogCapture::install();

    let (bank, to, lines) = say(&mut mgr, player, ".orgvaultexpand 40").await;

    assert!(bank.is_empty(), "nothing reaches the base: {bank:?}");
    assert!(
        to.iter().all(|e| *e == player),
        "nothing to the witness: {to:?}"
    );
    assert_eq!(
        lines,
        vec![".orgvaultexpand needs GM access. Nothing was charged.".to_string()]
    );
    let rejected: Vec<_> = capture
        .all()
        .into_iter()
        .filter(|c| c.target == "bank" && c.has_field("event", "expand_rejected"))
        .collect();
    assert_eq!(rejected.len(), 1, "{rejected:#?}");
    assert_eq!(rejected[0].level, tracing::Level::WARN);
    assert!(rejected[0].has_field("reason", "not_gm"));
    assert!(rejected[0].has_field("scope", "team"));
    assert!(rejected[0].has_field("player_id", "78"));
}
