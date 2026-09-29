//! Bink movies: when one opens, and whether it played to the end.
//!
//! `SGW.exe` imports `_BinkOpen@8` and `_BinkClose@4` from `binkw32.dll`
//! (IAT `0x017effa4` and `0x017effa8`). The client opens a movie from memory:
//! `FUN_00509820` reads the whole file through the package file cache and
//! calls `_BinkOpen_8(buffer, 0x4004400)`, so the first argument is a buffer
//! (the flags include the from-memory bit), not a name; the file name is not
//! available at this seam. What is available is the movie's own header, which
//! identifies a movie well enough (size, frame count, frame rate), and how far
//! it got before it closed.
//!
//! - **`client.media.bink_open`**: `ok` (the handle is non-null: a corrupt or
//!   unsupported movie opens to `NULL` and the client then silently has no
//!   cinematic), the flags, and the header.
//! - **`client.media.bink_close`**: the frame it reached and the frame count,
//!   and `completed`. A cinematic that the player skipped, or that a
//!   level script cut, closes early; one that ran out closes on its last
//!   frame.
//!
//! The header layout (`BINK`, Bink 1.x SDK), read from the handle: `Width`
//! `+0x00`, `Height` `+0x04`, `Frames` `+0x08`, `FrameNum` `+0x0c` (the frame
//! to be displayed next, 1-based), `LastFrameNum` `+0x10`, `FrameRate`
//! `+0x14` and `FrameRateDiv` `+0x18` (numerator and divisor).
//!
//! `binkw32.dll` exports the two names decorated (`_BinkOpen@8`,
//! `_BinkClose@4`, checked in the DLL's export table 2026-09-28), both
//! `__stdcall`. Movies are rare (a handful a session), so nothing here is
//! rate-limited beyond the shared per-key bucket.
//!
//! Static evidence only; not yet seen from a live client. The header offsets
//! come from the SDK layout, not from this build's code: `open` reports the
//! numbers as read and `plausible` says whether they look like a movie
//! (non-zero dimensions and frame count), so a wrong layout shows up as
//! `plausible = false` rather than as wrong data.

use serde_json::json;

use crate::hooks::sinks::emit::Fields;
use crate::hooks::sinks::mem::{self, Reader};

/// IAT slot of `_BinkOpen@8`.
pub const IAT_BINK_OPEN: usize = 0x017e_ffa4;
/// IAT slot of `_BinkClose@4`.
pub const IAT_BINK_CLOSE: usize = 0x017e_ffa8;

/// Header field offsets.
pub const WIDTH_OFFSET: usize = 0x00;
/// `Height`.
pub const HEIGHT_OFFSET: usize = 0x04;
/// `Frames`.
pub const FRAMES_OFFSET: usize = 0x08;
/// `FrameNum`.
pub const FRAME_NUM_OFFSET: usize = 0x0c;
/// `FrameRate` numerator.
pub const RATE_OFFSET: usize = 0x14;
/// `FrameRateDiv`.
pub const RATE_DIV_OFFSET: usize = 0x18;

/// Telemetry target of an open.
pub const OPEN_TARGET: &str = "client.media.bink_open";
/// Telemetry target of a close.
pub const CLOSE_TARGET: &str = "client.media.bink_close";

/// A movie header.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Header {
    /// Width in pixels.
    pub width: u32,
    /// Height in pixels.
    pub height: u32,
    /// Frame count.
    pub frames: u32,
    /// The frame to be displayed next (1-based).
    pub frame_num: u32,
    /// Frames per second, when the rate fields are sane.
    pub fps: Option<f32>,
}

/// Read a header from a `BINK*`.
pub fn read_header(read: Reader, bink: usize) -> Option<Header> {
    let rate = mem::read_u32(read, bink + RATE_OFFSET)?;
    let div = mem::read_u32(read, bink + RATE_DIV_OFFSET)?;
    Some(Header {
        width: mem::read_u32(read, bink + WIDTH_OFFSET)?,
        height: mem::read_u32(read, bink + HEIGHT_OFFSET)?,
        frames: mem::read_u32(read, bink + FRAMES_OFFSET)?,
        frame_num: mem::read_u32(read, bink + FRAME_NUM_OFFSET)?,
        fps: (div != 0).then(|| rate as f32 / div as f32),
    })
}

impl Header {
    /// Whether the numbers look like a movie: 16x16 or bigger up to 8192,
    /// at least one frame, no more than a million.
    pub fn plausible(&self) -> bool {
        (16..=8192).contains(&self.width)
            && (16..=8192).contains(&self.height)
            && (1..=1_000_000).contains(&self.frames)
    }

    /// Whether the movie ran to (within one frame of) its end.
    pub fn completed(&self) -> bool {
        self.frames.saturating_sub(self.frame_num) <= 1
    }
}

/// The fields of an open.
pub fn open_fields(flags: u32, header: Option<Header>) -> Fields {
    let mut f: Fields = vec![
        ("ok", json!(header.is_some())),
        ("flags", json!(format!("0x{flags:08x}"))),
    ];
    if let Some(h) = header {
        f.push(("width", json!(h.width)));
        f.push(("height", json!(h.height)));
        f.push(("frames", json!(h.frames)));
        if let Some(fps) = h.fps.filter(|v| v.is_finite()) {
            f.push(("fps", json!((f64::from(fps) * 100.0).round() / 100.0)));
        }
        f.push(("plausible", json!(h.plausible())));
    }
    f
}

/// The fields of a close.
pub fn close_fields(header: Option<Header>) -> Fields {
    match header {
        Some(h) => vec![
            ("frame_num", json!(h.frame_num)),
            ("frames", json!(h.frames)),
            ("completed", json!(h.completed())),
        ],
        None => vec![("completed", json!(false))],
    }
}

#[cfg(all(target_os = "windows", target_arch = "x86"))]
mod x86 {
    use std::ffi::c_void;
    use std::panic::AssertUnwindSafe;
    use std::sync::atomic::{AtomicUsize, Ordering};

    use super::*;
    use crate::hooks::sinks::emit::emit;
    use crate::hooks::sinks::install::SlotImport;
    use crate::hooks::sinks::mem::process_reader;

    pub(in crate::hooks) static ORIG_OPEN: AtomicUsize = AtomicUsize::new(0);
    pub(in crate::hooks) static ORIG_CLOSE: AtomicUsize = AtomicUsize::new(0);

    pub(in crate::hooks) const IMPORT_OPEN: SlotImport = SlotImport {
        slot: IAT_BINK_OPEN,
        module: "binkw32.dll",
        symbol: c"_BinkOpen@8",
    };
    pub(in crate::hooks) const IMPORT_CLOSE: SlotImport = SlotImport {
        slot: IAT_BINK_CLOSE,
        module: "binkw32.dll",
        symbol: c"_BinkClose@4",
    };

    /// `HBINK __stdcall BinkOpen(const void* name_or_buffer, U32 flags)`.
    #[allow(improper_ctypes_definitions)]
    pub(in crate::hooks) unsafe extern "stdcall-unwind" fn open_detour(
        source: *const c_void,
        flags: u32,
    ) -> *mut c_void {
        let orig = ORIG_OPEN.load(Ordering::Acquire);
        if orig == 0 {
            return std::ptr::null_mut();
        }
        let original: unsafe extern "stdcall-unwind" fn(*const c_void, u32) -> *mut c_void =
            unsafe { std::mem::transmute(orig) };
        let handle = original(source, flags);
        let _ = std::panic::catch_unwind(AssertUnwindSafe(|| {
            let header = if handle.is_null() {
                None
            } else {
                read_header(&process_reader, handle as usize)
            };
            emit(
                OPEN_TARGET,
                if handle.is_null() { "warn" } else { "info" },
                "media.bink_open",
                open_fields(flags, header),
            );
        }));
        handle
    }

    /// `void __stdcall BinkClose(HBINK)`.
    #[allow(improper_ctypes_definitions)]
    pub(in crate::hooks) unsafe extern "stdcall-unwind" fn close_detour(bink: *mut c_void) {
        // Read the header first: closing frees the handle.
        let _ = std::panic::catch_unwind(AssertUnwindSafe(|| {
            let header = (!bink.is_null())
                .then(|| read_header(&process_reader, bink as usize))
                .flatten();
            emit(
                CLOSE_TARGET,
                "info",
                "media.bink_close",
                close_fields(header),
            );
        }));
        let orig = ORIG_CLOSE.load(Ordering::Acquire);
        if orig != 0 {
            let original: unsafe extern "stdcall-unwind" fn(*mut c_void) =
                unsafe { std::mem::transmute(orig) };
            original(bink);
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        use crate::hooks::sinks::emit::take_captured;

        static SEEN: AtomicUsize = AtomicUsize::new(0);

        /// A `BINK` header in real memory: 640x360, 900 frames, 30/1 fps,
        /// at frame 12.
        fn header_bytes() -> Vec<u8> {
            let mut b = vec![0u8; 0x40];
            for (off, v) in [
                (WIDTH_OFFSET, 640u32),
                (HEIGHT_OFFSET, 360),
                (FRAMES_OFFSET, 900),
                (FRAME_NUM_OFFSET, 12),
                (RATE_OFFSET, 30),
                (RATE_DIV_OFFSET, 1),
            ] {
                b[off..off + 4].copy_from_slice(&v.to_le_bytes());
            }
            b
        }

        unsafe extern "stdcall-unwind" fn fake_open(
            source: *const c_void,
            flags: u32,
        ) -> *mut c_void {
            SEEN.store(source as usize ^ flags as usize, Ordering::SeqCst);
            // A null source models a movie that fails to open.
            if source.is_null() {
                std::ptr::null_mut()
            } else {
                HEADER.load(Ordering::SeqCst) as *mut c_void
            }
        }
        static HEADER: AtomicUsize = AtomicUsize::new(0);

        unsafe extern "stdcall-unwind" fn fake_close(bink: *mut c_void) {
            SEEN.store(bink as usize, Ordering::SeqCst);
        }

        #[test]
        fn open_and_close_report_the_header_and_forward_arguments() {
            ORIG_OPEN.store(fake_open as *const () as usize, Ordering::SeqCst);
            ORIG_CLOSE.store(fake_close as *const () as usize, Ordering::SeqCst);
            let bytes = header_bytes();
            HEADER.store(bytes.as_ptr() as usize, Ordering::SeqCst);
            let _ = take_captured();

            let buf = [0u8; 4];
            let h = unsafe { open_detour(buf.as_ptr().cast(), 0x0400_4400) };
            assert_eq!(h as usize, bytes.as_ptr() as usize);
            assert_eq!(
                SEEN.load(Ordering::SeqCst),
                buf.as_ptr() as usize ^ 0x0400_4400
            );
            let events = take_captured();
            assert_eq!(events.len(), 1);
            assert_eq!(events[0].target, OPEN_TARGET);
            assert_eq!(events[0].level, "info");
            assert_eq!(events[0].get("ok"), Some(&json!(true)));
            assert_eq!(events[0].get("width"), Some(&json!(640)));
            assert_eq!(events[0].get("frames"), Some(&json!(900)));
            assert_eq!(events[0].get("fps"), Some(&json!(30.0)));
            assert_eq!(events[0].get("plausible"), Some(&json!(true)));

            // A movie that fails to open is a warning.
            let h = unsafe { open_detour(std::ptr::null(), 0) };
            assert!(h.is_null());
            let events = take_captured();
            assert_eq!(events[0].level, "warn");
            assert_eq!(events[0].get("ok"), Some(&json!(false)));

            // Closing at frame 12 of 900 is cut short.
            unsafe { close_detour(bytes.as_ptr() as *mut c_void) };
            assert_eq!(SEEN.load(Ordering::SeqCst), bytes.as_ptr() as usize);
            let events = take_captured();
            assert_eq!(events[0].target, CLOSE_TARGET);
            assert_eq!(events[0].get("frame_num"), Some(&json!(12)));
            assert_eq!(events[0].get("completed"), Some(&json!(false)));

            // A null handle is forwarded and reported as not completed.
            unsafe { close_detour(std::ptr::null_mut()) };
            let events = take_captured();
            assert_eq!(events[0].get("completed"), Some(&json!(false)));
        }
    }
}

#[cfg(all(target_os = "windows", target_arch = "x86"))]
pub(in crate::hooks) use x86::{
    close_detour, open_detour, IMPORT_CLOSE, IMPORT_OPEN, ORIG_CLOSE, ORIG_OPEN,
};

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hooks::sinks::mem::fake::FakeMemory;

    fn movie(m: &mut FakeMemory, at: usize, frames: u32, frame_num: u32) {
        for (off, v) in [
            (WIDTH_OFFSET, 1280u32),
            (HEIGHT_OFFSET, 720),
            (FRAMES_OFFSET, frames),
            (FRAME_NUM_OFFSET, frame_num),
            (RATE_OFFSET, 24000),
            (RATE_DIV_OFFSET, 1001),
        ] {
            m.put(at + off, &v.to_le_bytes());
        }
    }

    #[test]
    fn a_header_reads_and_derives_the_frame_rate() {
        let mut m = FakeMemory::new();
        movie(&mut m, 0x1000, 300, 1);
        let h = read_header(&m.reader(), 0x1000).unwrap();
        assert_eq!(
            (h.width, h.height, h.frames, h.frame_num),
            (1280, 720, 300, 1)
        );
        assert!((h.fps.unwrap() - 23.976).abs() < 0.01);
        assert!(h.plausible());
        // A zero divisor is not a rate.
        m.put(0x1000 + RATE_DIV_OFFSET, &0u32.to_le_bytes());
        assert_eq!(read_header(&m.reader(), 0x1000).unwrap().fps, None);
    }

    /// A wrong layout shows up as an implausible header, not as wrong data.
    #[test]
    fn garbage_is_implausible() {
        let mut m = FakeMemory::new();
        m.put(0x1000, &[0xFF; 0x20]);
        assert!(!read_header(&m.reader(), 0x1000).unwrap().plausible());
        m.put(0x2000, &[0u8; 0x20]);
        assert!(!read_header(&m.reader(), 0x2000).unwrap().plausible());
    }

    #[test]
    fn completion_allows_the_last_frame_to_be_pending() {
        let mk = |frames, frame_num| Header {
            width: 640,
            height: 360,
            frames,
            frame_num,
            fps: None,
        };
        assert!(mk(300, 300).completed());
        assert!(mk(300, 299).completed());
        assert!(!mk(300, 298).completed());
        assert!(!mk(300, 1).completed());
        // Past the end (a rewind and replay) still counts.
        assert!(mk(300, 400).completed());
    }

    #[test]
    fn a_failed_open_has_no_header_fields() {
        let f = open_fields(0x0400_4400, None);
        let get = |k: &str| f.iter().find(|(n, _)| *n == k).map(|(_, v)| v.clone());
        assert_eq!(get("ok"), Some(json!(false)));
        assert_eq!(get("flags"), Some(json!("0x04004400")));
        assert_eq!(get("width"), None);
        assert_eq!(close_fields(None), vec![("completed", json!(false))]);
    }

    #[test]
    fn the_iat_slots_are_the_import_directorys() {
        assert_eq!(IAT_BINK_OPEN, 0x017e_ffa4);
        assert_eq!(IAT_BINK_CLOSE, 0x017e_ffa8);
    }
}
