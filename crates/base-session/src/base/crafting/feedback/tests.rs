//! `reject`: the visible line, the optional code, the `rejected` event with
//! its identity and compared values, the counters, and the send-failure
//! WARN.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use cimmeria_mercury::encryption::EncryptionVersion;
use cimmeria_observability::testing::{counter_total, install as install_meter};

use super::*;
use crate::base::crafting::telemetry::{METRIC_REJECTIONS, METRIC_REQUESTS};
use crate::base::crafting::test_players::{OneSession, SESSION_ACCOUNT_ID};
use crate::cell::messages::CraftVerb;
use crate::test_support::{Captured, LogCapture};

const ENTITY: u32 = 4260;
const PLAYER_ID: i32 = 4261;

fn packet(seq: u32, method: u16, args: &[u8]) -> Vec<u8> {
    build_player_entity_method_packet(
        &[0u8; 32],
        seq,
        &[],
        ENTITY,
        method,
        args,
        EncryptionVersion::V1,
    )
}

fn crafting_event(all: Vec<Captured>, event: &str) -> Captured {
    all.into_iter()
        .find(|c| c.target == "crafting" && c.has_field("event", event))
        .unwrap_or_else(|| panic!("no crafting event={event}"))
}

/// `reject` sends exactly one packet to the player's own client: the
/// `CHAN_FEEDBACK` line carrying the rejection text, byte for byte.
/// Removing the send (a silent rejection) fails the packet count; a line on
/// any other channel fails the byte comparison.
#[tokio::test]
async fn reject_sends_the_text_line_to_the_players_client() {
    let session = OneSession::new(ENTITY, 55720);
    let why = CraftReject::not_available(&CraftVerb::Respec);

    reject("respecCrafting", ENTITY, PLAYER_ID, &why, session.client()).await;

    assert_eq!(session.typed.len(), 1, "nothing to any other address");
    assert_eq!(
        session.typed.filter_to(session.addr),
        vec![packet(
            0,
            method_idx::ON_PLAYER_COMMUNICATION,
            &feedback_text_args("Crafting respec is not available yet."),
        )],
        "exactly the text line, no onErrorCode"
    );
}

/// The `rejected` event carries the verb, the reason, the full identity
/// (`account_id`, `player_id`, `entity_id`) on the event itself, and the
/// values the rule compared. Dropping any identity field fails it.
#[tokio::test]
async fn rejected_event_carries_identity_and_compared_values() {
    let capture = LogCapture::install();
    let session = OneSession::new(ENTITY, 55721);
    let why = CraftReject::ParadigmTooLow {
        discipline_id: 82,
        discipline: "Ceramic Composites".into(),
        paradigm_id: 2,
        paradigm: "Human",
        required: 3,
        have: 1,
    };

    reject(
        "spendAppliedSciencePoints",
        ENTITY,
        PLAYER_ID,
        &why,
        session.client(),
    )
    .await;

    let event = crafting_event(capture.all(), "rejected");
    assert_eq!(event.level, tracing::Level::INFO);
    for (field, value) in [
        ("verb", "spendAppliedSciencePoints"),
        ("reason", "paradigm_too_low"),
        ("account_id", &SESSION_ACCOUNT_ID.to_string()),
        ("player_id", &PLAYER_ID.to_string()),
        ("entity_id", &ENTITY.to_string()),
        ("discipline_id", "82"),
        ("paradigm_id", "2"),
        ("paradigm_level", "1"),
        ("required_level", "3"),
    ] {
        assert!(event.has_field(field, value), "{field}={value}: {event:#?}");
    }
    assert!(
        !event.fields.contains_key("prerequisite_id"),
        "fields the rule did not compare are omitted: {event:#?}"
    );
}

/// A reason that maps a condition code (no ASP -> 214) sends the text line
/// first, then `onErrorCode(0, 0, 214)`: two packets, in that order, both
/// to the player. Unmapping the code, or sending the code instead of the
/// text, fails the comparison.
#[tokio::test]
async fn coded_reject_sends_the_text_then_on_error_code() {
    let session = OneSession::new(ENTITY, 55722);

    reject(
        "spendAppliedSciencePoints",
        ENTITY,
        PLAYER_ID,
        &CraftReject::NoAppliedSciencePoints { asp: 0 },
        session.client(),
    )
    .await;

    assert_eq!(
        session.typed.filter_to(session.addr),
        vec![
            packet(
                0,
                method_idx::ON_PLAYER_COMMUNICATION,
                &feedback_text_args("You have no applied science points."),
            ),
            packet(1, method_idx::ON_ERROR_CODE, &[0, 0, 0, 0, 0, 0xD6, 0x00]),
        ]
    );
}

/// Counter emission: every refusal, including a server failure, adds one
/// to `crafting_rejections_total{verb, reason}` and one to
/// `crafting_requests_total{verb, outcome = rejected}`. Removing either
/// `record_*` call fails it.
#[tokio::test]
async fn reject_counts_the_rejection_and_the_request_outcome() {
    install_meter();
    let session = OneSession::new(ENTITY, 55723);
    // A verb label no other test uses, so parallel tests cannot move it.
    let verb = "research";
    let not_available = [("verb", verb), ("reason", "not_available_yet")];
    let unavailable = [("verb", verb), ("reason", "unavailable")];
    let rejected = [("verb", verb), ("outcome", "rejected")];
    let before = (
        counter_total(METRIC_REJECTIONS, &not_available),
        counter_total(METRIC_REJECTIONS, &unavailable),
        counter_total(METRIC_REQUESTS, &rejected),
    );

    let why = CraftReject::not_available(&CraftVerb::Research {
        item_id: 1,
        kickers: vec![],
    });
    reject(verb, ENTITY, PLAYER_ID, &why, session.client()).await;
    reject(
        verb,
        ENTITY,
        PLAYER_ID,
        &CraftReject::Unavailable { action: "Research" },
        session.client(),
    )
    .await;

    assert_eq!(
        counter_total(METRIC_REJECTIONS, &not_available) - before.0,
        1
    );
    assert_eq!(counter_total(METRIC_REJECTIONS, &unavailable) - before.1, 1);
    assert_eq!(counter_total(METRIC_REQUESTS, &rejected) - before.2, 2);
    assert_eq!(
        counter_total(METRIC_REQUESTS, &[("verb", verb), ("outcome", "accepted")]),
        0,
        "a refusal is never counted as accepted"
    );
}

/// A refusal line that cannot be sent (no address for the entity) is a
/// WARN naming the reason, not a silent drop.
#[tokio::test]
async fn unsent_refusal_line_is_a_warn() {
    let capture = LogCapture::install();
    let session = OneSession::new(ENTITY, 55724);
    let empty = Arc::new(Mutex::new(HashMap::new()));
    let client = CraftClient {
        entity_to_addr: &empty,
        ..session.client()
    };

    reject(
        "craft",
        ENTITY,
        PLAYER_ID,
        &CraftReject::Unavailable { action: "Crafting" },
        client,
    )
    .await;

    let warn = capture
        .find_event(
            tracing::Level::WARN,
            "crafting refusal line not sent",
            "entity_to_addr_miss",
        )
        .expect("send-failure WARN");
    assert_eq!(warn.target, "crafting");
    assert!(warn.has_field("entity_id", &ENTITY.to_string()));
    assert!(warn.has_field("player_id", &PLAYER_ID.to_string()));
}

/// The line is `SYSTEM`, flags 0, on `CHAN_FEEDBACK` (9), then the text.
#[test]
fn feedback_text_rides_chan_feedback() {
    let args = feedback_text_args("hi");
    let speaker_end = 4 + "SYSTEM".len() * 2;
    assert_eq!(u32::from_le_bytes(args[0..4].try_into().unwrap()), 6);
    assert_eq!(args[speaker_end], 0, "speaker flags");
    assert_eq!(args[speaker_end + 1], CHAN_FEEDBACK, "channel");
    assert_eq!(
        CHAN_FEEDBACK, 9,
        "CHAN_feedback rides the registered tell channel"
    );
    assert_eq!(&args[speaker_end + 2..speaker_end + 6], &2u32.to_le_bytes());
    assert_eq!(&args[speaker_end + 6..], &[b'h', 0, b'i', 0]);
}

/// `onErrorCode`: system 0, instance 0, then the code as UINT16.
#[test]
fn error_code_args_are_system_instance_code() {
    assert_eq!(error_code_args(214), [0, 0, 0, 0, 0, 0xD6, 0x00]);
}
