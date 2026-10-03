//! The client's Mercury receive path: datagram filter, reliable window,
//! fragment reassembly and the bundle message loop.
//!
//! | Function | Address | Signature | Thread |
//! |---|---|---|---|
//! | `Nub::processFilteredPacket` | `0x01580840` | `thiscall(nub, addr, packet) -> int`, `ret 8` | network |
//! | `UnAckedHandler::queueAckForPacket` | `0x0158cba0` | `thiscall(channel, 4 stack words) -> ptr`, `ret 0x10` | network |
//! | `Nub::processPacket` | `0x0157fd20` | `thiscall(nub, addr, packet, channel) -> int`, `ret 0xc` | network |
//! | `Nub::processOrderedPacket` | `0x0157c820` | `thiscall(nub, message) -> int`, `ret 4` | game |
//! | `Bundle::iterator::unpack` | `0x01579830` | `thiscall(iterator, element) -> ptr`, `ret 4` | game |
//!
//! What each seam tells, and the rules behind it, is in
//! `docs/reverse-engineering/findings/client-mercury-receive-path.md`. The
//! detours only read memory (through the checked reader) and call the
//! original; every decision about what to report is in the portable
//! [`crate::hooks::mercury_recv`] module, which the unit tests cover.
//!
//! Events: `client.mercury.packet_in` (per packet, gated), `client.mercury.fragment`
//! (per fragment, always), `client.mercury.bundle` (start of an assembled bundle
//! and the end of every bundle) and `client.mercury.error` (paired with every
//! non-happy one). A non-happy outcome bypasses every throttle.

use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::ffi::c_void;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::OnceLock;

use super::entity_lifecycle::guarded;
use crate::hooks::emit::emit;
use crate::hooks::entity_trace::{
    self as trace,
    map::{LiveMem, Mem},
};
use crate::hooks::mercury_recv::{
    bundle::{self, BundleTrace, Element},
    channel, delta, fragment,
    iterator::{IterRaw, ITER_BYTES},
    nub, packet,
    report::{self, Report, WindowNote},
    tail::{self, flag, Gate, Tail},
    window::{self, GapTracker, WindowState},
    Fields, MAX_DATAGRAM,
};
use crate::queue::Producer;

pub(super) const ADDR_PROCESS_FILTERED_PACKET: usize = 0x01580840;
pub(super) const ADDR_QUEUE_ACK: usize = 0x0158cba0;
pub(super) const ADDR_PROCESS_PACKET: usize = 0x0157fd20;
pub(super) const ADDR_PROCESS_ORDERED_PACKET: usize = 0x0157c820;
pub(super) const ADDR_BUNDLE_UNPACK: usize = 0x01579830;

static FILTERED_TRAMPOLINE: OnceLock<usize> = OnceLock::new();
static QUEUE_ACK_TRAMPOLINE: OnceLock<usize> = OnceLock::new();
static PROCESS_PACKET_TRAMPOLINE: OnceLock<usize> = OnceLock::new();
static ORDERED_TRAMPOLINE: OnceLock<usize> = OnceLock::new();
static UNPACK_TRAMPOLINE: OnceLock<usize> = OnceLock::new();

/// Until when (monotonic ms, `0` = never) a fragment group is treated as in
/// flight: every packet is then worth an event (the packets of an
/// assembling bundle are what the trace is for). It is refreshed by each
/// fragment that leaves a group open and cleared when one completes; the
/// window keeps an abandoned group from making every later packet noisy.
static GROUP_OPEN_UNTIL_MS: AtomicU64 = AtomicU64::new(0);

/// How long an open group keeps the per-packet events on after its last
/// fragment.
const GROUP_OPEN_WINDOW_MS: u64 = 5_000;

static EPOCH: OnceLock<std::time::Instant> = OnceLock::new();

/// Monotonic milliseconds, never `0`.
fn now_ms() -> u64 {
    EPOCH
        .get_or_init(std::time::Instant::now)
        .elapsed()
        .as_millis() as u64
        + 1
}

fn group_in_flight() -> bool {
    now_ms() < GROUP_OPEN_UNTIL_MS.load(Ordering::Relaxed)
}

fn set_group_in_flight(open: bool) {
    let until = if open {
        now_ms() + GROUP_OPEN_WINDOW_MS
    } else {
        0
    };
    GROUP_OPEN_UNTIL_MS.store(until, Ordering::Relaxed);
}

thread_local! {
    /// What `queueAckForPacket` did with the packet the filter is on
    /// (network thread).
    static NOTE: Cell<Option<WindowNote>> = const { Cell::new(None) };
    /// Each channel has its own expected reliable sequence. Entries leave
    /// when the gap closes, so reconnects cannot inherit stale state.
    static GAPS: RefCell<HashMap<u32, GapTracker>> = RefCell::new(HashMap::new());
    /// The bundle the message loop is walking (game thread).
    static BUNDLE: RefCell<Option<BundleTrace>> = const { RefCell::new(None) };
}

pub(super) unsafe fn install_all(producer: &Producer) {
    let hooks: [(&str, usize, *mut c_void, &OnceLock<usize>); 5] = [
        (
            "mercury_process_filtered_packet",
            ADDR_PROCESS_FILTERED_PACKET,
            filtered_detour as *mut c_void,
            &FILTERED_TRAMPOLINE,
        ),
        (
            "mercury_queue_ack",
            ADDR_QUEUE_ACK,
            queue_ack_detour as *mut c_void,
            &QUEUE_ACK_TRAMPOLINE,
        ),
        (
            "mercury_process_packet",
            ADDR_PROCESS_PACKET,
            process_packet_detour as *mut c_void,
            &PROCESS_PACKET_TRAMPOLINE,
        ),
        (
            "mercury_process_ordered_packet",
            ADDR_PROCESS_ORDERED_PACKET,
            ordered_detour as *mut c_void,
            &ORDERED_TRAMPOLINE,
        ),
        (
            "mercury_bundle_unpack",
            ADDR_BUNDLE_UNPACK,
            unpack_detour as *mut c_void,
            &UNPACK_TRAMPOLINE,
        ),
    ];
    for (name, addr, detour, slot) in hooks {
        super::install_one(producer, name, addr, detour, slot);
    }
}

/// Emit a report through its gate, and its paired error if it has one.
fn emit_report(r: Report, stage: &'static str) {
    let fields = match r.gate {
        Gate::Always => r.fields.clone(),
        Gate::Throttled => {
            match trace::with_suppressed(r.fields.clone(), trace::throttle(r.target, 0)) {
                Some(f) => f,
                None => return,
            }
        }
    };
    let error = r
        .error
        .as_deref()
        .map(|reason| report::error_fields(stage, reason, &fields));
    emit(r.target, r.level, fields);
    if let Some(e) = error {
        emit("client.mercury.error", "warn", e);
    }
}

/// `len` and the bytes of `packet`'s data, or `None` if unreadable or
/// implausible.
fn packet_bytes(pkt: u32) -> Option<(usize, Vec<u8>)> {
    let len = LiveMem.u32_at(pkt.wrapping_add(packet::LEN))? as usize;
    if len == 0 || len > MAX_DATAGRAM {
        return None;
    }
    let bytes = LiveMem.bytes_at(pkt.wrapping_add(packet::DATA), len)?;
    Some((len, bytes))
}

fn window_state(channel_ptr: u32) -> Option<WindowState> {
    Some(WindowState {
        in_seq_at: LiveMem.u32_at(channel_ptr.wrapping_add(channel::IN_SEQ_AT))?,
        buffered: LiveMem.u32_at(channel_ptr.wrapping_add(channel::BUFFERED))?,
    })
}

// ---------------------------------------------------------------------
// processFilteredPacket

type FilteredFn = unsafe extern "thiscall-unwind" fn(*mut c_void, *mut c_void, *mut c_void) -> i32;

/// The datagram filter: every packet the client receives passes through it.
/// The footers are parsed from a copy before the original strips them.
#[allow(improper_ctypes_definitions)]
unsafe extern "thiscall-unwind" fn filtered_detour(
    this: *mut c_void,
    addr: *mut c_void,
    pkt: *mut c_void,
) -> i32 {
    let Some(&t) = FILTERED_TRAMPOLINE.get() else {
        return 0;
    };
    let original: FilteredFn = std::mem::transmute(t);
    let parsed: Option<(Tail, String)> = guarded(|| {
        packet_bytes(pkt as u32).map(|(_, b)| {
            (
                tail::parse(&b),
                crate::hooks::mercury_recv::wire::fingerprint(&b),
            )
        })
    })
    .flatten();
    let _ = NOTE.try_with(|c| c.set(None));
    let bad_before = LiveMem.u32_at((this as u32).wrapping_add(nub::BAD_PACKETS));
    let result = original(this, addr, pkt);
    guarded(|| {
        let Some((tail, fingerprint)) = parsed else {
            return;
        };
        let note = NOTE.try_with(Cell::take).unwrap_or(None);
        let bad = delta(
            bad_before,
            LiveMem.u32_at((this as u32).wrapping_add(nub::BAD_PACKETS)),
        );
        let r = report::with_filter_input_fingerprint(
            report::packet(&tail, note.as_ref(), result, bad, group_in_flight()),
            &fingerprint,
        );
        emit_report(r, "packet");
    });
    result
}

// ---------------------------------------------------------------------
// queueAckForPacket

type QueueAckFn =
    unsafe extern "thiscall-unwind" fn(*mut c_void, usize, usize, usize, usize) -> usize;

/// The reliable window. Its four stack words are forwarded untouched; only
/// the channel (`this`) is read, before and after.
#[allow(improper_ctypes_definitions)]
unsafe extern "thiscall-unwind" fn queue_ack_detour(
    this: *mut c_void,
    a1: usize,
    a2: usize,
    a3: usize,
    a4: usize,
) -> usize {
    let Some(&t) = QUEUE_ACK_TRAMPOLINE.get() else {
        return 0;
    };
    let original: QueueAckFn = std::mem::transmute(t);
    let chan = this as u32;
    let before = guarded(|| window_state(chan)).flatten();
    let result = original(this, a1, a2, a3, a4);
    guarded(|| {
        let (Some(before), Some(after)) = (before, window_state(chan)) else {
            return;
        };
        let window = LiveMem
            .u32_at(chan.wrapping_add(channel::WINDOW))
            .unwrap_or(0);
        let _ = NOTE.try_with(|c| {
            c.set(Some(WindowNote {
                before,
                after,
                window,
            }))
        });
        let events = GAPS.try_with(|cell| {
            let mut gaps = cell.borrow_mut();
            let tracker = gaps.entry(chan).or_default();
            let events = tracker.observe(before, after, now_ms());
            if !tracker.is_open() {
                gaps.remove(&chan);
            }
            events
        });
        if let Ok(events) = events {
            for gap in events {
                emit(
                    "client.mercury.rx_gap",
                    if gap.event == "rx_gap_stall" {
                        "warn"
                    } else {
                        "info"
                    },
                    window::gap_fields(gap, chan, a4 as u32),
                );
            }
        }
    });
    result
}

// ---------------------------------------------------------------------
// processPacket

type ProcessPacketFn =
    unsafe extern "thiscall-unwind" fn(*mut c_void, *mut c_void, *mut c_void, *mut c_void) -> i32;

/// Fragment reassembly. Only a fragment (flag `0x20`) costs more than one
/// read; the channel's group is read before and after the original.
#[allow(improper_ctypes_definitions)]
unsafe extern "thiscall-unwind" fn process_packet_detour(
    this: *mut c_void,
    addr: *mut c_void,
    pkt: *mut c_void,
    chan: *mut c_void,
) -> i32 {
    let Some(&t) = PROCESS_PACKET_TRAMPOLINE.get() else {
        return 0;
    };
    let original: ProcessPacketFn = std::mem::transmute(t);
    let pre = guarded(|| read_fragment(pkt as u32, chan as u32)).flatten();
    let result = original(this, addr, pkt, chan);
    if let Some((tail, pre_group)) = pre {
        guarded(|| {
            let post = (!chan.is_null())
                .then(|| fragment::read_group(&LiveMem, chan as u32))
                .flatten()
                .flatten();
            let outcome = if chan.is_null() {
                fragment::Outcome::NoChannel
            } else {
                fragment::classify(&tail, pre_group.as_ref(), post.as_ref(), result)
            };
            set_group_in_flight(post.is_some());
            emit_report(
                report::fragment(&tail, &outcome, pre_group.as_ref(), post.as_ref()),
                "fragment",
            );
        });
    }
    result
}

/// The fragment's footers and the channel's open group, or `None` for a
/// packet that is not a fragment.
fn read_fragment(pkt: u32, chan: u32) -> Option<(Tail, Option<fragment::GroupView>)> {
    let flags = LiveMem.u32_at(pkt.wrapping_add(packet::DATA))? as u8;
    if flags & flag::FRAGMENT == 0 {
        return None;
    }
    let (_, bytes) = packet_bytes(pkt)?;
    let seq = LiveMem.u32_at(pkt.wrapping_add(packet::SEQ))?;
    let tail = tail::at_process_packet(&bytes, seq);
    let group = if chan == 0 {
        None
    } else {
        fragment::read_group(&LiveMem, chan).flatten()
    };
    Some((tail, group))
}

// ---------------------------------------------------------------------
// processOrderedPacket and unpack

type OrderedFn = unsafe extern "thiscall-unwind" fn(*mut c_void, *mut c_void) -> i32;

/// Ends the bundle on the way out, including a C++ exception unwinding
/// through the original: the trace moves to [`BUNDLE_LAST`] for the report
/// and `BUNDLE` is empty again, so a stale trace never tags the next bundle.
struct BundleGuard;

impl Drop for BundleGuard {
    fn drop(&mut self) {
        if let Ok(Some(t)) = BUNDLE.try_with(|b| b.replace(None)) {
            let _ = BUNDLE_LAST.try_with(|c| c.set(Some(t)));
        }
    }
}

/// The message loop of one bundle.
#[allow(improper_ctypes_definitions)]
unsafe extern "thiscall-unwind" fn ordered_detour(this: *mut c_void, msg: *mut c_void) -> i32 {
    let Some(&t) = ORDERED_TRAMPOLINE.get() else {
        return 0;
    };
    let original: OrderedFn = std::mem::transmute(t);
    let nub_ptr = this as u32;
    let _ = BUNDLE_LAST.try_with(|c| c.set(None));
    let before = guarded(|| begin_bundle(nub_ptr, msg as u32)).flatten();
    let result = {
        let _guard = BundleGuard;
        original(this, msg)
    };
    let Some((dispatched_before, aborted_before)) = before else {
        return result;
    };
    guarded(|| {
        let Some(trace) = BUNDLE_LAST.try_with(Cell::take).ok().flatten() else {
            return;
        };
        let dispatched = delta(
            dispatched_before,
            LiveMem.u32_at(nub_ptr.wrapping_add(nub::DISPATCHED)),
        );
        let aborted = delta(
            aborted_before,
            LiveMem.u32_at(nub_ptr.wrapping_add(nub::BUNDLES_ABORTED)),
        );
        let next_id = (bundle::exit_of(result) == bundle::Exit::UnknownMessageId
            && trace.fault.is_none())
        .then(|| trace.position_of(trace.consumed))
        .flatten()
        .and_then(|(ptr, off)| LiveMem.bytes_at(ptr.wrapping_add(off), 1))
        .map(|b| b[0]);
        emit_report(
            report::bundle_end(&trace, result, dispatched, aborted, next_id),
            "bundle",
        );
    });
    result
}

thread_local! {
    /// The finished trace, handed from the guard to the report.
    static BUNDLE_LAST: Cell<Option<BundleTrace>> = const { Cell::new(None) };
}

/// Start tracing the bundle in `msg`: read its packet chain, remember the
/// nub's dispatch and abort counters, report an assembled bundle's start.
/// Returns the counters, or `None` if the message could not be read.
fn begin_bundle(nub_ptr: u32, msg: u32) -> Option<(Option<u32>, Option<u32>)> {
    // ClientIncomingMessage +0x10 -> bundle object, whose +4 is the head
    // packet (`0x0157c820`: `*(*(param_1 + 0x10) + 4)`).
    let bundle_obj = LiveMem.u32_at(msg.wrapping_add(0x10))?;
    let head = LiveMem.u32_at(bundle_obj.wrapping_add(4))?;
    let chain = bundle::read_chain(&LiveMem, head)?;
    let trace = BundleTrace::new(chain);
    if let Some(r) = report::bundle_start(&trace) {
        emit_report(r, "bundle");
    }
    let _ = BUNDLE.try_with(|b| b.replace(Some(trace)));
    Some((
        LiveMem.u32_at(nub_ptr.wrapping_add(nub::DISPATCHED)),
        LiveMem.u32_at(nub_ptr.wrapping_add(nub::BUNDLES_ABORTED)),
    ))
}

type UnpackFn = unsafe extern "thiscall-unwind" fn(*mut c_void, *mut c_void) -> *mut c_void;

/// One message header parsed by the loop. Outside a traced bundle this is a
/// pass-through; inside, the iterator is read before and after.
#[allow(improper_ctypes_definitions)]
unsafe extern "thiscall-unwind" fn unpack_detour(
    this: *mut c_void,
    element: *mut c_void,
) -> *mut c_void {
    let Some(&t) = UNPACK_TRAMPOLINE.get() else {
        return std::ptr::null_mut();
    };
    let original: UnpackFn = std::mem::transmute(t);
    // Allocation-free look at the iterator the client is about to use.
    let entry = guarded(|| IterEntry::read(this as u32)).flatten();
    let tracing = BUNDLE.try_with(|b| b.borrow().is_some()).unwrap_or(false);
    let pre = tracing
        .then(|| guarded(|| read_iter(this as u32)).flatten())
        .flatten();
    let result = original(this, element);
    if let Some(e) = entry {
        guarded(|| e.report(this as u32, element as u32));
    }
    if let Some(pre) = pre {
        guarded(|| {
            let (Some(post), elem) = (read_iter(this as u32), read_element(element as u32)) else {
                return;
            };
            // On an error the iterator's length fields are stale; the
            // message's own bytes name the fault.
            let raw = (post.unpacked().flag == 0x20)
                .then(|| {
                    let s = pre.state();
                    let left = usize::from(s.packet_len).saturating_sub(usize::from(s.cursor));
                    LiveMem.bytes_at(
                        s.packet
                            .wrapping_add(packet::DATA)
                            .wrapping_add(u32::from(s.cursor)),
                        left.min(24),
                    )
                })
                .flatten();
            let _ = BUNDLE.try_with(|b| {
                if let Some(t) = b.borrow_mut().as_mut() {
                    t.on_unpack_with_bytes(pre.state(), post.unpacked(), elem, raw.as_deref());
                }
            });
        });
    }
    result
}

/// The iterator fields `unpack` decides with, read before the call.
///
/// The client's `Bundle::iterator` copy constructor (`0x01578e90`) never
/// initializes `+0x14`, the next-request offset, so it holds stack residue.
/// When it equals the cursor, `unpack` treats an ordinary message as a
/// request: it reads a 4-byte reply id and a 2-byte next-request offset
/// out of the message body and misparses everything after it.
#[derive(Clone, Copy)]
struct IterEntry {
    packet: u32,
    packet_len: u16,
    cursor: u16,
    next_request: u16,
    /// The packet's flags byte (payload byte 0); `0x01` = has requests.
    packet_flags: u8,
}

impl IterEntry {
    /// Fault-safe reads (`ReadProcessMemory`): a bad pointer is a `None`,
    /// never an access violation inside the client.
    fn read(it: u32) -> Option<Self> {
        let raw = LiveMem.bytes_at(it, 0x16)?;
        let u16_at = |o: usize| u16::from_le_bytes([raw[o], raw[o + 1]]);
        let packet = u32::from_le_bytes(raw[0..4].try_into().ok()?);
        let packet_flags = LiveMem.bytes_at(packet.wrapping_add(packet::DATA), 1)?[0];
        Some(Self {
            packet,
            packet_len: u16_at(4),
            cursor: u16_at(6),
            next_request: u16_at(0x14),
            packet_flags,
        })
    }

    /// Takes the request path although the packet carries no requests.
    fn is_request_misparse(&self) -> bool {
        self.next_request == self.cursor && self.packet_flags & 0x01 == 0
    }

    /// Report a request misparse and any unpack fault, with the client's
    /// own post-call numbers and the message's raw bytes.
    fn report(&self, it: u32, element: u32) {
        let post = read_iter(it);
        let failed = post.map(|p| p.unpacked().flag == 0x20).unwrap_or(false);
        let misparse = self.is_request_misparse();
        if !misparse && !failed {
            return;
        }
        let left = usize::from(self.packet_len).saturating_sub(usize::from(self.cursor));
        let bytes = LiveMem
            .bytes_at(
                self.packet
                    .wrapping_add(packet::DATA)
                    .wrapping_add(u32::from(self.cursor)),
                left.min(24),
            )
            .map(|b| b.iter().map(|x| format!("{x:02x}")).collect::<String>());
        let elem = read_element(element);
        let mut fields: Fields = vec![
            ("cursor", serde_json::json!(self.cursor)),
            ("packet_len", serde_json::json!(self.packet_len)),
            ("next_request_offset", serde_json::json!(self.next_request)),
            ("packet_flags", serde_json::json!(self.packet_flags)),
            (
                "request_path",
                serde_json::json!(self.next_request == self.cursor),
            ),
            ("request_misparse", serde_json::json!(misparse)),
            ("failed", serde_json::json!(failed)),
            ("bytes", serde_json::json!(bytes)),
            ("elem_style", serde_json::json!(elem.map(|e| e.style))),
            ("elem_param", serde_json::json!(elem.map(|e| e.param))),
        ];
        if let Some(p) = post {
            let u = p.unpacked();
            fields.push(("msg_id", serde_json::json!(u.msg_id)));
            fields.push(("decoded_len", serde_json::json!(u.decoded_len)));
        }
        let target = if misparse {
            "client.mercury.request_misparse"
        } else {
            "client.mercury.unpack_fault"
        };
        emit(target, "warn", fields);
    }
}

fn read_iter(it: u32) -> Option<IterRaw> {
    IterRaw::parse(&LiveMem.bytes_at(it, ITER_BYTES)?)
}

fn read_element(el: u32) -> Option<Element> {
    Element::parse(&LiveMem.bytes_at(el, 8)?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    static SERIAL: Mutex<()> = Mutex::new(());

    /// The detour hands its stack words through untouched and returns the
    /// original's value, even for a wild channel pointer (every read is
    /// checked, so nothing faults).
    #[test]
    fn queue_ack_detour_forwards_all_four_stack_words() {
        let _s = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
        static SEEN: [std::sync::atomic::AtomicUsize; 5] =
            [const { std::sync::atomic::AtomicUsize::new(0) }; 5];
        unsafe extern "thiscall-unwind" fn original(
            this: *mut c_void,
            a1: usize,
            a2: usize,
            a3: usize,
            a4: usize,
        ) -> usize {
            SEEN[0].store(this as usize, Ordering::SeqCst);
            SEEN[1].store(a1, Ordering::SeqCst);
            SEEN[2].store(a2, Ordering::SeqCst);
            SEEN[3].store(a3, Ordering::SeqCst);
            SEEN[4].store(a4, Ordering::SeqCst);
            0xABCD
        }
        QUEUE_ACK_TRAMPOLINE
            .set(original as *const () as usize)
            .expect("only this test sets the trampoline");
        let r = unsafe { queue_ack_detour(0x1000 as *mut c_void, 1, 2, 3, 4) };
        assert_eq!(r, 0xABCD);
        let seen: Vec<usize> = SEEN.iter().map(|s| s.load(Ordering::SeqCst)).collect();
        assert_eq!(seen, vec![0x1000, 1, 2, 3, 4]);
    }

    /// The filter detour forwards the result and its three arguments; a
    /// null packet (nothing readable) reports nothing and cannot fault.
    #[test]
    fn filtered_detour_forwards_arguments_and_result() {
        let _s = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
        static SEEN: [std::sync::atomic::AtomicUsize; 3] =
            [const { std::sync::atomic::AtomicUsize::new(0) }; 3];
        unsafe extern "thiscall-unwind" fn original(
            this: *mut c_void,
            addr: *mut c_void,
            pkt: *mut c_void,
        ) -> i32 {
            SEEN[0].store(this as usize, Ordering::SeqCst);
            SEEN[1].store(addr as usize, Ordering::SeqCst);
            SEEN[2].store(pkt as usize, Ordering::SeqCst);
            -11
        }
        FILTERED_TRAMPOLINE
            .set(original as *const () as usize)
            .expect("only this test sets the trampoline");
        let r = unsafe {
            filtered_detour(
                0x1000 as *mut c_void,
                0x2000 as *mut c_void,
                std::ptr::null_mut(),
            )
        };
        assert_eq!(r, -11);
        assert_eq!(SEEN[0].load(Ordering::SeqCst), 0x1000);
        assert_eq!(SEEN[1].load(Ordering::SeqCst), 0x2000);
        assert_eq!(SEEN[2].load(Ordering::SeqCst), 0);
    }

    /// An open group turns the per-packet events on for a while, and a
    /// completed one turns them off again.
    #[test]
    fn a_group_in_flight_expires() {
        let _s = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
        set_group_in_flight(true);
        assert!(group_in_flight());
        set_group_in_flight(false);
        assert!(!group_in_flight());
    }

    /// A C++ exception out of the message loop must not leave a stale trace
    /// on the game thread (it would be attributed to the next bundle).
    #[test]
    fn a_throwing_message_loop_leaves_no_bundle_behind() {
        let _s = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
        unsafe extern "thiscall-unwind" fn original(_: *mut c_void, _: *mut c_void) -> i32 {
            BUNDLE.with(|b| {
                b.replace(Some(BundleTrace::new(vec![bundle::PacketView {
                    ptr: 1,
                    len: 10,
                    seq: 1,
                }])))
            });
            panic!("engine error");
        }
        ORDERED_TRAMPOLINE
            .set(original as *const () as usize)
            .expect("only this test sets the trampoline");
        let caught = std::panic::catch_unwind(|| unsafe {
            ordered_detour(0x1000 as *mut c_void, 0x2000 as *mut c_void)
        });
        assert!(caught.is_err());
        assert!(BUNDLE.with(|b| b.borrow().is_none()));
    }

    /// Outside a traced bundle `unpack` is a pass-through that reads no
    /// iterator memory (the iterator pointer here is wild).
    #[test]
    fn unpack_detour_is_a_pass_through_outside_a_bundle() {
        let _s = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
        unsafe extern "thiscall-unwind" fn original(
            it: *mut c_void,
            el: *mut c_void,
        ) -> *mut c_void {
            (it as usize + el as usize) as *mut c_void
        }
        UNPACK_TRAMPOLINE
            .set(original as *const () as usize)
            .expect("only this test sets the trampoline");
        let r = unsafe { unpack_detour(0x10 as *mut c_void, 0x20 as *mut c_void) };
        assert_eq!(r as usize, 0x30);
    }
}
