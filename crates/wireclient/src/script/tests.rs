use std::time::{Duration, Instant};

use bytes::Bytes;

use super::*;
use crate::semantic::DialogDisplay;

const OWN: u32 = 8;
const NPC: u32 = 100_751;

fn msg(msg_id: u8, entity_id: Option<u32>, payload: Vec<u8>) -> S2CMessage {
    S2CMessage {
        msg_id,
        entity_id,
        class_id: None,
        method_index: None,
        sub_index: None,
        offset: 0,
        payload: Bytes::from(payload),
    }
}

/// An introduction (`0x05` own player, `0x09` anything else) as the bundle
/// decoder builds it.
fn intro(msg_id: u8, id: u32, class_id: u8) -> S2CMessage {
    let mut m = msg(msg_id, Some(id), id.to_le_bytes().to_vec());
    m.class_id = Some(class_id);
    m
}

fn tick_sync() -> S2CMessage {
    msg(MSG_TICK_SYNC, None, vec![0; 8])
}

/// `onDialogDisplay` (105, extended sub-slot 44) to `target`, `args` given.
fn dialog_msg(target: u32, args: &[u8]) -> S2CMessage {
    let mut payload = target.to_le_bytes().to_vec();
    payload.push(44);
    payload.extend_from_slice(args);
    let mut m = msg(0xBD, Some(target), payload);
    m.sub_index = Some(44);
    m.method_index = Some(105);
    m
}

fn dialog_args(dialog_id: i32) -> Vec<u8> {
    [
        &(NPC as i32).to_le_bytes()[..],
        &dialog_id.to_le_bytes(),
        &0i32.to_le_bytes(),
        &[1],
        &0i32.to_le_bytes(),
    ]
    .concat()
}

/// A log entry `ms` milliseconds in, for the pure scan and trail tests.
fn observed(ms: u64, m: S2CMessage) -> Observed {
    Observed {
        at: Duration::from_millis(ms),
        msg: m,
        event: None,
    }
}

fn is_msg(id: u8) -> impl FnMut(&Observed) -> bool {
    move |o| o.msg.msg_id == id
}

#[test]
fn scan_does_not_return_a_match_before_the_cursor() {
    let log = vec![
        observed(0, intro(0x09, 1, 1)),
        observed(1, tick_sync()),
        observed(2, intro(0x09, 2, 1)),
    ];
    assert_eq!(scan(&log, 0, &mut is_msg(0x09)), Some(0));
    // From 1, the create at 0 is behind the cursor: the next one is found.
    assert_eq!(scan(&log, 1, &mut is_msg(0x09)), Some(2));
    assert_eq!(scan(&log, 3, &mut is_msg(0x09)), None);
    // A cursor past the end (a log that has not grown) is no match, not a panic.
    assert_eq!(scan(&log, 9, &mut is_msg(0x09)), None);
}

/// The scan `expect_in_order` runs: each step from just past the previous
/// match, gaps allowed, order enforced.
#[test]
fn scan_finds_ordered_steps_with_gaps() {
    let log = vec![
        observed(0, intro(0x09, 1, 1)), // a
        observed(1, tick_sync()),       // gap
        observed(2, intro(0x0C, 1, 1)), // b
        observed(3, tick_sync()),       // gap
        observed(4, intro(0x09, 2, 1)), // c
    ];
    let mut cursor = 0;
    let mut hits = Vec::new();
    for id in [0x09, 0x0C, 0x09] {
        let i = scan(&log, cursor, &mut is_msg(id)).expect("step matches");
        hits.push(i);
        cursor = i + 1;
    }
    assert_eq!(hits, [0, 2, 4]);

    // The reverse order cannot match: the 0x0C is behind the second 0x09.
    let second_create = scan(&log, 1, &mut is_msg(0x09)).expect("second create");
    assert_eq!(scan(&log, second_create + 1, &mut is_msg(0x0C)), None);
}

#[test]
fn trail_lists_the_last_n_entries_in_order() {
    let log: Vec<Observed> = (0..5u32)
        .map(|i| observed(u64::from(i) * 1000, intro(0x09, 100 + i, 1)))
        .collect();
    let trail = format_trail(&log, 3, Some(OWN));
    let lines: Vec<&str> = trail.lines().collect();
    assert_eq!(lines.len(), 3, "{trail}");
    assert!(
        lines[0].starts_with("+2.000s msg 0x09 entity 102"),
        "{trail}"
    );
    assert!(
        lines[2].starts_with("+4.000s msg 0x09 entity 104"),
        "{trail}"
    );
    // Asking for more than there is lists everything; an empty log says so.
    assert_eq!(format_trail(&log, 30, Some(OWN)).lines().count(), 5);
    assert_eq!(format_trail(&[], 30, Some(OWN)), "(nothing observed)");
}

/// The line shape the packet names: time, msg id, entity, method, event.
#[test]
fn trail_line_names_the_method_and_the_event() {
    let mut o = observed(12_345, dialog_msg(OWN, &dialog_args(2982)));
    o.event = Some(S2CEvent::DialogDisplay(DialogDisplay {
        entity_id: NPC as i32,
        dialog_id: 2982,
        mission_flags: 0,
        is_immediate: 1,
        mission_id: 0,
    }));
    let line = trail_line(&o, Some(OWN));
    assert!(
        line.starts_with("+12.345s msg 0xbd entity 8 m105 DialogDisplay(DialogDisplay {"),
        "{line}"
    );
    assert!(line.contains("dialog_id: 2982"), "{line}");

    // On another target the idbase is unknown: the raw sub-slot, no guess.
    let npc = observed(0, dialog_msg(NPC, &[]));
    assert_eq!(
        trail_line(&npc, Some(OWN)),
        "+0.000s msg 0xbd entity 100751 sub44 len 5"
    );

    // A direct method carries its index in the id byte; no event shows the length.
    let direct = observed(1, msg(0x83, Some(NPC), NPC.to_le_bytes().to_vec()));
    assert_eq!(
        trail_line(&direct, Some(OWN)),
        "+0.001s msg 0x83 entity 100751 m3 len 4"
    );
}

#[test]
fn trail_line_cuts_a_long_event() {
    let mut o = observed(0, msg(0xC6, Some(OWN), vec![0; 4]));
    o.event = Some(S2CEvent::RemoveItem {
        item_ids: (0..500).collect(),
    });
    let line = trail_line(&o, Some(OWN));
    assert!(line.ends_with("..."), "{line}");
    assert!(line.len() < 250, "{} chars", line.len());
}

#[test]
fn keepalive_is_due_after_250ms() {
    let last = Instant::now();
    assert!(!keepalive_due(last, last));
    assert!(!keepalive_due(last, last + Duration::from_millis(249)));
    assert!(keepalive_due(last, last + Duration::from_millis(250)));
    assert!(keepalive_due(last, last + Duration::from_secs(3)));
    // A `now` before `last` (a send between reading the clock and checking)
    // is not due, and does not panic.
    assert!(!keepalive_due(last + Duration::from_millis(10), last));
}

/// A tickSync-only bundle is skipped; one with anything else in it is logged
/// whole, tickSync included, so a bundle is never partly dropped.
#[test]
fn a_bundle_mixing_tick_sync_is_logged_whole() {
    let mut mirror = EntityMirror::new();
    let mut log = Vec::new();
    let mut queue = Vec::new();
    let at = Duration::from_millis(5);

    let n = ingest_messages(&mut mirror, vec![tick_sync()], at, &mut log, &mut queue);
    assert_eq!(n, Ok(0));
    assert!(log.is_empty());
    assert_eq!(
        ingest_messages(&mut mirror, vec![], at, &mut log, &mut queue),
        Ok(0)
    );

    let mixed = vec![tick_sync(), intro(0x09, NPC, 1), tick_sync()];
    let n = ingest_messages(&mut mirror, mixed, at, &mut log, &mut queue);
    assert_eq!(n, Ok(3));
    let ids: Vec<u8> = log.iter().map(|o| o.msg.msg_id).collect();
    assert_eq!(ids, [MSG_TICK_SYNC, 0x09, MSG_TICK_SYNC]);
    assert!(log.iter().all(|o| o.at == at));
    assert!(mirror.get(NPC).is_some(), "the create was applied");
}

/// Every message is logged with its decoded event, in bundle order.
#[test]
fn ingest_logs_each_message_with_its_event() {
    let mut mirror = EntityMirror::new();
    let mut log = Vec::new();
    let mut queue = Vec::new();
    let bundle = vec![
        intro(0x05, OWN, 2),
        intro(0x09, NPC, 1),
        dialog_msg(OWN, &dialog_args(2982)),
    ];
    let n = ingest_messages(&mut mirror, bundle, Duration::ZERO, &mut log, &mut queue);
    assert_eq!(n, Ok(3));
    assert_eq!(mirror.own_id(), Some(OWN));
    assert!(log[0].event.is_none() && log[1].event.is_none());
    assert!(
        matches!(&log[2].event, Some(S2CEvent::DialogDisplay(d)) if d.dialog_id == 2982),
        "{:?}",
        log[2].event
    );
}

/// A message the mirror rejects is an error, but neither it nor what follows
/// it in the bundle is dropped from the log; what follows is not applied.
#[test]
fn a_rejected_message_and_the_rest_of_its_bundle_are_still_logged() {
    let mut mirror = EntityMirror::new();
    let mut log = Vec::new();
    let mut queue = Vec::new();
    let mut short = dialog_args(2982);
    short.pop();
    let bundle = vec![
        intro(0x05, OWN, 2),
        dialog_msg(OWN, &short),
        intro(0x09, NPC, 1),
    ];
    let result = ingest_messages(&mut mirror, bundle, Duration::ZERO, &mut log, &mut queue);
    assert!(
        matches!(
            result,
            Err(MirrorError::Semantic {
                method_index: 105,
                ..
            })
        ),
        "{result:?}"
    );
    assert_eq!(log.len(), 3, "all three logged");
    assert!(
        mirror.get(NPC).is_none(),
        "nothing applied past the bad message"
    );
    assert!(
        queue.is_empty(),
        "no requestEntityUpdate for an unapplied create"
    );
}

/// One `requestEntityUpdate` per introduced entity, never for the own player
/// or a client-only id; queued in arrival order, bytes as the client sends.
#[test]
fn creates_queue_one_request_entity_update_each() {
    let mut mirror = EntityMirror::new();
    let mut log = Vec::new();
    let mut queue = Vec::new();
    let bundle = vec![
        intro(0x05, OWN, 2),
        intro(0x09, OWN, 2), // a re-introduction of the own player
        intro(0x09, NPC, 1),
        intro(0x09, NPC + 1, 1),
        intro(0x09, CLIENT_ONLY_ID_FLOOR, 1),
        msg(0x0C, Some(NPC), NPC.to_le_bytes().to_vec()),
    ];
    ingest_messages(&mut mirror, bundle, Duration::ZERO, &mut log, &mut queue)
        .expect("bundle applies");
    let expected = [
        GameSession::request_entity_update(NPC),
        GameSession::request_entity_update(NPC + 1),
    ]
    .concat();
    assert_eq!(queue, expected);
    assert_eq!(&queue[..7], [0x07, 4, 0, 0x8F, 0x89, 0x01, 0x00]);
}

#[test]
fn entity_update_target_is_only_for_other_networked_creates() {
    assert_eq!(
        entity_update_target(&intro(0x09, NPC, 1), Some(OWN)),
        Some(NPC)
    );
    // Before the own id is known every networked create counts.
    assert_eq!(entity_update_target(&intro(0x09, OWN, 2), None), Some(OWN));
    assert_eq!(entity_update_target(&intro(0x09, OWN, 2), Some(OWN)), None);
    assert_eq!(entity_update_target(&intro(0x05, NPC, 2), Some(OWN)), None);
    assert_eq!(
        entity_update_target(&intro(0x09, CLIENT_ONLY_ID_FLOOR - 1, 1), Some(OWN)),
        Some(CLIENT_ONLY_ID_FLOOR - 1)
    );
    assert_eq!(
        entity_update_target(&intro(0x09, CLIENT_ONLY_ID_FLOOR, 1), Some(OWN)),
        None
    );
}
