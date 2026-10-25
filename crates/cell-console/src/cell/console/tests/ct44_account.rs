//! CT-44 console: `.account identities|unlink|linkcode` forwards the GM's
//! command to the base with the GM's own ids (client-tls-oidc C10). The
//! cell checks the account name and the identity id by hand and sends
//! nothing for a malformed line.
//!
//! The base half (the DB reads, the feedback lines) is tested in the base
//! crate. Filter prefix: `ct44_account`.

use tokio::sync::mpsc;

use super::{decode_feedback, setup};
use crate::cell::console::chat::{handle_chat_message, CHAN_SAY};
use crate::cell::messages::{AccountCellToBase, CellToBaseMsg};
use crate::cell::space_manager::SpaceManager;
use crate::test_support::{Captured, LogCapture, LogCaptureGuard};
use cimmeria_common::EntityId;
use cimmeria_content_engine::chain::ChainEngine;

const GM_ACCOUNT: u32 = 31;
const GM_PLAYER: i32 = 41;

/// Say `text` as `speaker`; return every message sent and the feedback
/// lines among them.
async fn say(
    mgr: &mut SpaceManager,
    speaker: u32,
    text: &str,
) -> (Vec<CellToBaseMsg>, Vec<String>) {
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
    let (mut msgs, mut lines) = (Vec::new(), Vec::new());
    while let Ok(msg) = rx.try_recv() {
        if let Some(t) = decode_feedback(&msg) {
            lines.push(t);
        }
        msgs.push(msg);
    }
    (msgs, lines)
}

fn accounts(msgs: &[CellToBaseMsg]) -> Vec<&AccountCellToBase> {
    msgs.iter()
        .filter_map(|m| match m {
            CellToBaseMsg::Account(a) => Some(a),
            _ => None,
        })
        .collect()
}

fn gm_actions(capture: &LogCaptureGuard) -> Vec<Captured> {
    capture
        .all()
        .into_iter()
        .filter(|c| c.target == "account_admin" && c.has_field("event", "gm_action"))
        .collect()
}

fn with_identity(mgr: &mut SpaceManager, gm: u32) {
    let e = mgr.get_entity_mut(gm).unwrap();
    e.account_id = Some(GM_ACCOUNT);
    e.player_id = Some(GM_PLAYER);
}

/// `.account identities <account>` reaches the base with the GM's ids and
/// the typed account name. Fails if the registry row or the dispatch arm is
/// removed, or if the cell has no `Account` message (C10: the cell has no
/// pool, so the base must read the identities).
#[tokio::test]
async fn account_identities_forwards_to_base() {
    let (mut mgr, gm, _npc) = setup();
    with_identity(&mut mgr, gm);
    let (msgs, lines) = say(&mut mgr, gm, ".account identities Player_1234").await;
    assert_eq!(
        accounts(&msgs),
        vec![&AccountCellToBase::Identities {
            entity_id: gm,
            account_id: Some(GM_ACCOUNT),
            account_name: "Player_1234".to_string(),
        }],
        "{msgs:?} {lines:?}"
    );
}

/// `.account unlink <account> <identity_id>` parses the id as an `i64`.
#[tokio::test]
async fn account_unlink_parses_identity_id() {
    let (mut mgr, gm, _npc) = setup();
    with_identity(&mut mgr, gm);
    let (msgs, _) = say(&mut mgr, gm, ".account unlink Player_1234 42").await;
    assert_eq!(
        accounts(&msgs),
        vec![&AccountCellToBase::Unlink {
            entity_id: gm,
            account_id: Some(GM_ACCOUNT),
            account_name: "Player_1234".to_string(),
            identity_id: 42,
        }],
        "{msgs:?}"
    );
}

/// `.account linkcode <account>` forwards the link-code request.
#[tokio::test]
async fn account_linkcode_forwards() {
    let (mut mgr, gm, _npc) = setup();
    with_identity(&mut mgr, gm);
    let (msgs, _) = say(&mut mgr, gm, ".account linkcode Player_1234").await;
    assert_eq!(
        accounts(&msgs),
        vec![&AccountCellToBase::LinkCode {
            entity_id: gm,
            account_id: Some(GM_ACCOUNT),
            account_name: "Player_1234".to_string(),
        }],
        "{msgs:?}"
    );
}

/// A too-short name, a name with a bad character, a 21-character name, a
/// zero identity id and a bad sub-command each answer feedback and send
/// nothing to the base. Fails if the cell forwards a GM typo to the DB.
#[tokio::test]
async fn account_bad_name_and_usage_stay_on_cell() {
    let (mut mgr, gm, _npc) = setup();
    with_identity(&mut mgr, gm);
    let cases = [
        ".account identities a",
        ".account identities x!y",
        ".account identities x y",
        ".account identities abcdefghijklmnopqrstu",
        ".account unlink Player_1234 0",
        ".account ban Player_1234",
    ];
    for text in cases {
        let (msgs, lines) = say(&mut mgr, gm, text).await;
        assert!(accounts(&msgs).is_empty(), "{text}: {msgs:?}");
        assert!(!lines.is_empty(), "{text}: no feedback");
    }
}

/// The base logs the kind of every cell message, and `.account` is "account".
#[test]
fn account_kind_label() {
    let msg = CellToBaseMsg::Account(AccountCellToBase::LinkCode {
        entity_id: 1,
        account_id: None,
        account_name: "Player_1234".to_string(),
    });
    assert_eq!(msg.kind(), "account");
}

/// The word `.account` is matched, not a prefix of it.
#[test]
fn is_account_command_matches_word_only() {
    use crate::cell::console::account::is_account_command;
    assert!(is_account_command(".account x"));
    assert!(!is_account_command(".accountx"));
}

/// A non-GM's `.account` is refused like every GM command ("is a GM
/// command", never sent to the base) and also logs `gm_action result=refused
/// reason=not_gm` on the `account_admin` target. Fails if the chat hook is
/// removed (no `account_admin` row) or the generic refusal is (the line
/// reaches the witness).
#[tokio::test]
async fn account_is_refused_for_a_non_gm() {
    let (mut mgr, player, _npc) = setup();
    with_identity(&mut mgr, player);
    mgr.create_entity(2, "Agnos", [11.0, 0.0, 11.0], [0.0; 3])
        .unwrap();
    mgr.connect_entity(2);
    {
        let p = mgr.get_entity_mut(player).unwrap();
        p.access_level = 0;
        p.witnesses.insert(EntityId(2));
    }

    let capture = LogCapture::install();
    let (msgs, lines) = say(&mut mgr, player, ".account unlink Player_1234 42").await;

    assert!(accounts(&msgs).is_empty(), "{msgs:?}");
    assert!(
        msgs.iter()
            .all(|m| !matches!(m, CellToBaseMsg::EntityMethodCall { entity_id: 2, .. })),
        "nothing reaches the witness: {msgs:?}"
    );
    assert!(
        lines.iter().any(|l| l.contains("is a GM command")),
        "{lines:?}"
    );
    let events = gm_actions(&capture);
    assert_eq!(events.len(), 1, "{events:#?}");
    let e = &events[0];
    assert_eq!(e.level, tracing::Level::INFO);
    assert!(e.has_field("result", "refused"));
    assert!(e.has_field("reason", "not_gm"));
    assert!(e.has_field("account_id", &GM_ACCOUNT.to_string()));
    assert!(e.has_field("entity_id", &player.to_string()));
}

/// A closed base channel logs `gm_action result=refused
/// reason=base_channel_closed` with the error, and never the typed account
/// name. Fails if the `warn!` row or its reason changes.
#[tokio::test]
async fn account_logs_a_closed_base_channel() {
    let (mut mgr, gm, _npc) = setup();
    with_identity(&mut mgr, gm);
    let (tx, rx) = mpsc::channel::<CellToBaseMsg>(4);
    drop(rx);

    let capture = LogCapture::install();
    crate::cell::console::dispatch::exec(
        "account",
        gm,
        &["linkcode", "Player_1234"],
        None,
        &tx,
        &mut mgr,
        &ChainEngine::new(),
    )
    .await;

    let events = gm_actions(&capture);
    assert_eq!(events.len(), 1, "{events:#?}");
    let e = &events[0];
    assert_eq!(e.level, tracing::Level::WARN);
    assert!(e.has_field("reason", "base_channel_closed"));
    assert!(e.has_field("action", "account_linkcode"));
    assert!(e.has_field("account_id", &GM_ACCOUNT.to_string()));
    assert!(e.fields.contains_key("error"));
    assert!(
        e.fields.values().all(|v| !v.contains("Player_1234")),
        "the typed name is never logged: {e:?}"
    );
}
