//! SS-U1: the GM mail tools `.mail`, `.mailbox` and `.mail_expire` on the
//! cell: the parsers, the message handed to the base, the refusals (type 12)
//! and the non-GM gate. The base half is tested in `cimmeria-base-methods`
//! (`mail/tests/gm_live.rs`).

use cimmeria_content_engine::chain::ChainEngine;
use tokio::sync::mpsc;
use tracing::Level;

use super::pt07_giveability::{say, world};
use super::setup;
use crate::cell::console::handle_console_command;
use crate::cell::console::mail::{
    parse_mail, parse_mail_expire, MailArgs, DEFAULT_SUBJECT, MAIL_EXPIRE_USAGE,
};
use crate::cell::messages::{CellToBaseMsg, MailGmActor, MailGmCellToBase};
use crate::test_support::LogCapture;

fn args(line: &str) -> Vec<&str> {
    line.split_whitespace().collect()
}

/// Every option, in any order, then the subject.
#[test]
fn mail_parse_reads_every_option_and_the_subject() {
    assert_eq!(
        parse_mail(&args("item 5001 3 to Bob cash 250 Payday at the gate")),
        Ok(MailArgs {
            to: Some("Bob".into()),
            cash: 250,
            item: Some((5001, 3)),
            cod: None,
            subject: "Payday at the gate".into(),
        })
    );
    assert_eq!(
        parse_mail(&args("TO bob Item 42 Cod 99")),
        Ok(MailArgs {
            to: Some("bob".into()),
            cash: 0,
            item: Some((42, 1)),
            cod: Some(99),
            subject: DEFAULT_SUBJECT.into(),
        })
    );
}

/// A bare `.mail` mails the GM a text-only mail with the default subject;
/// the first non-option word starts the subject, even if an option word
/// follows it.
#[test]
fn mail_parse_defaults_and_subject_boundary() {
    assert_eq!(
        parse_mail(&[]),
        Ok(MailArgs {
            to: None,
            cash: 0,
            item: None,
            cod: None,
            subject: DEFAULT_SUBJECT.into(),
        })
    );
    let parsed = parse_mail(&args("hello cash 5")).unwrap();
    assert_eq!(parsed.cash, 0);
    assert_eq!(parsed.subject, "hello cash 5");
    // A non-number after the type id is the subject, not the quantity.
    let parsed = parse_mail(&args("item 7 Seven")).unwrap();
    assert_eq!(parsed.item, Some((7, 1)));
    assert_eq!(parsed.subject, "Seven");
}

/// Each malformed option is refused with its own reason.
#[test]
fn mail_parse_refuses_malformed_options() {
    for (line, reason) in [
        ("to", "no_recipient_name"),
        ("cash", "invalid_cash"),
        ("cash -5", "invalid_cash"),
        ("cash lots", "invalid_cash"),
        ("cash 99999999999", "invalid_cash"),
        ("item", "invalid_item_type"),
        ("item 0", "invalid_item_type"),
        ("item 5 0", "invalid_item_quantity"),
        ("item 5 -2", "invalid_item_quantity"),
        ("cod 0 item 5", "invalid_cod"),
        ("cod 10", "cod_without_item"),
        ("cod 10 item 5 cash 3", "cod_with_cash"),
    ] {
        let r = parse_mail(&args(line)).expect_err(line);
        assert_eq!(r.reason, reason, "{line}");
        assert!(r.line.starts_with(".mail: "), "{line}: {}", r.line);
    }
}

#[test]
fn mail_expire_parse_needs_a_positive_id() {
    assert_eq!(parse_mail_expire(&["12"]), Ok(12));
    for bad in [&[][..], &["x"], &["0"], &["-3"]] {
        let r = parse_mail_expire(bad).unwrap_err();
        assert_eq!(r.reason, "no_mail_id");
        assert_eq!(r.line, MAIL_EXPIRE_USAGE);
    }
}

fn gm_world() -> (crate::cell::space_manager::SpaceManager, u32) {
    let (mut mgr, gm, _npc) = setup();
    let e = mgr.get_entity_mut(gm).unwrap();
    e.account_id = Some(7);
    e.player_id = Some(70);
    (mgr, gm)
}

async fn run(
    mgr: &mut crate::cell::space_manager::SpaceManager,
    gm: u32,
    line: &str,
) -> Vec<CellToBaseMsg> {
    let (tx, mut rx) = mpsc::channel(16);
    handle_console_command(gm, line, &tx, mgr, &ChainEngine::new()).await;
    std::iter::from_fn(|| rx.try_recv().ok()).collect()
}

/// The `MailGm` messages among `msgs`, which must be all of them.
fn mail_gm(msgs: Vec<CellToBaseMsg>) -> Vec<MailGmCellToBase> {
    msgs.into_iter()
        .map(|m| match m {
            CellToBaseMsg::MailGm(g) => g,
            other => panic!("expected only MailGm, got {other:?}"),
        })
        .collect()
}

/// `.mail` hands the base one `MailGm::Send` with the GM's ids from the
/// cell entity and the parsed options; nothing else is sent.
#[tokio::test]
async fn mail_forwards_one_send_with_the_gm_ids() {
    let (mut mgr, gm) = gm_world();
    let msgs = run(&mut mgr, gm, ".mail to Bob cash 10 item 5001 2 Hi").await;
    assert_eq!(
        mail_gm(msgs),
        vec![MailGmCellToBase::Send {
            actor: MailGmActor {
                entity_id: gm,
                player_id: 70,
                account_id: Some(7),
            },
            to: Some("Bob".into()),
            cash: 10,
            item: Some((5001, 2)),
            cod: None,
            subject: "Hi".into(),
        }]
    );
}

/// `.mailbox` with and without a name.
#[tokio::test]
async fn mailbox_forwards_the_name() {
    let (mut mgr, gm) = gm_world();
    let actor = MailGmActor {
        entity_id: gm,
        player_id: 70,
        account_id: Some(7),
    };
    assert_eq!(
        mail_gm(run(&mut mgr, gm, ".mailbox").await),
        vec![MailGmCellToBase::Mailbox { actor, name: None }]
    );
    assert_eq!(
        mail_gm(run(&mut mgr, gm, ".mailbox Bob").await),
        vec![MailGmCellToBase::Mailbox {
            actor,
            name: Some("Bob".into())
        }]
    );
}

/// Type 12: a malformed `.mail` goes no further than the cell: a WARN
/// `mail.gm_rejected` with the reason and the GM's ids, the usage line, and
/// no message to the base.
#[tokio::test]
async fn mail_refusal_logs_reason_and_sends_nothing_to_the_base() {
    let (mut mgr, gm) = gm_world();
    let capture = LogCapture::install();
    let msgs = run(&mut mgr, gm, ".mail cod 10").await;
    let row = capture
        .find_event(Level::WARN, "GM mail command refused", "cod_without_item")
        .expect("mail.gm_rejected reason=cod_without_item");
    assert!(row.has_field("event", "mail.gm_rejected"));
    assert!(row.has_field("account_id", "7") && row.has_field("player_id", "70"));
    assert!(
        !msgs.iter().any(|m| matches!(m, CellToBaseMsg::MailGm(_))),
        "{msgs:?}"
    );
    let lines: Vec<String> = msgs.iter().filter_map(super::decode_feedback).collect();
    assert_eq!(lines.len(), 1, "{lines:?}");
    assert!(
        lines[0].starts_with(".mail: `cod` needs an `item`"),
        "{lines:?}"
    );
}

/// Type 12: `.mail_expire` is refused until SS-M4 with a line naming the
/// mail and `reason=expiry_not_available`; a bare one gets its usage line.
#[tokio::test]
async fn mail_expire_is_refused_until_expiry_exists() {
    let (mut mgr, gm) = gm_world();
    let capture = LogCapture::install();
    let msgs = run(&mut mgr, gm, ".mail_expire 42").await;
    capture
        .find_event(
            Level::WARN,
            "GM mail command refused",
            "expiry_not_available",
        )
        .expect("mail.gm_rejected reason=expiry_not_available");
    let lines: Vec<String> = msgs.iter().filter_map(super::decode_feedback).collect();
    assert_eq!(lines.len(), 1, "{lines:?}");
    assert!(lines[0].contains("mail 42 was not changed"), "{lines:?}");
    assert!(!msgs.iter().any(|m| matches!(m, CellToBaseMsg::MailGm(_))));

    let msgs = run(&mut mgr, gm, ".mail_expire").await;
    let lines: Vec<String> = msgs.iter().filter_map(super::decode_feedback).collect();
    assert_eq!(lines, vec![MAIL_EXPIRE_USAGE.to_string()]);
}

/// A player (access level 0) typing any of the three gets the "GM command"
/// line and nothing reaches the base: no mail is minted.
#[tokio::test]
async fn non_gm_mail_commands_are_refused() {
    for line in [".mail cash 1000000 item 5001", ".mailbox", ".mail_expire 1"] {
        let (mut mgr, _npc) = world(0);
        let msgs = say(&mut mgr, line).await;
        assert!(
            !msgs.iter().any(|m| matches!(m, CellToBaseMsg::MailGm(_))),
            "{line}: {msgs:?}"
        );
        let lines: Vec<String> = msgs.iter().filter_map(super::decode_feedback).collect();
        assert_eq!(lines.len(), 1, "{line}: {lines:?}");
        assert!(lines[0].contains("is a GM command"), "{line}: {lines:?}");
    }
}
