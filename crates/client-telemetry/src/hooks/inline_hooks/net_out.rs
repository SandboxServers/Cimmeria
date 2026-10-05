//! Outbound entity and base methods: `client.net.out`.
//!
//! `RouteOutgoingEntityRpc` (`0x00c6fc40`) is the generic sender every
//! `Event_NetOut_*` handler ends in (`docs/reverse-engineering/findings/
//! black-market-client-io.md` §1, `docs/reverse-engineering/findings/
//! client-entity-lifecycle.md`):
//!
//! ```text
//! stdcall(Entity* entity, EntityDescription* desc, MethodDescription* method, args)
//! ret 0x10
//! ```
//!
//! `entity` is null for "the local player". `method+0x00` is the method's
//! name (an MSVC `std::string` object), `method+0x1c & 3` is the route
//! (`2` = base method through the proxy, otherwise a cell/entity method),
//! `method+0x44` the wire message id and `method+0x48` the extended
//! sub-index (negative when unused). The function does nothing when the
//! connection is offline, so an event here means the client *tried* to
//! send; the server's inbound span says whether it arrived.
//!
//! The detour forwards all four arguments untouched. For a method in the
//! ability allowlist it also decodes `args` with the game's own read-only
//! bag readers and watches whether the router reaches a `start*Message`
//! (`client.ability.sent` / `press_dropped`, [`super::ability::route`]).
//! Thread: the main thread (Lua and UI handlers send from there).

use std::ffi::c_void;
use std::sync::OnceLock;

use serde_json::json;

use super::entity_lifecycle::guarded;
use crate::hooks::entity_trace::{
    self as trace,
    map::{self, LiveMem, Mem},
};
use crate::queue::Producer;

pub(super) const ADDR_ROUTE_OUTGOING_RPC: usize = 0x00c6fc40;

/// `GameEntityManager` singleton pointer.
const ADDR_ENTITY_MANAGER: usize = 0x01ef244c;

/// Longest method name read.
const MAX_NAME_CHARS: usize = 64;

static ROUTE_TRAMPOLINE: OnceLock<usize> = OnceLock::new();

pub(super) unsafe fn install_all(producer: &Producer) {
    super::install_one(
        producer,
        "net_out_route",
        ADDR_ROUTE_OUTGOING_RPC,
        route_detour as *mut c_void,
        &ROUTE_TRAMPOLINE,
    );
}

/// `"base"` or `"cell"` from the method's flag byte.
pub(crate) fn route_of(flags: u32) -> &'static str {
    if flags & 3 == 2 {
        "base"
    } else {
        "cell"
    }
}

/// The fields of one `client.net.out`. `sub_index` is the extended index
/// byte, reported only when the method uses one (non-negative).
pub(crate) fn out_fields(
    method: Option<&str>,
    flags: u32,
    msg_id: i32,
    sub_index: i32,
    entity_id: Option<i32>,
    to_local_player: bool,
) -> trace::Fields {
    let mut f: trace::Fields = vec![
        ("method", json!(method)),
        ("route", json!(route_of(flags))),
        ("msg_id", json!(msg_id)),
    ];
    if sub_index >= 0 {
        f.push(("sub_index", json!(sub_index)));
    }
    if let Some(id) = entity_id {
        f.push(("entity_id", json!(id)));
    }
    f.push(("to_local_player", json!(to_local_player)));
    f
}

type RouteFn =
    unsafe extern "stdcall-unwind" fn(*mut c_void, *mut c_void, *mut c_void, *mut c_void);

#[allow(improper_ctypes_definitions)]
unsafe extern "stdcall-unwind" fn route_detour(
    entity: *mut c_void,
    desc: *mut c_void,
    method: *mut c_void,
    args: *mut c_void,
) {
    let Some(&t) = ROUTE_TRAMPOLINE.get() else {
        return;
    };
    let original: RouteFn = unsafe { std::mem::transmute(t) };
    guarded(|| observe(entity, method));
    let probe = super::ability::route::RouteProbe::begin(entity, method, args);
    original(entity, desc, method, args);
    if let Some(p) = probe {
        p.finish();
    }
}

fn observe(entity: *mut c_void, method: *mut c_void) {
    let m = method as u32;
    let word = |off: u32| LiveMem.u32_at(m.wrapping_add(off));
    let (Some(flags), Some(msg_id), Some(sub)) = (word(0x1c), word(0x44), word(0x48)) else {
        return;
    };
    let name = crate::msvc_string::read_checked(
        method as usize,
        crate::msvc_string::Width::Narrow,
        MAX_NAME_CHARS,
    )
    .map(|d| d.text);
    // A null entity means the local player, whose id the manager holds.
    let manager = LiveMem.u32_at(ADDR_ENTITY_MANAGER as u32).unwrap_or(0);
    let local_id = LiveMem
        .u32_at(manager.wrapping_add(map::manager::LOCAL_PLAYER_ID))
        .map(|v| v as i32);
    let (entity_id, to_local) = if entity.is_null() {
        (local_id, true)
    } else {
        let id = LiveMem
            .u32_at((entity as u32).wrapping_add(map::entity::ID))
            .map(|v| v as i32);
        (id, id.is_some() && id == local_id)
    };
    let key = entity_id.unwrap_or(0);
    // Per method as well as per entity: a chatty method must not hide a
    // rare one for the same entity.
    let target: &'static str = "client.net.out";
    let fields = out_fields(
        name.as_deref(),
        flags,
        msg_id as i32,
        sub as i32,
        entity_id,
        to_local,
    );
    let name_key = name.as_deref().unwrap_or("?");
    let decision = trace::throttle(&format!("{target}:{name_key}"), key);
    if let Some(f) = trace::with_suppressed(fields, decision) {
        crate::hooks::emit::emit(target, "info", f);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_route_comes_from_the_low_flag_bits() {
        assert_eq!(route_of(2), "base");
        assert_eq!(route_of(0x1e), "base", "only the low two bits count");
        assert_eq!(route_of(0), "cell");
        assert_eq!(route_of(1), "cell");
        assert_eq!(route_of(3), "cell");
    }

    #[test]
    fn a_base_method_to_the_local_player() {
        let f = out_fields(Some("createCharacter"), 2, 0x12, -1, Some(7), true);
        let get = |k: &str| f.iter().find(|(n, _)| *n == k).map(|(_, v)| v.clone());
        assert_eq!(get("method"), Some(json!("createCharacter")));
        assert_eq!(get("route"), Some(json!("base")));
        assert_eq!(get("msg_id"), Some(json!(18)));
        assert_eq!(get("sub_index"), None, "negative means no extended index");
        assert_eq!(get("entity_id"), Some(json!(7)));
        assert_eq!(get("to_local_player"), Some(json!(true)));
    }

    #[test]
    fn an_extended_index_is_reported() {
        let f = out_fields(None, 0, 0x3d, 4, None, false);
        assert!(f.iter().any(|(k, v)| *k == "sub_index" && *v == json!(4)));
        assert!(f.iter().any(|(k, v)| *k == "method" && v.is_null()));
        assert!(!f.iter().any(|(k, _)| *k == "entity_id"));
    }
}

#[cfg(test)]
mod detour_tests {
    use super::*;
    use std::sync::atomic::{AtomicU32, Ordering};

    static SEEN: [AtomicU32; 4] = [const { AtomicU32::new(0) }; 4];

    /// The router's four stdcall arguments reach the original untouched, with
    /// a method descriptor that is not readable.
    #[test]
    fn route_detour_forwards_all_four_arguments() {
        unsafe extern "stdcall-unwind" fn original(
            a: *mut c_void,
            b: *mut c_void,
            c: *mut c_void,
            d: *mut c_void,
        ) {
            SEEN[0].store(a as u32, Ordering::SeqCst);
            SEEN[1].store(b as u32, Ordering::SeqCst);
            SEEN[2].store(c as u32, Ordering::SeqCst);
            SEEN[3].store(d as u32, Ordering::SeqCst);
        }
        ROUTE_TRAMPOLINE
            .set(original as *const () as usize)
            .expect("only this test sets the trampoline");
        unsafe {
            route_detour(
                std::ptr::null_mut(),
                0x2000 as *mut c_void,
                0x3000 as *mut c_void,
                0x4000 as *mut c_void,
            )
        };
        let got: Vec<u32> = SEEN.iter().map(|a| a.load(Ordering::SeqCst)).collect();
        assert_eq!(got, vec![0, 0x2000, 0x3000, 0x4000]);
    }
}
