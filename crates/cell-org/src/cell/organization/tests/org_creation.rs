//! ORG-05 on the cell: the registrar's eligible answer opens the dialog,
//! cell method 94 is honoured only against the pending creation, and the
//! base's result settles it. Rejections are negative-log tests (TESTING.md
//! type 12): the row, its `reason`, what the client got, and that nothing
//! reached the base.

use cimmeria_cell_world::cell::org_creation::OrgCreationResources;
use std::time::{Duration, Instant};

use cimmeria_entity::organization::OrgType;
use cimmeria_wire::cell::client_methods::player::build_launch_organization_creation;

use super::*;
use crate::cell::messages::OrgCellToBase;
use crate::cell::org_creation::{PENDING_CREATION_ATTEMPTS, PENDING_CREATION_TTL};
use crate::cell::organization::creation::{
    self, IN_FLIGHT_TEXT, NAME_INVALID_TEXT, NO_PENDING_TEXT, PENDING_EXPIRED_TEXT,
    RATE_LIMITED_TEXT,
};
use crate::test_support::LogCapture;

const ALICE: u32 = 11;
const ALICE_PID: i32 = 1;
const NPC: u32 = 900;

/// Everything queued for the base: client calls and org messages apart.
fn split(rx: &mut mpsc::Receiver<CellToBaseMsg>) -> (Vec<Sent>, Vec<OrgCellToBase>) {
    let mut calls = Vec::new();
    let mut org = Vec::new();
    while let Ok(msg) = rx.try_recv() {
        match msg {
            CellToBaseMsg::EntityMethodCall {
                entity_id,
                method_index,
                args,
            } => calls.push((entity_id, method_index, args)),
            CellToBaseMsg::Org(o) => org.push(o),
            other => panic!("unexpected {other:?}"),
        }
    }
    (calls, org)
}

/// CM 94's wire form: a `WSTRING`.
fn cm94(name: &str) -> Vec<u8> {
    let units: Vec<u16> = name.encode_utf16().collect();
    let mut buf = (units.len() as u32).to_le_bytes().to_vec();
    for u in units {
        buf.extend_from_slice(&u.to_le_bytes());
    }
    buf
}

/// The refusal pair: 134 `(0, code)` then the line.
fn refusal(code: u8, text: &str) -> Vec<(u16, Vec<u8>)> {
    vec![(134, vec![0, code]), (28, line(text))]
}

fn creation_rows(capture: &LogCaptureGuard, event: &str) -> Vec<crate::test_support::Captured> {
    capture
        .all()
        .into_iter()
        .filter(|c| {
            c.target == "org"
                && c.level == Level::INFO
                && c.has_field("event", event)
                && c.fields.contains_key("outcome")
        })
        .collect()
}

/// Alice has a Command offer open, as the base's eligible answer left it.
async fn offered(mgr: &mut SpaceManager, org_type: OrgType) {
    let (tx, mut rx) = channel();
    creation::on_registrar_eligible(ALICE_PID, ALICE, NPC, org_type, &tx, mgr).await;
    let (calls, org) = split(&mut rx);
    assert_eq!(
        to(&calls, ALICE),
        vec![(135, build_launch_organization_creation(org_type))]
    );
    assert!(org.is_empty());
}

/// The eligible answer records the offer and opens the dialog with the
/// type the registrar named; one `org.registrar_open` row says `ok`.
#[tokio::test]
async fn registrar_eligible_opens_the_dialog_and_records_the_offer() {
    let capture = LogCapture::install();
    let mut mgr = world(&["Alice"]);
    offered(&mut mgr, OrgType::Team).await;

    let p = mgr
        .resources
        .org_creations()
        .get(ALICE_PID)
        .expect("the offer");
    assert_eq!((p.org_type, p.npc_entity_id), (OrgType::Team, NPC));
    assert_eq!(p.attempts_left, PENDING_CREATION_ATTEMPTS);
    let rows = creation_rows(&capture, "org.registrar_open");
    assert_eq!(rows.len(), 1, "{rows:#?}");
    assert!(rows[0].has_field("outcome", "ok"));
    assert!(rows[0].has_field("player_id", "1"));
    assert!(rows[0].has_field("account_id", &account_of(1).to_string()));
    assert!(rows[0].has_field("npc_entity_id", &NPC.to_string()));
    assert!(capture
        .all()
        .iter()
        .any(|c| c.level == Level::DEBUG && c.has_field("event", "pending_creation_created")));
}

/// CAT-M-03: cell method 94 with no offer founds nothing. The client gets
/// 134 `(0, NO_PENDING_CREATION)` and a line, the row says
/// `no_pending_creation`, and nothing reaches the base.
#[tokio::test]
async fn create_rejects_without_pending_creation() {
    let capture = LogCapture::install();
    let mut mgr = world(&["Alice"]);
    let (tx, mut rx) = channel();

    creation::on_organization_creation(ALICE, &cm94("SG-1"), &tx, &mut mgr).await;

    let (calls, org) = split(&mut rx);
    assert!(org.is_empty(), "nothing may reach the base: {org:?}");
    assert_eq!(to(&calls, ALICE), refusal(5, NO_PENDING_TEXT));
    let rows = creation_rows(&capture, "org.create");
    assert_eq!(rows.len(), 1, "{rows:#?}");
    assert!(rows[0].has_field("reason", "no_pending_creation"));
    assert!(rows[0].has_field("name_units", "4"));
}

/// With an offer open, the name goes to the base normalised, with the
/// offer's type (the wire carries none), and the offer is in flight: a
/// second name before the answer is refused.
#[tokio::test]
async fn create_forwards_the_offer_type_and_blocks_a_second_name() {
    let capture = LogCapture::install();
    let mut mgr = world(&["Alice"]);
    offered(&mut mgr, OrgType::Command).await;
    let (tx, mut rx) = channel();

    creation::on_organization_creation(ALICE, &cm94("  Tok'ra   Rising "), &tx, &mut mgr).await;
    creation::on_organization_creation(ALICE, &cm94("Another"), &tx, &mut mgr).await;

    let (calls, org) = split(&mut rx);
    assert_eq!(
        org,
        vec![OrgCellToBase::Create {
            player_id: ALICE_PID,
            entity_id: ALICE,
            org_type: OrgType::Command,
            name: "Tok'ra Rising".into(),
        }]
    );
    assert_eq!(to(&calls, ALICE), refusal(6, IN_FLIGHT_TEXT));
    let rows = creation_rows(&capture, "org.create");
    assert_eq!(rows.len(), 1, "the forwarded name's row is the base's");
    assert!(rows[0].has_field("reason", "creation_in_flight"));
}

/// CAT-M-03: a name that breaks D-ORG10 (empty, 61 units, a bidi control)
/// is refused on the cell, costs an attempt, and never reaches the base;
/// the third refusal spends the window, and a fourth name is rate limited
/// even though it is valid.
#[tokio::test]
async fn create_rejects_invalid_names() {
    let capture = LogCapture::install();
    let mut mgr = world(&["Alice"]);
    offered(&mut mgr, OrgType::Team).await;
    let (tx, mut rx) = channel();

    let too_long = "A".repeat(61);
    for name in ["", too_long.as_str(), "Bad\u{202E}Name"] {
        creation::on_organization_creation(ALICE, &cm94(name), &tx, &mut mgr).await;
    }
    creation::on_organization_creation(ALICE, &cm94("Valid Name"), &tx, &mut mgr).await;

    let (calls, org) = split(&mut rx);
    assert!(
        org.is_empty(),
        "no invalid name may reach the base: {org:?}"
    );
    let mut want = Vec::new();
    for _ in 0..3 {
        want.extend(refusal(2, NAME_INVALID_TEXT));
    }
    want.extend(refusal(6, RATE_LIMITED_TEXT));
    assert_eq!(to(&calls, ALICE), want);
    assert_eq!(
        mgr.resources
            .org_creations()
            .get(ALICE_PID)
            .unwrap()
            .attempts_left,
        0
    );
    let rows = creation_rows(&capture, "org.create");
    let reasons: Vec<_> = rows
        .iter()
        .map(|r| r.fields.get("reason").cloned().unwrap_or_default())
        .collect();
    assert_eq!(
        reasons,
        vec![
            "text_invalid",
            "text_invalid",
            "text_invalid",
            "rate_limited"
        ]
    );
    assert!(rows[0].has_field("text_reason", "too_short"));
    assert!(rows[1].has_field("text_reason", "too_long"));
    assert!(rows[2].has_field("attempts_left", "0"));
}

/// The base's answer settles the offer: a refusal charges it (and frees it
/// for the next name), a creation closes it.
#[tokio::test]
async fn create_result_charges_or_closes_the_offer() {
    let capture = LogCapture::install();
    let mut mgr = world(&["Alice"]);
    offered(&mut mgr, OrgType::Team).await;
    let (tx, mut rx) = channel();

    creation::on_organization_creation(ALICE, &cm94("Taken"), &tx, &mut mgr).await;
    creation::on_create_result(ALICE_PID, ALICE, false, &mut mgr);
    let p = *mgr.resources.org_creations().get(ALICE_PID).unwrap();
    assert_eq!(
        (p.attempts_left, p.in_flight),
        (PENDING_CREATION_ATTEMPTS - 1, false)
    );

    creation::on_organization_creation(ALICE, &cm94("Free"), &tx, &mut mgr).await;
    creation::on_create_result(ALICE_PID, ALICE, true, &mut mgr);
    assert!(mgr.resources.org_creations().get(ALICE_PID).is_none());
    let (_, org) = split(&mut rx);
    assert_eq!(org.len(), 2, "both names reached the base");
    assert!(capture
        .all()
        .iter()
        .any(|c| c.level == Level::DEBUG && c.has_field("event", "pending_creation_consumed")));

    // With the offer closed, another name is refused.
    creation::on_organization_creation(ALICE, &cm94("Again"), &tx, &mut mgr).await;
    let (calls, org) = split(&mut rx);
    assert!(org.is_empty());
    assert_eq!(to(&calls, ALICE), refusal(5, NO_PENDING_TEXT));
}

/// A change of space (gate travel) or the 5-minute expiry ends the offer:
/// the name is refused as `pending_expired`, with a transition row naming
/// the cause.
#[tokio::test]
async fn an_offer_from_another_space_or_too_old_is_expired() {
    let capture = LogCapture::install();
    let mut mgr = world(&["Alice"]);
    offered(&mut mgr, OrgType::Team).await;
    let (tx, mut rx) = channel();

    // Move the offer's space off Alice's, as if she had gated since.
    let here = mgr.get_entity_space_id(ALICE).unwrap();
    mgr.resources.org_creations_mut().clear(ALICE_PID);
    mgr.resources
        .org_creations_mut()
        .open(ALICE_PID, OrgType::Team, NPC, here + 1, Instant::now())
        .unwrap();
    creation::on_organization_creation(ALICE, &cm94("Moved"), &tx, &mut mgr).await;

    let long_ago = Instant::now()
        .checked_sub(PENDING_CREATION_TTL + Duration::from_secs(1))
        .expect("uptime exceeds the offer window");
    mgr.resources
        .org_creations_mut()
        .open(ALICE_PID, OrgType::Team, NPC, here, long_ago)
        .unwrap();
    creation::on_organization_creation(ALICE, &cm94("Late"), &tx, &mut mgr).await;

    let (calls, org) = split(&mut rx);
    assert!(org.is_empty());
    let mut want = refusal(5, PENDING_EXPIRED_TEXT);
    want.extend(refusal(5, PENDING_EXPIRED_TEXT));
    assert_eq!(to(&calls, ALICE), want);
    let causes: Vec<_> = capture
        .all()
        .into_iter()
        .filter(|c| c.has_field("event", "pending_creation_expired"))
        .map(|c| c.fields.get("cause").cloned().unwrap_or_default())
        .collect();
    assert_eq!(causes, vec!["space_changed", "ttl"]);
    assert_eq!(creation_rows(&capture, "org.create").len(), 2);
}

/// The eligible answer names both ids from the base; an entity that is no
/// longer that character opens nothing (WARN `org.actor_mismatch`).
#[tokio::test]
async fn eligible_for_a_recycled_entity_opens_nothing() {
    let capture = LogCapture::install();
    let mut mgr = world(&["Alice"]);
    let (tx, mut rx) = channel();

    creation::on_registrar_eligible(ALICE_PID + 50, ALICE, NPC, OrgType::Team, &tx, &mut mgr).await;

    let (calls, org) = split(&mut rx);
    assert!(calls.is_empty() && org.is_empty());
    assert!(mgr.resources.org_creations().is_empty());
    assert!(capture
        .all()
        .iter()
        .any(|c| c.level == Level::WARN && c.has_field("event", "org.actor_mismatch")));
    let rows = creation_rows(&capture, "org.registrar_open");
    assert_eq!(rows.len(), 1);
    assert!(rows[0].has_field("reason", "actor_mismatch"));
}

/// Spending the window, then clicking the registrar again, does not open a
/// fresh dialog: the player is told to wait and the row says
/// `rate_limited`.
#[tokio::test]
async fn reopening_an_exhausted_offer_is_rate_limited() {
    let capture = LogCapture::install();
    let mut mgr = world(&["Alice"]);
    offered(&mut mgr, OrgType::Team).await;
    let (tx, mut rx) = channel();
    for _ in 0..PENDING_CREATION_ATTEMPTS {
        creation::on_organization_creation(ALICE, &cm94(""), &tx, &mut mgr).await;
    }
    split(&mut rx);

    creation::on_registrar_eligible(ALICE_PID, ALICE, NPC, OrgType::Team, &tx, &mut mgr).await;

    let (calls, _) = split(&mut rx);
    assert_eq!(to(&calls, ALICE), vec![(28, line(RATE_LIMITED_TEXT))]);
    let rows = creation_rows(&capture, "org.registrar_open");
    assert!(rows.last().unwrap().has_field("reason", "rate_limited"));
}

/// Logging out drops the offer.
#[tokio::test]
async fn disconnect_drops_the_offer() {
    let mut mgr = world(&["Alice"]);
    offered(&mut mgr, OrgType::Team).await;
    creation::on_disconnect(ALICE, &mut mgr);
    assert!(mgr.resources.org_creations().is_empty());
}

/// Negative seam: a name that cannot reach the base (the channel is
/// closed) is WARN `org.create_forward_failed` (`cell_to_base_closed`), is
/// charged as an attempt so the offer is not stuck in flight, and ends in
/// one `base_unreachable` row.
#[tokio::test]
async fn a_closed_base_channel_warns_and_charges_the_attempt() {
    let capture = LogCapture::install();
    let mut mgr = world(&["Alice"]);
    offered(&mut mgr, OrgType::Team).await;
    let (tx, rx) = channel();
    drop(rx);

    creation::on_organization_creation(ALICE, &cm94("Lost"), &tx, &mut mgr).await;

    let p = *mgr.resources.org_creations().get(ALICE_PID).unwrap();
    assert_eq!(
        (p.attempts_left, p.in_flight),
        (PENDING_CREATION_ATTEMPTS - 1, false)
    );
    let warn = capture
        .all()
        .into_iter()
        .find(|c| c.level == Level::WARN && c.has_field("event", "org.create_forward_failed"))
        .expect("the dropped forward must WARN");
    assert!(warn.has_field("reason", "cell_to_base_closed"));
    assert!(warn.has_field("player_id", "1"));
    let rows = creation_rows(&capture, "org.create");
    assert_eq!(rows.len(), 1);
    assert!(rows[0].has_field("reason", "base_unreachable"));
}
