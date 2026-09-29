//! Entity lifecycle on the client: `enterAoI`, `createEntity`,
//! `enterWorld`, the appearance request, `leaveAoI` and destroy.
//!
//! Anchors (all in `docs/reverse-engineering/findings/client-entity-lifecycle.md`,
//! read from the QA `SGW.exe` on 2026-09-28; `ret N` gives the argument count):
//!
//! | Function | Address | Signature |
//! |---|---|---|
//! | `EntityManager::onEntityEnter` (`enterAoI`) | `0x00dd24f0` | `thiscall(this, id, space, vehicle)`, `ret 0xc` |
//! | `EntityManager::onEntityCreate` | `0x00dd2270` | `thiscall(this, id, type, space, vehicle, stream)`, `ret 0x14` |
//! | `EntityManager::enterWorld` | `0x00dd1d00` | `thiscall(this, entity, space, vehicle, flag)`, `ret 0x10` |
//! | `EntityManager::onEntityLeave` (`leaveAoI`) | `0x00dd2800` | `thiscall(this, id, cache_stamp)`, `ret 8` |
//! | entity destroy | `0x00dd1120` | `thiscall(this, entity, arg)`, `ret 8` |
//! | `GameEntity` appearance request | `0x00e69150` | `thiscall(entity, const std::string* reason)`, `ret 4` |
//! | appearance job scheduler | `0x00e998e0` | `thiscall(this, entity)`, `ret 4` |
//!
//! All run on the Mercury network thread. The detours read the manager's
//! maps through the checked reader before and after the original (see
//! [`entity_trace::map`]) and emit one `client.entity.*` event per call,
//! throttled per (event, entity) so an entity that never spoke before
//! always gets through.
//!
//! **What the events answer.** The client only renders an entity that
//! entered the world. `client.entity.create` says whether the create
//! entered it (`outcome = entered_world`) or parked it in the cache map
//! (`outcome = parked`); `client.entity.enter` says whether an `enterAoI`
//! for a parked entity entered it; `client.entity.appearance_request` says
//! whether the appearance job was scheduled; and `queued_msgs` on every
//! event counts the methods and properties the client is holding for an
//! entity that is not in the world.

use std::ffi::c_void;
use std::sync::OnceLock;

use serde_json::json;

use crate::hooks::emit::emit;
use crate::hooks::entity_trace::{
    self as trace,
    map::{self, LiveMem, Mem, Snapshot},
    AppearanceScope, OpScope,
};
use crate::queue::Producer;

pub(super) const ADDR_ENTER_AOI: usize = 0x00dd24f0;
pub(super) const ADDR_CREATE_ENTITY: usize = 0x00dd2270;
pub(super) const ADDR_ENTER_WORLD: usize = 0x00dd1d00;
pub(super) const ADDR_LEAVE_AOI: usize = 0x00dd2800;
pub(super) const ADDR_DESTROY_ENTITY: usize = 0x00dd1120;
pub(super) const ADDR_APPEARANCE_REQUEST: usize = 0x00e69150;
pub(super) const ADDR_APPEARANCE_SCHEDULE: usize = 0x00e998e0;

static ENTER_AOI_TRAMPOLINE: OnceLock<usize> = OnceLock::new();
static CREATE_ENTITY_TRAMPOLINE: OnceLock<usize> = OnceLock::new();
static ENTER_WORLD_TRAMPOLINE: OnceLock<usize> = OnceLock::new();
static LEAVE_AOI_TRAMPOLINE: OnceLock<usize> = OnceLock::new();
static DESTROY_ENTITY_TRAMPOLINE: OnceLock<usize> = OnceLock::new();
static APPEARANCE_REQUEST_TRAMPOLINE: OnceLock<usize> = OnceLock::new();
static APPEARANCE_SCHEDULE_TRAMPOLINE: OnceLock<usize> = OnceLock::new();

/// Longest appearance `reason` label read.
const MAX_REASON_CHARS: usize = 64;

pub(super) unsafe fn install_all(producer: &Producer) {
    let hooks: [(&str, usize, *mut c_void, &OnceLock<usize>); 7] = [
        (
            "entity_enter_aoi",
            ADDR_ENTER_AOI,
            enter_aoi_detour as *mut c_void,
            &ENTER_AOI_TRAMPOLINE,
        ),
        (
            "entity_create",
            ADDR_CREATE_ENTITY,
            create_entity_detour as *mut c_void,
            &CREATE_ENTITY_TRAMPOLINE,
        ),
        (
            "entity_enter_world",
            ADDR_ENTER_WORLD,
            enter_world_detour as *mut c_void,
            &ENTER_WORLD_TRAMPOLINE,
        ),
        (
            "entity_leave_aoi",
            ADDR_LEAVE_AOI,
            leave_aoi_detour as *mut c_void,
            &LEAVE_AOI_TRAMPOLINE,
        ),
        (
            "entity_destroy",
            ADDR_DESTROY_ENTITY,
            destroy_entity_detour as *mut c_void,
            &DESTROY_ENTITY_TRAMPOLINE,
        ),
        (
            "entity_appearance_request",
            ADDR_APPEARANCE_REQUEST,
            appearance_request_detour as *mut c_void,
            &APPEARANCE_REQUEST_TRAMPOLINE,
        ),
        (
            "entity_appearance_schedule",
            ADDR_APPEARANCE_SCHEDULE,
            appearance_schedule_detour as *mut c_void,
            &APPEARANCE_SCHEDULE_TRAMPOLINE,
        ),
    ];
    for (name, addr, detour, slot) in hooks {
        super::install_one(producer, name, addr, detour, slot);
    }
}

/// Snapshot of entity `id` in the manager at `mgr`.
pub(super) fn snap(mgr: *mut c_void, id: i32) -> Snapshot {
    map::snapshot(&LiveMem, mgr as u32, id)
}

/// Run `f`, swallowing a panic so telemetry can never unwind into the game.
pub(super) fn guarded<R>(f: impl FnOnce() -> R) -> Option<R> {
    std::panic::catch_unwind(std::panic::AssertUnwindSafe(f)).ok()
}

/// Emit `fields` under `target` when the per-(event, entity) throttle
/// lets it through.
pub(super) fn emit_throttled(
    target: &'static str,
    level: &'static str,
    entity: i32,
    fields: trace::Fields,
) {
    if let Some(f) = trace::with_suppressed(fields, trace::throttle(target, entity)) {
        emit(target, level, f);
    }
}

// ---------------------------------------------------------------------
// enterAoI

type EnterAoiFn = unsafe extern "thiscall-unwind" fn(*mut c_void, i32, u32, u32);

/// `enterAoI(id, space, vehicle)`: the server says the entity is in range.
#[allow(improper_ctypes_definitions)]
unsafe extern "thiscall-unwind" fn enter_aoi_detour(
    this: *mut c_void,
    id: i32,
    space: u32,
    vehicle: u32,
) {
    let Some(&t) = ENTER_AOI_TRAMPOLINE.get() else {
        return;
    };
    let original: EnterAoiFn = unsafe { std::mem::transmute(t) };
    let op = OpScope::begin("enter");
    let before = guarded(|| snap(this, id));
    original(this, id, space, vehicle);
    guarded(|| {
        if let Some(before) = before {
            let after = snap(this, id);
            emit_throttled(
                "client.entity.enter",
                "info",
                id,
                trace::enter_fields(id, space, vehicle, &before, &after, op.entered_world()),
            );
        }
    });
}

// ---------------------------------------------------------------------
// createEntity

type CreateEntityFn =
    unsafe extern "thiscall-unwind" fn(*mut c_void, i32, u32, u32, u32, *mut c_void);

/// `createEntity(id, type, space, vehicle, stream)`: the server sends the
/// entity's state. The stream is read for its length before the original
/// consumes it.
#[allow(improper_ctypes_definitions)]
unsafe extern "thiscall-unwind" fn create_entity_detour(
    this: *mut c_void,
    id: i32,
    type_id: u32,
    space: u32,
    vehicle: u32,
    stream: *mut c_void,
) {
    let Some(&t) = CREATE_ENTITY_TRAMPOLINE.get() else {
        return;
    };
    let original: CreateEntityFn = unsafe { std::mem::transmute(t) };
    let op = OpScope::begin("create");
    let before = guarded(|| (snap(this, id), stream_remaining(stream)));
    original(this, id, type_id, space, vehicle, stream);
    guarded(|| {
        if let Some((before, payload_len)) = before {
            let after = snap(this, id);
            emit_throttled(
                "client.entity.create",
                "info",
                id,
                trace::create_fields(
                    id,
                    type_id,
                    space,
                    vehicle,
                    payload_len,
                    &before,
                    &after,
                    op.entered_world(),
                ),
            );
        }
    });
}

/// `BinaryIStream::remainingLength()` (vtable slot 2): the bytes left in
/// `stream`. The vtable and slot are validated against the image first.
pub(super) fn stream_remaining(stream: *mut c_void) -> Option<u32> {
    let vtable = LiveMem.u32_at(stream as u32)?;
    let slot = LiveMem.u32_at(vtable.checked_add(8)?)?;
    if !(0x0040_0000..0x0200_0000).contains(&slot) {
        return None;
    }
    // SAFETY: `slot` is a code address inside the client image, read from
    // the stream's own vtable; the game calls the same slot with no
    // arguments in `onEntityMethod` (`0x00dd2b80`).
    let remaining: unsafe extern "thiscall" fn(*mut c_void) -> u32 =
        unsafe { std::mem::transmute(slot as usize) };
    Some(unsafe { remaining(stream) })
}

// ---------------------------------------------------------------------
// enterWorld

type EnterWorldFn = unsafe extern "thiscall-unwind" fn(*mut c_void, *mut c_void, u32, u32, u32);

/// `enterWorld(entity, space, vehicle, flag)`: the entity becomes live and
/// its appearance is requested (inside, through `0x00e69150`).
#[allow(improper_ctypes_definitions)]
unsafe extern "thiscall-unwind" fn enter_world_detour(
    this: *mut c_void,
    entity: *mut c_void,
    space: u32,
    vehicle: u32,
    flag: u32,
) {
    let Some(&t) = ENTER_WORLD_TRAMPOLINE.get() else {
        return;
    };
    let original: EnterWorldFn = unsafe { std::mem::transmute(t) };
    guarded(|| {
        let via = trace::note_entered_world();
        let (id, type_id) = entity_ids(entity);
        if let Some(id) = id {
            emit_throttled(
                "client.entity.entered_world",
                "info",
                id,
                vec![
                    ("entity_id", json!(id)),
                    ("type_id", json!(type_id)),
                    ("space_id", json!(space)),
                    ("vehicle_id", json!(vehicle)),
                    ("via", json!(via)),
                ],
            );
        }
    });
    original(this, entity, space, vehicle, flag);
}

/// `(entity id, type id)` of an `Entity*`, `None` if it is unreadable.
pub(super) fn entity_ids(entity: *mut c_void) -> (Option<i32>, Option<u16>) {
    let base = entity as u32;
    let id = LiveMem
        .u32_at(base.wrapping_add(map::entity::ID))
        .map(|v| v as i32);
    let ty = LiveMem
        .u32_at(base.wrapping_add(map::entity::TYPE_ID))
        .map(|v| (v & 0xffff) as u16);
    (id, ty)
}

// ---------------------------------------------------------------------
// leaveAoI and destroy

type LeaveAoiFn = unsafe extern "thiscall-unwind" fn(*mut c_void, i32, u32);

/// `leaveAoI(id, cache_stamp)`: the server says the entity left range.
#[allow(improper_ctypes_definitions)]
unsafe extern "thiscall-unwind" fn leave_aoi_detour(this: *mut c_void, id: i32, stamp: u32) {
    let Some(&t) = LEAVE_AOI_TRAMPOLINE.get() else {
        return;
    };
    let original: LeaveAoiFn = unsafe { std::mem::transmute(t) };
    let op = OpScope::begin("leave");
    let before = guarded(|| snap(this, id));
    original(this, id, stamp);
    guarded(|| {
        if let Some(before) = before {
            let after = snap(this, id);
            emit_throttled(
                "client.entity.leave",
                "info",
                id,
                trace::leave_fields(id, stamp, &before, &after, op.destroyed()),
            );
        }
    });
}

type DestroyFn = unsafe extern "thiscall-unwind" fn(*mut c_void, *mut c_void, u32);

/// Entity destroy: removes it from both maps and deletes it. Reported
/// before the original, while the entity is still readable.
#[allow(improper_ctypes_definitions)]
unsafe extern "thiscall-unwind" fn destroy_entity_detour(
    this: *mut c_void,
    entity: *mut c_void,
    arg: u32,
) {
    let Some(&t) = DESTROY_ENTITY_TRAMPOLINE.get() else {
        return;
    };
    let original: DestroyFn = unsafe { std::mem::transmute(t) };
    guarded(|| {
        trace::note_destroyed();
        let (id, type_id) = entity_ids(entity);
        if let Some(id) = id {
            let enter_count = LiveMem
                .u32_at((entity as u32).wrapping_add(map::entity::ENTER_COUNT))
                .map(|v| v as i32);
            emit_throttled(
                "client.entity.destroyed",
                "info",
                id,
                vec![
                    ("entity_id", json!(id)),
                    ("type_id", json!(type_id)),
                    ("enter_count", json!(enter_count)),
                ],
            );
        }
    });
    original(this, entity, arg);
}

// ---------------------------------------------------------------------
// Appearance

type AppearanceRequestFn = unsafe extern "thiscall-unwind" fn(*mut c_void, *const c_void);

/// `GameEntity` appearance request. Called from `enterWorld`, and from
/// `setAppearance`, `setTint`, `setBodySetName`, `setStaticMeshName`,
/// `setFlags` and the client-component add/remove: each names itself in
/// the `reason` string. Reported after the original, once it is known
/// whether the job was scheduled.
#[allow(improper_ctypes_definitions)]
unsafe extern "thiscall-unwind" fn appearance_request_detour(
    entity: *mut c_void,
    reason: *const c_void,
) {
    let Some(&t) = APPEARANCE_REQUEST_TRAMPOLINE.get() else {
        return;
    };
    let original: AppearanceRequestFn = unsafe { std::mem::transmute(t) };
    let scope = AppearanceScope::begin();
    original(entity, reason);
    guarded(|| {
        let (id, type_id) = entity_ids(entity);
        let Some(id) = id else {
            return;
        };
        let hold = LiveMem
            .u32_at((entity as u32).wrapping_add(0x30))
            .map(|w| ((w >> 16) & 0xff) as u8);
        let outcome = trace::appearance_outcome(scope.scheduled(), hold);
        let reason = crate::msvc_string::read_checked(
            reason as usize,
            crate::msvc_string::Width::Narrow,
            MAX_REASON_CHARS,
        )
        .map(|d| d.text);
        // The per-(event, entity) bucket is keyed by the outcome too, so a
        // stream of `scheduled` cannot hide the first `not_ready`.
        let target = "client.entity.appearance_request";
        let key = match outcome {
            "scheduled" => "client.entity.appearance_request.scheduled",
            _ => "client.entity.appearance_request.other",
        };
        let fields = vec![
            ("entity_id", json!(id)),
            ("type_id", json!(type_id)),
            ("outcome", json!(outcome)),
            ("reason", json!(reason)),
            ("hold_byte", json!(hold)),
        ];
        if let Some(f) = trace::with_suppressed(fields, trace::throttle(key, id)) {
            emit(target, "info", f);
        }
    });
}

type AppearanceScheduleFn = unsafe extern "thiscall-unwind" fn(*mut c_void, *mut c_void);

/// The appearance job scheduler: only marks that the request reached it.
#[allow(improper_ctypes_definitions)]
unsafe extern "thiscall-unwind" fn appearance_schedule_detour(
    this: *mut c_void,
    entity: *mut c_void,
) {
    trace::note_scheduled();
    let Some(&t) = APPEARANCE_SCHEDULE_TRAMPOLINE.get() else {
        return;
    };
    let original: AppearanceScheduleFn = unsafe { std::mem::transmute(t) };
    original(this, entity);
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU32, Ordering};

    static ARGS: [AtomicU32; 6] = [const { AtomicU32::new(0) }; 6];

    /// The tests share `ARGS`; run one at a time.
    static SERIAL: std::sync::Mutex<()> = std::sync::Mutex::new(());

    fn set(i: usize, v: u32) {
        ARGS[i].store(v, Ordering::SeqCst);
    }

    fn get(i: usize) -> u32 {
        ARGS[i].load(Ordering::SeqCst)
    }

    /// Every lifecycle detour hands all of its arguments to the original,
    /// even with a manager pointer that is not readable (the checked reader
    /// reports `None`; nothing faults), and none reports an event without a
    /// producer.
    #[test]
    fn lifecycle_detours_forward_every_argument() {
        let _serial = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
        unsafe extern "thiscall-unwind" fn enter(this: *mut c_void, id: i32, sp: u32, ve: u32) {
            set(0, this as u32);
            set(1, id as u32);
            set(2, sp);
            set(3, ve);
        }
        unsafe extern "thiscall-unwind" fn create(
            this: *mut c_void,
            id: i32,
            ty: u32,
            sp: u32,
            ve: u32,
            stream: *mut c_void,
        ) {
            set(0, this as u32);
            set(1, id as u32);
            set(2, ty);
            set(3, sp);
            set(4, ve);
            set(5, stream as u32);
        }
        unsafe extern "thiscall-unwind" fn leave(this: *mut c_void, id: i32, stamp: u32) {
            set(0, this as u32);
            set(1, id as u32);
            set(2, stamp);
        }
        ENTER_AOI_TRAMPOLINE
            .set(enter as *const () as usize)
            .unwrap();
        CREATE_ENTITY_TRAMPOLINE
            .set(create as *const () as usize)
            .unwrap();
        LEAVE_AOI_TRAMPOLINE
            .set(leave as *const () as usize)
            .unwrap();

        let mgr = 0x1000 as *mut c_void;
        unsafe { enter_aoi_detour(mgr, 11, 2, 3) };
        assert_eq!((get(0), get(1), get(2), get(3)), (0x1000, 11, 2, 3));

        unsafe { create_entity_detour(mgr, 12, 0x1a, 2, 3, std::ptr::null_mut()) };
        assert_eq!(
            (get(0), get(1), get(2), get(3), get(4), get(5)),
            (0x1000, 12, 0x1a, 2, 3, 0)
        );

        unsafe { leave_aoi_detour(mgr, 13, 77) };
        assert_eq!((get(0), get(1), get(2)), (0x1000, 13, 77));
    }

    /// `enterWorld` and destroy forward their arguments and read the entity
    /// through the checked reader, and the appearance request reports the
    /// scheduler having run inside it.
    #[test]
    fn world_destroy_and_appearance_detours_forward() {
        let _serial = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
        unsafe extern "thiscall-unwind" fn enter_world(
            this: *mut c_void,
            e: *mut c_void,
            sp: u32,
            ve: u32,
            flag: u32,
        ) {
            set(0, this as u32);
            set(1, e as u32);
            set(2, sp);
            set(3, ve);
            set(4, flag);
        }
        unsafe extern "thiscall-unwind" fn destroy(this: *mut c_void, e: *mut c_void, a: u32) {
            set(0, this as u32);
            set(1, e as u32);
            set(2, a);
        }
        unsafe extern "thiscall-unwind" fn appearance(e: *mut c_void, reason: *const c_void) {
            set(0, e as u32);
            set(1, reason as u32);
            // The real function reaches the scheduler on the ready path.
            trace::note_scheduled();
        }
        unsafe extern "thiscall-unwind" fn schedule(this: *mut c_void, e: *mut c_void) {
            set(0, this as u32);
            set(1, e as u32);
        }
        ENTER_WORLD_TRAMPOLINE
            .set(enter_world as *const () as usize)
            .unwrap();
        DESTROY_ENTITY_TRAMPOLINE
            .set(destroy as *const () as usize)
            .unwrap();
        APPEARANCE_REQUEST_TRAMPOLINE
            .set(appearance as *const () as usize)
            .unwrap();
        APPEARANCE_SCHEDULE_TRAMPOLINE
            .set(schedule as *const () as usize)
            .unwrap();

        // A readable `Entity`: id 9, enter count 1, type 7.
        let mut entity = [0u32; 16];
        entity[3] = 9;
        entity[4] = 1;
        entity[5] = 7;
        let e = entity.as_mut_ptr() as *mut c_void;
        let mgr = 0x1000 as *mut c_void;

        unsafe { enter_world_detour(mgr, e, 2, 3, 4) };
        assert_eq!(
            (get(0), get(1), get(2), get(3), get(4)),
            (0x1000, e as u32, 2, 3, 4)
        );
        assert_eq!(entity_ids(e), (Some(9), Some(7)));

        unsafe { destroy_entity_detour(mgr, e, 5) };
        assert_eq!((get(0), get(1), get(2)), (0x1000, e as u32, 5));

        unsafe { appearance_request_detour(e, 0x2000 as *const c_void) };
        assert_eq!((get(0), get(1)), (e as u32, 0x2000));

        unsafe { appearance_schedule_detour(mgr, e) };
        assert_eq!((get(0), get(1)), (0x1000, e as u32));
    }
}
