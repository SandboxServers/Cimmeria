//! Inbound entity methods and properties, and the deferred-message queue.
//!
//! | Function | Address | Signature |
//! |---|---|---|
//! | `EntityManager::onEntityMethod` | `0x00dd2b80` | `thiscall(this, id, msg_id, stream)`, `ret 0xc` |
//! | `EntityManager::onEntityProperty` | `0x00dd29d0` | `thiscall(this, id, msg_id, stream)`, `ret 0xc` |
//! | queued-message replay | `0x00dd1e40` | `thiscall(this, entity) -> bool`, `ret 4` |
//!
//! `onEntityMethod` finds the entity in the manager's world map and calls
//! `Client_NetIn_EntityMethodDispatch` (`0x00c6f8f0`); for the local player
//! it dispatches even when the map has no node; for any other entity it
//! **queues** the message in the deferred map at `manager+0x3c` instead.
//! `onEntityProperty` ignores the message for a known entity (it only asks
//! the stream for its length) and queues it for an unknown one. The
//! replay (`0x00dd1e40`) is the other, and only, caller of the dispatcher:
//! it flushes an entity's queue once the entity enters the world. So
//! hooking these two entry points plus the replay tags every dispatch,
//! and the dispatcher itself, which `cimmeria-client-patches` hooks, is
//! left untouched.
//!
//! The `Ctx` set around the original names the entity (and message id)
//! for events created inside the dispatch: `client.cme.event` for an
//! `Event_NetIn_*` and `client.dispatch.method_dropped`.
//!
//! Events: `client.mercury.entity_method` and `client.mercury.entity_property`
//! (`debug` when delivered, `info` when queued) and
//! `client.entity.queue_replay` (`info`).

use std::ffi::c_void;
use std::sync::OnceLock;

use serde_json::json;

use super::entity_lifecycle::{emit_throttled, entity_ids, guarded, snap, stream_remaining};
use crate::hooks::emit::emit;
use crate::hooks::entity_trace::{
    self as trace,
    map::{self, LiveMem, Lookup, Mem},
    Ctx, CtxScope, OpScope,
};
use crate::hooks::name_throttle::Decision;
use crate::queue::Producer;

pub(super) const ADDR_ENTITY_METHOD: usize = 0x00dd2b80;
pub(super) const ADDR_ENTITY_PROPERTY: usize = 0x00dd29d0;
pub(super) const ADDR_QUEUE_REPLAY: usize = 0x00dd1e40;

static ENTITY_METHOD_TRAMPOLINE: OnceLock<usize> = OnceLock::new();
static ENTITY_PROPERTY_TRAMPOLINE: OnceLock<usize> = OnceLock::new();
static QUEUE_REPLAY_TRAMPOLINE: OnceLock<usize> = OnceLock::new();

pub(super) unsafe fn install_all(producer: &Producer) {
    let hooks: [(&str, usize, *mut c_void, &OnceLock<usize>); 3] = [
        (
            "entity_method",
            ADDR_ENTITY_METHOD,
            entity_method_detour as *mut c_void,
            &ENTITY_METHOD_TRAMPOLINE,
        ),
        (
            "entity_property",
            ADDR_ENTITY_PROPERTY,
            entity_property_detour as *mut c_void,
            &ENTITY_PROPERTY_TRAMPOLINE,
        ),
        (
            "entity_queue_replay",
            ADDR_QUEUE_REPLAY,
            queue_replay_detour as *mut c_void,
            &QUEUE_REPLAY_TRAMPOLINE,
        ),
    ];
    for (name, addr, detour, slot) in hooks {
        super::install_one(producer, name, addr, detour, slot);
    }
}

/// What `onEntityMethod` / `onEntityProperty` will do with a message, worked
/// out the way the game does, before the original runs.
struct Plan {
    path: &'static str,
    type_id: Option<u16>,
    len: Option<u32>,
}

/// Decide the delivery path of a message for entity `id`. `is_method` is
/// `true` for `onEntityMethod`, which dispatches for the local player even
/// when the world map has no node for it; `onEntityProperty` ignores a
/// known entity's message and has no player exception.
fn plan(mgr: *mut c_void, id: i32, stream: *mut c_void, is_method: bool) -> Plan {
    let base = mgr as u32;
    let world = map::find(&LiveMem, base.wrapping_add(map::manager::WORLD_MAP), id);
    let mut type_id = None;
    let in_world = match world {
        Lookup::Found(node) => {
            // The entity pointer is at node+0x10.
            if let Some(e) = LiveMem.u32_at(node.wrapping_add(0x10)) {
                type_id = LiveMem
                    .u32_at(e.wrapping_add(map::entity::TYPE_ID))
                    .map(|w| (w & 0xffff) as u16);
            }
            true
        }
        _ => false,
    };
    let is_player = is_method
        && LiveMem
            .u32_at(base.wrapping_add(map::manager::LOCAL_PLAYER_ENTITY))
            .filter(|&p| p != 0)
            .and_then(|p| LiveMem.u32_at(p.wrapping_add(map::entity::ID)))
            == Some(id as u32);
    Plan {
        path: if is_method {
            trace::delivery_path(in_world, is_player)
        } else {
            trace::property_path(in_world)
        },
        type_id,
        len: stream_remaining(stream),
    }
}

/// The fields of one `client.mercury.entity_*` event. A queued message also
/// carries where the entity is and how many messages wait for it.
fn message_fields(mgr: *mut c_void, id: i32, msg_id: u32, plan: &Plan) -> trace::Fields {
    let mut f: trace::Fields = vec![
        ("entity_id", json!(id)),
        ("msg_id", json!(msg_id)),
        ("path", json!(plan.path)),
    ];
    if let Some(t) = plan.type_id {
        f.push(("type_id", json!(t)));
    }
    if let Some(n) = plan.len {
        f.push(("len", json!(n)));
    }
    if plan.path == "queued" {
        let s = snap(mgr, id);
        f.push(("place", json!(s.place().as_str())));
        if let Some(q) = s.queued_msgs {
            f.push(("queued_msgs", json!(q)));
        }
        if let Some(e) = s.entity() {
            f.push(("enter_count", json!(e.enter_count)));
        }
    }
    f
}

fn level_of(path: &str) -> &'static str {
    if path == "queued" {
        "info"
    } else {
        "debug"
    }
}

// ---------------------------------------------------------------------

type MessageFn = unsafe extern "thiscall-unwind" fn(*mut c_void, i32, u32, *mut c_void);

/// `onEntityMethod(id, msg_id, stream)`.
#[allow(improper_ctypes_definitions)]
unsafe extern "thiscall-unwind" fn entity_method_detour(
    this: *mut c_void,
    id: i32,
    msg_id: u32,
    stream: *mut c_void,
) {
    let Some(&t) = ENTITY_METHOD_TRAMPOLINE.get() else {
        return;
    };
    let original: MessageFn = unsafe { std::mem::transmute(t) };
    let (planned, decision) = decide(this, id, stream, true, "client.mercury.entity_method");
    let ctx = Ctx {
        entity_id: id,
        type_id: planned.as_ref().and_then(|p| p.type_id),
        msg_id,
    };
    {
        let _scope = CtxScope::enter(ctx);
        original(this, id, msg_id, stream);
    }
    report(
        this,
        id,
        msg_id,
        planned,
        decision,
        "client.mercury.entity_method",
    );
}

/// `onEntityProperty(id, msg_id, stream)`.
#[allow(improper_ctypes_definitions)]
unsafe extern "thiscall-unwind" fn entity_property_detour(
    this: *mut c_void,
    id: i32,
    msg_id: u32,
    stream: *mut c_void,
) {
    let Some(&t) = ENTITY_PROPERTY_TRAMPOLINE.get() else {
        return;
    };
    let original: MessageFn = unsafe { std::mem::transmute(t) };
    let (planned, decision) = decide(this, id, stream, false, "client.mercury.entity_property");
    original(this, id, msg_id, stream);
    report(
        this,
        id,
        msg_id,
        planned,
        decision,
        "client.mercury.entity_property",
    );
}

/// Run the throttle first, so a suppressed message costs no memory reads.
fn decide(
    mgr: *mut c_void,
    id: i32,
    stream: *mut c_void,
    is_method: bool,
    target: &'static str,
) -> (Option<Plan>, Option<Decision>) {
    let decision = trace::throttle(target, id);
    if matches!(decision, Decision::Suppress) {
        return (None, None);
    }
    (guarded(|| plan(mgr, id, stream, is_method)), Some(decision))
}

/// Emit the event for a message that was planned.
fn report(
    mgr: *mut c_void,
    id: i32,
    msg_id: u32,
    planned: Option<Plan>,
    decision: Option<Decision>,
    target: &'static str,
) {
    guarded(|| {
        let (Some(p), Some(d)) = (planned, decision) else {
            return;
        };
        if let Some(f) = trace::with_suppressed(message_fields(mgr, id, msg_id, &p), d) {
            emit(target, level_of(p.path), f);
        }
    });
}

type ReplayFn = unsafe extern "thiscall-unwind" fn(*mut c_void, *mut c_void) -> u32;

/// Replay of an entity's queued messages, through the dispatcher. Tags the
/// dispatch with the entity, and reports how many messages were waiting.
#[allow(improper_ctypes_definitions)]
unsafe extern "thiscall-unwind" fn queue_replay_detour(
    this: *mut c_void,
    entity: *mut c_void,
) -> u32 {
    let Some(&t) = QUEUE_REPLAY_TRAMPOLINE.get() else {
        return 0;
    };
    let original: ReplayFn = unsafe { std::mem::transmute(t) };
    let (id, type_id) = guarded(|| entity_ids(entity)).unwrap_or((None, None));
    let Some(id) = id else {
        return original(this, entity);
    };
    let waiting = guarded(|| snap(this, id).queued_msgs).flatten();
    let _op = OpScope::begin("replay");
    let result = {
        // The messages carry no ids of their own: they all belong to `id`.
        let _scope = CtxScope::enter(Ctx {
            entity_id: id,
            type_id,
            msg_id: 0,
        });
        original(this, entity)
    };
    guarded(|| {
        // Only a replay that had something to do is news.
        if waiting.is_some_and(|n| n > 0) || result & 0xff != 0 {
            emit_throttled(
                "client.entity.queue_replay",
                "info",
                id,
                vec![
                    ("entity_id", json!(id)),
                    ("type_id", json!(type_id)),
                    ("queued_msgs", json!(waiting)),
                    ("replayed", json!(result & 0xff != 0)),
                ],
            );
        }
    });
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hooks::entity_trace::current_ctx;
    use std::sync::atomic::{AtomicU32, Ordering};
    use std::sync::Mutex;

    static SEEN_CTX: Mutex<Option<Ctx>> = Mutex::new(None);

    /// The tests share `SEEN_CTX` and `SEEN_ARGS`; run one at a time.
    static SERIAL: Mutex<()> = Mutex::new(());
    static SEEN_ARGS: [AtomicU32; 3] = [const { AtomicU32::new(0) }; 3];

    /// The original sees the entity it is dispatching for, the detour hands
    /// every argument through, and the context is gone afterwards. A wild
    /// manager pointer must not fault: every read is checked.
    #[test]
    fn method_detour_tags_the_dispatch_and_forwards_arguments() {
        let _serial = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
        unsafe extern "thiscall-unwind" fn original(
            this: *mut c_void,
            id: i32,
            msg: u32,
            stream: *mut c_void,
        ) {
            SEEN_ARGS[0].store(this as u32, Ordering::SeqCst);
            SEEN_ARGS[1].store(id as u32, Ordering::SeqCst);
            SEEN_ARGS[2].store(msg, Ordering::SeqCst);
            assert!(stream.is_null());
            *SEEN_CTX.lock().unwrap() = current_ctx();
        }
        ENTITY_METHOD_TRAMPOLINE
            .set(original as *const () as usize)
            .expect("only this test sets the trampoline");

        unsafe { entity_method_detour(0x1000 as *mut c_void, 4242, 0x5d, std::ptr::null_mut()) };

        assert_eq!(SEEN_ARGS[0].load(Ordering::SeqCst), 0x1000);
        assert_eq!(SEEN_ARGS[1].load(Ordering::SeqCst), 4242);
        assert_eq!(SEEN_ARGS[2].load(Ordering::SeqCst), 0x5d);
        let ctx = SEEN_CTX
            .lock()
            .unwrap()
            .expect("context set during the call");
        assert_eq!((ctx.entity_id, ctx.msg_id), (4242, 0x5d));
        assert_eq!(current_ctx(), None, "context cleared after the call");
    }

    /// A C++ exception from the dispatch must not leave a stale context on
    /// the network thread (it would tag unrelated events).
    #[test]
    fn a_throwing_property_dispatch_unwinds_and_leaves_no_context() {
        let _serial = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
        unsafe extern "thiscall-unwind" fn original(
            _: *mut c_void,
            _: i32,
            _: u32,
            _: *mut c_void,
        ) {
            panic!("engine error");
        }
        ENTITY_PROPERTY_TRAMPOLINE
            .set(original as *const () as usize)
            .expect("only this test sets the trampoline");
        let caught = std::panic::catch_unwind(|| unsafe {
            entity_property_detour(0x1000 as *mut c_void, 9, 1, std::ptr::null_mut())
        });
        assert!(caught.is_err());
        assert_eq!(current_ctx(), None);
    }

    /// The replay detour names the entity from the entity object itself and
    /// forwards the original's result.
    #[test]
    fn replay_detour_tags_with_the_entity_and_forwards_the_result() {
        let _serial = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
        unsafe extern "thiscall-unwind" fn original(_: *mut c_void, _: *mut c_void) -> u32 {
            *SEEN_CTX.lock().unwrap() = current_ctx();
            1
        }
        QUEUE_REPLAY_TRAMPOLINE
            .set(original as *const () as usize)
            .expect("only this test sets the trampoline");
        // An `Entity` with id 5 at +0xc and type 26 at +0x14.
        let mut entity = [0u32; 8];
        entity[3] = 5;
        entity[5] = 26;
        let ptr = entity.as_mut_ptr() as *mut c_void;

        let r = unsafe { queue_replay_detour(0x1000 as *mut c_void, ptr) };
        assert_eq!(r, 1);
        let ctx = SEEN_CTX.lock().unwrap().expect("context set during replay");
        assert_eq!((ctx.entity_id, ctx.type_id), (5, Some(26)));
        assert_eq!(current_ctx(), None);
    }
}
