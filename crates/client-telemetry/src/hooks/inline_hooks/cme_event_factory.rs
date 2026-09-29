//! CME event-factory hook: every CME event the client creates by name.
//!
//! `0x00a5c0f0` is the lookup in the client's CME event registry, a
//! `std::map<std::string, factory>` owned by the singleton at `0x01f11fc4`
//! (getter `0x0155f790`). `CMERegistry::RegisterAllEventEmitHandlers`
//! (`0x005c75d0`) fills it at startup with one factory per event class,
//! keyed by the full class name (`Event_Action_MouseClick`,
//! `Event_NetIn_...`). The lookup finds the name and calls the stored
//! factory, which returns a fresh event object; the caller fills its fields
//! and fires it.
//!
//! The load-bearing caller is `Client_NetIn_EntityMethodDispatch`
//! (`0x00c6f8f0`): when an inbound entity method *is* routed, it creates the
//! method's event here before filling the arguments and firing it. So this
//! hook is the positive half of the dispatch oracle, and the counterpart of
//! `client.dispatch.method_dropped`: that one reports a method the client
//! discarded, this one names every method it accepted. The other callers
//! (headless Ghidra xrefs, 2026-09-28) are three server-connection
//! functions, three system-option policies, `USGWAnimNotify_Event::Notify`
//! (A), the NetOut minigame emitter, and two not yet classified
//! (`0x00956620`, `0x00a4cfd0`).
//!
//! Signature, from the disassembly: `__thiscall(registry*, const
//! std::string& name) -> event*`, `ret 4`. The detour reads the name (a
//! live argument, bounded to [`MAX_NAME_CHARS`]) before the original runs
//! and forwards everything untouched.
//!
//! This anchor used to be called `CmeEventSignal_LookupByName`, and the
//! old CME subscriber install called it with a C string where it takes a
//! `std::string`, then called `0x00a5c150` as "subscribe". `0x00a5c150` is
//! the same map's `count(name)`: it inserts nothing. That install is gone;
//! see `docs/reverse-engineering/findings/cme-event-signal.md`.

#[cfg(all(target_os = "windows", target_arch = "x86"))]
use std::ffi::c_void;
use std::sync::{Mutex, OnceLock};
use std::time::Instant;

#[cfg(all(target_os = "windows", target_arch = "x86"))]
use crate::queue::Producer;

use crate::hooks::entity_trace::Ctx;
use crate::hooks::name_throttle::{Decision, NameThrottle};

/// Entry of the CME event-registry lookup.
#[cfg(all(target_os = "windows", target_arch = "x86"))]
pub(super) const ADDR_CME_EVENT_FACTORY: usize = 0x00a5c0f0;

/// Longest event name read. The longest class name in the registry is
/// well under this; a longer size means a corrupt argument.
pub(crate) const MAX_NAME_CHARS: usize = 128;

/// Telemetry target for the events this hook emits.
pub(crate) const TARGET: &str = "client.cme.event";

#[cfg(all(target_os = "windows", target_arch = "x86"))]
static CME_EVENT_FACTORY_TRAMPOLINE: OnceLock<usize> = OnceLock::new();

/// One throttle for the whole hook. Held for a hash lookup only; the
/// detour runs on the network thread (NetIn dispatch) and the main thread
/// (input actions), so the lock is lightly contended at worst.
static THROTTLE: Mutex<Option<NameThrottle>> = Mutex::new(None);

/// Monotonic origin for the throttle clock.
static EPOCH: OnceLock<Instant> = OnceLock::new();

/// What family an event belongs to, from its class-name prefix. Lets a
/// SigNoz query ask for "every inbound server method" without a regex.
pub(crate) fn kind_of(name: &str) -> &'static str {
    const KINDS: [(&str, &str); 8] = [
        ("Event_NetIn_", "net_in"),
        ("Event_NetOut_", "net_out"),
        ("Event_Net_", "net"),
        ("Event_Action_", "action"),
        ("Event_UI_", "ui"),
        ("Event_SlashCmd_", "slash_cmd"),
        ("Event_Cache_", "cache"),
        ("Event_", "other"),
    ];
    KINDS
        .iter()
        .find(|(prefix, _)| name.starts_with(prefix))
        .map_or("unnamed", |(_, kind)| kind)
}

/// Level for an event of `kind`. An inbound server method the client
/// accepted is a dispatch, so it is `info`, like the server's dispatch
/// spans; everything else is `debug`.
pub(crate) fn level_of(kind: &str) -> &'static str {
    if kind == "net_in" {
        "info"
    } else {
        "debug"
    }
}

/// Run the throttle for `name`. Poisoning is ignored: a panic elsewhere
/// must not silence the hook.
pub(crate) fn throttle(name: &str) -> Decision {
    let now_ms = EPOCH.get_or_init(Instant::now).elapsed().as_millis() as u64;
    let mut guard = THROTTLE.lock().unwrap_or_else(|e| e.into_inner());
    guard
        .get_or_insert_with(NameThrottle::new)
        .check(name, now_ms)
}

/// The fields of one `client.cme.event`, or `None` when throttled.
pub(crate) fn event_fields(
    name: &str,
    truncated: bool,
    decision: Decision,
) -> Option<Vec<(&'static str, serde_json::Value)>> {
    let Decision::Emit { suppressed } = decision else {
        return None;
    };
    let mut fields = vec![
        ("event", serde_json::json!(name)),
        ("kind", serde_json::json!(kind_of(name))),
    ];
    if suppressed > 0 {
        fields.push(("suppressed", serde_json::json!(suppressed)));
    }
    if truncated {
        fields.push(("truncated", serde_json::json!(true)));
    }
    Some(fields)
}

/// The fields of one `client.cme.event` raised while the network thread was
/// dispatching an inbound method for `ctx.entity_id`: the plain fields plus
/// `entity_id`, `type_id` (when known) and `msg_id`. Throttled per (event,
/// entity) by the caller, so an entity's first sighting of an event is
/// never lost behind another entity's flood of it.
pub(crate) fn entity_event_fields(
    name: &str,
    truncated: bool,
    decision: Decision,
    ctx: &Ctx,
) -> Option<Vec<(&'static str, serde_json::Value)>> {
    let mut fields = event_fields(name, truncated, decision)?;
    fields.push(("entity_id", serde_json::json!(ctx.entity_id)));
    if let Some(t) = ctx.type_id {
        fields.push(("type_id", serde_json::json!(t)));
    }
    if ctx.msg_id != 0 {
        fields.push(("msg_id", serde_json::json!(ctx.msg_id)));
    }
    Some(fields)
}

#[cfg(all(target_os = "windows", target_arch = "x86"))]
pub(super) unsafe fn install_cme_event_factory(producer: &Producer) {
    super::install_one(
        producer,
        "cme_event_factory",
        ADDR_CME_EVENT_FACTORY,
        cme_event_factory_detour as *mut c_void,
        &CME_EVENT_FACTORY_TRAMPOLINE,
    );
}

/// Read the name, throttle, emit, then run the original.
#[cfg(all(target_os = "windows", target_arch = "x86"))]
fn observe(name_obj: *const c_void) {
    // SAFETY: `name_obj` is the `const std::string&` argument of the hooked
    // call, alive until the call returns.
    let decoded = unsafe {
        crate::msvc_string::read(
            name_obj as *const u8,
            crate::msvc_string::Width::Narrow,
            MAX_NAME_CHARS,
        )
    };
    let (name, truncated) = match decoded {
        Some(d) => (d.text, d.truncated),
        None => ("<unreadable>".to_string(), false),
    };
    let kind = kind_of(&name);
    // An inbound server method the network thread is dispatching for an
    // entity: tag it, and throttle per (event, entity) instead of per name.
    let ctx = if kind == "net_in" {
        crate::hooks::entity_trace::current_ctx()
    } else {
        None
    };
    let fields = match &ctx {
        Some(c) => entity_event_fields(
            &name,
            truncated,
            crate::hooks::entity_trace::throttle(&name, c.entity_id),
            c,
        ),
        None => event_fields(&name, truncated, throttle(&name)),
    };
    let Some(fields) = fields else {
        return;
    };
    let level = level_of(kind);

    #[cfg(feature = "lab-bridge")]
    crate::bridge::events::push(
        "cme.event",
        crate::bridge::crash::now_ms(),
        serde_json::Value::Object(
            fields
                .iter()
                .map(|(k, v)| (k.to_string(), v.clone()))
                .collect(),
        ),
    );

    if let Some(p) = crate::boot::producer() {
        let mut b = crate::events::ClientNativeEvent::builder(TARGET, level);
        for (k, v) in fields {
            b = b.field(k, v);
        }
        p.try_emit(b);
    }
}

/// Detour for the registry lookup.
///
/// **Threads:** the network thread (NetIn dispatch) and the main thread.
/// Non-blocking apart from the throttle's short critical section; reads
/// nothing but its own argument.
#[cfg(all(target_os = "windows", target_arch = "x86"))]
#[allow(improper_ctypes_definitions)]
unsafe extern "thiscall-unwind" fn cme_event_factory_detour(
    registry: *mut c_void,
    name: *const c_void,
) -> *mut c_void {
    let _ = std::panic::catch_unwind(|| observe(name));
    // The first event proves the registry is populated: dump its catalog.
    let _ = std::panic::catch_unwind(|| crate::hooks::cme_catalog::start_once(registry as usize));

    match CME_EVENT_FACTORY_TRAMPOLINE.get() {
        Some(t) => {
            let original: unsafe extern "thiscall-unwind" fn(
                *mut c_void,
                *const c_void,
            ) -> *mut c_void = unsafe { std::mem::transmute(*t) };
            original(registry, name)
        }
        // Null is the lookup's own "no such event" answer; every caller
        // checks for it.
        None => std::ptr::null_mut(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kinds_follow_the_class_name_prefix() {
        assert_eq!(kind_of("Event_NetIn_onDialogDisplay"), "net_in");
        assert_eq!(kind_of("Event_NetOut_DialogButtonChoice"), "net_out");
        assert_eq!(kind_of("Event_Net_Disconnected"), "net");
        assert_eq!(kind_of("Event_Action_MouseClick"), "action");
        assert_eq!(kind_of("Event_UI_DuelTimerStart"), "ui");
        assert_eq!(kind_of("Event_Cache_ElementReady"), "cache");
        assert_eq!(kind_of("Event_Kismet_SequenceFinished"), "other");
        assert_eq!(kind_of("<unreadable>"), "unnamed");
    }

    /// Only an accepted inbound server method is a dispatch-level event.
    #[test]
    fn only_net_in_is_info() {
        assert_eq!(level_of("net_in"), "info");
        for k in ["net_out", "net", "action", "ui", "other", "unnamed"] {
            assert_eq!(level_of(k), "debug", "{k}");
        }
    }

    #[test]
    fn fields_carry_name_kind_and_counts() {
        let f = event_fields(
            "Event_NetIn_onDialogDisplay",
            false,
            Decision::Emit { suppressed: 0 },
        )
        .unwrap();
        assert_eq!(
            f,
            vec![
                ("event", serde_json::json!("Event_NetIn_onDialogDisplay")),
                ("kind", serde_json::json!("net_in")),
            ]
        );
        let f = event_fields("x", true, Decision::Emit { suppressed: 7 }).unwrap();
        assert!(f.contains(&("suppressed", serde_json::json!(7))));
        assert!(f.contains(&("truncated", serde_json::json!(true))));
        assert!(event_fields("x", false, Decision::Suppress).is_none());
    }

    /// A NetIn event created while dispatching for an entity carries the
    /// entity id, the type when known, and the message id.
    #[test]
    fn entity_events_carry_the_entity() {
        let ctx = Ctx {
            entity_id: 4242,
            type_id: Some(26),
            msg_id: 0x5d,
        };
        let f = entity_event_fields(
            "Event_NetIn_onStaticMeshNameUpdate",
            false,
            Decision::Emit { suppressed: 2 },
            &ctx,
        )
        .unwrap();
        for (k, v) in [
            ("entity_id", serde_json::json!(4242)),
            ("type_id", serde_json::json!(26)),
            ("msg_id", serde_json::json!(0x5d)),
            ("suppressed", serde_json::json!(2)),
            ("kind", serde_json::json!("net_in")),
        ] {
            assert!(f.contains(&(k, v.clone())), "{k}={v} in {f:?}");
        }
        let bare = Ctx {
            type_id: None,
            msg_id: 0,
            ..ctx
        };
        let f = entity_event_fields(
            "Event_NetIn_x",
            false,
            Decision::Emit { suppressed: 0 },
            &bare,
        )
        .unwrap();
        assert!(!f.iter().any(|(k, _)| *k == "type_id" || *k == "msg_id"));
        assert!(entity_event_fields("x", false, Decision::Suppress, &ctx).is_none());
    }

    /// The shared throttle lets a first-seen name through.
    #[test]
    fn a_first_seen_name_is_emitted() {
        assert!(matches!(
            throttle("Event_NetIn_test_only_name"),
            Decision::Emit { .. }
        ));
    }

    /// The detour hands both arguments and the return value through, and a
    /// C++ exception from the original unwinds through it (the other
    /// detours' contract, #915).
    #[cfg(all(target_os = "windows", target_arch = "x86"))]
    #[test]
    fn forwards_arguments_and_return_and_lets_exceptions_through() {
        use std::sync::atomic::{AtomicUsize, Ordering};
        static SEEN: [AtomicUsize; 2] = [const { AtomicUsize::new(0) }; 2];
        static THROW: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
        unsafe extern "thiscall-unwind" fn original(
            registry: *mut c_void,
            name: *const c_void,
        ) -> *mut c_void {
            SEEN[0].store(registry as usize, Ordering::SeqCst);
            SEEN[1].store(name as usize, Ordering::SeqCst);
            if THROW.load(Ordering::SeqCst) {
                panic!("engine error");
            }
            0x5555 as *mut c_void
        }
        CME_EVENT_FACTORY_TRAMPOLINE
            .set(original as *const () as usize)
            .expect("only this test sets the trampoline");

        // A real std::string layout (inline storage) as the argument.
        let mut obj = [0u8; crate::msvc_string::OBJECT_SIZE];
        obj[4..4 + 13].copy_from_slice(b"Event_UI_Test");
        obj[0x14..0x18].copy_from_slice(&13u32.to_le_bytes());
        obj[0x18..0x1c].copy_from_slice(&15u32.to_le_bytes());
        let name = obj.as_ptr() as *const c_void;

        let ret = unsafe { cme_event_factory_detour(0x1111 as *mut c_void, name) };
        assert_eq!(ret as usize, 0x5555);
        assert_eq!(SEEN[0].load(Ordering::SeqCst), 0x1111);
        assert_eq!(SEEN[1].load(Ordering::SeqCst), name as usize);

        THROW.store(true, Ordering::SeqCst);
        let caught = std::panic::catch_unwind(|| unsafe {
            cme_event_factory_detour(0x1111 as *mut c_void, name)
        });
        assert!(caught.is_err());
    }
}
