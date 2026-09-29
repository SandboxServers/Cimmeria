//! The catalog of every CME event type the client can create, dumped once.
//!
//! The registry `RegisterAllEventEmitHandlers` (`0x005c75d0`) fills at
//! startup is a `std::map<std::string, factory>` whose object is the
//! singleton at `0x01f11fc4` (getter `0x0155f790`), and whose address is
//! the `this` of every `0x00a5c0f0` call. A node is
//!
//! ```text
//! +0x00 _Left  +0x04 _Parent  +0x08 _Right
//! +0x0c std::string key (28 bytes: proxy, inline/heap buffer, size, cap)
//! +0x28 factory function pointer
//! ```
//!
//! (`0x00a5c0f0` finds the node, then `call [node+0x28]`; the string
//! layout is `msvc_string`'s). The first `client.cme.event` the client
//! raises proves the registry is populated, so that is when a worker thread
//! walks the tree in order (read-only, every read through the checked
//! reader) and emits the names in chunks of [`CHUNK_NAMES`] as
//! `client.cme.catalog`, then one `client.cme.catalog_done` with the total
//! and the count per event family. Names sort as the map sorts them, so two
//! sessions of one build produce identical catalogs, and a diff of two
//! builds' catalogs is a diff of their event sets.

use super::entity_trace::map::Mem;
use crate::msvc_string::{self, Storage, Width};

/// Names per `client.cme.catalog` event.
pub(crate) const CHUNK_NAMES: usize = 40;

/// Longest event name kept.
const MAX_NAME_CHARS: usize = 128;

/// Nodes visited before the walk gives up (the registry holds about a
/// thousand; a cycle must not spin forever).
const MAX_NODES: usize = 20_000;

const KEY_OFFSET: u32 = 0x0c;
const FACTORY_OFFSET: u32 = 0x28;

/// One registered event type.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Entry {
    /// The class name, e.g. `Event_NetIn_onDialogDisplay`.
    pub name: String,
    /// The factory's address.
    pub factory: u32,
}

/// Read the `std::string` object at `addr` through `mem`.
fn read_string(mem: &dyn Mem, addr: u32) -> Option<String> {
    let mut header = [0u8; msvc_string::OBJECT_SIZE];
    for (i, chunk) in header.chunks_mut(4).enumerate() {
        chunk.copy_from_slice(&mem.u32_at(addr.checked_add(i as u32 * 4)?)?.to_le_bytes());
    }
    let (storage, _) = msvc_string::locate(&header, Width::Narrow, MAX_NAME_CHARS);
    let bytes = match storage {
        Storage::Invalid => return None,
        Storage::Inline(b) => b,
        Storage::Heap { ptr, len_bytes } => {
            let mut out = Vec::with_capacity(len_bytes);
            let mut off = 0u32;
            while (off as usize) < len_bytes {
                let word = mem.u32_at(ptr.checked_add(off)?)?.to_le_bytes();
                let take = (len_bytes - off as usize).min(4);
                out.extend_from_slice(&word[..take]);
                off += 4;
            }
            out
        }
    };
    Some(msvc_string::decode_bytes(&bytes, Width::Narrow))
}

/// Every entry of the string-keyed `std::map` object at `map`, in key
/// order. `None` when the tree cannot be read at all; a read that fails
/// part-way keeps what was read.
pub(crate) fn walk(mem: &dyn Mem, map: u32) -> Option<Vec<Entry>> {
    let head = mem.u32_at(map.checked_add(4)?)?;
    if head == 0 {
        return None;
    }
    let root = mem.u32_at(head.checked_add(4)?)?;
    let mut out = Vec::new();
    if root == head {
        return Some(out);
    }
    // In-order from the leftmost node.
    let mut node = root;
    loop {
        let left = mem.u32_at(node)?;
        if left == head {
            break;
        }
        node = left;
    }
    for _ in 0..MAX_NODES {
        if let (Some(name), Some(factory)) = (
            read_string(mem, node.wrapping_add(KEY_OFFSET)),
            mem.u32_at(node.wrapping_add(FACTORY_OFFSET)),
        ) {
            out.push(Entry { name, factory });
        }
        // Successor.
        let Some(right) = mem.u32_at(node.wrapping_add(8)) else {
            break;
        };
        if right != head {
            node = right;
            loop {
                match mem.u32_at(node) {
                    Some(l) if l != head => node = l,
                    _ => break,
                }
            }
            continue;
        }
        loop {
            let Some(parent) = mem.u32_at(node.wrapping_add(4)) else {
                return Some(out);
            };
            if parent == head {
                return Some(out);
            }
            let Some(parent_right) = mem.u32_at(parent.wrapping_add(8)) else {
                return Some(out);
            };
            let came_from_right = parent_right == node;
            node = parent;
            if !came_from_right {
                break;
            }
        }
    }
    Some(out)
}

/// The families the summary counts, by class-name prefix.
pub(crate) fn family_of(name: &str) -> &'static str {
    super::inline_hooks::event_kind(name)
}

/// The fields of the chunk `index` of `names`.
pub(crate) fn chunk_fields(index: usize, names: &[&str]) -> Vec<(&'static str, serde_json::Value)> {
    vec![
        ("chunk", serde_json::json!(index)),
        ("count", serde_json::json!(names.len())),
        ("names", serde_json::json!(names.join(","))),
    ]
}

/// The fields of the closing `client.cme.catalog_done`: the total, the
/// number of chunks, and one `kind_<family>` count per event family.
pub(crate) fn done_fields(entries: &[Entry]) -> Vec<(&'static str, serde_json::Value)> {
    use std::collections::BTreeMap;
    let mut kinds: BTreeMap<&'static str, u64> = BTreeMap::new();
    for e in entries {
        *kinds.entry(family_of(&e.name)).or_default() += 1;
    }
    let mut f = vec![
        ("total", serde_json::json!(entries.len())),
        (
            "chunks",
            serde_json::json!(entries.len().div_ceil(CHUNK_NAMES)),
        ),
    ];
    for (k, n) in kinds {
        let key: &'static str = match k {
            "net_in" => "kind_net_in",
            "net_out" => "kind_net_out",
            "net" => "kind_net",
            "action" => "kind_action",
            "ui" => "kind_ui",
            "slash_cmd" => "kind_slash_cmd",
            "cache" => "kind_cache",
            "other" => "kind_other",
            _ => "kind_unnamed",
        };
        f.push((key, serde_json::json!(n)));
    }
    f
}

#[cfg(all(target_os = "windows", target_arch = "x86"))]
mod native {
    use super::*;
    use std::sync::atomic::{AtomicBool, Ordering};

    static STARTED: AtomicBool = AtomicBool::new(false);

    /// Start the one-time catalog dump for the registry at `registry`.
    /// Later calls do nothing. The walk runs on its own thread so the
    /// thread that raised the first event (the game's) never waits on it.
    pub(crate) fn start_once(registry: usize) {
        if STARTED.swap(true, Ordering::AcqRel) {
            return;
        }
        let _ = std::thread::Builder::new()
            .name("cimmeria-cme-catalog".into())
            .spawn(move || {
                // The registry is full long before the first event, but a
                // walk of a half-built tree would emit a partial catalog.
                std::thread::sleep(std::time::Duration::from_secs(2));
                let _ = std::panic::catch_unwind(|| dump(registry));
            });
    }

    fn dump(registry: usize) {
        use crate::hooks::entity_trace::map::LiveMem;
        let Some(entries) = walk(&LiveMem, registry as u32) else {
            crate::hooks::emit::emit(
                "client.cme.catalog_done",
                "warn",
                vec![("error", serde_json::json!("registry_unreadable"))],
            );
            return;
        };
        for (i, chunk) in entries.chunks(CHUNK_NAMES).enumerate() {
            let names: Vec<&str> = chunk.iter().map(|e| e.name.as_str()).collect();
            crate::hooks::emit::emit("client.cme.catalog", "info", chunk_fields(i, &names));
        }
        crate::hooks::emit::emit("client.cme.catalog_done", "info", done_fields(&entries));
    }
}

#[cfg(all(target_os = "windows", target_arch = "x86"))]
pub(crate) use native::start_once;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hooks::entity_trace::map::fake::FakeMem;

    /// Write a std::string object with an inline name at `addr`.
    fn put_inline(m: &mut FakeMem, addr: u32, s: &str) {
        assert!(s.len() < 16);
        let mut obj = [0u8; 28];
        obj[4..4 + s.len()].copy_from_slice(s.as_bytes());
        obj[0x14..0x18].copy_from_slice(&(s.len() as u32).to_le_bytes());
        obj[0x18..0x1c].copy_from_slice(&15u32.to_le_bytes());
        for (i, c) in obj.chunks(4).enumerate() {
            m.set(
                addr + i as u32 * 4,
                u32::from_le_bytes(c.try_into().unwrap()),
            );
        }
    }

    /// A string longer than the inline buffer lives on the heap.
    fn put_heap(m: &mut FakeMem, addr: u32, heap: u32, s: &str) {
        let mut obj = [0u8; 28];
        obj[4..8].copy_from_slice(&heap.to_le_bytes());
        obj[0x14..0x18].copy_from_slice(&(s.len() as u32).to_le_bytes());
        obj[0x18..0x1c].copy_from_slice(&(s.len() as u32 + 15).to_le_bytes());
        for (i, c) in obj.chunks(4).enumerate() {
            m.set(
                addr + i as u32 * 4,
                u32::from_le_bytes(c.try_into().unwrap()),
            );
        }
        let mut padded = s.as_bytes().to_vec();
        while !padded.len().is_multiple_of(4) {
            padded.push(0);
        }
        for (i, c) in padded.chunks(4).enumerate() {
            m.set(
                heap + i as u32 * 4,
                u32::from_le_bytes(c.try_into().unwrap()),
            );
        }
    }

    fn node(m: &mut FakeMem, at: u32, l: u32, p: u32, r: u32, factory: u32) {
        m.set(at, l);
        m.set(at + 4, p);
        m.set(at + 8, r);
        m.set(at + FACTORY_OFFSET, factory);
    }

    /// A three-node tree, b at the root, a left, c right, so the in-order
    /// walk has to climb from a leaf and go down a right subtree.
    fn tree() -> FakeMem {
        let mut m = FakeMem::default();
        let (map, head) = (0x1000u32, 0x2000u32);
        let (a, b, c) = (0x3000u32, 0x3100u32, 0x3200u32);
        m.set(map + 4, head);
        m.set(head, a); // min
        m.set(head + 4, b); // root
        m.set(head + 8, c); // max
        node(&mut m, b, a, head, c, 0x0b0b);
        node(&mut m, a, head, b, head, 0x0a0a);
        node(&mut m, c, head, b, head, 0x0c0c);
        put_inline(&mut m, b + KEY_OFFSET, "Event_NetIn_b");
        put_inline(&mut m, a + KEY_OFFSET, "Event_Action_a");
        put_heap(
            &mut m,
            c + KEY_OFFSET,
            0x9000,
            "Event_Kismet_SomethingLongerThanSixteen",
        );
        m
    }

    #[test]
    fn an_in_order_walk_visits_every_node_once_in_key_order() {
        let m = tree();
        let entries = walk(&m, 0x1000).unwrap();
        assert_eq!(
            entries,
            vec![
                Entry {
                    name: "Event_Action_a".into(),
                    factory: 0x0a0a
                },
                Entry {
                    name: "Event_NetIn_b".into(),
                    factory: 0x0b0b
                },
                Entry {
                    name: "Event_Kismet_SomethingLongerThanSixteen".into(),
                    factory: 0x0c0c
                },
            ]
        );
    }

    #[test]
    fn an_empty_registry_is_an_empty_catalog_and_garbage_is_none() {
        let mut m = FakeMem::default();
        m.set(0x1004, 0x2000);
        m.set(0x2004, 0x2000); // root == head
        assert_eq!(walk(&m, 0x1000), Some(vec![]));
        assert_eq!(walk(&FakeMem::default(), 0x1000), None);
    }

    /// A cycle in the tree cannot spin the walker forever.
    #[test]
    fn a_cyclic_tree_terminates() {
        let mut m = tree();
        // c's right child points back at the root.
        m.set(0x3200 + 8, 0x3100);
        m.set(0x3100 + 4, 0x3200);
        let entries = walk(&m, 0x1000).unwrap();
        assert!(entries.len() <= MAX_NODES);
    }

    #[test]
    fn chunks_and_summary() {
        let names: Vec<Entry> = (0..95)
            .map(|i| Entry {
                name: if i % 2 == 0 {
                    format!("Event_NetIn_{i}")
                } else {
                    format!("Event_Action_{i}")
                },
                factory: i,
            })
            .collect();
        let d = done_fields(&names);
        let get = |k: &str| d.iter().find(|(n, _)| *n == k).map(|(_, v)| v.clone());
        assert_eq!(get("total"), Some(serde_json::json!(95)));
        assert_eq!(get("chunks"), Some(serde_json::json!(3)));
        assert_eq!(get("kind_net_in"), Some(serde_json::json!(48)));
        assert_eq!(get("kind_action"), Some(serde_json::json!(47)));
        let f = chunk_fields(2, &["a", "b"]);
        assert_eq!(f[2].1, serde_json::json!("a,b"));
        assert_eq!(f[1].1, serde_json::json!(2));
    }
}
