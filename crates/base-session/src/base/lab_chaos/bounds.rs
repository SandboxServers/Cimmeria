//! The bounds every knob is held to. [`super::LabChaos::set`] refuses a value
//! outside them rather than clamping it, so a run never silently tests
//! something other than what it asked for.

use super::{ChaosRefused, FlushChaos, NetChaos};

pub(super) fn check_net(net: &NetChaos) -> Result<(), ChaosRefused> {
    if net.loss_per_thousand > 1000 {
        return Err(ChaosRefused::BadKnob("net loss is over 1000 per thousand"));
    }
    if net.dup_per_thousand > 1000 {
        return Err(ChaosRefused::BadKnob("net dup is over 1000 per thousand"));
    }
    if net.latency_ms > 5000 {
        return Err(ChaosRefused::BadKnob("net latency is over 5000 ms"));
    }
    if net.jitter_ms > 5000 {
        return Err(ChaosRefused::BadKnob("net jitter is over 5000 ms"));
    }
    if net.reorder > 1000 {
        return Err(ChaosRefused::BadKnob("net reorder is over 1000 packets"));
    }
    Ok(())
}

/// The longest shift filler: `updateFrequencyNotification` (2 bytes) plus a
/// 36-byte `voiceData`. 38 is the 1-byte residue class plus one record
/// (37), which reaches every cursor a 1-byte filler would (CH-07 filler
/// design, `docs/analysis/lab-chaos/worknotes/CH-07-filler.md`).
pub const SHIFT_LEN_MAX: u16 = 38;

/// One phase-1 introduction record: CREATE_ENTITY (11 bytes) then
/// UPDATE_AVATAR (26 bytes).
pub const PHASE1_RECORD_LEN: u16 = 37;

/// The most records a single-packet phase-1 bundle holds with the longest
/// filler ahead of them (34). `Pack` is refused above it rather than
/// clamped, so a run never silently tests a smaller bundle than it asked for.
pub const PACK_RECORDS_MAX: u16 = ((cimmeria_mercury::packet::FRAGMENT_BODY_SIZE
    - SHIFT_LEN_MAX as usize)
    / PHASE1_RECORD_LEN as usize) as u16;

pub(super) fn check_flush(flush: &FlushChaos) -> Result<(), ChaosRefused> {
    match *flush {
        FlushChaos::Delay { ms } if ms > 60_000 => {
            Err(ChaosRefused::BadKnob("flush delay is over 60000 ms"))
        }
        FlushChaos::Pack { records } if !(1..=PACK_RECORDS_MAX).contains(&records) => Err(
            ChaosRefused::BadKnob("flush pack records is not 1 to 34 (one packet)"),
        ),
        FlushChaos::Shift { len } if len != 0 && !(2..=SHIFT_LEN_MAX).contains(&len) => {
            Err(ChaosRefused::BadKnob("flush shift len is not 0 or 2 to 38"))
        }
        _ => Ok(()),
    }
}
