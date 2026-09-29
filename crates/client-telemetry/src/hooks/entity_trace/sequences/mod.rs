//! The client `SequenceManager`'s silent drops of a server `onSequence`.
//!
//! A server `onSequence` (client method 1) becomes an
//! `Event_NetIn_onSequence`, which `SequenceManager` turns into a request
//! (`FUN_00d13780`) filed in a `std::multimap<int, Request*>` at
//! `SequenceManager+0x48`, keyed by the Kismet event-set sequence id. When
//! the cooked sequence data is ready the cache fires
//! `Event_Cache_ElementReady` (`0x00d06f30`), which walks every request
//! filed under that id and either plays it (`0x00d06dd0`), defers it to
//! `Event_AppearanceJob_Completed`, or drops it without a word. Then it
//! erases the id's requests and prunes stale ones. Every branch and
//! offset here is from the QA `SGW.exe` (Ghidra, 2026-09-29); see
//! `docs/reverse-engineering/findings/npc-attack-presentation.md`.
//!
//! This module works out, from memory alone and before the original runs,
//! which requests the ready handler will drop and why. It never writes and
//! never calls game code; every read goes through [`Mem`], which the DLL
//! backs with `ReadProcessMemory`. The detours are in
//! `hooks::inline_hooks::sequence_manager`.

use serde_json::json;

use super::map::{self, Lookup, Mem};
use super::Fields;

/// Addresses and offsets in the QA build.
pub mod layout {
    /// `GameEntityManager::instance_` (read by `FUN_00c66ad0`).
    pub const ENTITY_MANAGER_SINGLETON: u32 = 0x01ef_244c;
    /// The request multimap inside `SequenceManager`.
    pub const REQUESTS: u32 = 0x48;
    /// A client `Entity`'s pawn pointer; `0` until the pawn exists.
    pub const ENTITY_PAWN: u32 = 0x08;
    /// The cooked sequence data's Kismet event id.
    pub const DATA_EVENT_ID: u32 = 0x10;
    /// The event id the ready handler defers instead of dropping.
    pub const DEFERRED_EVENT_ID: u32 = 5001;

    /// `Event_Cache_ElementReady` payload (the handler's first argument).
    pub mod ready {
        /// `1` when the element loaded.
        pub const STATUS: u32 = 0x00;
        /// The sequence id the element is for (the multimap key).
        pub const KEY: u32 = 0x04;
        /// The cooked sequence data, `0` when there is none.
        pub const DATA: u32 = 0x08;
    }

    /// A sequence request, as `FUN_00d13780` fills it.
    pub mod request {
        /// `KismetEventSetSeqID`.
        pub const SEQUENCE_ID: u32 = 0x00;
        /// `SourceID`.
        pub const SOURCE_ID: u32 = 0x0c;
        /// `TargetID`.
        pub const TARGET_ID: u32 = 0x10;
        /// `_time64` when the request was filed (low word).
        pub const CREATED: u32 = 0x14;
        /// `ViewType` (sign-extended byte).
        pub const VIEW_TYPE: u32 = 0x1c;
        /// `InstanceId`.
        pub const INSTANCE_ID: u32 = 0x20;
    }

    /// The prune after every ready event (`FUN_00d05450(5, map)`) runs
    /// only while the map holds more than this many requests...
    pub const PRUNE_MIN_SIZE: u32 = 5;
    /// ...and erases every request at least this many seconds old.
    pub const PRUNE_AGE_SECS: u32 = 31;
}

/// Why a request was dropped. The string is the `path` field.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DropPath {
    /// The ready handler found no client entity for the Source.
    NoSourceEntity,
    /// The Source entity has no pawn, and the sequence is not one the
    /// handler defers (event 5001, `ViewType` 1 or 2).
    NoSourcePawn,
    /// The Source has a pawn but the cache delivered no sequence data.
    NoCookedData,
    /// The play step (`0x00d06dd0`) culled it: its nearer endpoint is
    /// beyond the view-distance setting of the local viewer.
    CulledByDistance,
    /// The play step asked for a Kismet instance and got none (sequences
    /// switched off, the active-instance cap reached, or the script
    /// failed to load).
    InstanceRefused,
    /// Pruned: filed more than 30 s ago and its data never became ready.
    Expired,
}

impl DropPath {
    /// Stable field value.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::NoSourceEntity => "no_source_entity",
            Self::NoSourcePawn => "no_source_pawn",
            Self::NoCookedData => "no_cooked_data",
            Self::CulledByDistance => "culled_by_distance",
            Self::InstanceRefused => "instance_refused",
            Self::Expired => "expired",
        }
    }

    /// `debug` for the view-distance cull, which the client does by design
    /// in any large fight; `info` for the rest, which are the finding.
    pub fn level(self) -> &'static str {
        match self {
            Self::CulledByDistance => "debug",
            _ => "info",
        }
    }
}

/// The ids of one request, as far as they could be read.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct RequestIds {
    /// `KismetEventSetSeqID`.
    pub sequence_id: Option<i32>,
    /// `SourceID`.
    pub source_id: Option<i32>,
    /// `TargetID`.
    pub target_id: Option<i32>,
    /// `ViewType`.
    pub view_type: Option<i32>,
    /// `InstanceId`.
    pub instance_id: Option<u32>,
    /// Filing time (`_time64`, low word).
    pub created: Option<u32>,
}

/// Read the ids of the request at `req`.
pub fn read_request(mem: &dyn Mem, req: u32) -> RequestIds {
    use layout::request::*;
    let i = |off| mem.u32_at(req.wrapping_add(off)).map(|v| v as i32);
    RequestIds {
        sequence_id: i(SEQUENCE_ID),
        source_id: i(SOURCE_ID),
        target_id: i(TARGET_ID),
        view_type: i(VIEW_TYPE),
        instance_id: mem.u32_at(req.wrapping_add(INSTANCE_ID)),
        created: mem.u32_at(req.wrapping_add(CREATED)),
    }
}

/// One dropped request.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SequenceDrop {
    /// Why.
    pub path: DropPath,
    /// Its ids.
    pub ids: RequestIds,
    /// The Kismet event id of the cooked data, when there is some.
    pub event_id: Option<u32>,
    /// Seconds since it was filed (only for [`DropPath::Expired`]).
    pub age_secs: Option<u32>,
}

impl SequenceDrop {
    /// The fields of its `client.sequence.dropped` event.
    pub fn fields(&self, stage: &'static str) -> Fields {
        let mut f: Fields = vec![("path", json!(self.path.as_str())), ("stage", json!(stage))];
        let ids = &self.ids;
        f.push(("sequence_id", json!(ids.sequence_id)));
        f.push(("entity_id", json!(ids.source_id)));
        f.push(("target_id", json!(ids.target_id)));
        if let Some(v) = ids.view_type {
            f.push(("view_type", json!(v)));
        }
        if let Some(v) = ids.instance_id {
            f.push(("instance_id", json!(v)));
        }
        if let Some(e) = self.event_id {
            f.push(("event_id", json!(e)));
        }
        if let Some(a) = self.age_secs {
            f.push(("age_secs", json!(a)));
        }
        f
    }
}

// ---------------------------------------------------------------------
// The multimap

/// A tree node is `{ _Left, _Parent, _Right, key, value }` (VS2005/2008);
/// every leaf link points back at the head sentinel.
mod node {
    pub const LEFT: u32 = 0x00;
    pub const PARENT: u32 = 0x04;
    pub const RIGHT: u32 = 0x08;
    pub const KEY: u32 = 0x0c;
    pub const VALUE: u32 = 0x10;
}

/// Walk limit for one descent, as in [`map::find`].
const MAX_DEPTH: usize = 96;
/// Requests read under one sequence id. The ready handler's own range is
/// a handful; this only stops a corrupt tree.
pub const MAX_REQUESTS: usize = 256;
/// Requests the prune check reads. Every read is a `ReadProcessMemory` on
/// the game thread, so a backlog is sampled from its lowest ids instead
/// of scanned whole (a pruned request past it goes unreported).
pub const MAX_PRUNE_SCAN: usize = 64;

/// The in-order successor of `n` in the tree whose head is `head`, or
/// `None` when a read fails or the walk runs away.
fn successor(mem: &dyn Mem, head: u32, n: u32) -> Option<u32> {
    let right = mem.u32_at(n.wrapping_add(node::RIGHT))?;
    if right != head {
        let mut cur = right;
        for _ in 0..MAX_DEPTH {
            let left = mem.u32_at(cur.wrapping_add(node::LEFT))?;
            if left == head {
                return Some(cur);
            }
            cur = left;
        }
        return None;
    }
    let mut cur = n;
    for _ in 0..MAX_DEPTH {
        let parent = mem.u32_at(cur.wrapping_add(node::PARENT))?;
        if parent == head {
            return Some(head);
        }
        if mem.u32_at(parent.wrapping_add(node::RIGHT))? != cur {
            return Some(parent);
        }
        cur = parent;
    }
    None
}

/// `(key, value)` of every node from `first` on, in order, while `keep`
/// holds, up to `limit` nodes. `None` if a read fails.
fn walk(
    mem: &dyn Mem,
    head: u32,
    first: u32,
    limit: usize,
    keep: impl Fn(i32) -> bool,
) -> Option<Vec<(i32, u32)>> {
    let mut out = Vec::new();
    let mut cur = first;
    while cur != head && out.len() < limit {
        if cur == 0 {
            return None;
        }
        let key = mem.u32_at(cur.wrapping_add(node::KEY))? as i32;
        if !keep(key) {
            break;
        }
        out.push((key, mem.u32_at(cur.wrapping_add(node::VALUE))?));
        cur = successor(mem, head, cur)?;
    }
    Some(out)
}

/// The first node whose key is not below `key` (the head if none).
fn lower_bound(mem: &dyn Mem, head: u32, key: i32) -> Option<u32> {
    let mut cur = mem.u32_at(head.wrapping_add(node::PARENT))?;
    let mut best = head;
    let mut steps = 0;
    while cur != head {
        steps += 1;
        if steps > MAX_DEPTH || cur == 0 {
            return None;
        }
        let k = mem.u32_at(cur.wrapping_add(node::KEY))? as i32;
        if k < key {
            cur = mem.u32_at(cur.wrapping_add(node::RIGHT))?;
        } else {
            best = cur;
            cur = mem.u32_at(cur.wrapping_add(node::LEFT))?;
        }
    }
    Some(best)
}

/// The multimap object's head and size.
fn head_and_size(mem: &dyn Mem, multimap: u32) -> Option<(u32, u32)> {
    let head = mem.u32_at(multimap.wrapping_add(4))?;
    let size = mem.u32_at(multimap.wrapping_add(8))?;
    (head != 0).then_some((head, size))
}

/// Request pointers filed under `key`.
pub fn requests_for(mem: &dyn Mem, multimap: u32, key: i32) -> Option<Vec<u32>> {
    let (head, _) = head_and_size(mem, multimap)?;
    let first = lower_bound(mem, head, key)?;
    Some(
        walk(mem, head, first, MAX_REQUESTS, |k| k == key)?
            .into_iter()
            .map(|(_, v)| v)
            .collect(),
    )
}

// ---------------------------------------------------------------------
// The ready handler

/// What the ready handler will do with one request.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Fate {
    /// Handed to the play step, which may still cull it.
    Play,
    /// Parked until the Source's appearance job completes.
    Deferred,
    /// Dropped.
    Dropped(DropPath),
    /// A read failed; say nothing.
    Unknown,
}

/// The fate of one request, worked out as `0x00d06f30` does: look the
/// Source up in the entity manager's world map, check its pawn, then the
/// cooked data. `data` is the ready event's data pointer and `event_id`
/// the Kismet event id read from it.
pub fn fate(mem: &dyn Mem, ids: &RequestIds, data: u32, event_id: Option<u32>) -> Fate {
    let Some(source) = ids.source_id else {
        return Fate::Unknown;
    };
    let Some(mgr) = mem.u32_at(layout::ENTITY_MANAGER_SINGLETON) else {
        return Fate::Unknown;
    };
    if mgr == 0 {
        return Fate::Unknown;
    }
    let pawn = match map::find(mem, mgr.wrapping_add(map::manager::WORLD_MAP), source) {
        Lookup::Found(n) => {
            let Some(entity) = mem.u32_at(n.wrapping_add(node::VALUE)) else {
                return Fate::Unknown;
            };
            if entity == 0 {
                None
            } else {
                match mem.u32_at(entity.wrapping_add(layout::ENTITY_PAWN)) {
                    Some(p) => Some(p),
                    None => return Fate::Unknown,
                }
            }
        }
        Lookup::Absent => None,
        Lookup::Unreadable => return Fate::Unknown,
    };
    let entity_exists = pawn.is_some();
    match pawn {
        Some(p) if p != 0 => {
            if data == 0 {
                Fate::Dropped(DropPath::NoCookedData)
            } else {
                Fate::Play
            }
        }
        _ => {
            let deferrable = event_id == Some(layout::DEFERRED_EVENT_ID)
                || matches!(ids.view_type, Some(1) | Some(2));
            match (deferrable, entity_exists) {
                (true, true) => Fate::Deferred,
                (_, false) => Fate::Dropped(DropPath::NoSourceEntity),
                (false, true) => Fate::Dropped(DropPath::NoSourcePawn),
            }
        }
    }
}

/// Every request the ready handler with payload `evt`, on the
/// `SequenceManager` at `mgr`, is about to drop, with `now` the current
/// `_time64` (low word). Empty when anything is unreadable.
pub fn plan_ready(mem: &dyn Mem, mgr: u32, evt: u32, now: u32) -> Vec<SequenceDrop> {
    let mut drops = Vec::new();
    let multimap = mgr.wrapping_add(layout::REQUESTS);
    let Some((head, size)) = head_and_size(mem, multimap) else {
        return drops;
    };
    let Some(status) = mem.u32_at(evt.wrapping_add(layout::ready::STATUS)) else {
        return drops;
    };
    let mut ready_key = None;
    let mut removed = 0u32;
    if status == 1 {
        let (Some(key), Some(data)) = (
            mem.u32_at(evt.wrapping_add(layout::ready::KEY)),
            mem.u32_at(evt.wrapping_add(layout::ready::DATA)),
        ) else {
            return drops;
        };
        let key = key as i32;
        ready_key = Some(key);
        let event_id = if data == 0 {
            None
        } else {
            mem.u32_at(data.wrapping_add(layout::DATA_EVENT_ID))
        };
        let Some(reqs) = requests_for(mem, multimap, key) else {
            return drops;
        };
        removed = reqs.len() as u32;
        for req in reqs {
            let ids = read_request(mem, req);
            if let Fate::Dropped(path) = fate(mem, &ids, data, event_id) {
                drops.push(SequenceDrop {
                    path,
                    ids,
                    event_id,
                    age_secs: None,
                });
            }
        }
    }
    // The prune runs after the ready id's requests are erased.
    if size.saturating_sub(removed) > layout::PRUNE_MIN_SIZE {
        let Some(first) = mem.u32_at(head.wrapping_add(node::LEFT)) else {
            return drops;
        };
        let Some(all) = walk(mem, head, first, MAX_PRUNE_SCAN, |_| true) else {
            return drops;
        };
        for (key, req) in all {
            if Some(key) == ready_key {
                continue;
            }
            let ids = read_request(mem, req);
            let Some(created) = ids.created else {
                continue;
            };
            let age = now.wrapping_sub(created);
            // A clock that went backwards reads as a huge age; the game
            // keeps those (its compare is signed 64-bit), so do we.
            if (layout::PRUNE_AGE_SECS..i32::MAX as u32).contains(&age) {
                drops.push(SequenceDrop {
                    path: DropPath::Expired,
                    ids,
                    event_id: None,
                    age_secs: Some(age),
                });
            }
        }
    }
    drops
}

/// The outcome of one play step (`0x00d06dd0`): whether it asked for a
/// Kismet instance (`0x00d067e0`) and, if so, whether it got one.
pub fn play_outcome(instance_requested: bool, instance: u32) -> Option<DropPath> {
    if !instance_requested {
        Some(DropPath::CulledByDistance)
    } else if instance == 0 {
        Some(DropPath::InstanceRefused)
    } else {
        None
    }
}

#[cfg(test)]
mod tests;
