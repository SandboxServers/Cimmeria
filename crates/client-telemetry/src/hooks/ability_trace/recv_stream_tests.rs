//! The receive path against a fake client address space laid out the way
//! the live client hands `onEntityMethod` its stream.

use serde_json::{json, Value};

use super::*;
use crate::hooks::entity_trace::map::fake::{build_map, FakeMem};

const MGR: u32 = 0x0010_0000;
const PLAYER: u32 = 0x0020_0000;
const NODES: u32 = 0x0030_0000;
/// The `EntityMethodMessage`'s `MemoryOStream` (full object).
const OSTREAM: u32 = 0x0040_0000;
/// Its buffer.
const BUF: u32 = 0x0050_0000;
/// A Nub-style `MemoryIStream`.
const ISTREAM: u32 = 0x0060_0000;
/// Another entity in the world (an `SGWMob`, type 7).
const MOB: u32 = 0x0070_0000;
const LOCAL_ID: i32 = 2;
const MOB_ID: i32 = 100_226;

fn get(f: &Fields, k: &str) -> Value {
    f.iter()
        .find(|(n, _)| *n == k)
        .map(|(_, v)| v.clone())
        .unwrap_or(Value::Null)
}

/// Bytes into word-addressed fake memory, zero-padded.
fn put_bytes(m: &mut FakeMem, at: u32, b: &[u8]) {
    for (i, c) in b.chunks(4).enumerate() {
        let mut w = [0u8; 4];
        w[..c.len()].copy_from_slice(c);
        m.set(at + 4 * i as u32, u32::from_le_bytes(w));
    }
}

/// The local player (id 2, `SGWGmPlayer` type 3, as on the colo run) and
/// a mob, both in the world map.
fn world() -> FakeMem {
    let mut m = FakeMem::default();
    m.set(MGR + map::manager::LOCAL_PLAYER_ENTITY, PLAYER);
    m.set(PLAYER + map::entity::ID, LOCAL_ID as u32);
    m.set(PLAYER + map::entity::TYPE_ID, 3);
    m.set(MOB + map::entity::ID, MOB_ID as u32);
    m.set(MOB + map::entity::TYPE_ID, 7);
    build_map(
        &mut m,
        MGR + map::manager::WORLD_MAP,
        NODES,
        &[(LOCAL_ID, PLAYER), (MOB_ID, MOB)],
    );
    m
}

/// `EntityMethodMessage`'s `MemoryOStream` holding `args`, nothing read
/// yet, as `0x01561a20` builds it. Returns the `BinaryIStream` subobject
/// `EntityMethodMessage::process` passes to `onEntityMethod`.
fn live_stream(m: &mut FakeMem, args: &[u8]) -> u32 {
    put_bytes(m, BUF, args);
    m.set(OSTREAM, 0x019c_e74c); // BinaryOStream vtable
    m.set(OSTREAM + 4, 0x019c_e734); // BinaryIStream vtable
    m.set(OSTREAM + 8, 0); // error flag
    m.set(OSTREAM + 0x0c, BUF); // buffer start
    m.set(OSTREAM + 0x10, BUF + args.len() as u32); // write cursor
    m.set(OSTREAM + 0x14, BUF + 0x40); // capacity end
    m.set(OSTREAM + 0x18, BUF); // read cursor
    OSTREAM + 4
}

/// `onTimerUpdate` (12) as the colo server sent it for Heal Focus (597):
/// the cooldown timer, type 2, 21 bytes.
fn heal_focus_cooldown() -> Vec<u8> {
    let mut b = 597i32.to_le_bytes().to_vec();
    b.push(2);
    b.extend(LOCAL_ID.to_le_bytes());
    b.extend(0i32.to_le_bytes());
    b.extend(32.0f32.to_le_bytes());
    b.extend(155.716f32.to_le_bytes());
    assert_eq!(b.len(), 21);
    b
}

/// `onStatUpdate` (20): one stat, 20 bytes, as on the colo run.
fn one_stat() -> Vec<u8> {
    let mut b = 1u32.to_le_bytes().to_vec();
    for v in [8i32, 0, 1570, 1570] {
        b.extend(v.to_le_bytes());
    }
    b
}

/// The live bug (2026-10-04, colo session 55959c2e): the client hands
/// `onEntityMethod` the `MemoryOStream` subobject, never a
/// `MemoryIStream`, and the hook returned silently for every message, so
/// `onTimerUpdate` (msg 12) and `onStatUpdate` (msg 20) on the local
/// `SGWGmPlayer` never became `client.ability.recv`. Both decode now.
#[test]
fn the_live_entity_method_message_stream_decodes() {
    let mut m = world();
    let stream = live_stream(&mut m, &heal_focus_cooldown());
    let (target, level, f) = observe(&m, MGR, LOCAL_ID, 12, stream).expect("a recv row");
    assert_eq!((target, level), (TARGET_RECV, "info"));
    assert_eq!(get(&f, "method"), json!("onTimerUpdate"));
    assert_eq!(get(&f, "msg_id"), json!(12));
    assert_eq!(get(&f, "len"), json!(21));
    assert_eq!(get(&f, "path"), json!("delivered"));
    assert_eq!(get(&f, "timer_id"), json!(597));
    assert_eq!(get(&f, "timer_type"), json!(2));
    assert_eq!(get(&f, "total_time"), json!(32.0));

    let mut m = world();
    let stream = live_stream(&mut m, &one_stat());
    let (target, _, f) = observe(&m, MGR, LOCAL_ID, 20, stream).expect("a recv row");
    assert_eq!(target, TARGET_RECV);
    assert_eq!(get(&f, "method"), json!("onStatUpdate"));
    assert_eq!(get(&f, "stats"), json!([[8, 0, 1570, 1570]]));
    assert_eq!(get(&f, "decode_error"), Value::Null);
}

#[test]
fn the_live_layout_reads_written_minus_read() {
    let mut m = world();
    let stream = live_stream(&mut m, &one_stat());
    let w = window(&m, stream).unwrap();
    assert_eq!((w.cursor, w.len, w.layout), (BUF, 20, "memory_ostream"));
    // Part read already: the window starts at the read cursor.
    m.set(OSTREAM + 0x18, BUF + 4);
    assert_eq!(window(&m, stream).unwrap().len, 16);
}

/// A Nub `MemoryIStream` (a direct dispatch) still decodes.
#[test]
fn a_memory_istream_still_decodes() {
    let mut m = world();
    let args = one_stat();
    put_bytes(&mut m, BUF, &args);
    m.set(ISTREAM, MEMORY_ISTREAM.vtable);
    m.set(ISTREAM + 8, BUF);
    m.set(ISTREAM + 0x0c, BUF + args.len() as u32);
    let w = window(&m, ISTREAM).unwrap();
    assert_eq!((w.cursor, w.len, w.layout), (BUF, 20, "memory_istream"));
    let (target, _, f) = observe(&m, MGR, MOB_ID, 20, ISTREAM).unwrap();
    assert_eq!(target, TARGET_RECV);
    assert_eq!(get(&f, "stats_count"), json!(1));
}

/// A subclass with its own vtable but the same `remaining` slot keeps the
/// layout.
#[test]
fn a_layout_is_recognised_by_its_remaining_slot() {
    let mut m = world();
    let stream = live_stream(&mut m, &one_stat());
    m.set(stream, 0x0123_4560);
    m.set(0x0123_4568, MEMORY_OSTREAM_ISTREAM.remaining);
    assert_eq!(window(&m, stream).unwrap().layout, "memory_ostream");
}

/// An unknown stream is a `warn` `recv_skipped` naming the method, the
/// message id and the vtable, never a silent return.
#[test]
fn an_unknown_stream_is_reported_not_dropped() {
    let mut m = world();
    let stream = live_stream(&mut m, &heal_focus_cooldown());
    m.set(stream, 0x0bad_0001);
    let (target, level, f) = observe(&m, MGR, LOCAL_ID, 12, stream).expect("a skipped row");
    assert_eq!((target, level), (TARGET_RECV_SKIPPED, "warn"));
    assert_eq!(get(&f, "reason"), json!("unknown_stream"));
    assert_eq!(get(&f, "msg_id"), json!(12));
    assert_eq!(get(&f, "method_index"), json!(12));
    assert_eq!(get(&f, "method"), json!("onTimerUpdate"));
    assert_eq!(get(&f, "vtable"), json!("0x0bad0001"));
    assert_eq!(get(&f, "entity_id"), json!(LOCAL_ID));
}

#[test]
fn a_cursor_past_the_end_is_a_bad_window() {
    let mut m = world();
    let stream = live_stream(&mut m, &one_stat());
    m.set(OSTREAM + 0x18, BUF + 0x100);
    let (target, _, f) = observe(&m, MGR, LOCAL_ID, 20, stream).unwrap();
    assert_eq!(target, TARGET_RECV_SKIPPED);
    assert_eq!(get(&f, "reason"), json!("bad_window"));
    assert_eq!(get(&f, "method_index"), json!(20));
}

/// Argument bytes that cannot be read are `read_failed`, with the method.
#[test]
fn unreadable_arguments_are_read_failed() {
    let mut m = world();
    let stream = live_stream(&mut m, &one_stat());
    m.words.retain(|&a, _| !(BUF..BUF + 0x40).contains(&a));
    let (target, level, f) = observe(&m, MGR, LOCAL_ID, 20, stream).unwrap();
    assert_eq!((target, level), (TARGET_RECV_SKIPPED, "warn"));
    assert_eq!(get(&f, "reason"), json!("read_failed"));
    assert_eq!(get(&f, "method"), json!("onStatUpdate"));
    assert_eq!(get(&f, "len"), json!(20));
}

/// A player-only method for an entity that is in no map is an `info`
/// `receiver_unknown`: it may be ours, so it is not dropped silently.
#[test]
fn a_player_method_for_an_unplaced_entity_is_receiver_unknown() {
    let mut m = world();
    let mut b = vec![60u8, 0];
    b.extend(597i32.to_le_bytes());
    b.extend(306u16.to_le_bytes());
    let stream = live_stream(&mut m, &b);
    let (target, level, f) = observe(&m, MGR, 999_999, 61, stream).unwrap();
    assert_eq!((target, level), (TARGET_RECV_SKIPPED, "info"));
    assert_eq!(get(&f, "reason"), json!("receiver_unknown"));
    assert_eq!(get(&f, "method"), json!("onErrorCode"));
    assert_eq!(get(&f, "method_index"), json!(121));
    assert_eq!(get(&f, "path"), json!("queued"));
}

/// Not ours costs nothing and reports nothing: an id no row has, and a
/// player-only index on a mob (its 28 is another method), with no stream
/// at all in memory.
#[test]
fn what_is_not_ours_is_neither_read_nor_reported() {
    let m = world();
    assert!(observe(&m, MGR, LOCAL_ID, 2, 0xdead_0000).is_none());
    assert!(observe(&m, MGR, MOB_ID, 28, 0xdead_0000).is_none());
}
