//! Matinee (cinematic) start and stop: `USeqAct_Interp::Activated` and
//! `DeActivated`.
//!
//! A Matinee is a Kismet `SeqAct_Interp` action: a timeline that moves
//! cameras and actors and fades the screen. Cinematics interact with an
//! open entity-visibility bug (entities that are present on the server are
//! not shown after a scripted scene), so the client's side of "a cinematic
//! started, and this is how it ended" is worth having.
//!
//! # Where it hooks
//!
//! Two slots of the `USeqAct_Interp` vtable (RTTI `.?AVUSeqAct_Interp@@`,
//! vtable `0x018ad494`):
//!
//! | Slot | Address | Method | Evidence |
//! |---|---|---|---|
//! | 84 | `0x007b0940` | `UpdateOp(float)` | processes the input impulses and clears them (not hooked here; the generic per-sequence tick is the `USequence::UpdateOp` vtable hook) |
//! | 85 | `0x007b06a0` | `Activated()` | calls `USequenceOp::Activated` first, then reads the input links and starts the interpolation; `__fastcall` on `this` only |
//! | 86 | `0x007a6730` | `DeActivated()` | tears the group actors down and fires the "finished" outputs (name inferred from the behaviour and from slot 86 of `USequenceOp`) |
//!
//! # Layout (read from the three functions)
//!
//! - `this + 0x8c` is the `InputLinks` array data, `this + 0x90` its count.
//!   Each entry is `0x28` bytes and its `bHasImpulse` bit is bit 0 of the
//!   byte at `+0x0c`. `UpdateOp` tests entries 0, 1, 2, 3 and 4 in that
//!   order and clears all five afterwards; they are `Play`, `Reverse`, `Stop`,
//!   `Pause`, `Change Dir` (the standard `SeqAct_Interp` inputs).
//! - `this + 0x104` is the interpolation position (a `float`), and
//!   `this + 0x11c` the `InterpData`, whose length is the `float` at `+0x90`
//!   (`DeActivated` compares them).
//! - `this + 0x2c` is the object name, `Name_N` for the Kismet node.
//!
//! On a client `Activated` runs when the server-driven or level-scripted
//! sequence reaches the node; `DeActivated` when it finishes or is stopped.
//! A `position` well short of `length` on deactivation means the scene was
//! cut short (skipped, or a hard stop).
//!
//! Static evidence only (2026-09-28: Ghidra decompile of slots 84-86); not
//! yet seen from a live client.

use serde_json::json;

use crate::hooks::sinks::emit::Fields;
use crate::hooks::sinks::mem::{self, Reader};

/// `USeqAct_Interp` vtable slot 85 (`Activated`).
pub const SLOT_ACTIVATED: usize = 0x018a_d5e8;
/// `USeqAct_Interp` vtable slot 86 (`DeActivated`).
pub const SLOT_DEACTIVATED: usize = 0x018a_d5ec;

/// Offset of the `InputLinks` array data pointer.
pub const INPUT_LINKS_OFFSET: usize = 0x8c;
/// Size of one input link.
pub const INPUT_LINK_STRIDE: usize = 0x28;
/// Offset of `bHasImpulse` inside a link.
pub const IMPULSE_OFFSET: usize = 0x0c;
/// Offset of the position (`float`).
pub const POSITION_OFFSET: usize = 0x104;
/// Offset of the `InterpData` pointer.
pub const INTERP_DATA_OFFSET: usize = 0x11c;
/// Offset of `InterpLength` inside the `InterpData`.
pub const INTERP_LENGTH_OFFSET: usize = 0x90;

/// Telemetry target.
pub const TARGET: &str = "client.engine.matinee";

/// The five inputs, in link order.
pub const INPUT_NAMES: [&str; 5] = ["play", "reverse", "stop", "pause", "change_dir"];

/// The names of the inputs that carry an impulse, given their flags.
pub fn impulse_names(flags: [bool; 5]) -> Vec<&'static str> {
    INPUT_NAMES
        .iter()
        .zip(flags)
        .filter_map(|(n, set)| set.then_some(*n))
        .collect()
}

/// Read the five impulse flags of a `SeqAct_Interp`.
pub fn read_impulses(read: Reader, this: usize) -> Option<[bool; 5]> {
    let data = mem::read_u32(read, this + INPUT_LINKS_OFFSET)? as usize;
    if data == 0 {
        return None;
    }
    let mut flags = [false; 5];
    for (i, slot) in flags.iter_mut().enumerate() {
        let byte = mem::read_u8(read, data + i * INPUT_LINK_STRIDE + IMPULSE_OFFSET)?;
        *slot = byte & 1 != 0;
    }
    Some(flags)
}

/// Read the position and the length of the interpolation.
pub fn read_progress(read: Reader, this: usize) -> (Option<f32>, Option<f32>) {
    let position = mem::read_u32(read, this + POSITION_OFFSET).map(f32::from_bits);
    let length = mem::read_u32(read, this + INTERP_DATA_OFFSET)
        .filter(|p| *p != 0)
        .and_then(|data| mem::read_u32(read, data as usize + INTERP_LENGTH_OFFSET))
        .map(f32::from_bits);
    (position, length)
}

/// Whether a deactivation left the scene short of its end: the position is
/// more than a quarter second before the length.
pub fn cut_short(position: Option<f32>, length: Option<f32>) -> Option<bool> {
    let (p, l) = (position?, length?);
    (p.is_finite() && l.is_finite()).then_some(l - p > 0.25)
}

/// A finite `f32` as a JSON number rounded to a hundredth of a second.
fn seconds(v: f32) -> serde_json::Value {
    json!((f64::from(v) * 100.0).round() / 100.0)
}

/// The fields of one activation.
pub fn activated_fields(name: &str, impulses: Option<[bool; 5]>, suppressed: u64) -> Fields {
    let mut f: Fields = vec![("event", json!("activated")), ("sequence", json!(name))];
    if let Some(flags) = impulses {
        f.push(("inputs", json!(impulse_names(flags).join(","))));
    }
    if suppressed > 0 {
        f.push(("suppressed", json!(suppressed)));
    }
    f
}

/// The fields of one deactivation.
pub fn deactivated_fields(
    name: &str,
    position: Option<f32>,
    length: Option<f32>,
    suppressed: u64,
) -> Fields {
    let mut f: Fields = vec![("event", json!("deactivated")), ("sequence", json!(name))];
    if let Some(p) = position.filter(|p| p.is_finite()) {
        f.push(("position", seconds(p)));
    }
    if let Some(l) = length.filter(|l| l.is_finite()) {
        f.push(("length", seconds(l)));
    }
    if let Some(short) = cut_short(position, length) {
        f.push(("cut_short", json!(short)));
    }
    if suppressed > 0 {
        f.push(("suppressed", json!(suppressed)));
    }
    f
}

#[cfg(all(target_os = "windows", target_arch = "x86"))]
mod x86 {
    use std::ffi::c_void;
    use std::panic::AssertUnwindSafe;
    use std::sync::atomic::{AtomicUsize, Ordering};

    use super::super::objects;
    use super::*;
    use crate::hooks::name_throttle::Decision;
    use crate::hooks::sinks::emit::emit;
    use crate::hooks::sinks::mem::process_reader;
    use crate::hooks::sinks::throttle::SinkThrottle;

    pub(in crate::hooks) static ORIG_ACTIVATED: AtomicUsize = AtomicUsize::new(0);
    pub(in crate::hooks) static ORIG_DEACTIVATED: AtomicUsize = AtomicUsize::new(0);

    static THROTTLE: SinkThrottle = SinkThrottle::new();

    fn sequence_name(this: usize) -> String {
        objects::object_name(&process_reader, this).unwrap_or_else(|| "<unknown>".to_string())
    }

    /// `USeqAct_Interp::Activated()`: `this` in ECX, no stack arguments.
    #[allow(improper_ctypes_definitions)]
    pub(in crate::hooks) unsafe extern "thiscall-unwind" fn activated_detour(this: *mut c_void) {
        // Read the impulses before the original runs; `UpdateOp` clears
        // them once it has processed them.
        let _ = std::panic::catch_unwind(AssertUnwindSafe(|| {
            let name = sequence_name(this as usize);
            let Decision::Emit { suppressed } = THROTTLE.check(&format!("activated:{name}")) else {
                return;
            };
            let impulses = read_impulses(&process_reader, this as usize);
            emit(
                TARGET,
                "info",
                "engine.matinee",
                activated_fields(&name, impulses, suppressed),
            );
        }));
        let orig = ORIG_ACTIVATED.load(Ordering::Acquire);
        if orig != 0 {
            let original: unsafe extern "thiscall-unwind" fn(*mut c_void) =
                unsafe { std::mem::transmute(orig) };
            original(this);
        }
    }

    /// `USeqAct_Interp::DeActivated()`: `this` in ECX, no stack arguments.
    #[allow(improper_ctypes_definitions)]
    pub(in crate::hooks) unsafe extern "thiscall-unwind" fn deactivated_detour(this: *mut c_void) {
        // The position is read before the call: `DeActivated` resets the
        // interpolation state.
        let _ = std::panic::catch_unwind(AssertUnwindSafe(|| {
            let name = sequence_name(this as usize);
            let Decision::Emit { suppressed } = THROTTLE.check(&format!("deactivated:{name}"))
            else {
                return;
            };
            let (position, length) = read_progress(&process_reader, this as usize);
            emit(
                TARGET,
                "info",
                "engine.matinee",
                deactivated_fields(&name, position, length, suppressed),
            );
        }));
        let orig = ORIG_DEACTIVATED.load(Ordering::Acquire);
        if orig != 0 {
            let original: unsafe extern "thiscall-unwind" fn(*mut c_void) =
                unsafe { std::mem::transmute(orig) };
            original(this);
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        use crate::hooks::sinks::emit::take_captured;

        static CALLS: AtomicUsize = AtomicUsize::new(0);

        unsafe extern "thiscall-unwind" fn fake(this: *mut c_void) {
            CALLS.store(this as usize, Ordering::SeqCst);
        }

        /// Both detours forward `this` and report even when the object is
        /// unreadable (a name of `<unknown>`), never faulting.
        #[test]
        fn both_detours_forward_this_and_tolerate_an_unreadable_object() {
            ORIG_ACTIVATED.store(fake as *const () as usize, Ordering::SeqCst);
            ORIG_DEACTIVATED.store(fake as *const () as usize, Ordering::SeqCst);
            let _ = take_captured();

            unsafe { activated_detour(0x1234 as *mut c_void) };
            assert_eq!(CALLS.load(Ordering::SeqCst), 0x1234);
            unsafe { deactivated_detour(0x5678 as *mut c_void) };
            assert_eq!(CALLS.load(Ordering::SeqCst), 0x5678);

            let events = take_captured();
            assert_eq!(events.len(), 2);
            assert_eq!(events[0].get("event"), Some(&json!("activated")));
            assert_eq!(events[0].get("sequence"), Some(&json!("<unknown>")));
            assert_eq!(events[0].get("inputs"), None, "impulses unreadable");
            assert_eq!(events[1].get("event"), Some(&json!("deactivated")));
        }
    }
}

#[cfg(all(target_os = "windows", target_arch = "x86"))]
pub(in crate::hooks) use x86::{
    activated_detour, deactivated_detour, ORIG_ACTIVATED, ORIG_DEACTIVATED,
};

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hooks::sinks::mem::fake::FakeMemory;

    /// An interp with impulses on Play and Pause, position 12.5 of 30.0.
    fn interp(m: &mut FakeMemory, at: usize, impulses: [bool; 5]) {
        let links = 0x8000usize;
        m.put(at + INPUT_LINKS_OFFSET, &(links as u32).to_le_bytes());
        for (i, set) in impulses.iter().enumerate() {
            m.put(
                links + i * INPUT_LINK_STRIDE + IMPULSE_OFFSET,
                &[u8::from(*set) | 0x10],
            );
        }
        m.put(at + POSITION_OFFSET, &12.5f32.to_bits().to_le_bytes());
        let data = 0x9000usize;
        m.put(at + INTERP_DATA_OFFSET, &(data as u32).to_le_bytes());
        m.put(
            data + INTERP_LENGTH_OFFSET,
            &30.0f32.to_bits().to_le_bytes(),
        );
    }

    #[test]
    fn impulses_read_bit_zero_of_each_link() {
        let mut m = FakeMemory::new();
        interp(&mut m, 0x1000, [true, false, false, true, false]);
        let flags = read_impulses(&m.reader(), 0x1000).unwrap();
        assert_eq!(flags, [true, false, false, true, false]);
        assert_eq!(impulse_names(flags), vec!["play", "pause"]);
        assert_eq!(impulse_names([false; 5]), Vec::<&str>::new());
    }

    #[test]
    fn a_null_input_array_is_not_read() {
        let mut m = FakeMemory::new();
        m.put(0x1000 + INPUT_LINKS_OFFSET, &0u32.to_le_bytes());
        assert_eq!(read_impulses(&m.reader(), 0x1000), None);
        assert_eq!(read_impulses(&m.reader(), 0x7000_0000), None);
    }

    #[test]
    fn progress_reads_position_and_the_interp_datas_length() {
        let mut m = FakeMemory::new();
        interp(&mut m, 0x1000, [false; 5]);
        assert_eq!(read_progress(&m.reader(), 0x1000), (Some(12.5), Some(30.0)));
        // No InterpData: the length is unknown, the position still reads.
        m.put(0x1000 + INTERP_DATA_OFFSET, &0u32.to_le_bytes());
        assert_eq!(read_progress(&m.reader(), 0x1000), (Some(12.5), None));
    }

    #[test]
    fn a_scene_is_cut_short_when_it_stops_well_before_its_end() {
        assert_eq!(cut_short(Some(12.5), Some(30.0)), Some(true));
        assert_eq!(cut_short(Some(29.9), Some(30.0)), Some(false));
        assert_eq!(cut_short(Some(30.0), Some(30.0)), Some(false));
        assert_eq!(cut_short(None, Some(30.0)), None);
        assert_eq!(cut_short(Some(f32::NAN), Some(30.0)), None);
    }

    #[test]
    fn fields_report_the_inputs_and_the_progress() {
        let f = activated_fields(
            "SeqAct_Interp_3",
            Some([true, false, false, false, false]),
            0,
        );
        let get = |k: &str| f.iter().find(|(n, _)| *n == k).map(|(_, v)| v.clone());
        assert_eq!(get("event"), Some(json!("activated")));
        assert_eq!(get("sequence"), Some(json!("SeqAct_Interp_3")));
        assert_eq!(get("inputs"), Some(json!("play")));

        let f = deactivated_fields("SeqAct_Interp_3", Some(12.504), Some(30.0), 2);
        let get = |k: &str| f.iter().find(|(n, _)| *n == k).map(|(_, v)| v.clone());
        assert_eq!(get("position"), Some(json!(12.5)));
        assert_eq!(get("length"), Some(json!(30.0)));
        assert_eq!(get("cut_short"), Some(json!(true)));
        assert_eq!(get("suppressed"), Some(json!(2)));

        let f = deactivated_fields("x", None, None, 0);
        assert!(!f.iter().any(|(k, _)| *k == "cut_short" || *k == "position"));
    }

    #[test]
    fn the_slots_are_vtable_85_and_86_of_the_interp_class() {
        // Vtable 0x018ad494; slot n is at base + 4n.
        assert_eq!(SLOT_ACTIVATED, 0x018a_d494 + 85 * 4);
        assert_eq!(SLOT_DEACTIVATED, 0x018a_d494 + 86 * 4);
    }
}
