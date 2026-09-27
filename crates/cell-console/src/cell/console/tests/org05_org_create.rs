//! ORG-05: `.org_create <team|command> <name>` forwards a `GmCreate` for the
//! GM's own character, and the console's own refusals (not a GM, a bad
//! type) each write one `org.gm_action` row (TESTING.md type 12). The base
//! half (the access-level re-check and the creation) is tested in
//! `cimmeria-base-session` (`organization::creation::tests::handlers`).

use cimmeria_content_engine::chain::ChainEngine;
use cimmeria_entity::organization::OrgType;
use tokio::sync::mpsc;
use tracing::Level;

use super::{decode_feedback, exec, setup};
use crate::cell::messages::{CellToBaseMsg, OrgCellToBase};
use crate::cell::space_manager::SpaceManager;
use crate::test_support::LogCapture;

/// The console fixture's caller (entity 1) as character 7101 at
/// `access_level`.
fn fixture(access_level: u32) -> (SpaceManager, u32) {
    let (mut mgr, caller, _npc) = setup();
    let e = mgr.get_entity_mut(caller).unwrap();
    e.player_id = Some(7101);
    e.account_id = Some(8101);
    e.access_level = access_level;
    (mgr, caller)
}

async fn run(mgr: &mut SpaceManager, caller: u32, args: &[&str]) -> Vec<CellToBaseMsg> {
    let (tx, mut rx) = mpsc::channel(16);
    exec(
        "org_create",
        caller,
        args,
        None,
        &tx,
        mgr,
        &ChainEngine::new(),
    )
    .await;
    drop(tx);
    let mut out = Vec::new();
    while let Some(m) = rx.recv().await {
        out.push(m);
    }
    out
}

/// A GM's command forwards exactly one `GmCreate` for their own character,
/// with the type parsed case-insensitively and the name rejoined; the base
/// writes the outcome row, so the console writes none.
#[tokio::test]
async fn gm_org_create_forwards_for_the_gm() {
    let (mut mgr, gm) = fixture(2);
    let capture = LogCapture::install();

    let msgs = run(&mut mgr, gm, &["Command", "The", "Lucian", "Alliance"]).await;

    let org: Vec<&OrgCellToBase> = msgs
        .iter()
        .filter_map(|m| match m {
            CellToBaseMsg::Org(o) => Some(o),
            _ => None,
        })
        .collect();
    assert_eq!(
        org,
        vec![&OrgCellToBase::GmCreate {
            player_id: 7101,
            entity_id: gm,
            org_type: OrgType::Command,
            name: "The Lucian Alliance".into(),
        }]
    );
    assert!(capture
        .all()
        .iter()
        .all(|c| !c.has_field("event", "org.gm_action")));
}

/// Not a GM, or a type that is neither team nor command: a line, one
/// `rejected` row with the console's reason, and nothing for the base.
#[tokio::test]
async fn org_create_refusals_are_visible_and_logged() {
    for (access, args, reason, line) in [
        (
            0,
            vec!["team", "Nope"],
            "not_gm",
            ".org_create is a GM command.",
        ),
        (
            2,
            vec!["squad", "Nope"],
            "usage",
            "Usage: .org_create <team|command> <name>",
        ),
    ] {
        let (mut mgr, caller) = fixture(access);
        let capture = LogCapture::install();

        let msgs = run(&mut mgr, caller, &args).await;

        assert!(
            msgs.iter().all(|m| !matches!(m, CellToBaseMsg::Org(_))),
            "{reason}: nothing may reach the base"
        );
        let lines: Vec<String> = msgs.iter().filter_map(decode_feedback).collect();
        assert_eq!(lines, vec![line.to_owned()], "{reason}");
        let rows: Vec<_> = capture
            .all()
            .into_iter()
            .filter(|c| c.has_field("event", "org.gm_action"))
            .collect();
        assert_eq!(rows.len(), 1, "{reason}: {rows:#?}");
        let r = &rows[0];
        assert_eq!((r.level, r.target.as_str()), (Level::INFO, "org"));
        for (k, v) in [
            ("action", "gm_org_create"),
            ("outcome", "rejected"),
            ("reason", reason),
            ("player_id", "7101"),
            ("account_id", "8101"),
        ] {
            assert!(r.has_field(k, v), "{k}={v}: {:?}", r.fields);
        }
    }
}
