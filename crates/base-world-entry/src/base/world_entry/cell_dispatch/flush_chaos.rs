//! Lab chaos flush modes (lab-chaos campaign, CH-07): disturb one session's
//! deferred-AoI flush on demand, so the lab can reproduce #1341-class bundle
//! faults against the real client. Compiled only with the `lab-chaos`
//! feature. With no registry installed, or nothing set for the session,
//! [`apply`] changes nothing and the flush is byte-identical.
//!
//! - `Delay { ms }`: the flush waits `ms` before it sends anything.
//! - `Reorder { seed }`: a seeded shuffle of the introductions inside each
//!   lifecycle segment. An entity's phase-1 record and its cascade move
//!   together, and nothing else moves.
//! - `Pack { records }`: the first `records` phase-1 records go in one
//!   single-packet bundle, the shape before #1341's mitigation; the rest
//!   follow in a second phase-1 bundle.
//! - `Shift { len }` / `ShiftSweep { seed }`: the Pack shape with as many
//!   records as fit in one packet, and `len` bytes of filler ahead of them,
//!   which moves every record's cursor in the client's bundle loop by `len`.
//!   Filler design and evidence: `docs/analysis/lab-chaos/worknotes/CH-07-filler.md`.

use std::net::SocketAddr;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};

use cimmeria_base_session::base::lab_chaos::FlushChaos;
use cimmeria_base_session::base::lab_chaos::LabChaos;
use cimmeria_base_session::base::lab_chaos::PHASE1_RECORD_LEN;
use cimmeria_base_session::base::lab_chaos::SHIFT_LEN_MAX;
use cimmeria_mercury::packet::FRAGMENT_BODY_SIZE;

use crate::mercury::game_clock::UPDATE_FREQUENCY_HZ;

use super::super::super::deferred_aoi::DeferredAoiMsg;
use super::intro_bundles::Phase1Shape;

/// `updateFrequencyNotification`, CONSTANT_LENGTH 1: always 2 bytes.
const MSG_UPDATE_FREQUENCY_NOTIFICATION: u8 = 0x02;
/// `voiceData`, WORD_LENGTH: a 3-byte header (id, u16 LE length), then the
/// body. With no voice handler registered the client drops it.
const MSG_VOICE_DATA: u8 = 0x33;
const UPDATE_FREQUENCY_LEN: u16 = 2;
const VOICE_DATA_HEADER_LEN: u16 = 3;
/// The longest single `voiceData` filler. 37 and 38 add an
/// `updateFrequencyNotification` instead, so the 1-byte residue class is
/// reached with one record less (`L + 37`), which a 1-byte message cannot.
const VOICE_DATA_MAX_LEN: u16 = 36;

/// Counts `ShiftSweep` draws in this process, so each flush draws a new
/// length. The draw index is logged with the length, so a run's lengths can
/// be read back from its log even when several sessions sweep at once.
static SWEEP_DRAWS: AtomicU64 = AtomicU64::new(0);

/// The flush's chaos, read once per flush. `witness_id` is the flushing
/// player's entity. Returns the wait before sending and the phase-1 shape.
pub(super) fn apply(
    addr: SocketAddr,
    witness_id: u32,
    witness_name: Option<&'static str>,
    segments: &mut [Vec<DeferredAoiMsg>],
) -> (Option<Duration>, Phase1Shape) {
    let unchanged = (None, Phase1Shape::default());
    let Some(registry) = registry() else {
        return unchanged;
    };
    // A logOff to character select keeps the session's address but ends the
    // player entity; chaos set for the last character must not reach the
    // next one (CH-04 worknote).
    registry.clear_if_not(addr, witness_id);
    let Some(mode) = registry.flush_for(addr, Instant::now()) else {
        return unchanged;
    };
    // A flush with no introductions (the self-method flush ahead of a
    // cinematic hold) has nothing to disturb. Leaving it alone keeps a delay
    // on the introductions and keeps sweep draws to flushes that use them.
    let introductions: usize = segments.iter().map(|s| introduction_slots(s).len()).sum();
    if introductions == 0 {
        return unchanged;
    }
    match mode {
        FlushChaos::Delay { ms } => {
            tracing::info!(
                target: "lab.chaos",
                event = "flush_delayed",
                witness_id,
                witness_name,
                delay_ms = ms,
                introductions,
                "lab chaos: deferred-AoI flush held back"
            );
            (
                Some(Duration::from_millis(u64::from(ms))),
                Phase1Shape::default(),
            )
        }
        FlushChaos::Reorder { seed } => {
            let mut rng = SplitMix64(seed);
            for segment in segments.iter_mut() {
                shuffle_introductions(segment, &mut rng);
            }
            tracing::info!(
                target: "lab.chaos",
                event = "flush_reordered",
                witness_id,
                witness_name,
                seed,
                introductions,
                "lab chaos: deferred-AoI introductions shuffled"
            );
            unchanged
        }
        FlushChaos::Pack { records } => {
            tracing::info!(
                target: "lab.chaos",
                event = "flush_packed",
                witness_id,
                witness_name,
                records,
                introductions,
                "lab chaos: first phase-1 records packed into one packet"
            );
            (
                None,
                Phase1Shape {
                    filler: Vec::new(),
                    first_bundle_records: Some(usize::from(records)),
                },
            )
        }
        FlushChaos::Shift { len } => (None, shift_shape(len, witness_id, witness_name)),
        FlushChaos::ShiftSweep { seed } => {
            let draw = SWEEP_DRAWS.fetch_add(1, Ordering::Relaxed);
            let len = sweep_len(seed, draw);
            tracing::info!(
                target: "lab.chaos",
                event = "flush_shift_swept",
                witness_id,
                witness_name,
                seed,
                draw,
                len,
                "lab chaos: shift sweep drew a filler length"
            );
            (None, shift_shape(len, witness_id, witness_name))
        }
    }
}

/// Shift implies the Pack shape: under #1341's one-message-per-packet
/// mitigation every message sits at cursor 1, and a filler in its own
/// packet would shift nothing. So the first phase-1 bundle holds the filler
/// and as many records as fit in one packet behind it.
fn shift_shape(len: u16, witness_id: u32, witness_name: Option<&'static str>) -> Phase1Shape {
    let Some(filler) = filler(len) else {
        // `LabChaos::set` refuses these lengths, so only a registry bug
        // reaches here; send the ordinary flush rather than a wrong shift.
        tracing::warn!(
            target: "lab.chaos",
            event = "flush_shift_refused",
            witness_id,
            witness_name,
            len,
            "lab chaos: no filler has this length, flush sent unshifted"
        );
        return Phase1Shape::default();
    };
    let records = (FRAGMENT_BODY_SIZE - usize::from(len)) / usize::from(PHASE1_RECORD_LEN);
    tracing::info!(
        target: "lab.chaos",
        event = "flush_shift_packs",
        witness_id,
        witness_name,
        len,
        filler_messages = filler.len(),
        first_bundle_records = records,
        "lab chaos: phase-1 records packed into one packet behind the shift filler"
    );
    Phase1Shape {
        filler,
        first_bundle_records: Some(records),
    }
}

/// The filler messages for a shift of `len` bytes, in send order: none for
/// 0, `updateFrequencyNotification` for 2, one `voiceData` for 3 to 36, and
/// both for 37 and 38. `None` for 1 (no message is 1 byte) and above
/// [`SHIFT_LEN_MAX`].
pub(super) fn filler(len: u16) -> Option<Vec<Vec<u8>>> {
    match len {
        0 => Some(Vec::new()),
        UPDATE_FREQUENCY_LEN => Some(vec![update_frequency()]),
        3..=VOICE_DATA_MAX_LEN => Some(vec![voice_data(len - VOICE_DATA_HEADER_LEN)]),
        37..=SHIFT_LEN_MAX => Some(vec![
            update_frequency(),
            voice_data(len - UPDATE_FREQUENCY_LEN - VOICE_DATA_HEADER_LEN),
        ]),
        _ => None,
    }
}

/// Re-states the tick rate the connect bundle already sent, so the client's
/// state does not change.
fn update_frequency() -> Vec<u8> {
    vec![MSG_UPDATE_FREQUENCY_NOTIFICATION, UPDATE_FREQUENCY_HZ]
}

/// A `voiceData` message with a `body_len`-byte zero body.
fn voice_data(body_len: u16) -> Vec<u8> {
    let mut msg = Vec::with_capacity(usize::from(VOICE_DATA_HEADER_LEN + body_len));
    msg.push(MSG_VOICE_DATA);
    msg.extend_from_slice(&body_len.to_le_bytes());
    msg.resize(msg.len() + usize::from(body_len), 0);
    msg
}

/// The `draw`th sweep length for `seed`, uniform over 0 and 2 to
/// [`SHIFT_LEN_MAX`]; never 1.
pub(super) fn sweep_len(seed: u64, draw: u64) -> u16 {
    // 0, then 2..=SHIFT_LEN_MAX: SHIFT_LEN_MAX values in all.
    let pick = splitmix64(seed ^ splitmix64(draw)) % u64::from(SHIFT_LEN_MAX);
    if pick == 0 {
        0
    } else {
        // pick < SHIFT_LEN_MAX, so it fits.
        pick as u16 + 1
    }
}

/// Which message of a phase-1 record a client cursor names.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(not(test), allow(dead_code))]
pub(super) enum CursorFamily {
    /// The record's CREATE_ENTITY, at cursor `1 + L + 37j`.
    Create,
    /// The record's UPDATE_AVATAR, at cursor `12 + L + 37j`.
    Avatar,
}

/// The shift length `L` and record index `j` that put record `j`'s `family`
/// message at client bundle cursor `r` (1-based, as the client's
/// `request_misparse` reports it) in the first packet of a flush of
/// `records` introductions. `None` when no record can sit there: `r` before
/// the first record, `j` past the last record, or past the one packet.
///
/// The residue `L = 1` has no filler, so it uses `L = 38` on record `j - 1`,
/// which lands on the same cursor. The lab picks a `Shift` length with this;
/// the tests pin it against the bytes the flush sends.
#[cfg_attr(not(test), allow(dead_code))]
pub(super) fn shift_for_cursor(r: u32, family: CursorFamily, records: u32) -> Option<(u16, u32)> {
    let first_cursor = match family {
        CursorFamily::Create => 1,
        CursorFamily::Avatar => 12,
    };
    let record_len = u32::from(PHASE1_RECORD_LEN);
    let from_first = r.checked_sub(first_cursor)?;
    let (mut len, mut j) = (from_first % record_len, from_first / record_len);
    if len == 1 {
        j = j.checked_sub(1)?;
        len += record_len;
    }
    let fits = usize::try_from(len + record_len * (j + 1)).ok()? <= FRAGMENT_BODY_SIZE;
    if j >= records || !fits {
        return None;
    }
    Some((u16::try_from(len).ok()?, j))
}

/// Indices of the introductions in `segment`, in order.
fn introduction_slots(segment: &[DeferredAoiMsg]) -> Vec<usize> {
    segment
        .iter()
        .enumerate()
        .filter(|(_, msg)| matches!(msg, DeferredAoiMsg::EnteredAoI { .. }))
        .map(|(i, _)| i)
        .collect()
}

/// Fisher-Yates over the introduction slots only, so every other message
/// keeps its place and the tail's encounter order is untouched.
fn shuffle_introductions(segment: &mut [DeferredAoiMsg], rng: &mut SplitMix64) {
    let slots = introduction_slots(segment);
    for k in (1..slots.len()).rev() {
        // k < slots.len(), so both casts are lossless.
        let pick = (rng.next() % (k as u64 + 1)) as usize;
        segment.swap(slots[k], slots[pick]);
    }
}

/// SplitMix64: a fixed, documented generator, so a seed replays the same
/// order on every build (`rand`'s `StdRng` does not promise that).
struct SplitMix64(u64);

impl SplitMix64 {
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        mix(self.0)
    }
}

fn splitmix64(x: u64) -> u64 {
    mix(x.wrapping_add(0x9E37_79B9_7F4A_7C15))
}

fn mix(mut z: u64) -> u64 {
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

/// The installed registry. Tests in this crate cannot install the process
/// registry (only the server's startup may, `lab_chaos_callers`), so they
/// lend one to their own thread instead.
fn registry() -> Option<&'static LabChaos> {
    #[cfg(test)]
    if let Some(lent) = TEST_REGISTRY.with(std::cell::Cell::get) {
        return Some(lent);
    }
    cimmeria_base_session::base::lab_chaos::current()
}

#[cfg(test)]
thread_local! {
    static TEST_REGISTRY: std::cell::Cell<Option<&'static LabChaos>> =
        const { std::cell::Cell::new(None) };
}

/// Lends `registry` to flushes on this thread (a `#[tokio::test]` runs on
/// its test's thread), or withdraws it with `None`.
#[cfg(test)]
pub(super) fn lend_registry(registry: Option<&'static LabChaos>) {
    TEST_REGISTRY.with(|lent| lent.set(registry));
}
