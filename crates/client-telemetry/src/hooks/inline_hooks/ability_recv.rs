//! `client.ability.recv` from inside the `onEntityMethod` detour.
//!
//! `EntityManager::onEntityMethod` (`0x00dd2b80`, network thread) gets the
//! message's arguments as a `MemoryIStream`: vtable `0x01b18e38`, `+0x08`
//! the read cursor, `+0x0c` the end (slot 2, `0x0157af60`, is
//! `end - cursor`). The bytes `[cursor, end)` are the arguments in `.def`
//! order, after the message id; for an extended id the first byte is the
//! sub-index. They are read here, through `ReadProcessMemory`, before the
//! original runs, and the cursor is never moved, so the game reads exactly
//! what it would have. The decoding is `hooks::ability_trace::recv`.

use std::ffi::c_void;

use crate::hooks::ability_trace::{
    recv::{self, Message},
    recv_methods::{self, Receiver, PLAYER_TYPE_IDS},
    TARGET_RECV,
};
use crate::hooks::emit::emit;
use crate::hooks::entity_trace::{
    self as trace,
    map::{self, LiveMem, Lookup, Mem},
};

/// `MemoryIStream`'s vtable in the QA build.
const MEMORY_ISTREAM_VTABLE: u32 = 0x01b1_8e38;
/// Its `remaining` (`end - cursor`), vtable slot 2. A subclass that keeps
/// this slot keeps the cursor and end where this one reads them.
const MEMORY_ISTREAM_REMAINING: u32 = 0x0157_af60;
/// `MemoryIStream` read cursor.
const STREAM_CURSOR: u32 = 0x08;
/// `MemoryIStream` end.
const STREAM_END: u32 = 0x0c;

/// The cursor and remaining length of `stream`, when it is a
/// `MemoryIStream` (or keeps its layout).
fn stream_window(stream: u32) -> Option<(u32, u32)> {
    let vtable = LiveMem.u32_at(stream)?;
    if vtable != MEMORY_ISTREAM_VTABLE
        && LiveMem.u32_at(vtable.checked_add(8)?)? != MEMORY_ISTREAM_REMAINING
    {
        return None;
    }
    let cursor = LiveMem.u32_at(stream.wrapping_add(STREAM_CURSOR))?;
    let end = LiveMem.u32_at(stream.wrapping_add(STREAM_END))?;
    Some((cursor, end.checked_sub(cursor)?))
}

/// Who the message is for, its delivery path (the way `onEntityMethod`
/// decides it), and whether it is the local player.
fn receiver(mgr: u32, id: i32) -> (Receiver, &'static str, bool) {
    let is_local = LiveMem
        .u32_at(mgr.wrapping_add(map::manager::LOCAL_PLAYER_ENTITY))
        .filter(|&p| p != 0)
        .and_then(|p| LiveMem.u32_at(p.wrapping_add(map::entity::ID)))
        == Some(id as u32);
    let type_id = match map::find(&LiveMem, mgr.wrapping_add(map::manager::WORLD_MAP), id) {
        Lookup::Found(node) => LiveMem
            .u32_at(node.wrapping_add(0x10))
            .and_then(|e| LiveMem.u32_at(e.wrapping_add(map::entity::TYPE_ID)))
            .map(|w| (w & 0xffff) as u16)
            .or(Some(u16::MAX)),
        _ => None,
    };
    let receiver = match type_id {
        _ if is_local => Receiver::Player,
        Some(t) if PLAYER_TYPE_IDS.contains(&t) => Receiver::Player,
        Some(_) => Receiver::Other,
        None => Receiver::Unknown,
    };
    (
        receiver,
        trace::delivery_path(type_id.is_some(), is_local),
        is_local,
    )
}

/// Decode and emit the message, if it is one of ours. Called before the
/// original; reads only.
pub(super) fn report(mgr: *mut c_void, id: i32, msg_id: u32, stream: *mut c_void) {
    if !recv_methods::may_be_wanted(msg_id) {
        return;
    }
    let Some((cursor, len)) = stream_window(stream as u32) else {
        return;
    };
    let (receiver, path, local) = receiver(mgr as u32, id);
    let msg = Message {
        msg_id,
        entity_id: id,
        receiver,
        path,
        local,
        len,
    };
    let read = |n: usize| LiveMem.bytes_at(cursor, n.min(len as usize));
    if let Some((level, fields)) = recv::report(&msg, read) {
        emit(TARGET_RECV, level, fields);
    }
}
