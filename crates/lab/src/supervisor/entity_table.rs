//! `client_entity_table` — walk the client's BigWorld entity maps.
//!
//! Layout (Ghidra RE, verified live 2026-09-29 on this client build):
//!
//! - `GameEntityManager*` singleton pointer at VA `0x01EF244C`; player
//!   entity id at `+0x14`.
//! - Three MSVC `std::map<int, T>` members: entities (`Entity*`) at
//!   `+0x18`, limbo (`Entity*`) at `+0x24`, pending enter counts (`int`)
//!   at `+0x30`. Each map is `{ allocator/comp, _Myhead, _Mysize }`, so the
//!   head node is at `map + 4` and the size at `map + 8`.
//! - Red-black tree node: left `+0`, parent `+4`, right `+8`, key `+0xC`,
//!   value `+0x10`, color `+0x14` (byte), isnil `+0x15` (byte). The head
//!   node's parent is the root; leaves point at the head (isnil = 1).
//! - `Entity`: vtable `+0`, actor/appearance pointer `+0x08` (null ⇒ never
//!   rendered), id `+0xC`, enter count `+0x10`, flags `+0x30` (bit `0x100`
//!   set once rendered). `isReady()` is vtable slot 1 (thiscall, bool in
//!   AL; `0x00dff830` for the base class on this build).
//!
//! Reads are coarse on purpose: one read per tree node (0x18 bytes) and
//! one per entity (0x34 bytes), never a read per field. The walk once ran
//! hundreds of tiny reads that starved the watchdog's heartbeat and got a
//! healthy client killed.

use std::collections::HashSet;

use serde_json::{json, Value};

use super::Supervisor;

/// Unslid VA of the `GameEntityManager*` singleton pointer.
pub const MANAGER_PTR_VA: u32 = 0x01EF_244C;
pub const PLAYER_ID_OFF: u32 = 0x14;
pub const ENTITIES_MAP_OFF: u32 = 0x18;
pub const LIMBO_MAP_OFF: u32 = 0x24;
pub const PENDING_MAP_OFF: u32 = 0x30;
/// Bytes of the manager read in one go: player id through the third map.
const MANAGER_SPAN: u32 = PENDING_MAP_OFF + 0xC;
/// Bytes of a tree node read in one go.
pub const NODE_LEN: u32 = 0x18;
/// Bytes of an entity read in one go (through the flags word at +0x30).
pub const ENTITY_LEN: u32 = 0x34;
/// Entity flag set once the entity has been rendered.
pub const RENDERED_FLAG: u32 = 0x100;
/// Hard cap on nodes visited per map (a map changing under the walk, or a
/// bad pointer, must not loop forever).
pub const DEFAULT_MAX_NODES: usize = 4096;

/// Client memory as the walker sees it.
pub trait Memory {
    async fn read(&mut self, addr: u32, len: u32) -> Result<Vec<u8>, String>;
}

fn u32_at(b: &[u8], off: usize) -> u32 {
    b.get(off..off + 4)
        .map(|s| u32::from_le_bytes([s[0], s[1], s[2], s[3]]))
        .unwrap_or(0)
}

/// One decoded tree node.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Node {
    pub left: u32,
    pub parent: u32,
    pub right: u32,
    pub key: u32,
    pub value: u32,
    pub is_nil: bool,
}

impl Node {
    pub fn decode(b: &[u8]) -> Result<Self, String> {
        if b.len() < 0x16 {
            return Err(format!("short node read ({} bytes)", b.len()));
        }
        Ok(Self {
            left: u32_at(b, 0),
            parent: u32_at(b, 4),
            right: u32_at(b, 8),
            key: u32_at(b, 0xC),
            value: u32_at(b, 0x10),
            is_nil: b[0x15] != 0,
        })
    }
}

async fn read_node<M: Memory>(mem: &mut M, addr: u32) -> Result<Node, String> {
    Node::decode(&mem.read(addr, NODE_LEN).await?).map_err(|e| format!("node {addr:#x}: {e}"))
}

/// A walked map: its recorded size and `(key, value)` pairs in key order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MapWalk {
    pub size: u32,
    pub entries: Vec<(u32, u32)>,
    pub truncated: bool,
}

/// Extra node reads allowed beyond the entry cap: the left chain an
/// in-order walk reads ahead of its first entry. An MSVC red-black tree
/// of 2^32 nodes is at most 64 deep, so a sane map never hits this; a
/// stale or corrupt one (a long acyclic chain) stops here instead of
/// issuing unbounded `mem_read`s behind the heartbeat.
const MAX_TREE_DEPTH: usize = 64;

/// In-order walk of an MSVC `std::map` given its head node and size.
/// Emits at most `max_nodes` entries and reads at most `max_nodes` +
/// [`MAX_TREE_DEPTH`] nodes.
pub async fn walk_tree<M: Memory>(
    mem: &mut M,
    head: u32,
    size: u32,
    max_nodes: usize,
) -> Result<MapWalk, String> {
    if head == 0 {
        return Err("map head is null".into());
    }
    let root = read_node(mem, head).await?.parent;
    let mut entries = Vec::new();
    let mut seen = HashSet::new();
    let mut stack: Vec<Node> = Vec::new();
    let mut cur = root;
    let limit = max_nodes.min(size as usize);
    let read_budget = limit.saturating_add(MAX_TREE_DEPTH);
    let mut truncated = false;
    'walk: loop {
        while cur != head && cur != 0 {
            if seen.len() >= read_budget {
                truncated = true;
                break 'walk;
            }
            if !seen.insert(cur) {
                return Err(format!("tree cycle at node {cur:#x} (map changing?)"));
            }
            let n = read_node(mem, cur).await?;
            if n.is_nil {
                break;
            }
            stack.push(n);
            cur = n.left;
        }
        let Some(n) = stack.pop() else { break };
        if entries.len() >= limit {
            truncated = entries.len() < size as usize;
            break 'walk;
        }
        entries.push((n.key, n.value));
        cur = n.right;
    }
    Ok(MapWalk {
        size,
        entries,
        truncated,
    })
}

/// One entity's fields, from a single 0x34-byte read.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EntityFields {
    pub vtable: u32,
    pub actor: u32,
    pub id: u32,
    pub enter_count: u32,
    pub flags: u32,
}

impl EntityFields {
    pub fn decode(b: &[u8]) -> Result<Self, String> {
        if b.len() < ENTITY_LEN as usize {
            return Err(format!("short entity read ({} bytes)", b.len()));
        }
        Ok(Self {
            vtable: u32_at(b, 0),
            actor: u32_at(b, 8),
            id: u32_at(b, 0xC),
            enter_count: u32_at(b, 0x10),
            flags: u32_at(b, 0x30),
        })
    }

    pub fn to_json(self, key: u32, ptr: u32) -> Value {
        json!({
            "id": key,
            "ptr": format!("{ptr:#x}"),
            "vtable": format!("{:#x}", self.vtable),
            "id_field_matches": self.id == key,
            "enter_count": self.enter_count as i32,
            "actor": format!("{:#x}", self.actor),
            "rendered": self.actor != 0,
            "rendered_flag": self.flags & RENDERED_FLAG != 0,
            "flags": format!("{:#x}", self.flags),
        })
    }
}

/// The manager header: player id and the three maps' `(head, size)`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ManagerHeader {
    pub player_id: u32,
    pub entities: (u32, u32),
    pub limbo: (u32, u32),
    pub pending: (u32, u32),
}

impl ManagerHeader {
    /// Decode from a read that starts at `manager + PLAYER_ID_OFF`.
    pub fn decode(b: &[u8]) -> Result<Self, String> {
        let base = PLAYER_ID_OFF as usize;
        if b.len() < (MANAGER_SPAN as usize - base) {
            return Err(format!("short manager read ({} bytes)", b.len()));
        }
        let map = |off: u32| {
            let o = off as usize - base;
            (u32_at(b, o + 4), u32_at(b, o + 8))
        };
        Ok(Self {
            player_id: u32_at(b, 0),
            entities: map(ENTITIES_MAP_OFF),
            limbo: map(LIMBO_MAP_OFF),
            pending: map(PENDING_MAP_OFF),
        })
    }
}

/// Everything the walker reads, without `isReady` (a native call).
#[derive(Debug, Clone)]
pub struct EntityTable {
    pub manager: u32,
    pub header: ManagerHeader,
    pub entities: MapWalk,
    /// `(key, ptr, fields)` for each entity kept by the id filter.
    pub details: Vec<(u32, u32, Result<EntityFields, String>)>,
    pub limbo: MapWalk,
    pub pending: MapWalk,
}

/// Read the manager and walk all three maps. `ids` filters the per-entity
/// detail reads (the id lists are always complete).
pub async fn read_table<M: Memory>(
    mem: &mut M,
    slide: i64,
    ids: Option<&[u32]>,
    max_nodes: usize,
) -> Result<EntityTable, String> {
    let ptr_va = (MANAGER_PTR_VA as i64 + slide) as u32;
    let manager = u32_at(&mem.read(ptr_va, 4).await?, 0);
    if manager == 0 {
        return Err(format!(
            "GameEntityManager pointer at {ptr_va:#x} is null (not in the world yet?)"
        ));
    }
    let header = ManagerHeader::decode(
        &mem.read(manager + PLAYER_ID_OFF, MANAGER_SPAN - PLAYER_ID_OFF)
            .await?,
    )?;
    let entities = walk_tree(mem, header.entities.0, header.entities.1, max_nodes).await?;
    let limbo = walk_tree(mem, header.limbo.0, header.limbo.1, max_nodes).await?;
    let pending = walk_tree(mem, header.pending.0, header.pending.1, max_nodes).await?;
    let mut details = Vec::new();
    for &(key, ptr) in &entities.entries {
        if ids.is_some_and(|f| !f.contains(&key)) {
            continue;
        }
        let fields = if ptr == 0 {
            Err("null entity pointer".to_string())
        } else {
            match mem.read(ptr, ENTITY_LEN).await {
                Ok(b) => EntityFields::decode(&b),
                Err(e) => Err(e),
            }
        };
        details.push((key, ptr, fields));
    }
    Ok(EntityTable {
        manager,
        header,
        entities,
        details,
        limbo,
        pending,
    })
}

/// Reads over the bridge. Straight to the bridge client, not through the
/// command journal: reads are fault-guarded and never quarantined, and a
/// walk would flush the 64-entry journal the crash report depends on.
struct BridgeMemory<'a>(&'a Supervisor);

impl Memory for BridgeMemory<'_> {
    async fn read(&mut self, addr: u32, len: u32) -> Result<Vec<u8>, String> {
        let v = self
            .0
            .bridge
            .call(
                "mem_read",
                json!({ "addr": format!("{addr:#x}"), "len": len }),
            )
            .await
            .map_err(|e| format!("mem_read {addr:#x}+{len}: {e}"))?;
        let hex = v
            .get("hex")
            .and_then(Value::as_str)
            .ok_or_else(|| format!("mem_read {addr:#x}: no hex in {v}"))?;
        decode_hex(hex)
    }
}

pub fn decode_hex(hex: &str) -> Result<Vec<u8>, String> {
    if !hex.len().is_multiple_of(2) {
        return Err("odd-length hex".into());
    }
    (0..hex.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&hex[i..i + 2], 16).map_err(|e| format!("hex: {e}")))
        .collect()
}

impl Supervisor {
    /// `client_entity_table`.
    pub async fn entity_table(
        &self,
        ids: Option<Vec<u32>>,
        is_ready: bool,
        max_nodes: Option<usize>,
    ) -> Result<Value, String> {
        let info = self
            .bridge
            .call("module_info", json!({}))
            .await
            .map_err(|e| format!("module_info: {e}"))?;
        let slide = info.get("slide").and_then(Value::as_i64).unwrap_or(0);
        let mut mem = BridgeMemory(self);
        let t = read_table(
            &mut mem,
            slide,
            ids.as_deref(),
            max_nodes.unwrap_or(DEFAULT_MAX_NODES),
        )
        .await?;

        let mut rows = Vec::with_capacity(t.details.len());
        for (key, ptr, fields) in &t.details {
            let mut row = match fields {
                Ok(f) => f.to_json(*key, *ptr),
                Err(e) => json!({ "id": key, "ptr": format!("{ptr:#x}"), "error": e }),
            };
            if let (true, Ok(f)) = (is_ready, fields) {
                row["is_ready"] = self.call_is_ready(*ptr, f.vtable).await;
            }
            rows.push(row);
        }
        let in_limbo: Vec<u32> = t.limbo.entries.iter().map(|e| e.0).collect();
        let pending: Vec<Value> = t
            .pending
            .entries
            .iter()
            .map(|(k, v)| json!({ "id": k, "count": *v as i32 }))
            .collect();
        Ok(json!({
            "manager": format!("{:#x}", t.manager),
            "slide": slide,
            "player_id": t.header.player_id,
            "entities": {
                "size": t.entities.size,
                "truncated": t.entities.truncated,
                "ids": t.entities.entries.iter().map(|e| e.0).collect::<Vec<_>>(),
                "details": rows,
            },
            "limbo": { "size": t.limbo.size, "ids": in_limbo },
            "pending_enter_counts": { "size": t.pending.size, "entries": pending },
        }))
    }

    /// Call the entity's `isReady()` (vtable slot 1) through the journaled
    /// native-call path. The slot is read from the entity's own vtable so a
    /// subclass override is honoured.
    async fn call_is_ready(&self, entity: u32, vtable: u32) -> Value {
        let mut mem = BridgeMemory(self);
        let func = match mem.read(vtable.wrapping_add(4), 4).await {
            Ok(b) => u32_at(&b, 0),
            Err(e) => return json!({ "error": e }),
        };
        match self
            .bridge_call(
                "call_native",
                json!({
                    "addr": format!("{func:#x}"),
                    "conv": "thiscall",
                    "args": [format!("{entity:#x}")],
                    "ret": "u32",
                }),
            )
            .await
        {
            Ok(v) => match v.get("ret_u32").and_then(Value::as_u64) {
                // bool in AL: the upper bytes of EAX are garbage.
                Some(r) => json!(r & 0xFF != 0),
                None => {
                    json!({ "error": format!("no ret_u32 in {v}"), "fn": format!("{func:#x}") })
                }
            },
            Err(e) => json!({ "error": e, "fn": format!("{func:#x}") }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    /// A sparse fake of client memory, counting reads.
    #[derive(Default)]
    struct FakeMem {
        bytes: HashMap<u32, u8>,
        reads: Vec<(u32, u32)>,
    }

    impl FakeMem {
        fn put_u32(&mut self, addr: u32, v: u32) {
            for (i, b) in v.to_le_bytes().iter().enumerate() {
                self.bytes.insert(addr + i as u32, *b);
            }
        }
        fn put_u8(&mut self, addr: u32, v: u8) {
            self.bytes.insert(addr, v);
        }
        fn node(&mut self, at: u32, left: u32, parent: u32, right: u32, key: u32, value: u32) {
            self.put_u32(at, left);
            self.put_u32(at + 4, parent);
            self.put_u32(at + 8, right);
            self.put_u32(at + 0xC, key);
            self.put_u32(at + 0x10, value);
            self.put_u8(at + 0x15, 0);
        }
        fn head(&mut self, at: u32, root: u32) {
            self.put_u32(at + 4, root);
            self.put_u8(at + 0x15, 1);
        }
    }

    impl Memory for FakeMem {
        async fn read(&mut self, addr: u32, len: u32) -> Result<Vec<u8>, String> {
            self.reads.push((addr, len));
            Ok((0..len)
                .map(|i| *self.bytes.get(&(addr + i)).unwrap_or(&0))
                .collect())
        }
    }

    /// head 0x1000; tree:     20
    ///                      /    \
    ///                    10      30
    fn three_node_tree(m: &mut FakeMem) {
        let head = 0x1000;
        m.head(head, 0x2000);
        m.node(0x2000, 0x2100, head, 0x2200, 20, 0xA20);
        m.node(0x2100, head, 0x2000, head, 10, 0xA10);
        m.node(0x2200, head, 0x2000, head, 30, 0xA30);
    }

    #[tokio::test]
    async fn walk_is_in_key_order_with_one_read_per_node() {
        let mut m = FakeMem::default();
        three_node_tree(&mut m);
        let w = walk_tree(&mut m, 0x1000, 3, 100).await.unwrap();
        assert_eq!(w.entries, vec![(10, 0xA10), (20, 0xA20), (30, 0xA30)]);
        assert!(!w.truncated);
        // Head + three nodes, each read once, each a whole node.
        assert_eq!(m.reads.len(), 4);
        assert!(m.reads.iter().all(|&(_, len)| len == NODE_LEN));
    }

    #[tokio::test]
    async fn empty_map_walks_to_nothing() {
        let mut m = FakeMem::default();
        m.head(0x1000, 0x1000);
        let w = walk_tree(&mut m, 0x1000, 0, 100).await.unwrap();
        assert!(w.entries.is_empty());
    }

    #[tokio::test]
    async fn walk_stops_at_the_node_cap() {
        let mut m = FakeMem::default();
        three_node_tree(&mut m);
        let w = walk_tree(&mut m, 0x1000, 3, 2).await.unwrap();
        assert_eq!(w.entries.len(), 2);
        assert!(w.truncated);
    }

    /// A corrupt map whose left chain runs far past any real tree depth
    /// stops at the read budget instead of reading the whole chain.
    #[tokio::test]
    async fn a_long_left_chain_stops_at_the_read_budget() {
        let mut m = FakeMem::default();
        let head = 0x1000;
        let chain = 1_000u32;
        m.head(head, 0x10_0000);
        for i in 0..chain {
            let at = 0x10_0000 + i * 0x20;
            let left = if i + 1 == chain { head } else { at + 0x20 };
            m.node(at, left, head, head, chain - i, i);
        }
        let w = walk_tree(&mut m, head, chain, 5).await.unwrap();
        assert!(w.truncated);
        assert!(w.entries.is_empty());
        // Head + the budget, not the 1000-node chain.
        assert_eq!(m.reads.len(), 1 + 5 + MAX_TREE_DEPTH);
    }

    /// A corrupt tree (a child pointing back at an ancestor) is an error,
    /// not an endless walk.
    #[tokio::test]
    async fn a_cycle_is_reported() {
        let mut m = FakeMem::default();
        m.head(0x1000, 0x2000);
        m.node(0x2000, 0x2100, 0x1000, 0x1000, 20, 1);
        m.node(0x2100, 0x2000, 0x2000, 0x1000, 10, 2);
        let e = walk_tree(&mut m, 0x1000, 5, 100).await.unwrap_err();
        assert!(e.contains("cycle"));
    }

    #[test]
    fn entity_fields_decode_from_one_block() {
        let mut b = vec![0u8; ENTITY_LEN as usize];
        b[0..4].copy_from_slice(&0x0180_0000u32.to_le_bytes());
        b[8..12].copy_from_slice(&0x0DEA_D000u32.to_le_bytes());
        b[0xC..0x10].copy_from_slice(&77u32.to_le_bytes());
        b[0x10..0x14].copy_from_slice(&1u32.to_le_bytes());
        b[0x30..0x34].copy_from_slice(&0x0000_0104u32.to_le_bytes());
        let f = EntityFields::decode(&b).unwrap();
        let j = f.to_json(77, 0x5000);
        assert_eq!(j["enter_count"], 1);
        assert_eq!(j["rendered"], true);
        assert_eq!(j["rendered_flag"], true);
        assert_eq!(j["id_field_matches"], true);
        assert!(EntityFields::decode(&b[..0x20]).is_err());
    }

    /// Never-rendered entity: null actor pointer, flag clear.
    #[test]
    fn a_null_actor_reads_as_not_rendered() {
        let b = vec![0u8; ENTITY_LEN as usize];
        let j = EntityFields::decode(&b).unwrap().to_json(5, 0x10);
        assert_eq!(j["rendered"], false);
        assert_eq!(j["rendered_flag"], false);
        assert_eq!(j["id_field_matches"], false);
    }

    #[tokio::test]
    async fn full_table_reads_the_manager_maps_and_filtered_entities() {
        let mut m = FakeMem::default();
        let slide = 0x10000i64;
        let mgr = 0x9000;
        m.put_u32((MANAGER_PTR_VA as i64 + slide) as u32, mgr);
        m.put_u32(mgr + PLAYER_ID_OFF, 20);
        // Entities map: head 0x1000, size 3.
        three_node_tree(&mut m);
        m.put_u32(mgr + ENTITIES_MAP_OFF + 4, 0x1000);
        m.put_u32(mgr + ENTITIES_MAP_OFF + 8, 3);
        // Limbo and pending: empty maps with their own heads.
        m.head(0x3000, 0x3000);
        m.put_u32(mgr + LIMBO_MAP_OFF + 4, 0x3000);
        m.head(0x3100, 0x3100);
        m.put_u32(mgr + PENDING_MAP_OFF + 4, 0x3100);
        // Entity 20 at 0xA20: rendered.
        m.put_u32(0xA20 + 8, 0x7777);
        m.put_u32(0xA20 + 0xC, 20);

        let t = read_table(&mut m, slide, Some(&[20]), 100).await.unwrap();
        assert_eq!(t.header.player_id, 20);
        assert_eq!(t.entities.entries.len(), 3);
        assert_eq!(t.details.len(), 1);
        let (key, ptr, fields) = &t.details[0];
        assert_eq!((*key, *ptr), (20, 0xA20));
        assert_eq!(fields.as_ref().unwrap().actor, 0x7777);
        assert!(t.limbo.entries.is_empty() && t.pending.entries.is_empty());
        // Coarse reads only: nothing smaller than a pointer.
        assert!(m.reads.iter().all(|&(_, len)| len >= 4));
    }

    #[tokio::test]
    async fn a_null_manager_is_a_clear_error() {
        let mut m = FakeMem::default();
        let e = read_table(&mut m, 0, None, 10).await.unwrap_err();
        assert!(e.contains("null"));
    }

    #[test]
    fn hex_decodes() {
        assert_eq!(decode_hex("00ff10").unwrap(), vec![0, 0xff, 0x10]);
        assert!(decode_hex("abc").is_err());
    }
}
