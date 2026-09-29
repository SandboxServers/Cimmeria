use super::layout::{self, request};
use super::*;
use crate::hooks::entity_trace::map::fake::{build_map, FakeMem};
use crate::hooks::entity_trace::map::manager;

const SEQ_MGR: u32 = 0x0200_0000;
const ENT_MGR: u32 = 0x0300_0000;
const EVT: u32 = 0x0400_0000;
const DATA: u32 = 0x0410_0000;
const NOW: u32 = 1_000_000;

/// A request block at `ptr`.
fn request(m: &mut FakeMem, ptr: u32, seq: i32, source: i32, view: i32, created: u32) {
    m.set(ptr + request::SEQUENCE_ID, seq as u32);
    m.set(ptr + request::SOURCE_ID, source as u32);
    m.set(ptr + request::TARGET_ID, 7);
    m.set(ptr + request::CREATED, created);
    m.set(ptr + request::VIEW_TYPE, view as u32);
    m.set(ptr + request::INSTANCE_ID, 0x55);
}

/// The entity manager singleton with a world map of `(id, pawn)` entities;
/// entity blocks are at `0x0500_0000 + id * 0x100`.
fn entities(m: &mut FakeMem, list: &[(i32, u32)]) {
    m.set(layout::ENTITY_MANAGER_SINGLETON, ENT_MGR);
    let entries: Vec<(i32, u32)> = list
        .iter()
        .map(|&(id, pawn)| {
            let ent = 0x0500_0000 + id as u32 * 0x100;
            m.set(ent + layout::ENTITY_PAWN, pawn);
            (id, ent)
        })
        .collect();
    build_map(m, ENT_MGR + manager::WORLD_MAP, 0x0600_0000, &entries);
}

/// The ready event for `key`, with cooked data of Kismet event `event_id`
/// (or none).
fn ready(m: &mut FakeMem, status: u32, key: i32, event_id: Option<u32>) {
    m.set(EVT + layout::ready::STATUS, status);
    m.set(EVT + layout::ready::KEY, key as u32);
    match event_id {
        Some(e) => {
            m.set(EVT + layout::ready::DATA, DATA);
            m.set(DATA + layout::DATA_EVENT_ID, e);
        }
        None => m.set(EVT + layout::ready::DATA, 0),
    }
}

/// File `reqs` (`(key, request ptr)`, sorted) in the SequenceManager.
fn file(m: &mut FakeMem, reqs: &[(i32, u32)]) {
    build_map(m, SEQ_MGR + layout::REQUESTS, 0x0700_0000, reqs);
}

fn paths(d: &[SequenceDrop]) -> Vec<(&'static str, Option<i32>)> {
    d.iter()
        .map(|d| (d.path.as_str(), d.ids.source_id))
        .collect()
}

/// The case from the colo: a request whose Source has a pawn plays; one
/// whose Source is unknown, or known without a pawn, is dropped, each
/// with its own path. Requests under other ids are not touched.
#[test]
fn the_ready_handler_drops_what_the_client_drops() {
    let mut m = FakeMem::default();
    entities(&mut m, &[(10, 0xAAAA), (11, 0)]);
    request(&mut m, 0x0800_0000, 3, 10, 0, NOW);
    request(&mut m, 0x0800_0100, 3, 11, 0, NOW);
    request(&mut m, 0x0800_0200, 3, 12, 0, NOW);
    request(&mut m, 0x0800_0300, 15, 12, 0, NOW);
    file(
        &mut m,
        &[
            (3, 0x0800_0000),
            (3, 0x0800_0100),
            (3, 0x0800_0200),
            (15, 0x0800_0300),
        ],
    );
    ready(&mut m, 1, 3, Some(42));

    let d = plan_ready(&m, SEQ_MGR, EVT, NOW);
    assert_eq!(
        paths(&d),
        vec![("no_source_pawn", Some(11)), ("no_source_entity", Some(12))]
    );
    assert_eq!(d[0].ids.sequence_id, Some(3));
    assert_eq!(d[0].event_id, Some(42));
}

/// Event 5001 and `ViewType` 1/2 wait for the appearance job instead of
/// dropping, but only when the Source exists at all.
#[test]
fn deferrable_sequences_wait_for_a_pawn_but_not_for_a_missing_entity() {
    let mut m = FakeMem::default();
    entities(&mut m, &[(11, 0)]);
    request(&mut m, 0x0800_0000, 3, 11, 1, NOW);
    request(&mut m, 0x0800_0100, 3, 12, 2, NOW);
    file(&mut m, &[(3, 0x0800_0000), (3, 0x0800_0100)]);
    ready(&mut m, 1, 3, Some(9));
    assert_eq!(
        paths(&plan_ready(&m, SEQ_MGR, EVT, NOW)),
        vec![("no_source_entity", Some(12))]
    );

    // Event 5001 defers a ViewType-0 request too.
    ready(&mut m, 1, 3, Some(layout::DEFERRED_EVENT_ID));
    request(&mut m, 0x0800_0000, 3, 11, 0, NOW);
    assert_eq!(
        paths(&plan_ready(&m, SEQ_MGR, EVT, NOW)),
        vec![("no_source_entity", Some(12))]
    );
}

/// A Source with a pawn but no cooked data is dropped as such.
#[test]
fn a_ready_event_without_data_drops_even_a_live_source() {
    let mut m = FakeMem::default();
    entities(&mut m, &[(10, 0xAAAA)]);
    request(&mut m, 0x0800_0000, 3, 10, 0, NOW);
    file(&mut m, &[(3, 0x0800_0000)]);
    ready(&mut m, 1, 3, None);
    assert_eq!(
        paths(&plan_ready(&m, SEQ_MGR, EVT, NOW)),
        vec![("no_cooked_data", Some(10))]
    );
}

/// The prune runs only above five requests (after the ready id's are
/// gone) and takes those 31 s old or more.
#[test]
fn stale_requests_are_reported_as_expired_only_when_the_prune_runs() {
    let mut m = FakeMem::default();
    entities(&mut m, &[(10, 0xAAAA)]);
    let mut reqs = Vec::new();
    for i in 0..6u32 {
        let ptr = 0x0800_0000 + i * 0x100;
        // Two are old enough, one is exactly one second short.
        let created = match i {
            1 => NOW - 31,
            2 => NOW - 400,
            3 => NOW - 30,
            _ => NOW,
        };
        request(&mut m, ptr, 20 + i as i32, 10, 0, created);
        reqs.push((20 + i as i32, ptr));
    }
    file(&mut m, &reqs);
    // A status other than 1 erases nothing, so all six count.
    ready(&mut m, 0, 99, None);
    let d = plan_ready(&m, SEQ_MGR, EVT, NOW);
    assert!(d.iter().all(|d| d.path == DropPath::Expired));
    assert_eq!(
        d.iter().map(|d| d.age_secs).collect::<Vec<_>>(),
        vec![Some(31), Some(400)]
    );

    // A ready event for one of them leaves five: no prune.
    ready(&mut m, 1, 25, Some(1));
    assert!(plan_ready(&m, SEQ_MGR, EVT, NOW).is_empty());
}

/// The successor walk handles a balanced tree, climbing back up to the
/// parent after a left subtree, and stops at the key's last duplicate.
#[test]
fn equal_range_walks_a_balanced_tree() {
    let mut m = FakeMem::default();
    let map = 0x0900_0000;
    let head = 0x0900_1000;
    let (a, b, c) = (0x0900_2000, 0x0900_3000, 0x0900_4000);
    // b is the root; a (key 5) left, c (key 6) right; b has key 5.
    m.set(map + 4, head);
    m.set(map + 8, 3);
    m.set(head, a);
    m.set(head + 4, b);
    m.set(head + 8, c);
    for (n, k, v, l, p, r) in [
        (a, 5, 0xA, head, b, head),
        (b, 5, 0xB, a, head, c),
        (c, 6, 0xC, head, b, head),
    ] {
        m.set(n, l);
        m.set(n + 4, p);
        m.set(n + 8, r);
        m.set(n + 0xc, k as u32);
        m.set(n + 0x10, v);
    }
    assert_eq!(requests_for(&m, map, 5), Some(vec![0xA, 0xB]));
    assert_eq!(requests_for(&m, map, 6), Some(vec![0xC]));
    assert_eq!(requests_for(&m, map, 7), Some(vec![]));
}

/// Unreadable memory yields no events rather than invented ones.
#[test]
fn unreadable_state_reports_nothing() {
    let mut m = FakeMem::default();
    ready(&mut m, 1, 3, Some(1));
    assert!(plan_ready(&m, SEQ_MGR, EVT, NOW).is_empty());

    // Requests are readable but the entity manager is not.
    request(&mut m, 0x0800_0000, 3, 10, 0, NOW);
    file(&mut m, &[(3, 0x0800_0000)]);
    assert!(plan_ready(&m, SEQ_MGR, EVT, NOW).is_empty());
}

/// A cyclic tree cannot hang the network thread.
#[test]
fn a_cyclic_tree_is_bounded() {
    let mut m = FakeMem::default();
    let map = 0x0900_0000;
    let head = 0x0900_1000;
    let n = 0x0900_2000;
    m.set(map + 4, head);
    m.set(map + 8, 1);
    m.set(head + 4, n);
    // n's right points to itself and its left to head: the successor of n
    // is n's leftmost right descendant, n itself, forever.
    m.set(n, head);
    m.set(n + 4, head);
    m.set(n + 8, n);
    m.set(n + 0xc, 5);
    m.set(n + 0x10, 1);
    let got = requests_for(&m, map, 5).unwrap();
    assert_eq!(got.len(), MAX_REQUESTS);
}

#[test]
fn the_play_step_outcome() {
    assert_eq!(play_outcome(false, 0), Some(DropPath::CulledByDistance));
    assert_eq!(play_outcome(true, 0), Some(DropPath::InstanceRefused));
    assert_eq!(play_outcome(true, 0x1234), None);
}

#[test]
fn drop_fields_name_the_sequence_the_source_and_the_path() {
    let d = SequenceDrop {
        path: DropPath::NoSourceEntity,
        ids: RequestIds {
            sequence_id: Some(3),
            source_id: Some(100_307),
            target_id: Some(4),
            view_type: Some(0),
            instance_id: Some(9),
            created: Some(1),
        },
        event_id: Some(12),
        age_secs: None,
    };
    let f: std::collections::HashMap<_, _> = d.fields("cache_ready").into_iter().collect();
    assert_eq!(f["path"], "no_source_entity");
    assert_eq!(f["stage"], "cache_ready");
    assert_eq!(f["sequence_id"], 3);
    assert_eq!(f["entity_id"], 100_307);
    assert_eq!(f["target_id"], 4);
    assert_eq!(f["event_id"], 12);
    assert!(!f.contains_key("age_secs"));
    assert_eq!(DropPath::CulledByDistance.level(), "debug");
    assert_eq!(DropPath::NoSourcePawn.level(), "info");
}
