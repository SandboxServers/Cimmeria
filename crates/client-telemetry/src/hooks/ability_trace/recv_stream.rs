//! Where `client.ability.recv` finds a message's argument bytes, and the
//! `client.ability.recv_skipped` row for a candidate it could not decode.
//!
//! `EntityManager::onEntityMethod` (`0x00dd2b80`) takes a `BinaryIStream*`,
//! and in the live client that stream is **not** the Nub's `MemoryIStream`.
//! SGW's `SGWMessageQueue` (vtable `0x01b14f3c`) is the connection's
//! message handler on the network thread. Its `onEntityMethod` (`0x01563630`,
//! slot 9) copies the remaining argument bytes into an
//! `EntityMethodMessage` (`.?AVEntityMethodMessage@Detail@@`, ctor
//! `0x01561a20`) that owns a `MemoryOStream` at `+0x0c`, and queues it. The
//! game thread later runs `EntityMethodMessage::process` (`0x01561ac0`),
//! which calls the `EntityManager`'s `onEntityMethod(id, msg_id, stream)`
//! with that `MemoryOStream`'s `BinaryIStream` subobject (`+0x10`). Both
//! layouts are known here, so a direct Nub dispatch decodes too:
//!
//! | Stream | vtable | `remaining` (slot 2) | read cursor | end |
//! |---|---|---|---|---|
//! | `MemoryOStream`'s `BinaryIStream` subobject (the live path) | `0x019ce734` | `0x00dd3f80` | `+0x14` | `+0x0c` |
//! | `MemoryIStream` (a direct Nub dispatch) | `0x01b18e38` | `0x0157af60` | `+0x08` | `+0x0c` |
//!
//! (`MemoryOStream`, full object: `+0` `BinaryOStream` vtable, `+4`
//! `BinaryIStream` vtable, `+8` error flag, `+0xc` buffer start, `+0x10`
//! write cursor, `+0x14` capacity end, `+0x18` read cursor; slot 2 of the
//! subobject is `[+0xc] - [+0x14]` relative to it, i.e. written minus read.)
//!
//! Until 2026-10-04 the hook knew only the `MemoryIStream` layout and
//! returned silently for every live message, so no `client.ability.recv`
//! row was ever emitted. Every candidate that is not decoded now leaves a
//! rate-limited `client.ability.recv_skipped` row with its reason.

use serde_json::json;

use super::recv::{self, Message};
use super::recv_methods::{self, Receiver, RecvMethod};
use super::{TARGET_RECV, TARGET_RECV_SKIPPED};
use crate::hooks::entity_trace::{
    self as trace,
    map::{self, Lookup, Mem},
    Fields,
};

/// One `BinaryIStream` layout the hook can read without moving it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Layout {
    /// Short name, for tests and diagnostics.
    pub name: &'static str,
    /// The object's vtable in the QA build.
    pub vtable: u32,
    /// Its `remaining` (vtable slot 2). A subclass that keeps this slot
    /// keeps the cursor and the end where this layout reads them.
    pub remaining: u32,
    /// Offset of the read cursor.
    pub cursor: u32,
    /// Offset of the end of the readable bytes.
    pub end: u32,
}

/// The `MemoryOStream`'s `BinaryIStream` subobject that
/// `EntityMethodMessage::process` passes (every live message).
pub(crate) const MEMORY_OSTREAM_ISTREAM: Layout = Layout {
    name: "memory_ostream",
    vtable: 0x019c_e734,
    remaining: 0x00dd_3f80,
    cursor: 0x14,
    end: 0x0c,
};

/// The Nub's stack `MemoryIStream` (`processOrderedPacket`, `0x0157caab`).
pub(crate) const MEMORY_ISTREAM: Layout = Layout {
    name: "memory_istream",
    vtable: 0x01b1_8e38,
    remaining: 0x0157_af60,
    cursor: 0x08,
    end: 0x0c,
};

/// Every layout, the live one first.
pub(crate) const LAYOUTS: [Layout; 2] = [MEMORY_OSTREAM_ISTREAM, MEMORY_ISTREAM];

/// A larger window than this is a misread, not a message (Mercury bundles
/// are a few KiB; a fragmented one stays far below this).
const MAX_WINDOW: u32 = 1 << 20;

/// `recv_skipped` reasons.
pub(crate) mod reason {
    /// The stream's vtable is neither known layout.
    pub(crate) const UNKNOWN_STREAM: &str = "unknown_stream";
    /// The stream, its cursor or its end could not be read, or the cursor
    /// is past the end.
    pub(crate) const BAD_WINDOW: &str = "bad_window";
    /// The sub-index or the argument bytes could not be read.
    pub(crate) const READ_FAILED: &str = "read_failed";
    /// A player-only method for an entity the hook could not place.
    pub(crate) const RECEIVER_UNKNOWN: &str = "receiver_unknown";
}

/// Why a candidate message was not decoded.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Skip {
    /// One of [`reason`].
    pub reason: &'static str,
    /// The `onEntityMethod` message id.
    pub msg_id: u32,
    /// The method, when it is known without the argument bytes.
    pub method: Option<&'static RecvMethod>,
    /// The stream's vtable, for [`reason::UNKNOWN_STREAM`].
    pub vtable: Option<u32>,
}

impl Skip {
    pub(crate) fn new(
        reason: &'static str,
        msg_id: u32,
        method: Option<&'static RecvMethod>,
    ) -> Self {
        Self {
            reason,
            msg_id,
            method,
            vtable: None,
        }
    }

    /// `warn` for a fault in the hook; `info` for a message that could
    /// not be placed (normal around AoI churn, but still a gap).
    fn level(&self) -> &'static str {
        if self.reason == reason::RECEIVER_UNKNOWN {
            "info"
        } else {
            "warn"
        }
    }
}

/// The readable bytes of a stream: `[cursor, cursor + len)`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Window {
    pub cursor: u32,
    pub len: u32,
    pub layout: &'static str,
}

/// The window of `stream`, read without moving it. `Err` carries the
/// reason and, when it was readable, the vtable.
pub(crate) fn window(mem: &dyn Mem, stream: u32) -> Result<Window, (&'static str, Option<u32>)> {
    let vtable = mem.u32_at(stream).ok_or((reason::BAD_WINDOW, None))?;
    let slot2 = vtable.checked_add(8).and_then(|a| mem.u32_at(a));
    let layout = LAYOUTS
        .iter()
        .find(|l| l.vtable == vtable || slot2 == Some(l.remaining))
        .ok_or((reason::UNKNOWN_STREAM, Some(vtable)))?;
    let bad = (reason::BAD_WINDOW, Some(vtable));
    let cursor = mem.u32_at(stream.wrapping_add(layout.cursor)).ok_or(bad)?;
    let end = mem.u32_at(stream.wrapping_add(layout.end)).ok_or(bad)?;
    let len = end
        .checked_sub(cursor)
        .filter(|&n| n <= MAX_WINDOW)
        .ok_or(bad)?;
    Ok(Window {
        cursor,
        len,
        layout: layout.name,
    })
}

/// Who the message is for, its delivery path (the way `onEntityMethod`
/// decides it), and whether it is the local player.
pub(crate) fn receiver(mem: &dyn Mem, mgr: u32, id: i32) -> (Receiver, &'static str, bool) {
    let is_local = mem
        .u32_at(mgr.wrapping_add(map::manager::LOCAL_PLAYER_ENTITY))
        .filter(|&p| p != 0)
        .and_then(|p| mem.u32_at(p.wrapping_add(map::entity::ID)))
        == Some(id as u32);
    let type_id = match map::find(mem, mgr.wrapping_add(map::manager::WORLD_MAP), id) {
        Lookup::Found(node) => mem
            .u32_at(node.wrapping_add(map::node::VALUE))
            .and_then(|e| mem.u32_at(e.wrapping_add(map::entity::TYPE_ID)))
            .map(|w| (w & 0xffff) as u16)
            .or(Some(u16::MAX)),
        _ => None,
    };
    let receiver = match type_id {
        _ if is_local => Receiver::Player,
        Some(t) if recv_methods::PLAYER_TYPE_IDS.contains(&t) => Receiver::Player,
        Some(_) => Receiver::Other,
        None => Receiver::Unknown,
    };
    (
        receiver,
        trace::delivery_path(type_id.is_some(), is_local),
        is_local,
    )
}

/// The `onEntityMethod` detour's whole receive path, before the original
/// runs: `(target, level, fields)` for a decoded `client.ability.recv`, a
/// `client.ability.recv_skipped`, or nothing (not ours, or throttled).
/// Reads only.
pub(crate) fn observe(
    mem: &dyn Mem,
    mgr: u32,
    id: i32,
    msg_id: u32,
    stream: u32,
) -> Option<(&'static str, &'static str, Fields)> {
    if !recv_methods::may_be_wanted(msg_id) {
        return None;
    }
    let (receiver, path, local) = receiver(mem, mgr, id);
    // A direct id that is not ours for this receiver costs no stream read.
    if !recv_methods::is_extended(msg_id)
        && receiver != Receiver::Unknown
        && recv_methods::resolve(msg_id, receiver, None).is_none()
    {
        return None;
    }
    let w = match window(mem, stream) {
        Ok(w) => w,
        Err((why, vtable)) => {
            let skip = Skip {
                vtable,
                ..Skip::new(why, msg_id, direct_method(msg_id))
            };
            return skipped(&skip, id, path, None);
        }
    };
    let msg = Message {
        msg_id,
        entity_id: id,
        receiver,
        path,
        local,
        len: w.len,
    };
    let read = |n: usize| mem.bytes_at(w.cursor, n.min(w.len as usize));
    match recv::report(&msg, read) {
        Ok(Some((level, fields))) => Some((TARGET_RECV, level, fields)),
        Ok(None) => None,
        Err(skip) => skipped(&skip, id, path, Some(w.len)),
    }
}

/// The table row a direct (non-extended) id names, whatever the receiver.
fn direct_method(msg_id: u32) -> Option<&'static RecvMethod> {
    if recv_methods::is_extended(msg_id) {
        return None;
    }
    recv_methods::resolve(msg_id, Receiver::Player, None).map(|(m, _)| m)
}

/// The `recv_skipped` row for `skip`, through its per-reason bucket.
fn skipped(
    skip: &Skip,
    entity_id: i32,
    path: &'static str,
    len: Option<u32>,
) -> Option<(&'static str, &'static str, Fields)> {
    let key = format!("recv_skipped:{}", skip.reason);
    let f = super::admit(&key, || {
        let mut f: Fields = vec![
            ("reason", json!(skip.reason)),
            ("msg_id", json!(skip.msg_id)),
            ("method_index", json!(skip.method.map(|m| m.index))),
            ("method", json!(skip.method.map(|m| m.name))),
            ("entity_id", json!(entity_id)),
            ("path", json!(path)),
        ];
        if let Some(n) = len {
            f.push(("len", json!(n)));
        }
        if let Some(v) = skip.vtable {
            f.push(("vtable", json!(format!("0x{v:08x}"))));
        }
        f
    })?;
    Some((TARGET_RECV_SKIPPED, skip.level(), f))
}

#[cfg(test)]
#[path = "recv_stream_tests.rs"]
mod tests;
