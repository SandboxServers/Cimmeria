//! The detours on the imports in [`imports`](super::imports): installed with
//! the inline hooks, recording into the running version read's probe.

use std::ffi::c_void;
use std::panic::AssertUnwindSafe;
use std::sync::atomic::{AtomicUsize, Ordering};

use windows_sys::Win32::Foundation::{GetLastError, SetLastError};

use super::detours::{note, probing};
use super::imports::*;
use super::{stream_held, stream_read};
use crate::hooks::entity_trace::map::LiveMem;
use crate::hooks::inline_hooks::entity_lifecycle::guarded;
use crate::hooks::name_throttle::Decision;
use crate::hooks::seams::file_io::{is_pak, MAX_PATH_CHARS, TARGET_PAK_OPEN};
use crate::hooks::sinks::emit::emit;
use crate::hooks::sinks::install::{iat, SlotImport};
use crate::hooks::sinks::mem;
use crate::hooks::sinks::text;
use crate::hooks::sinks::throttle::SinkThrottle;

static ORIG_ISTREAM_READ: AtomicUsize = AtomicUsize::new(0);
static ORIG_WSOPEN_S: AtomicUsize = AtomicUsize::new(0);
static ORIG_READ: AtomicUsize = AtomicUsize::new(0);
static ORIG_LSEEK: AtomicUsize = AtomicUsize::new(0);

static OPEN_THROTTLE: SinkThrottle = SinkThrottle::new();

const IMPORT_ISTREAM_READ: SlotImport = SlotImport {
    slot: IAT_ISTREAM_READ,
    module: "msvcp80.dll",
    symbol: c"?read@?$basic_istream@DU?$char_traits@D@std@@@std@@QAEAAV12@PADH@Z",
};
const IMPORT_WSOPEN_S: SlotImport = SlotImport {
    slot: IAT_WSOPEN_S,
    module: "msvcr80.dll",
    symbol: c"_wsopen_s",
};
const IMPORT_READ: SlotImport = SlotImport {
    slot: IAT_READ,
    module: "msvcr80.dll",
    symbol: c"_read",
};
const IMPORT_LSEEK: SlotImport = SlotImport {
    slot: IAT_LSEEK,
    module: "msvcr80.dll",
    symbol: c"_lseek",
};

pub(in crate::hooks::inline_hooks::cooked_cache) unsafe fn install(
    producer: &crate::queue::Producer,
) {
    iat(
        producer,
        "cooked_stream_read",
        IMPORT_ISTREAM_READ,
        istream_read_detour as *const () as usize,
        &ORIG_ISTREAM_READ,
    );
    iat(
        producer,
        "cooked_crt_open",
        IMPORT_WSOPEN_S,
        wsopen_s_detour as *const () as usize,
        &ORIG_WSOPEN_S,
    );
    iat(
        producer,
        "cooked_crt_read",
        IMPORT_READ,
        read_detour as *const () as usize,
        &ORIG_READ,
    );
    iat(
        producer,
        "cooked_crt_seek",
        IMPORT_LSEEK,
        lseek_detour as *const () as usize,
        &ORIG_LSEEK,
    );
}

/// Whether the file calls under a version read are being counted.
pub(in crate::hooks::inline_hooks::cooked_cache) fn counting() -> bool {
    ORIG_READ.load(Ordering::Acquire) != 0 || ORIG_LSEEK.load(Ordering::Acquire) != 0
}

type IstreamReadFn = unsafe extern "thiscall-unwind" fn(*mut c_void, *mut u8, i32) -> *mut c_void;

/// `basic_istream<char>& basic_istream<char>::read(char*, streamsize)`.
/// Outside a version read this is one thread-local test and the call.
#[allow(improper_ctypes_definitions)]
unsafe extern "thiscall-unwind" fn istream_read_detour(
    this: *mut c_void,
    buffer: *mut u8,
    count: i32,
) -> *mut c_void {
    let orig = ORIG_ISTREAM_READ.load(Ordering::Acquire);
    if orig == 0 {
        return this;
    }
    let original: IstreamReadFn = unsafe { std::mem::transmute(orig) };
    if !probing() {
        return original(this, buffer, count);
    }
    // Before the read: what read entry wrote into the stream.
    let held = guarded(|| stream_held(&LiveMem, this as u32)).flatten();
    let result = original(this, buffer, count);
    let requested = u32::try_from(count).unwrap_or(0);
    if let Some(Some(read)) = guarded(|| stream_read(&LiveMem, this as u32, requested, held)) {
        // The version read makes one; keep the first if that changes.
        note(|probe| {
            if probe.stream.is_none() {
                probe.stream = Some(read);
            }
        });
    }
    result
}

fn report_open(path: String, oflag: i32, shflag: i32, error: i32, fd: Option<i32>) {
    let key = format!("{oflag}:{error}:{}", path.to_ascii_lowercase());
    let Decision::Emit { suppressed } = OPEN_THROTTLE.check(&key) else {
        return;
    };
    emit(
        TARGET_PAK_OPEN,
        crt_open_level(error),
        "io.pak_open",
        crt_open_fields(&path, oflag, shflag, error, fd, suppressed),
    );
}

type WsopenSFn = unsafe extern "C-unwind" fn(*mut i32, *const u16, i32, i32, i32) -> i32;

/// `errno_t _wsopen_s(int* fd, const wchar_t* name, int oflag, int shflag, int pmode)`.
#[allow(improper_ctypes_definitions)]
unsafe extern "C-unwind" fn wsopen_s_detour(
    fd: *mut i32,
    name: *const u16,
    oflag: i32,
    shflag: i32,
    pmode: i32,
) -> i32 {
    let orig = ORIG_WSOPEN_S.load(Ordering::Acquire);
    if orig == 0 {
        // EINVAL: unreachable, the slot only points here once it is set.
        return 22;
    }
    let original: WsopenSFn = unsafe { std::mem::transmute(orig) };
    let error = original(fd, name, oflag, shflag, pmode);
    // What the open left; formatting and queueing below may change it.
    let last_error = unsafe { GetLastError() };
    let _ = std::panic::catch_unwind(AssertUnwindSafe(|| {
        if let Some(s) = mem::wide_at(name as usize, MAX_PATH_CHARS) {
            if is_pak(&s.units) {
                let opened = (error == 0)
                    .then(|| mem::read_i32(&mem::process_reader, fd as usize))
                    .flatten();
                report_open(text::decode_wide(&s.units), oflag, shflag, error, opened);
            }
        }
    }));
    unsafe { SetLastError(last_error) };
    error
}

type ReadFn = unsafe extern "C-unwind" fn(i32, *mut c_void, u32) -> i32;

/// `int _read(int fd, void* buffer, unsigned count)`.
#[allow(improper_ctypes_definitions)]
unsafe extern "C-unwind" fn read_detour(fd: i32, buffer: *mut c_void, count: u32) -> i32 {
    let orig = ORIG_READ.load(Ordering::Acquire);
    if orig == 0 {
        return -1;
    }
    let original: ReadFn = unsafe { std::mem::transmute(orig) };
    let result = original(fd, buffer, count);
    if probing() {
        note(|probe| {
            if let Some(io) = probe.crt.as_mut() {
                count_read(io, count, result);
            }
        });
    }
    result
}

type LseekFn = unsafe extern "C-unwind" fn(i32, i32, i32) -> i32;

/// `long _lseek(int fd, long offset, int origin)`.
#[allow(improper_ctypes_definitions)]
unsafe extern "C-unwind" fn lseek_detour(fd: i32, offset: i32, origin: i32) -> i32 {
    let orig = ORIG_LSEEK.load(Ordering::Acquire);
    if orig == 0 {
        return -1;
    }
    let original: LseekFn = unsafe { std::mem::transmute(orig) };
    let result = original(fd, offset, origin);
    if probing() {
        note(|probe| {
            if let Some(io) = probe.crt.as_mut() {
                count_seek(io, result);
            }
        });
    }
    result
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::super::detours::probed;
    use super::super::{read_outcome, CrtIo, ReadProbe, StreamRead};
    use super::*;
    use crate::hooks::sinks::emit::take_captured;

    unsafe extern "C-unwind" fn fake_wsopen_s(
        fd: *mut i32,
        name: *const u16,
        _oflag: i32,
        _shflag: i32,
        _pmode: i32,
    ) -> i32 {
        // A name starting with `M` is missing.
        if unsafe { *name } == u16::from(b'M') {
            unsafe { SetLastError(2) };
            return 2;
        }
        unsafe { *fd = 7 };
        0
    }

    fn wide(s: &str) -> Vec<u16> {
        s.encode_utf16().chain(Some(0)).collect()
    }

    /// The zip library's open is reported for a `.pak`, opened or not,
    /// and the caller gets the runtime's own answer and last error.
    #[test]
    fn a_pak_opened_through_the_c_runtime_is_reported() {
        ORIG_WSOPEN_S.store(fake_wsopen_s as *const () as usize, Ordering::SeqCst);
        let _ = take_captured();
        let mut fd = -1;

        let other = wide("C:\\game\\notes.txt");
        assert_eq!(
            unsafe { wsopen_s_detour(&mut fd, other.as_ptr(), 0x8002, 0x20, 0x180) },
            0
        );
        assert!(take_captured().is_empty(), "only archives are reported");

        let pak = wide("C:\\cache\\TextStrings.pak");
        assert_eq!(
            unsafe { wsopen_s_detour(&mut fd, pak.as_ptr(), 0x8002, 0x20, 0x180) },
            0
        );
        let events = take_captured();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].target, TARGET_PAK_OPEN);
        assert_eq!(events[0].level, "info");
        assert_eq!(events[0].get("via"), Some(&json!("crt")));
        assert_eq!(events[0].get("opened"), Some(&json!(true)));
        assert_eq!(events[0].get("write"), Some(&json!(true)));
        assert_eq!(events[0].get("binary"), Some(&json!(true)));
        assert_eq!(events[0].get("disposition"), Some(&json!("open_existing")));
        assert_eq!(events[0].get("share_mode"), Some(&json!("deny_write")));
        assert_eq!(events[0].get("fd"), Some(&json!(7)));

        let missing = wide("Missing_local.pak");
        assert_eq!(
            unsafe { wsopen_s_detour(&mut fd, missing.as_ptr(), 0x8000, 0x20, 0x180) },
            2
        );
        assert_eq!(unsafe { GetLastError() }, 2, "the last error survives");
        let events = take_captured();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].level, "warn");
        assert_eq!(events[0].get("opened"), Some(&json!(false)));
        assert_eq!(events[0].get("error_name"), Some(&json!("no_such_file")));
        assert_eq!(events[0].get("fd"), None);
    }

    unsafe extern "C-unwind" fn fake_read(_fd: i32, _buffer: *mut c_void, count: u32) -> i32 {
        // Thirty bytes are there, whatever is asked for.
        count.min(30) as i32
    }

    unsafe extern "C-unwind" fn fake_lseek(_fd: i32, offset: i32, _origin: i32) -> i32 {
        offset
    }

    /// File calls are counted into a running version read and nowhere
    /// else, and pass their results through.
    #[test]
    fn file_calls_are_counted_only_under_a_version_read() {
        ORIG_READ.store(fake_read as *const () as usize, Ordering::SeqCst);
        ORIG_LSEEK.store(fake_lseek as *const () as usize, Ordering::SeqCst);
        let mut buffer = [0u8; 64];
        let ptr = buffer.as_mut_ptr().cast::<c_void>();

        // Outside: passed through, nothing to record into.
        assert_eq!(unsafe { read_detour(3, ptr, 16) }, 16);

        let (_, probe) = probed(|| unsafe {
            assert_eq!(lseek_detour(3, 1024, 0), 1024);
            assert_eq!(lseek_detour(3, -1, 0), -1);
            assert_eq!(read_detour(3, ptr, 30), 30);
            assert_eq!(read_detour(3, ptr, 64), 30);
        });
        assert_eq!(
            probe.crt,
            Some(CrtIo {
                reads: 2,
                read_bytes: 60,
                short_reads: 1,
                seeks: 2,
                failed_seeks: 1,
            })
        );
    }

    /// A stream as the detour reads it: the vbtable, `gcount`, the
    /// `basic_ios` with its state and buffer, and the buffer's put area.
    struct FakeStream {
        words: Box<[u32; 32]>,
        _vbtable: Box<[u32; 2]>,
        _buffer: Box<[u32; 16]>,
        _put: Box<[u32; 2]>,
    }

    const IOS: usize = 0x20;

    impl FakeStream {
        fn holding(written: u32) -> Self {
            let put = Box::new([0x5000u32, 0x5000 + written]);
            let mut buffer = Box::new([0u32; 16]);
            buffer[0x14 / 4] = &put[0] as *const u32 as u32;
            buffer[0x24 / 4] = &put[1] as *const u32 as u32;
            let vbtable = Box::new([0u32, IOS as u32]);
            let mut words = Box::new([0u32; 32]);
            words[0] = vbtable.as_ptr() as u32;
            words[(IOS + 0x28) / 4] = buffer.as_ptr() as u32;
            Self {
                words,
                _vbtable: vbtable,
                _buffer: buffer,
                _put: put,
            }
        }

        fn this(&mut self) -> *mut c_void {
            self.words.as_mut_ptr().cast()
        }
    }

    /// A read that finds nothing: `gcount` 0, eof and fail set, the
    /// destination untouched. What Wine's `strstreambuf` did in 2026-10.
    unsafe extern "thiscall-unwind" fn read_nothing(
        this: *mut c_void,
        _buffer: *mut u8,
        _count: i32,
    ) -> *mut c_void {
        let words = this.cast::<u32>();
        unsafe {
            *words.add(1) = 0;
            *words.add((IOS + 0x08) / 4) |= 3;
        }
        this
    }

    /// The read's request, result and the stream's state reach the
    /// probe; the game's call is passed through either way.
    #[test]
    fn a_stream_read_under_a_version_read_is_recorded() {
        ORIG_ISTREAM_READ.store(read_nothing as *const () as usize, Ordering::SeqCst);
        let mut stream = FakeStream::holding(4);
        let this = stream.this();
        let mut out = [0u8; 4];

        // Outside a version read: the call, and nothing else.
        assert_eq!(
            unsafe { istream_read_detour(this, out.as_mut_ptr(), 4) },
            this
        );

        let (_, probe) = probed(|| unsafe {
            assert_eq!(istream_read_detour(this, out.as_mut_ptr(), 4), this);
        });
        assert_eq!(
            probe.stream,
            Some(StreamRead {
                held: Some(4),
                requested: 4,
                count: 0,
                state: 3,
            })
        );
        assert_eq!(
            read_outcome(ReadProbe {
                entry_ok: Some(true),
                ..probe
            }),
            "stream_read_short"
        );
    }
}
