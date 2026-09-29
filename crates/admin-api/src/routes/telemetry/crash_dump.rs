//! Minidumps inside an end-of-session bundle.
//!
//! The telemetry DLL writes `Binaries/sessions/crash-<session>-<ts>.dmp`
//! when SGW.exe crashes, and the launcher's bundle ships everything under
//! `sessions/`. The bundle replay reads entries as text, so a dump used
//! to be skipped as non-UTF-8 at DEBUG. This reads just enough of the
//! minidump to say which exception it holds, so the arrival is one
//! `launcher.bundle.crash_dump` row in SigNoz even when the live
//! `client.crash` event never left the machine.
//!
//! Layout (`minidumpapiset.h`): a 32-byte `MINIDUMP_HEADER` whose
//! `NumberOfStreams` / `StreamDirectoryRva` point at 12-byte
//! `MINIDUMP_DIRECTORY` entries; stream type 6 is the
//! `MINIDUMP_EXCEPTION_STREAM`: `ThreadId`, 4 alignment bytes, then
//! `MINIDUMP_EXCEPTION` (`ExceptionCode`, `ExceptionFlags`, a 64-bit
//! record pointer, the 64-bit `ExceptionAddress`). All little-endian.

/// `MDMP` read as a little-endian u32.
const MINIDUMP_SIGNATURE: u32 = 0x504D_444D;
const EXCEPTION_STREAM: u32 = 6;
/// Directory entries we are willing to walk: real dumps carry a dozen or
/// so; this bounds the work a hostile file can ask for.
const MAX_STREAMS: u32 = 256;

/// Whether a bundle entry is a minidump, by name.
pub(super) fn is_crash_dump(path: &str) -> bool {
    let b = path.as_bytes();
    b.len() > 4 && b[b.len() - 4..].eq_ignore_ascii_case(b".dmp")
}

/// What a minidump says about its crash.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct MinidumpSummary {
    pub(super) valid: bool,
    pub(super) streams: u32,
    pub(super) exception: Option<MinidumpException>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct MinidumpException {
    pub(super) thread_id: u32,
    pub(super) code: u32,
    pub(super) address: u64,
}

fn u32_at(b: &[u8], at: usize) -> Option<u32> {
    Some(u32::from_le_bytes(b.get(at..at + 4)?.try_into().ok()?))
}

fn u64_at(b: &[u8], at: usize) -> Option<u64> {
    Some(u64::from_le_bytes(b.get(at..at + 8)?.try_into().ok()?))
}

/// Read the header and the exception stream. Never panics on a short
/// or corrupt file: a field that does not fit is reported as absent.
pub(super) fn summarize(bytes: &[u8]) -> MinidumpSummary {
    let invalid = MinidumpSummary {
        valid: false,
        streams: 0,
        exception: None,
    };
    if u32_at(bytes, 0) != Some(MINIDUMP_SIGNATURE) {
        return invalid;
    }
    let (Some(streams), Some(dir_rva)) = (u32_at(bytes, 8), u32_at(bytes, 12)) else {
        return invalid;
    };
    let mut exception = None;
    for i in 0..streams.min(MAX_STREAMS) as usize {
        let entry = dir_rva as usize + i * 12;
        let (Some(kind), Some(rva)) = (u32_at(bytes, entry), u32_at(bytes, entry + 8)) else {
            break;
        };
        if kind != EXCEPTION_STREAM {
            continue;
        }
        let rva = rva as usize;
        if let (Some(thread_id), Some(code), Some(address)) = (
            u32_at(bytes, rva),
            u32_at(bytes, rva + 8),
            u64_at(bytes, rva + 24),
        ) {
            exception = Some(MinidumpException {
                thread_id,
                code,
                address,
            });
        }
        break;
    }
    MinidumpSummary {
        valid: true,
        streams,
        exception,
    }
}

/// Emit the `launcher.bundle.crash_dump` row for one dump entry.
pub(super) fn log_crash_dump(
    claims: &crate::routes::dev_session::TokenClaims,
    path: &str,
    bytes: &[u8],
) -> MinidumpSummary {
    let summary = summarize(bytes);
    let hex32 = |v: u32| format!("0x{v:08x}");
    tracing::warn!(
        target: "launcher.bundle.crash_dump",
        session_id = %claims.sid,
        install_id = %claims.sub,
        path = %path,
        dump_bytes = bytes.len() as u64,
        minidump_valid = summary.valid,
        streams = summary.streams,
        exception_code = summary.exception.map(|e| hex32(e.code)),
        exception_address = summary.exception.map(|e| format!("0x{:08x}", e.address)),
        thread_id = summary.exception.map(|e| e.thread_id),
        "crash dump in bundle"
    );
    summary
}

#[cfg(test)]
pub(super) mod tests {
    use super::*;

    /// A minimal well-formed minidump: header, a two-entry directory
    /// (a thread list first, then the exception stream), and the
    /// exception stream.
    pub(crate) fn synthetic_minidump(code: u32, address: u32, thread_id: u32) -> Vec<u8> {
        let mut b = vec![0u8; 32];
        b[0..4].copy_from_slice(&MINIDUMP_SIGNATURE.to_le_bytes());
        b[4..8].copy_from_slice(&0xA793u32.to_le_bytes()); // version
        b[8..12].copy_from_slice(&2u32.to_le_bytes()); // streams
        b[12..16].copy_from_slice(&32u32.to_le_bytes()); // directory rva
        let exc_rva = 32 + 2 * 12;
        // Directory: ThreadListStream (3), then ExceptionStream (6).
        for (kind, size, rva) in [(3u32, 4u32, 0u32), (EXCEPTION_STREAM, 168, exc_rva)] {
            b.extend_from_slice(&kind.to_le_bytes());
            b.extend_from_slice(&size.to_le_bytes());
            b.extend_from_slice(&rva.to_le_bytes());
        }
        let mut exc = vec![0u8; 168];
        exc[0..4].copy_from_slice(&thread_id.to_le_bytes());
        exc[8..12].copy_from_slice(&code.to_le_bytes());
        exc[24..32].copy_from_slice(&u64::from(address).to_le_bytes());
        b.extend_from_slice(&exc);
        b
    }

    #[test]
    fn reads_the_exception_stream() {
        let dump = synthetic_minidump(0xC000_0005, 0x0041_6EC5, 4242);
        let s = summarize(&dump);
        assert!(s.valid);
        assert_eq!(s.streams, 2);
        assert_eq!(
            s.exception,
            Some(MinidumpException {
                thread_id: 4242,
                code: 0xC000_0005,
                address: 0x0041_6EC5,
            })
        );
    }

    #[test]
    fn not_a_minidump_is_invalid() {
        assert!(!summarize(b"hello world, not a dump").valid);
        assert!(!summarize(&[]).valid);
    }

    /// Truncation anywhere (a dump cut short by a dying process) must not
    /// panic; the exception is just absent.
    #[test]
    fn truncated_dump_never_panics() {
        let dump = synthetic_minidump(0xC000_0005, 0x10, 1);
        for len in 0..dump.len() {
            let s = summarize(&dump[..len]);
            if len < 4 {
                assert!(!s.valid);
            }
            if len < dump.len() - 140 {
                assert_eq!(s.exception, None, "len {len}");
            }
        }
    }

    /// A directory claiming billions of streams is walked only up to the
    /// cap and the bytes that exist.
    #[test]
    fn hostile_stream_count_is_bounded() {
        let mut dump = synthetic_minidump(1, 2, 3);
        dump[8..12].copy_from_slice(&u32::MAX.to_le_bytes());
        let s = summarize(&dump);
        assert!(s.valid);
        assert_eq!(s.streams, u32::MAX);
        assert!(s.exception.is_some());
    }

    #[test]
    fn dump_names_match_case_blind() {
        assert!(is_crash_dump("Binaries/sessions/crash-abc-1.dmp"));
        assert!(is_crash_dump("X.DMP"));
        assert!(!is_crash_dump("Binaries/sessions/crash-abc-1.json"));
        assert!(!is_crash_dump(".dmp"));
        assert!(!is_crash_dump("dmp"));
        // A multi-byte name must not be sliced mid-character.
        assert!(!is_crash_dump("sessions/crash-éé"));
    }
}
