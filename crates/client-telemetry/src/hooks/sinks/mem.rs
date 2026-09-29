//! Reading the client's memory from inside a detour without ever faulting.
//!
//! A detour reads arguments the game handed it: a wide string, an object
//! pointer, a table entry. The game's own code trusts them; a detour that
//! reads one wrong must not be the thing that crashes the client. Every
//! read here goes through a [`Reader`], which reports how many bytes it
//! could read and never raises. In the DLL that is `ReadProcessMemory` on
//! the current process, which reports an unmapped page as a failed call.
//! In the tests it is a byte map.
//!
//! The string readers walk in chunks that never cross a page boundary, so
//! a string that ends just before an unmapped page is still read whole.

/// Page size the chunking respects.
const PAGE: usize = 0x1000;

/// Largest single read. Small, so a short string near the end of a mapped
/// region costs one small read rather than a large failing one.
const CHUNK: usize = 128;

/// A memory reader: fill `buf` from `addr` and return how many bytes were
/// read (0 when the range is unreadable). Never panics, never faults.
pub type Reader<'a> = &'a dyn Fn(usize, &mut [u8]) -> usize;

/// A string read out of the client.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReadStr<T> {
    /// The units before the terminator (or up to the limit).
    pub units: Vec<T>,
    /// The limit was hit before a terminator.
    pub truncated: bool,
}

/// Read a NUL-terminated narrow string of at most `max_chars` bytes.
/// `None` when the first byte is unreadable.
pub fn read_ansi_z(read: Reader, addr: usize, max_chars: usize) -> Option<ReadStr<u8>> {
    let mut out: Vec<u8> = Vec::new();
    let mut cursor = addr;
    if addr == 0 {
        return None;
    }
    loop {
        let want = chunk_len(cursor);
        let mut buf = [0u8; CHUNK];
        let got = read(cursor, &mut buf[..want]);
        if got == 0 {
            // Unreadable from here on: nothing at all is a failure, a
            // partial string is what we have.
            return if out.is_empty() {
                None
            } else {
                Some(ReadStr {
                    units: out,
                    truncated: true,
                })
            };
        }
        for &b in &buf[..got] {
            if b == 0 {
                return Some(ReadStr {
                    units: out,
                    truncated: false,
                });
            }
            if out.len() >= max_chars {
                return Some(ReadStr {
                    units: out,
                    truncated: true,
                });
            }
            out.push(b);
        }
        cursor += got;
    }
}

/// Read a NUL-terminated wide (UTF-16) string of at most `max_chars` units.
/// `None` when the first unit is unreadable. An odd trailing byte at the end
/// of readable memory is dropped.
pub fn read_wide_z(read: Reader, addr: usize, max_chars: usize) -> Option<ReadStr<u16>> {
    let mut out: Vec<u16> = Vec::new();
    let mut cursor = addr;
    let mut carry: Option<u8> = None;
    if addr == 0 {
        return None;
    }
    loop {
        let want = chunk_len(cursor);
        let mut buf = [0u8; CHUNK];
        let got = read(cursor, &mut buf[..want]);
        if got == 0 {
            return if out.is_empty() {
                None
            } else {
                Some(ReadStr {
                    units: out,
                    truncated: true,
                })
            };
        }
        for &b in &buf[..got] {
            let Some(lo) = carry.take() else {
                carry = Some(b);
                continue;
            };
            let unit = u16::from_le_bytes([lo, b]);
            if unit == 0 {
                return Some(ReadStr {
                    units: out,
                    truncated: false,
                });
            }
            if out.len() >= max_chars {
                return Some(ReadStr {
                    units: out,
                    truncated: true,
                });
            }
            out.push(unit);
        }
        cursor += got;
    }
}

/// Bytes to request at `cursor`: up to [`CHUNK`], stopping at the end of the
/// page so one unmapped neighbour cannot fail a read that starts in a
/// mapped page.
fn chunk_len(cursor: usize) -> usize {
    let to_page_end = PAGE - (cursor % PAGE);
    to_page_end.min(CHUNK)
}

/// Read a little-endian `u32`.
pub fn read_u32(read: Reader, addr: usize) -> Option<u32> {
    let mut b = [0u8; 4];
    (addr != 0 && read(addr, &mut b) == 4).then(|| u32::from_le_bytes(b))
}

/// Read a little-endian `i32`.
pub fn read_i32(read: Reader, addr: usize) -> Option<i32> {
    read_u32(read, addr).map(|v| v as i32)
}

/// Read a byte.
pub fn read_u8(read: Reader, addr: usize) -> Option<u8> {
    let mut b = [0u8; 1];
    (addr != 0 && read(addr, &mut b) == 1).then_some(b[0])
}

/// The DLL's [`Reader`]: `ReadProcessMemory` on the current process.
#[cfg(windows)]
pub fn process_reader(addr: usize, buf: &mut [u8]) -> usize {
    use windows_sys::Win32::System::Diagnostics::Debug::ReadProcessMemory;
    use windows_sys::Win32::System::Threading::GetCurrentProcess;

    if buf.is_empty() || addr == 0 {
        return 0;
    }
    let mut copied = 0usize;
    // SAFETY: `buf` is a writable buffer of `buf.len()` bytes; the kernel
    // validates the source range and reports a bad one as a failed call.
    let ok = unsafe {
        ReadProcessMemory(
            GetCurrentProcess(),
            addr as *const core::ffi::c_void,
            buf.as_mut_ptr().cast(),
            buf.len(),
            &mut copied,
        )
    };
    if ok != 0 {
        copied
    } else {
        0
    }
}

/// Read a narrow string from the client's memory.
#[cfg(windows)]
pub fn ansi_at(addr: usize, max_chars: usize) -> Option<ReadStr<u8>> {
    read_ansi_z(&process_reader, addr, max_chars)
}

/// Read a wide string from the client's memory.
#[cfg(windows)]
pub fn wide_at(addr: usize, max_chars: usize) -> Option<ReadStr<u16>> {
    read_wide_z(&process_reader, addr, max_chars)
}

#[cfg(test)]
pub(crate) mod fake {
    //! A byte-map address space for the tests, with unmapped holes.
    //!
    //! Mapped in whole pages, like the real thing: any page a `put` touches
    //! is readable in full (unwritten bytes read as zero), and a read that
    //! reaches an unmapped page fails whole, as `ReadProcessMemory` does.

    use std::collections::{BTreeMap, BTreeSet};

    const PAGE: usize = 0x1000;

    /// Bytes by address, and which pages are mapped.
    #[derive(Default)]
    pub struct FakeMemory {
        bytes: BTreeMap<usize, u8>,
        pages: BTreeSet<usize>,
    }

    impl FakeMemory {
        pub fn new() -> Self {
            Self::default()
        }

        /// Map `data` at `addr`.
        pub fn put(&mut self, addr: usize, data: &[u8]) -> &mut Self {
            for (i, b) in data.iter().enumerate() {
                self.bytes.insert(addr + i, *b);
                self.pages.insert((addr + i) / PAGE);
            }
            self
        }

        /// Map a narrow string with its NUL.
        pub fn put_ansi(&mut self, addr: usize, s: &str) -> &mut Self {
            self.put(addr, s.as_bytes());
            self.put(addr + s.len(), &[0])
        }

        /// Map a wide string with its NUL.
        pub fn put_wide(&mut self, addr: usize, s: &str) -> &mut Self {
            let mut data: Vec<u8> = s.encode_utf16().flat_map(u16::to_le_bytes).collect();
            data.extend_from_slice(&[0, 0]);
            self.put(addr, &data)
        }

        /// The reader closure. Like `ReadProcessMemory`, a range that touches
        /// an unmapped page fails whole.
        pub fn reader(&self) -> impl Fn(usize, &mut [u8]) -> usize + '_ {
            move |addr, buf| {
                for (i, slot) in buf.iter_mut().enumerate() {
                    let at = addr + i;
                    if !self.pages.contains(&(at / PAGE)) {
                        return 0;
                    }
                    *slot = self.bytes.get(&at).copied().unwrap_or(0);
                }
                buf.len()
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::fake::FakeMemory;
    use super::*;

    #[test]
    fn a_narrow_string_is_read_to_its_terminator() {
        let mut m = FakeMemory::new();
        m.put_ansi(0x1000, "hello");
        let r = m.reader();
        assert_eq!(
            read_ansi_z(&r, 0x1000, 64),
            Some(ReadStr {
                units: b"hello".to_vec(),
                truncated: false
            })
        );
    }

    #[test]
    fn a_narrow_string_longer_than_the_limit_is_cut_and_flagged() {
        let mut m = FakeMemory::new();
        m.put_ansi(0x1000, &"a".repeat(300));
        let r = m.reader();
        let s = read_ansi_z(&r, 0x1000, 200).unwrap();
        assert_eq!(s.units.len(), 200);
        assert!(s.truncated);
        // Exactly at the limit is not truncated.
        let s = read_ansi_z(&r, 0x1000, 300).unwrap();
        assert_eq!(s.units.len(), 300);
        assert!(!s.truncated);
    }

    /// The reason for page-bounded chunks: a string that ends right before
    /// an unmapped page must read whole, not fail because a 128-byte read
    /// would have crossed into the hole.
    #[test]
    fn a_string_ending_at_an_unmapped_page_is_read_whole() {
        let mut m = FakeMemory::new();
        // Ends (NUL included) exactly at the page end 0x2000; 0x2000+ is unmapped.
        let s = "end-of-page";
        let start = 0x2000 - s.len() - 1;
        m.put_ansi(start, s);
        let r = m.reader();
        assert_eq!(
            read_ansi_z(&r, start, 64).unwrap().units,
            s.as_bytes().to_vec()
        );
    }

    #[test]
    fn an_unmapped_start_is_none_and_a_null_pointer_is_none() {
        let m = FakeMemory::new();
        let r = m.reader();
        assert_eq!(read_ansi_z(&r, 0x5000, 8), None);
        assert_eq!(read_ansi_z(&r, 0, 8), None);
        assert_eq!(read_wide_z(&r, 0x5000, 8), None);
        assert_eq!(read_wide_z(&r, 0, 8), None);
    }

    /// A string whose terminator is in an unmapped page (a corrupt pointer
    /// into the tail of a mapping) yields what was readable, flagged.
    #[test]
    fn a_string_running_into_a_hole_is_partial_and_flagged() {
        let mut m = FakeMemory::new();
        let start = 0x3000 - 4;
        m.put(start, b"abcd");
        let r = m.reader();
        let s = read_ansi_z(&r, start, 64).unwrap();
        assert_eq!(s.units, b"abcd".to_vec());
        assert!(s.truncated);
    }

    #[test]
    fn a_wide_string_is_read_to_its_terminator() {
        let mut m = FakeMemory::new();
        m.put_wide(0x4000, "Log: hi \u{e9}");
        let r = m.reader();
        let s = read_wide_z(&r, 0x4000, 64).unwrap();
        assert_eq!(String::from_utf16_lossy(&s.units), "Log: hi \u{e9}");
        assert!(!s.truncated);
    }

    #[test]
    fn a_wide_string_is_cut_at_the_limit() {
        let mut m = FakeMemory::new();
        m.put_wide(0x4000, &"w".repeat(100));
        let r = m.reader();
        let s = read_wide_z(&r, 0x4000, 40).unwrap();
        assert_eq!(s.units.len(), 40);
        assert!(s.truncated);
    }

    /// A wide string that spans two chunks: the byte pair straddling the
    /// chunk seam must not be split.
    #[test]
    fn a_wide_string_across_a_chunk_seam_is_intact() {
        let mut m = FakeMemory::new();
        let text = "0123456789".repeat(20); // 200 units = 400 bytes = 4 chunks
        m.put_wide(0x4000, &text);
        let r = m.reader();
        let s = read_wide_z(&r, 0x4000, 400).unwrap();
        assert_eq!(String::from_utf16_lossy(&s.units), text);
    }

    #[test]
    fn scalar_reads_are_little_endian_and_fail_on_holes() {
        let mut m = FakeMemory::new();
        m.put(0x100, &0x1234_5678u32.to_le_bytes());
        m.put(0x200, &(-5i32).to_le_bytes());
        m.put(0x300, &[0xAB]);
        let r = m.reader();
        assert_eq!(read_u32(&r, 0x100), Some(0x1234_5678));
        assert_eq!(read_i32(&r, 0x200), Some(-5));
        assert_eq!(read_u8(&r, 0x300), Some(0xAB));
        assert_eq!(read_u32(&r, 0x9000), None);
        assert_eq!(read_u32(&r, 0), None);
        drop(r);
        // A word that straddles into an unmapped page fails whole.
        m.put(0x1FFD, &[1, 2, 3]);
        let r = m.reader();
        assert_eq!(read_u32(&r, 0x1FFD), None);
    }

    #[test]
    fn chunks_stop_at_page_ends() {
        assert_eq!(chunk_len(0x1000), CHUNK);
        assert_eq!(chunk_len(0x1FF0), 0x10);
        assert_eq!(chunk_len(0x1FFF), 1);
    }
}
