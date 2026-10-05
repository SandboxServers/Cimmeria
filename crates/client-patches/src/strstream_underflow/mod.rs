//! A repair for the C++ runtime under Wine: `strstreambuf::underflow` in
//! Wine's `msvcp80.dll` returns end of file for a read that follows a write.
//!
//! The client reads its cooked-data cache through a `std::strstream`: it
//! writes an archive entry into a fresh dynamic `strstreambuf` and reads it
//! straight back (`SGW.exe` `0x00478e10`, `0x00478f00`; the stream
//! constructor `0x00478970` has 22 call sites). `underflow` is what makes
//! written bytes readable: it raises the high-water mark `_Seekhigh` to the
//! put pointer and widens the get area. Wine's reads the get pointer where
//! it means the put pointer (`dlls/msvcp90/ios.c`, `strstreambuf_underflow`),
//! so the mark never moves and the read gets nothing. The client does not
//! check, keeps version 0 for every cooked category, and is resynced in full
//! at every login. The evidence is Finding 9 of
//! `docs/reverse-engineering/findings/cooked-data-pipeline.md`.
//!
//! The repair, in a process running under Wine only:
//!
//! 1. **Probe.** Build a `strstreambuf(0)` through `msvcp80.dll`'s own
//!    exports, write four bytes, read four back. A runtime that returns them
//!    is left alone. One that does not is patched only if the object looks
//!    as this module expects after the write: four bytes in the put area,
//!    and the mark still at the buffer's start.
//! 2. **Swap one vtable slot.** The slot of `strstreambuf`'s vtable that
//!    holds the address `?underflow@strstreambuf@std@@MAEHXZ` exports is
//!    pointed at the shim (`runtime.rs`). The slot is found by that address, not by
//!    an index, and nothing is written if it is not found exactly once.
//! 3. **Probe again.** If the round trip still fails, the slot is put back.
//!
//! The shim raises `_Seekhigh` to the put pointer, as Microsoft's
//! `underflow` does first, and then calls the runtime's own function, which
//! with the mark raised does the rest correctly. On a runtime without the
//! fault that step is a no-op, so the shim is safe to leave in place if Wine
//! is fixed underneath it; the probe just stops installing it.
//!
//! No address in `SGW.exe` is touched, so this is outside the build
//! fingerprint's sites; [`boot`](crate::boot) still runs it only after the
//! fingerprint says this process is the client.

/// Offsets in a `strstreambuf` on i686: the MSVC 8 layout, which Wine's
/// `msvcp80.dll` has to reproduce because the client's inline code uses it.
pub mod layout {
    /// `basic_streambuf<char>::_IPfirst`: pointer to the put area's first
    /// pointer.
    pub const PUT_FIRST_PTR: usize = 0x14;
    /// `basic_streambuf<char>::_IPnext`: pointer to the put pointer.
    pub const PUT_NEXT_PTR: usize = 0x24;
    /// `strstreambuf::_Seekhigh`: one past the last character written.
    pub const SEEK_HIGH: usize = 0x44;
    /// `sizeof(strstreambuf)`.
    pub const SIZE: usize = 0x54;
}

/// Bytes the probe writes and expects back.
pub const PROBE: [u8; 4] = [0x80, 0x82, 0x00, 0x00];

/// Virtual functions in `strstreambuf`'s vtable.
pub const VTABLE_SLOTS: usize = 14;

/// The value `_Seekhigh` must take before an underflow, when writes have
/// passed it. `None` when it is already right or nothing was written.
pub fn raised(seek_high: u32, put_next: u32) -> Option<u32> {
    (put_next != 0 && seek_high < put_next).then_some(put_next)
}

/// One write-then-read through a fresh dynamic `strstreambuf`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RoundTrip {
    /// What `sputn` returned.
    pub written: i32,
    /// What `sgetn` returned.
    pub read: i32,
    /// Whether the bytes read are the bytes written.
    pub intact: bool,
    /// Put pointer minus the put area's start, after the write.
    pub put_span: Option<u32>,
    /// Whether `_Seekhigh` still sat at the put area's start after the write.
    pub mark_at_start: Option<bool>,
}

/// What a round trip says about the runtime.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Verdict {
    /// It reads back what it writes.
    Healthy,
    /// It loses the write, and the object is laid out as the shim expects.
    LosesWrites,
    /// The read is short, but not in the shape this module repairs.
    Unrecognised,
    /// The write itself did not go in.
    WriteFailed,
}

/// Judge one round trip.
pub fn verdict(trip: &RoundTrip) -> Verdict {
    let len = PROBE.len() as i32;
    if trip.written != len {
        Verdict::WriteFailed
    } else if trip.read == len && trip.intact {
        Verdict::Healthy
    } else if trip.put_span == Some(len as u32) && trip.mark_at_start == Some(true) {
        Verdict::LosesWrites
    } else {
        Verdict::Unrecognised
    }
}

/// The index of the one slot holding `target`. `None` when it is absent or
/// there are two: either way the vtable is not the one this module knows.
pub fn find_slot(slots: &[u32], target: u32) -> Option<usize> {
    let mut hits = slots
        .iter()
        .enumerate()
        .filter(|(_, slot)| **slot == target);
    match (hits.next(), hits.next()) {
        (Some((index, _)), None) if target != 0 => Some(index),
        _ => None,
    }
}

/// How the repair ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outcome {
    /// Not Wine: Microsoft's runtime does not have the fault.
    NotWine,
    /// `msvcp80.dll` is not loaded or lacks an export the probe needs.
    RuntimeIncomplete(&'static str),
    /// The probe faulted or raised.
    ProbeFaulted,
    /// The runtime reads back what it writes.
    Healthy,
    /// The runtime fails the probe in a way this module does not repair.
    NotRepairable(Verdict, RoundTrip),
    /// The vtable slot was not found, or could not be written.
    SlotUnavailable(&'static str),
    /// The shim went in and the probe still failed; the slot was restored.
    StillBroken(RoundTrip),
    /// The shim is in and the round trip now works.
    Repaired { before: RoundTrip, slot: usize },
}

impl Outcome {
    /// Whether the shim is installed.
    pub fn installed(&self) -> bool {
        matches!(self, Outcome::Repaired { .. })
    }
}

/// The log line for an outcome.
pub fn describe(outcome: &Outcome) -> String {
    match outcome {
        Outcome::NotWine => {
            "strstream: not running under Wine; the C++ runtime is left alone".into()
        }
        Outcome::RuntimeIncomplete(what) => {
            format!("strstream: {what}; the C++ runtime is left alone")
        }
        Outcome::ProbeFaulted => {
            "strstream: the read-after-write probe faulted; the C++ runtime is left alone".into()
        }
        Outcome::Healthy => {
            "strstream: msvcp80.dll reads back what it writes; no repair needed".into()
        }
        Outcome::NotRepairable(verdict, trip) => format!(
            "strstream: msvcp80.dll fails the read-after-write probe in a way this build does \
             not repair ({verdict:?}: wrote {}, read {}, put span {:?}, mark at start {:?}); \
             the C++ runtime is left alone",
            trip.written, trip.read, trip.put_span, trip.mark_at_start
        ),
        Outcome::SlotUnavailable(why) => format!(
            "strstream: msvcp80.dll loses a write before a read, and its underflow slot {why}; \
             not repaired"
        ),
        Outcome::StillBroken(trip) => format!(
            "strstream: the underflow shim did not help (wrote {}, read {}); slot restored, \
             not repaired",
            trip.written, trip.read
        ),
        Outcome::Repaired { before, slot } => format!(
            "strstream: msvcp80.dll loses a write before a read (wrote {}, read {}); underflow \
             shim installed in vtable slot {slot}, the round trip now reads {}",
            before.written,
            before.read,
            PROBE.len()
        ),
    }
}

#[cfg(all(windows, target_arch = "x86"))]
mod runtime;
#[cfg(test)]
mod tests;

#[cfg(all(windows, target_arch = "x86"))]
pub(crate) use runtime::repair;
