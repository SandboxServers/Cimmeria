//! The Mercury sequence join for sent ability methods (AB-C1). See
//! `hooks::ability_trace::seq_join` for the design.
//!
//! `Channel::send` runs where the game sends (the main thread);
//! `Nub::send` and the sequence counter run on the network thread. The
//! counter hook costs one thread-local check per reliable packet when no
//! ability bundle is in flight.

use std::cell::RefCell;
use std::ffi::c_void;
use std::sync::OnceLock;

use super::{guarded, install, report, with_joiner};
use crate::hooks::ability_trace::seq_join::sent_seq_outs;
use crate::hooks::entity_trace::map::{LiveMem, Mem};
use crate::queue::Producer;

/// `Channel::send` (`thiscall(channel) -> int`, plain `ret`).
pub(in crate::hooks::inline_hooks) const ADDR_CHANNEL_SEND: usize = 0x0157_6f90;
/// `Nub::send` (`thiscall(nub, addr, bundle, channel)`, `ret 0xc`).
pub(in crate::hooks::inline_hooks) const ADDR_NUB_SEND: usize = 0x0158_2160;
/// The reliable channel's sequence counter (`thiscall(channelInternal)
/// -> u32`); one caller, `0x0158258c` in `Nub::send`.
pub(in crate::hooks::inline_hooks) const ADDR_SEQ_NEXT: usize = 0x0158_bb40;

/// `Channel+0x28`: the bundle being filled; `Channel::send` detaches it.
const CHANNEL_BUNDLE: u32 = 0x28;
/// Sequence numbers kept per bundle (a bundle is a handful of packets).
const MAX_SEQS: usize = 64;

pub(super) static CHANNEL_SEND_TRAMPOLINE: OnceLock<usize> = OnceLock::new();
pub(super) static NUB_SEND_TRAMPOLINE: OnceLock<usize> = OnceLock::new();
pub(super) static SEQ_NEXT_TRAMPOLINE: OnceLock<usize> = OnceLock::new();

pub(super) unsafe fn install_all(producer: &Producer) {
    unsafe {
        install(
            producer,
            "ability_channel_send",
            ADDR_CHANNEL_SEND,
            channel_send_detour as *mut c_void,
            &CHANNEL_SEND_TRAMPOLINE,
        );
        install(
            producer,
            "ability_nub_send",
            ADDR_NUB_SEND,
            nub_send_detour as *mut c_void,
            &NUB_SEND_TRAMPOLINE,
        );
        install(
            producer,
            "ability_seq_next",
            ADDR_SEQ_NEXT,
            seq_next_detour as *mut c_void,
            &SEQ_NEXT_TRAMPOLINE,
        );
    }
}

thread_local! {
    /// The sequence numbers of the ability bundle `Nub::send` is sending
    /// on this thread; `None` outside one.
    static SEQS: RefCell<Option<Vec<u32>>> = const { RefCell::new(None) };
}

type ChannelSendFn = unsafe extern "thiscall-unwind" fn(*mut c_void) -> u32;

/// `Channel::send()`. The bundle is tagged before the original runs: the
/// original hands it to the network thread, which may send it before this
/// detour gets control back.
#[allow(improper_ctypes_definitions)]
pub(super) unsafe extern "thiscall-unwind" fn channel_send_detour(channel: *mut c_void) -> u32 {
    let Some(&t) = CHANNEL_SEND_TRAMPOLINE.get() else {
        return 0;
    };
    let original: ChannelSendFn = unsafe { std::mem::transmute(t) };
    let slot = (channel as u32).wrapping_add(CHANNEL_BUNDLE);
    let tagged = guarded(|| {
        if !with_joiner(|j| j.has_unsent()) {
            return None;
        }
        let bundle = LiveMem.u32_at(slot)?;
        with_joiner(|j| j.tag_bundle(bundle)).then_some(bundle)
    })
    .flatten();
    let r = unsafe { original(channel) };
    if let Some(bundle) = tagged {
        let _ = guarded(|| {
            // Still in the slot: the original had nothing to send yet.
            if LiveMem.u32_at(slot) == Some(bundle) {
                with_joiner(|j| j.untag_bundle(bundle));
            }
        });
    }
    r
}

/// Ends the sequence recording when dropped, including on an unwind.
struct Recording;

impl Recording {
    fn begin() -> Self {
        let _ = SEQS.try_with(|s| *s.borrow_mut() = Some(Vec::new()));
        Recording
    }

    fn take(&self) -> Vec<u32> {
        SEQS.try_with(|s| s.borrow_mut().take().unwrap_or_default())
            .unwrap_or_default()
    }
}

impl Drop for Recording {
    fn drop(&mut self) {
        let _ = SEQS.try_with(|s| s.borrow_mut().take());
    }
}

type NubSendFn = unsafe extern "thiscall-unwind" fn(*mut c_void, u32, *mut c_void, u32) -> u32;

/// `Nub::send(addr, bundle, channel)`. Only a tagged bundle is recorded;
/// the bundle is never read (it may be freed before this returns).
#[allow(improper_ctypes_definitions)]
pub(super) unsafe extern "thiscall-unwind" fn nub_send_detour(
    nub: *mut c_void,
    addr: u32,
    bundle: *mut c_void,
    channel: u32,
) -> u32 {
    let Some(&t) = NUB_SEND_TRAMPOLINE.get() else {
        return 0;
    };
    let original: NubSendFn = unsafe { std::mem::transmute(t) };
    let tags = guarded(|| with_joiner(|j| j.take_bundle(bundle as u32))).flatten();
    let Some(tags) = tags else {
        return unsafe { original(nub, addr, bundle, channel) };
    };
    let rec = Recording::begin();
    let r = unsafe { original(nub, addr, bundle, channel) };
    let seqs = rec.take();
    drop(rec);
    let outs = guarded(|| {
        let evicted = with_joiner(|j| j.take_evicted());
        sent_seq_outs(&tags, &seqs, evicted)
    })
    .unwrap_or_default();
    report(outs);
    r
}

type SeqNextFn = unsafe extern "thiscall-unwind" fn(*mut c_void) -> u32;

/// `FUN_0158bb40`: `eax = [ecx+0x4c]; [ecx+0x4c] = (eax + 1) & 0x0fffffff`.
#[allow(improper_ctypes_definitions)]
pub(super) unsafe extern "thiscall-unwind" fn seq_next_detour(ci: *mut c_void) -> u32 {
    let Some(&t) = SEQ_NEXT_TRAMPOLINE.get() else {
        return 0;
    };
    let original: SeqNextFn = unsafe { std::mem::transmute(t) };
    let seq = unsafe { original(ci) };
    let _ = SEQS.try_with(|s| {
        if let Ok(mut s) = s.try_borrow_mut() {
            if let Some(v) = s.as_mut() {
                if v.len() < MAX_SEQS {
                    v.push(seq);
                }
            }
        }
    });
    seq
}
