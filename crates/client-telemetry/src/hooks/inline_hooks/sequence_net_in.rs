//! The first `onSequence` drop: `SequenceManager::onSequence`
//! (`0x00d05790`), the `Event_NetIn_onSequence` handler (AB-C5).
//!
//! `thiscall(this, event, subject)`, `ret 8` (disassembled 2026-10-04; the
//! anchors finding gave it one argument). It reads `SourceID` with
//! `GetInt`, looks the entity up with `0x00dd0de0(manager, id, 0)`, which
//! with a 0 flag searches only the world map at `manager+0x18`, and at
//! `0x00d0585c` returns without a word when there is none. Only after that
//! does it file a request (`0x00d13780`), which the later drop paths
//! (`sequence_manager`) work from.
//!
//! The detour reads `SourceID` through the same getter and walks the same
//! map (read-only, through `ReadProcessMemory`) before the original runs.
//! When the entity is absent it reads the other ids and, after the
//! original, reports `client.sequence.dropped` with `path =
//! no_source_entity` and `stage = net_in`, through the same per-(path,
//! Source) throttle as the other drop paths.

use std::ffi::c_void;
use std::sync::OnceLock;

use super::entity_lifecycle::guarded;
use crate::hooks::ability_trace::event_bag::{Bag, LiveBag};
use crate::hooks::entity_trace::{
    map::{self, LiveMem, Lookup, Mem},
    sequences::{layout, DropPath, RequestIds, SequenceDrop},
};
use crate::queue::Producer;

pub(super) const ADDR_ON_SEQUENCE: usize = 0x00d0_5790;

static ON_SEQUENCE_TRAMPOLINE: OnceLock<usize> = OnceLock::new();

pub(super) unsafe fn install_all(producer: &Producer) {
    unsafe {
        super::install_one(
            producer,
            "sequence_net_in",
            ADDR_ON_SEQUENCE,
            on_sequence_detour as *mut c_void,
            &ON_SEQUENCE_TRAMPOLINE,
        )
    };
}

/// Whether the world map has `id`, as `0x00dd0de0(mgr, id, 0)` decides.
/// `None` when the manager or the map cannot be read.
fn in_world(mem: &dyn Mem, id: i32) -> Option<bool> {
    let mgr = mem
        .u32_at(layout::ENTITY_MANAGER_SINGLETON)
        .filter(|&m| m != 0)?;
    match map::find(mem, mgr.wrapping_add(map::manager::WORLD_MAP), id) {
        Lookup::Found(_) => Some(true),
        Lookup::Absent => Some(false),
        _ => None,
    }
}

/// The drop the handler is about to make, if any.
fn plan(bag: &dyn Bag, mem: &dyn Mem) -> Option<SequenceDrop> {
    let source = bag.int(b"SourceID\0")?;
    if in_world(mem, source)? {
        return None;
    }
    Some(SequenceDrop {
        path: DropPath::NoSourceEntity,
        ids: RequestIds {
            sequence_id: bag.int(b"KismetEventSetSeqID\0"),
            source_id: Some(source),
            target_id: bag.int(b"TargetID\0"),
            view_type: bag.byte(b"ViewType\0").map(|b| i32::from(b as i8)),
            instance_id: bag.int(b"InstanceId\0").map(|v| v as u32),
            created: None,
        },
        event_id: None,
        age_secs: None,
    })
}

type HandlerFn = unsafe extern "thiscall-unwind" fn(*mut c_void, *mut c_void, *mut c_void);

/// `Event_NetIn_onSequence` handler `(event, subject)`.
#[allow(improper_ctypes_definitions)]
unsafe extern "thiscall-unwind" fn on_sequence_detour(
    this: *mut c_void,
    event: *mut c_void,
    subject: *mut c_void,
) {
    let Some(&t) = ON_SEQUENCE_TRAMPOLINE.get() else {
        return;
    };
    let original: HandlerFn = unsafe { std::mem::transmute(t) };
    let drop = guarded(|| plan(&LiveBag(event), &LiveMem)).flatten();
    unsafe { original(this, event, subject) };
    if let Some(d) = drop {
        guarded(|| super::sequence_manager::report(&d, "net_in"));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hooks::ability_trace::event_bag::fake::FakeBag;
    use crate::hooks::entity_trace::map::fake::{build_map, FakeMem};
    use std::sync::atomic::{AtomicUsize, Ordering};

    const MGR: u32 = 0x5000;

    fn world(ids: &[i32]) -> FakeMem {
        let mut m = FakeMem::default();
        m.set(layout::ENTITY_MANAGER_SINGLETON, MGR);
        let entries: Vec<(i32, u32)> = ids.iter().map(|&i| (i, 0x9000)).collect();
        build_map(&mut m, MGR + map::manager::WORLD_MAP, 0x6000, &entries);
        m
    }

    fn bag(source: i32) -> FakeBag {
        let mut b = FakeBag::default();
        b.ints.insert("SourceID", source);
        b.ints.insert("KismetEventSetSeqID", 4711);
        b.ints.insert("TargetID", 88);
        b.ints.insert("InstanceId", 31337);
        b.bytes.insert("ViewType", 3);
        b
    }

    /// No world-map entity for the Source: the handler's silent return at
    /// `0x00d0585c`, with the ids and the cast id.
    #[test]
    fn a_source_with_no_entity_is_a_drop_with_its_ids() {
        let d = plan(&bag(77), &world(&[5, 9])).expect("dropped");
        assert_eq!(d.path, DropPath::NoSourceEntity);
        assert_eq!(d.ids.sequence_id, Some(4711));
        assert_eq!(d.ids.view_type, Some(3));
        let f = d.fields("net_in");
        let get = |k: &str| f.iter().find(|(n, _)| *n == k).map(|(_, v)| v.clone());
        assert_eq!(get("stage"), Some(serde_json::json!("net_in")));
        assert_eq!(get("cast_id"), Some(serde_json::json!(31337)));
        assert_eq!(get("entity_id"), Some(serde_json::json!(77)));
    }

    #[test]
    fn a_source_in_the_world_is_not_a_drop() {
        assert!(plan(&bag(9), &world(&[5, 9])).is_none());
    }

    /// An unreadable manager or a bag without `SourceID` claims nothing.
    #[test]
    fn unknowns_claim_no_drop() {
        assert!(plan(&bag(77), &FakeMem::default()).is_none());
        assert!(plan(&FakeBag::default(), &world(&[5])).is_none());
    }

    /// The handler pops two stack words (`ret 8`); both reach the original.
    #[test]
    fn the_detour_forwards_both_stack_arguments() {
        static SEEN: [AtomicUsize; 3] = [const { AtomicUsize::new(0) }; 3];
        unsafe extern "thiscall-unwind" fn original(
            this: *mut c_void,
            event: *mut c_void,
            subject: *mut c_void,
        ) {
            for (s, v) in SEEN
                .iter()
                .zip([this as usize, event as usize, subject as usize])
            {
                s.store(v, Ordering::SeqCst);
            }
        }
        let _ = ON_SEQUENCE_TRAMPOLINE.set(original as *const () as usize);
        unsafe {
            on_sequence_detour(
                0x10 as *mut c_void,
                std::ptr::null_mut(),
                0x30 as *mut c_void,
            )
        };
        let seen: Vec<usize> = SEEN.iter().map(|s| s.load(Ordering::SeqCst)).collect();
        assert_eq!(seen, [0x10, 0, 0x30]);
    }
}
