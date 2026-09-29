//! Read-only view of the client `EntityManager`'s entity maps.
//!
//! `EntityManager` (the `GameEntityManager` singleton at `0x01ef244c`)
//! keeps four `std::map`s, all keyed by entity id, in the VS2005/2008
//! layout (`docs/reverse-engineering/findings/client-entity-lifecycle.md`):
//!
//! | Offset | Holds | Node value |
//! |---|---|---|
//! | `+0x18` | entities **in the world** | `Entity*` at node `+0x10` |
//! | `+0x24` | entities created but **not entered** (the "cache") | `Entity*` at node `+0x10` |
//! | `+0x30` | pending enter records (`enterAoI` before `createEntity`) | record whose first word is the enter count |
//! | `+0x3c` | methods/properties queued for an entity that is not in the world | a vector of 8-byte entries at node `+0x14..+0x18` |
//!
//! An MSVC `std::map` object is `{ proxy, _Myhead, _Mysize }`. The head
//! node is a sentinel whose `_Parent` (`+4`) is the root; every real leaf
//! points back at the head instead of null. A node is `{ _Left, _Parent,
//! _Right, key, value... }`, so the key is at `+0xc` and the value at
//! `+0x10`. The lookup here is the standard lower-bound walk; it never
//! writes, and every read goes through [`Mem`], which the DLL backs with
//! `ReadProcessMemory` so a stale pointer reads as `None` instead of
//! faulting inside the game's network thread.

/// Byte reads of the client process. The DLL backs it with a checked
/// reader ([`LiveMem`]); tests back it with a fake address space.
pub trait Mem {
    /// The little-endian `u32` at `addr`, or `None` if unreadable.
    fn u32_at(&self, addr: u32) -> Option<u32>;

    /// `len` bytes at `addr`, or `None` if any part is unreadable. The
    /// default reads word by word; [`LiveMem`] overrides it with one
    /// `ReadProcessMemory` call, which the per-packet Mercury reader needs.
    fn bytes_at(&self, addr: u32, len: usize) -> Option<Vec<u8>> {
        let mut out = Vec::with_capacity(len);
        let mut at = addr;
        while out.len() < len {
            let word = self.u32_at(at)?;
            let take = (len - out.len()).min(4);
            out.extend_from_slice(&word.to_le_bytes()[..take]);
            at = at.wrapping_add(4);
        }
        Some(out)
    }
}

/// The live client address space, read through `ReadProcessMemory`.
#[cfg(all(target_os = "windows", target_arch = "x86"))]
pub struct LiveMem;

#[cfg(all(target_os = "windows", target_arch = "x86"))]
impl Mem for LiveMem {
    fn u32_at(&self, addr: u32) -> Option<u32> {
        let bytes = cimmeria_client_hookgate::os::read_bytes(addr as usize, 4)?;
        Some(u32::from_le_bytes(bytes.try_into().ok()?))
    }

    fn bytes_at(&self, addr: u32, len: usize) -> Option<Vec<u8>> {
        cimmeria_client_hookgate::os::read_bytes(addr as usize, len)
    }
}

/// `EntityManager` field offsets (`this`, i.e. the singleton).
pub mod manager {
    /// Local player `Entity*`.
    pub const LOCAL_PLAYER_ENTITY: u32 = 0x0c;
    /// Local player entity id.
    pub const LOCAL_PLAYER_ID: u32 = 0x14;
    /// Map of entities in the world.
    pub const WORLD_MAP: u32 = 0x18;
    /// Map of created-but-not-entered ("cached") entities.
    pub const CACHE_MAP: u32 = 0x24;
    /// Map of pending enter records.
    pub const PENDING_MAP: u32 = 0x30;
    /// Map of deferred method/property queues.
    pub const QUEUE_MAP: u32 = 0x3c;
}

/// `Entity` (BigWorld `Entity`, the `GameEntity` base) field offsets.
pub mod entity {
    /// Entity id.
    pub const ID: u32 = 0x0c;
    /// Enter count (AoI refcount).
    pub const ENTER_COUNT: u32 = 0x10;
    /// Entity type id (16 bits).
    pub const TYPE_ID: u32 = 0x14;
    /// Client flags (bit 0 `CEF_Remote`, bit 1 set at the end of `enterAoI`).
    pub const FLAGS: u32 = 0x18;
}

/// Node field offsets of a `std::map<int, T>`.
mod node {
    pub const LEFT: u32 = 0x00;
    pub const PARENT: u32 = 0x04;
    pub const RIGHT: u32 = 0x08;
    pub const KEY: u32 = 0x0c;
    pub const VALUE: u32 = 0x10;
}

/// A red-black tree of `n` nodes is at most about `2 log2(n + 1)` deep. The
/// walk gives up after this many steps, so a corrupt tree cannot loop.
const MAX_DEPTH: usize = 96;

/// Result of looking one key up in a map.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Lookup {
    /// The key is present; the node's address.
    Found(u32),
    /// The map is readable and has no such key.
    Absent,
    /// A read failed or the tree is not plausible.
    Unreadable,
}

/// Find `key` in the `std::map<int, _>` object at `map`.
pub fn find(mem: &dyn Mem, map: u32, key: i32) -> Lookup {
    let Some(head) = mem.u32_at(map.wrapping_add(4)) else {
        return Lookup::Unreadable;
    };
    if head == 0 {
        return Lookup::Unreadable;
    }
    let Some(mut cur) = mem.u32_at(head.wrapping_add(node::PARENT)) else {
        return Lookup::Unreadable;
    };
    let mut best = head;
    let mut steps = 0;
    while cur != head {
        steps += 1;
        if steps > MAX_DEPTH || cur == 0 {
            return Lookup::Unreadable;
        }
        let Some(k) = mem.u32_at(cur.wrapping_add(node::KEY)) else {
            return Lookup::Unreadable;
        };
        let next = if (k as i32) < key {
            mem.u32_at(cur.wrapping_add(node::RIGHT))
        } else {
            best = cur;
            mem.u32_at(cur.wrapping_add(node::LEFT))
        };
        match next {
            Some(n) => cur = n,
            None => return Lookup::Unreadable,
        }
    }
    if best == head {
        return Lookup::Absent;
    }
    match mem.u32_at(best.wrapping_add(node::KEY)) {
        Some(k) if k as i32 == key => Lookup::Found(best),
        Some(_) => Lookup::Absent,
        None => Lookup::Unreadable,
    }
}

/// One entity as the maps hold it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EntityInfo {
    /// `Entity*`.
    pub ptr: u32,
    /// Enter count (`+0x10`).
    pub enter_count: i32,
    /// Client flags (`+0x18`).
    pub flags: u32,
    /// Entity type id (`+0x14`, 16 bits).
    pub type_id: u16,
}

/// Where the manager holds an entity at one instant.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Snapshot {
    /// In the world map.
    pub world: Option<EntityInfo>,
    /// In the cache map (created, not entered).
    pub cache: Option<EntityInfo>,
    /// The pending enter count, if an `enterAoI` arrived before its create.
    pub pending_enter_count: Option<i32>,
    /// Methods and properties queued for it (the deferred-message queue).
    pub queued_msgs: Option<u32>,
    /// Whether it is the local player's id.
    pub is_local_player: bool,
    /// `false` when any read failed: the other fields are then a lower bound.
    pub complete: bool,
}

/// Where an entity is, as one word for a query.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Where {
    /// In the world map.
    World,
    /// In the cache map: created, never entered.
    Cache,
    /// Only a pending enter record exists (no entity yet).
    Pending,
    /// The manager knows nothing of it.
    None,
}

impl Where {
    /// Stable field value.
    pub fn as_str(self) -> &'static str {
        match self {
            Where::World => "world",
            Where::Cache => "cache",
            Where::Pending => "pending",
            Where::None => "none",
        }
    }
}

impl Snapshot {
    /// Where the entity is, world first.
    pub fn place(&self) -> Where {
        if self.world.is_some() {
            Where::World
        } else if self.cache.is_some() {
            Where::Cache
        } else if self.pending_enter_count.is_some() {
            Where::Pending
        } else {
            Where::None
        }
    }

    /// The entity record wherever it is held.
    pub fn entity(&self) -> Option<EntityInfo> {
        self.world.or(self.cache)
    }
}

fn entity_at(mem: &dyn Mem, node_addr: u32) -> Option<EntityInfo> {
    let ptr = mem.u32_at(node_addr.wrapping_add(node::VALUE))?;
    if ptr == 0 {
        return None;
    }
    Some(EntityInfo {
        ptr,
        enter_count: mem.u32_at(ptr.wrapping_add(entity::ENTER_COUNT))? as i32,
        flags: mem.u32_at(ptr.wrapping_add(entity::FLAGS))?,
        type_id: (mem.u32_at(ptr.wrapping_add(entity::TYPE_ID))? & 0xffff) as u16,
    })
}

/// Snapshot entity `id` in the manager at `mgr`.
pub fn snapshot(mem: &dyn Mem, mgr: u32, id: i32) -> Snapshot {
    let mut snap = Snapshot {
        complete: true,
        ..Snapshot::default()
    };
    let mut note = |l: Lookup| match l {
        Lookup::Unreadable => {
            snap.complete = false;
            None
        }
        Lookup::Absent => None,
        Lookup::Found(n) => Some(n),
    };
    let world = note(find(mem, mgr.wrapping_add(manager::WORLD_MAP), id));
    let cache = note(find(mem, mgr.wrapping_add(manager::CACHE_MAP), id));
    let pending = note(find(mem, mgr.wrapping_add(manager::PENDING_MAP), id));
    let queue = note(find(mem, mgr.wrapping_add(manager::QUEUE_MAP), id));
    if let Some(n) = world {
        snap.world = entity_at(mem, n);
        snap.complete &= snap.world.is_some();
    }
    if let Some(n) = cache {
        snap.cache = entity_at(mem, n);
        snap.complete &= snap.cache.is_some();
    }
    if let Some(n) = pending {
        // The record's first word is the enter count.
        snap.pending_enter_count = mem.u32_at(n.wrapping_add(node::VALUE)).map(|c| c as i32);
        snap.complete &= snap.pending_enter_count.is_some();
    }
    if let Some(n) = queue {
        let begin = mem.u32_at(n.wrapping_add(0x14));
        let end = mem.u32_at(n.wrapping_add(0x18));
        snap.queued_msgs = match (begin, end) {
            (Some(b), Some(e)) if e >= b => Some((e - b) / 8),
            _ => {
                snap.complete = false;
                None
            }
        };
    }
    snap.is_local_player =
        mem.u32_at(mgr.wrapping_add(manager::LOCAL_PLAYER_ID)) == Some(id as u32);
    snap
}

#[cfg(test)]
pub(crate) mod fake {
    //! A tiny fake address space and `std::map` builder for the tests.

    use super::*;
    use std::collections::HashMap;

    /// Sparse little-endian memory.
    #[derive(Default)]
    pub struct FakeMem {
        pub words: HashMap<u32, u32>,
    }

    impl FakeMem {
        pub fn set(&mut self, addr: u32, v: u32) {
            self.words.insert(addr, v);
        }
    }

    impl Mem for FakeMem {
        fn u32_at(&self, addr: u32) -> Option<u32> {
            self.words.get(&addr).copied()
        }
    }

    /// Lay out a `std::map<int, u32-ish>` at `map` from `(key, value_word)`
    /// pairs, as a degenerate right-leaning tree (valid for lower-bound).
    /// Nodes are 0x40 bytes apart starting at `nodes`; the head is at
    /// `nodes - 0x40`. Returns each node's address, in key order.
    pub fn build_map(mem: &mut FakeMem, map: u32, nodes: u32, entries: &[(i32, u32)]) -> Vec<u32> {
        let head = nodes - 0x40;
        mem.set(map, 0xDEAD_0001); // proxy
        mem.set(map + 4, head);
        mem.set(map + 8, entries.len() as u32);
        let addrs: Vec<u32> = (0..entries.len() as u32)
            .map(|i| nodes + i * 0x40)
            .collect();
        for (i, (k, v)) in entries.iter().enumerate() {
            let a = addrs[i];
            mem.set(a, head); // left
            mem.set(a + 4, if i == 0 { head } else { addrs[i - 1] }); // parent
            mem.set(
                a + 8,
                if i + 1 < addrs.len() {
                    addrs[i + 1]
                } else {
                    head
                },
            );
            mem.set(a + 0xc, *k as u32);
            mem.set(a + 0x10, *v);
        }
        // Head: _Left = min, _Parent = root, _Right = max.
        mem.set(head, addrs.first().copied().unwrap_or(head));
        mem.set(head + 4, addrs.first().copied().unwrap_or(head));
        mem.set(head + 8, addrs.last().copied().unwrap_or(head));
        addrs
    }
}

#[cfg(test)]
mod tests {
    use super::fake::*;
    use super::*;

    #[test]
    fn an_empty_map_has_nothing() {
        let mut m = FakeMem::default();
        build_map(&mut m, 0x1000, 0x2000, &[]);
        assert_eq!(find(&m, 0x1000, 5), Lookup::Absent);
    }

    #[test]
    fn a_present_key_is_found_and_a_missing_one_is_not() {
        let mut m = FakeMem::default();
        let nodes = build_map(&mut m, 0x1000, 0x2000, &[(3, 0), (7, 0), (11, 0)]);
        assert_eq!(find(&m, 0x1000, 3), Lookup::Found(nodes[0]));
        assert_eq!(find(&m, 0x1000, 7), Lookup::Found(nodes[1]));
        assert_eq!(find(&m, 0x1000, 11), Lookup::Found(nodes[2]));
        assert_eq!(find(&m, 0x1000, 8), Lookup::Absent);
        assert_eq!(find(&m, 0x1000, 99), Lookup::Absent);
        assert_eq!(find(&m, 0x1000, -1), Lookup::Absent);
    }

    /// Keys compare as signed ints, as the client's map does.
    #[test]
    fn negative_keys_sort_before_positive_ones() {
        let mut m = FakeMem::default();
        let nodes = build_map(&mut m, 0x1000, 0x2000, &[(-5, 0), (2, 0)]);
        assert_eq!(find(&m, 0x1000, -5), Lookup::Found(nodes[0]));
        assert_eq!(find(&m, 0x1000, 2), Lookup::Found(nodes[1]));
    }

    #[test]
    fn unreadable_memory_is_reported_not_guessed() {
        let m = FakeMem::default();
        assert_eq!(find(&m, 0x1000, 1), Lookup::Unreadable);
    }

    /// A tree whose node points at itself must not loop forever.
    #[test]
    fn a_cyclic_tree_gives_up() {
        let mut m = FakeMem::default();
        let nodes = build_map(&mut m, 0x1000, 0x2000, &[(1, 0)]);
        m.set(nodes[0] + 8, nodes[0]); // right -> itself
        m.set(nodes[0] + 0xc, 0); // key 0 < 1: walk right forever
        assert_eq!(find(&m, 0x1000, 1), Lookup::Unreadable);
    }

    fn manager(m: &mut FakeMem, mgr: u32) {
        // Four empty maps at the real offsets, each with its own head.
        for (i, off) in [
            manager::WORLD_MAP,
            manager::CACHE_MAP,
            manager::PENDING_MAP,
            manager::QUEUE_MAP,
        ]
        .into_iter()
        .enumerate()
        {
            build_map(m, mgr + off, 0x0100_0000 + i as u32 * 0x1_0000, &[]);
        }
        m.set(mgr + manager::LOCAL_PLAYER_ID, 1);
    }

    fn entity_block(m: &mut FakeMem, ptr: u32, enter: i32, type_id: u16, flags: u32) {
        m.set(ptr + entity::ENTER_COUNT, enter as u32);
        m.set(ptr + entity::TYPE_ID, u32::from(type_id) | 0xABCD_0000);
        m.set(ptr + entity::FLAGS, flags);
    }

    /// A cached entity that never entered: the state the Frost bug looks
    /// like. `place` is `cache`, the enter count is what the create left.
    #[test]
    fn a_created_but_not_entered_entity_is_in_the_cache_map() {
        let mut m = FakeMem::default();
        let mgr = 0x4000;
        manager(&mut m, mgr);
        entity_block(&mut m, 0x9000, 0, 0x1a, 0);
        build_map(
            &mut m,
            mgr + manager::CACHE_MAP,
            0x0110_0000,
            &[(4242, 0x9000)],
        );
        let s = snapshot(&m, mgr, 4242);
        assert_eq!(s.place(), Where::Cache);
        assert_eq!(
            s.entity(),
            Some(EntityInfo {
                ptr: 0x9000,
                enter_count: 0,
                flags: 0,
                type_id: 0x1a
            })
        );
        assert!(s.complete);
        assert!(!s.is_local_player);
    }

    #[test]
    fn world_wins_and_queue_and_pending_are_read() {
        let mut m = FakeMem::default();
        let mgr = 0x4000;
        manager(&mut m, mgr);
        entity_block(&mut m, 0x9000, 1, 7, 2);
        build_map(
            &mut m,
            mgr + manager::WORLD_MAP,
            0x0120_0000,
            &[(1, 0x9000)],
        );
        build_map(&mut m, mgr + manager::PENDING_MAP, 0x0130_0000, &[(1, 3)]);
        let q = build_map(&mut m, mgr + manager::QUEUE_MAP, 0x0140_0000, &[(1, 0)]);
        m.set(q[0] + 0x14, 0x5000);
        m.set(q[0] + 0x18, 0x5000 + 8 * 6);
        let s = snapshot(&m, mgr, 1);
        assert_eq!(s.place(), Where::World);
        assert_eq!(s.pending_enter_count, Some(3));
        assert_eq!(s.queued_msgs, Some(6));
        assert!(s.is_local_player);
        assert!(s.complete);
    }

    #[test]
    fn an_unknown_id_is_none_and_a_pending_only_id_is_pending() {
        let mut m = FakeMem::default();
        let mgr = 0x4000;
        manager(&mut m, mgr);
        assert_eq!(snapshot(&m, mgr, 55).place(), Where::None);
        build_map(&mut m, mgr + manager::PENDING_MAP, 0x0130_0000, &[(55, 1)]);
        assert_eq!(snapshot(&m, mgr, 55).place(), Where::Pending);
    }

    #[test]
    fn a_torn_read_marks_the_snapshot_incomplete() {
        let m = FakeMem::default();
        let s = snapshot(&m, 0x4000, 1);
        assert!(!s.complete);
        assert_eq!(s.place(), Where::None);
    }
}
