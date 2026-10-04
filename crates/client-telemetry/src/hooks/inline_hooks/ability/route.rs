//! The router side of AB-C1: decode an allowlisted method's arguments with
//! the game's own bag readers, watch whether `RouteOutgoingEntityRpc`
//! reaches a `start*Message`, and report `sent` or `press_dropped`.

use std::cell::Cell;
use std::ffi::c_void;
use std::sync::OnceLock;

use super::{guarded, install, next_send_id, report, with_joiner, with_pending};
use crate::hooks::ability_trace::decode::{self, ArgBag, DecodedCall, MethodSpec, SendCtx};
use crate::hooks::ability_trace::layout::{self, RoutePre};
use crate::hooks::ability_trace::press::{self, RouteOutcome};
use crate::hooks::ability_trace::seq_join::SentTag;
use crate::hooks::ability_trace::timing::{with_timing, Timing};
use crate::hooks::ability_trace::{now_ms, Out, TARGET_SENT};
use crate::hooks::entity_trace::map::{LiveMem, Mem};
use crate::msvc_string::{self, Width};
use crate::queue::Producer;

/// `startEntityMessage` (`thiscall(conn, msgId, entityId)`, `ret 8`).
pub(in crate::hooks::inline_hooks) const ADDR_START_ENTITY_MESSAGE: usize = 0x00dd_6a60;
/// `startProxyMessage` (`thiscall(conn, msgId)`, `ret 4`).
pub(in crate::hooks::inline_hooks) const ADDR_START_PROXY_MESSAGE: usize = 0x00dd_6980;
/// `GetInt` / `GetFloat` / `GetByte(event, const std::string* name, T* out)
/// -> bool`, `ret 8`: defined once, in `ability_trace::event_bag`.
pub(in crate::hooks::inline_hooks) use crate::hooks::ability_trace::event_bag::{
    ADDR_GET_BYTE, ADDR_GET_FLOAT, ADDR_GET_INT,
};

/// `MethodDescription` fields (finding, "Signatures").
mod method {
    /// `flags & 3 == 2` is the base route.
    pub(super) const FLAGS: u32 = 0x1c;
    /// Argument-name vector: `begin`, `end` (stride `0x1c`).
    pub(super) const ARG_NAMES_BEGIN: u32 = 0x34;
    pub(super) const ARG_NAMES_END: u32 = 0x38;
    /// Wire message id.
    pub(super) const MSG_ID: u32 = 0x44;
    /// Extended sub-index, negative when unused.
    pub(super) const SUB_INDEX: u32 = 0x48;
}

/// An MSVC `std::string` object's size.
const STRING_SIZE: u32 = 0x1c;
/// Longest method or argument name read.
const MAX_NAME_CHARS: usize = 64;
/// The allowlist's longest argument list is 4; anything above this many is
/// not a descriptor we know.
const MAX_ARGS: u32 = 8;

pub(super) static START_ENTITY_TRAMPOLINE: OnceLock<usize> = OnceLock::new();
pub(super) static START_PROXY_TRAMPOLINE: OnceLock<usize> = OnceLock::new();

pub(super) unsafe fn install_all(producer: &Producer) {
    unsafe {
        install(
            producer,
            "ability_start_entity_message",
            ADDR_START_ENTITY_MESSAGE,
            start_entity_detour as *mut c_void,
            &START_ENTITY_TRAMPOLINE,
        );
        install(
            producer,
            "ability_start_proxy_message",
            ADDR_START_PROXY_MESSAGE,
            start_proxy_detour as *mut c_void,
            &START_PROXY_TRAMPOLINE,
        );
    }
}

thread_local! {
    /// Set while the router runs an allowlisted call.
    static IN_ROUTE: Cell<bool> = const { Cell::new(false) };
    /// Set when a `start*Message` ran inside it.
    static REACHED_START: Cell<bool> = const { Cell::new(false) };
}

/// The event bag behind the router's `args`, read with the game's readers.
/// Each key is passed as the method descriptor's own `std::string`, so the
/// key spelling is exactly what the game uses.
struct LiveBag {
    event: *mut c_void,
    /// Trimmed argument name and the address of its `std::string`.
    names: Vec<(String, u32)>,
}

type GetFn<T> = unsafe extern "thiscall-unwind" fn(*mut c_void, *const c_void, *mut T) -> u8;

impl LiveBag {
    fn key(&self, name: &str) -> Option<*const c_void> {
        self.names
            .iter()
            .find(|(n, _)| n == name)
            .map(|(_, a)| *a as *const c_void)
    }

    fn read<T: Default>(&self, addr: usize, name: &str) -> Option<T> {
        let key = self.key(name)?;
        let get: GetFn<T> = unsafe { std::mem::transmute(addr) };
        let mut out = T::default();
        // The reader copies the event's property tree and looks the name
        // up; it writes `out` only on a hit and returns the hit in AL.
        let hit = unsafe { get(self.event, key, &mut out) };
        (hit != 0).then_some(out)
    }
}

impl ArgBag for LiveBag {
    fn int(&self, name: &str) -> Option<i32> {
        self.read(ADDR_GET_INT, name)
    }
    fn float(&self, name: &str) -> Option<f32> {
        self.read(ADDR_GET_FLOAT, name)
    }
    fn byte(&self, name: &str) -> Option<u8> {
        self.read(ADDR_GET_BYTE, name)
    }
}

/// What the router hook knows before the original runs.
struct Before {
    spec: &'static MethodSpec,
    names: Vec<(String, u32)>,
    flags: u32,
    msg_id: i32,
    sub_index: i32,
    pre: RoutePre,
    client_target_id: Option<i32>,
}

/// Read the descriptor; `None` for a method outside the allowlist (the
/// common case, which costs one name read).
fn read_before(entity: u32, method: u32) -> Option<Before> {
    let name = msvc_string::read_checked(method as usize, Width::Narrow, MAX_NAME_CHARS)?.text;
    let spec = decode::spec_for(&name)?;
    let word = |off: u32| LiveMem.u32_at(method.wrapping_add(off));
    let begin = word(method::ARG_NAMES_BEGIN).unwrap_or(0);
    let end = word(method::ARG_NAMES_END).unwrap_or(0);
    let count = if begin != 0 && end >= begin {
        ((end - begin) / STRING_SIZE).min(MAX_ARGS)
    } else {
        0
    };
    let names = (0..count)
        .filter_map(|i| {
            let addr = begin + i * STRING_SIZE;
            let text = msvc_string::read_checked(addr as usize, Width::Narrow, MAX_NAME_CHARS)?;
            Some((text.text.trim().to_owned(), addr))
        })
        .collect();
    Some(Before {
        spec,
        names,
        flags: word(method::FLAGS).unwrap_or(0),
        msg_id: word(method::MSG_ID).unwrap_or(0) as i32,
        sub_index: word(method::SUB_INDEX).unwrap_or(u32::MAX) as i32,
        pre: layout::route_pre(&LiveMem, entity),
        client_target_id: layout::client_target_id(&LiveMem),
    })
}

/// One allowlisted router call, from before the original to after it.
pub(in crate::hooks::inline_hooks) struct RouteProbe {
    before: Before,
    call: DecodedCall,
    prev_in_route: bool,
    prev_reached: bool,
}

impl RouteProbe {
    /// Called by the router detour before the original. Returns `None` for
    /// a method outside the allowlist. The bag readers are game code and
    /// run outside any `catch_unwind`, so a C++ exception from one unwinds
    /// the way the game expects instead of aborting.
    pub(in crate::hooks::inline_hooks) fn begin(
        entity: *mut c_void,
        method: *mut c_void,
        args: *mut c_void,
    ) -> Option<Self> {
        let before = guarded(|| read_before(entity as u32, method as u32)).flatten()?;
        let bag = LiveBag {
            event: args,
            names: before.names.clone(),
        };
        let call = if args.is_null() {
            DecodedCall {
                missing: before.spec.args.iter().map(|a| a.name).collect(),
                ..DecodedCall::default()
            }
        } else {
            decode::decode(before.spec, &bag)
        };
        Some(Self {
            before,
            call,
            prev_in_route: IN_ROUTE.try_with(|c| c.replace(true)).unwrap_or(false),
            prev_reached: REACHED_START
                .try_with(|c| c.replace(false))
                .unwrap_or(false),
        })
    }

    /// Called after the original returned: report `sent` or
    /// `press_dropped`.
    pub(in crate::hooks::inline_hooks) fn finish(self) {
        let reached = REACHED_START.try_with(Cell::get).unwrap_or(false);
        let outs = guarded(|| self.outcome(reached)).unwrap_or_default();
        report(outs);
    }

    fn outcome(&self, reached: bool) -> Vec<Out> {
        let b = &self.before;
        let now = now_ms();
        let pending = with_pending(|t| t.take(b.spec.name, self.call.ability_id, now));
        match press::route_outcome(reached, b.pre) {
            RouteOutcome::Sent => {
                let press_to_sent_ms = pending.map(|p| now.saturating_sub(p.at_ms));
                let ctx = SendCtx {
                    send_id: next_send_id(),
                    press_id: pending.map(|p| p.press_id),
                    press_to_sent_ms,
                    msg_id: b.msg_id,
                    sub_index: b.sub_index,
                    route: super::super::net_out::route_of(b.flags),
                    client_target_id: b.client_target_id,
                };
                with_joiner(|j| {
                    j.note_sent(SentTag {
                        send_id: ctx.send_id,
                        press_id: ctx.press_id,
                        method: b.spec.name,
                        ability_id: self.call.ability_id,
                    })
                });
                // AB-C6: the press-to-send interval, and the send held
                // open for its first answer.
                if let Some(ms) = press_to_sent_ms {
                    Timing::press_sent(b.spec.name, ms);
                }
                with_timing(|t| {
                    t.note_sent(
                        ctx.send_id,
                        ctx.press_id,
                        b.spec.name,
                        self.call.ability_id,
                        now,
                    )
                });
                vec![Out {
                    target: TARGET_SENT,
                    level: "info",
                    key: format!("{TARGET_SENT}:{}", b.spec.name),
                    fields: decode::sent_fields(b.spec, &self.call, &ctx),
                }]
            }
            RouteOutcome::Dropped(reason, at) => vec![press::route_dropped(
                b.spec.name,
                self.call.ability_id,
                pending,
                reason,
                at,
            )],
        }
    }
}

impl Drop for RouteProbe {
    fn drop(&mut self) {
        let _ = IN_ROUTE.try_with(|c| c.set(self.prev_in_route));
        let _ = REACHED_START.try_with(|c| c.set(self.prev_reached));
    }
}

fn note_start() {
    if IN_ROUTE.try_with(Cell::get).unwrap_or(false) {
        let _ = REACHED_START.try_with(|c| c.set(true));
    }
}

type StartEntityFn = unsafe extern "thiscall-unwind" fn(*mut c_void, u32, u32) -> u32;
type StartProxyFn = unsafe extern "thiscall-unwind" fn(*mut c_void, u32) -> u32;

/// `startEntityMessage(msgId, entityId)`: returns the message stream.
#[allow(improper_ctypes_definitions)]
pub(super) unsafe extern "thiscall-unwind" fn start_entity_detour(
    conn: *mut c_void,
    msg_id: u32,
    entity_id: u32,
) -> u32 {
    let Some(&t) = START_ENTITY_TRAMPOLINE.get() else {
        return 0;
    };
    let original: StartEntityFn = unsafe { std::mem::transmute(t) };
    note_start();
    unsafe { original(conn, msg_id, entity_id) }
}

/// `startProxyMessage(msgId)`: returns the message stream.
#[allow(improper_ctypes_definitions)]
pub(super) unsafe extern "thiscall-unwind" fn start_proxy_detour(
    conn: *mut c_void,
    msg_id: u32,
) -> u32 {
    let Some(&t) = START_PROXY_TRAMPOLINE.get() else {
        return 0;
    };
    let original: StartProxyFn = unsafe { std::mem::transmute(t) };
    note_start();
    unsafe { original(conn, msg_id) }
}

#[cfg(test)]
pub(super) mod test_access {
    //! Trampoline slots for the patch/call/unpatch tests.
    pub(in crate::hooks::inline_hooks::ability) use super::{
        START_ENTITY_TRAMPOLINE, START_PROXY_TRAMPOLINE,
    };
    use std::cell::Cell;

    /// Enter the router state a `RouteProbe` sets, for the tests.
    pub(in crate::hooks::inline_hooks::ability) fn enter_route() {
        super::IN_ROUTE.with(|c| c.set(true));
        super::REACHED_START.with(|c| c.set(false));
    }

    /// What a `start*Message` detour does inside the router.
    pub(in crate::hooks::inline_hooks::ability) fn mark_reached() {
        super::REACHED_START.with(|c| c.set(true));
    }

    /// Whether a `start*Message` was seen, and leave the router state.
    pub(in crate::hooks::inline_hooks::ability) fn leave_route() -> bool {
        super::IN_ROUTE.with(|c| c.set(false));
        super::REACHED_START.with(Cell::get)
    }
}
