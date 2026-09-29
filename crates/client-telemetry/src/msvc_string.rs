//! Bounded, read-only decoding of the client's MSVC `std::string` and
//! `std::wstring` objects.
//!
//! `SGW.exe` was built with the VS2005/2008 standard library. A string
//! object is 28 bytes on i686:
//!
//! ```text
//! +0x00  u32      allocator / iterator proxy (ignored)
//! +0x04  [u8;16]  inline buffer, or a heap pointer in its first 4 bytes
//! +0x14  u32      size, in characters
//! +0x18  u32      capacity, in characters
//! ```
//!
//! The data lives inline while the capacity is below the small-string
//! threshold: 16 characters for `std::string`, 8 for `std::wstring` (16
//! bytes either way). Both thresholds are read off the client's own
//! accessors: the event-registry key compare in `FUN_0158ea90`
//! (`cmp [key+0x18], 0x10; jb inline`) and the `std::wstring` element
//! accessor at `0x0046b250` (`cmp [this+0x18], 8; jb inline`). The CEGUI
//! `String` this client passes to `DefaultLogger::logEvent` is a
//! `std::wstring` (its stream insert at `0x00477ff0` reads the size at
//! `+0x14` and the characters through `0x0046b250`).
//!
//! The decode is split so the rules are testable on any host: [`locate`]
//! is pure and decides where the characters are and how many to read;
//! [`read`] is the one `unsafe` step that copies them out, and exists only
//! on the i686 DLL target, where the layout above is the real one.

/// Size of the MSVC string object on i686.
pub const OBJECT_SIZE: usize = 28;

/// Byte size of the inline buffer (both widths).
const INLINE_BYTES: usize = 16;

/// Character width of the string type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Width {
    /// `std::string`: one byte per character.
    Narrow,
    /// `std::wstring`: one UTF-16 unit per character.
    Wide,
}

impl Width {
    /// Bytes per character.
    pub const fn unit(self) -> usize {
        match self {
            Width::Narrow => 1,
            Width::Wide => 2,
        }
    }

    /// Capacity at or above which the characters are on the heap.
    pub const fn heap_threshold(self) -> u32 {
        (INLINE_BYTES / self.unit()) as u32
    }
}

/// Where a string's characters are, per [`locate`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Storage {
    /// Inside the object: these bytes (already cut to the size).
    Inline(Vec<u8>),
    /// On the heap at `ptr`: read `len_bytes` bytes.
    Heap {
        /// The heap pointer from the object.
        ptr: u32,
        /// How many bytes to read (at most `max_chars` characters).
        len_bytes: usize,
    },
    /// The header is not a plausible string (size above capacity, a null
    /// heap pointer, an inline size that does not fit). Nothing is read.
    Invalid,
}

/// A decoded string plus whether it was cut at the caller's cap.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Decoded {
    /// The characters read (lossy UTF-8 / UTF-16).
    pub text: String,
    /// `true` when the string was longer than the cap.
    pub truncated: bool,
}

/// Decide where the characters of the 28-byte object `obj` are, reading at
/// most `max_chars` characters. Pure: it never dereferences anything.
pub fn locate(obj: &[u8; OBJECT_SIZE], width: Width, max_chars: usize) -> (Storage, bool) {
    let word =
        |off: usize| u32::from_le_bytes([obj[off], obj[off + 1], obj[off + 2], obj[off + 3]]);
    let size = word(0x14);
    let cap = word(0x18);
    if size > cap {
        return (Storage::Invalid, false);
    }
    let truncated = size as usize > max_chars;
    let chars = (size as usize).min(max_chars);
    let len_bytes = chars * width.unit();
    if cap < width.heap_threshold() {
        // Inline: the whole string (plus its terminator) fits in 16 bytes.
        if size as usize * width.unit() > INLINE_BYTES {
            return (Storage::Invalid, false);
        }
        return (Storage::Inline(obj[4..4 + len_bytes].to_vec()), truncated);
    }
    let ptr = word(0x04);
    if ptr == 0 {
        return (Storage::Invalid, false);
    }
    (Storage::Heap { ptr, len_bytes }, truncated)
}

/// Turn the raw character bytes into text.
pub fn decode_bytes(bytes: &[u8], width: Width) -> String {
    match width {
        Width::Narrow => String::from_utf8_lossy(bytes).into_owned(),
        Width::Wide => {
            let (pairs, _odd) = bytes.as_chunks::<2>();
            let units: Vec<u16> = pairs.iter().map(|p| u16::from_le_bytes(*p)).collect();
            String::from_utf16_lossy(&units)
        }
    }
}

/// Copy a live MSVC string out of the client, reading at most `max_chars`
/// characters. `None` for a null pointer or an implausible header.
///
/// # Safety
///
/// `obj` must point at a live MSVC `std::string` (for [`Width::Narrow`])
/// or `std::wstring` (for [`Width::Wide`]) that stays alive for the
/// call: in practice, an argument of the hooked function being read from
/// inside its detour, before the original runs.
#[cfg(target_arch = "x86")]
pub unsafe fn read(obj: *const u8, width: Width, max_chars: usize) -> Option<Decoded> {
    if obj.is_null() {
        return None;
    }
    let mut header = [0u8; OBJECT_SIZE];
    // SAFETY: the caller guarantees `obj` is a live string object, which is
    // exactly OBJECT_SIZE bytes on i686.
    unsafe { std::ptr::copy_nonoverlapping(obj, header.as_mut_ptr(), OBJECT_SIZE) };
    let (storage, truncated) = locate(&header, width, max_chars);
    let bytes = match storage {
        Storage::Invalid => return None,
        Storage::Inline(b) => b,
        Storage::Heap { ptr, len_bytes } => {
            // SAFETY: a valid string's heap buffer holds at least `size`
            // characters; `len_bytes` never exceeds that.
            unsafe { std::slice::from_raw_parts(ptr as usize as *const u8, len_bytes) }.to_vec()
        }
    };
    Some(Decoded {
        text: decode_bytes(&bytes, width),
        truncated,
    })
}

/// Like [`read`], but every read goes through `ReadProcessMemory`, so a
/// dangling or garbage pointer yields `None` instead of an access
/// violation. Use it for objects the detour does not own (a string reached
/// through a pointer in a game structure, or an argument of a function
/// whose callers were not all inspected).
#[cfg(all(target_os = "windows", target_arch = "x86"))]
pub fn read_checked(obj: usize, width: Width, max_chars: usize) -> Option<Decoded> {
    use cimmeria_client_hookgate::os::read_bytes;
    let header: [u8; OBJECT_SIZE] = read_bytes(obj, OBJECT_SIZE)?.try_into().ok()?;
    let (storage, truncated) = locate(&header, width, max_chars);
    let bytes = match storage {
        Storage::Invalid => return None,
        Storage::Inline(b) => b,
        Storage::Heap { ptr, len_bytes } => read_bytes(ptr as usize, len_bytes)?,
    };
    Some(Decoded {
        text: decode_bytes(&bytes, width),
        truncated,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn header(buf: &[u8], ptr: Option<u32>, size: u32, cap: u32) -> [u8; OBJECT_SIZE] {
        let mut h = [0u8; OBJECT_SIZE];
        h[0..4].copy_from_slice(&0xDEAD_BEEFu32.to_le_bytes());
        match ptr {
            Some(p) => h[4..8].copy_from_slice(&p.to_le_bytes()),
            None => h[4..4 + buf.len()].copy_from_slice(buf),
        }
        h[0x14..0x18].copy_from_slice(&size.to_le_bytes());
        h[0x18..0x1c].copy_from_slice(&cap.to_le_bytes());
        h
    }

    #[test]
    fn a_short_narrow_string_is_inline() {
        let h = header(b"Event_Net\0", None, 9, 15);
        let (s, truncated) = locate(&h, Width::Narrow, 128);
        assert_eq!(s, Storage::Inline(b"Event_Net".to_vec()));
        assert!(!truncated);
    }

    /// The small-string threshold differs by width: capacity 15 is inline
    /// for `std::string` but 8 is already the heap for `std::wstring`.
    #[test]
    fn the_heap_threshold_is_sixteen_bytes_for_both_widths() {
        assert_eq!(Width::Narrow.heap_threshold(), 16);
        assert_eq!(Width::Wide.heap_threshold(), 8);
        let narrow = header(&[], Some(0x0100_0000), 20, 31);
        assert!(matches!(
            locate(&narrow, Width::Narrow, 128).0,
            Storage::Heap {
                ptr: 0x0100_0000,
                len_bytes: 20
            }
        ));
        let wide = header(&[], Some(0x0200_0000), 8, 8);
        assert!(matches!(
            locate(&wide, Width::Wide, 128).0,
            Storage::Heap {
                ptr: 0x0200_0000,
                len_bytes: 16
            }
        ));
    }

    #[test]
    fn a_short_wide_string_is_inline_and_decodes() {
        let bytes: Vec<u8> = "Lua".encode_utf16().flat_map(|u| u.to_le_bytes()).collect();
        let h = header(&bytes, None, 3, 7);
        let (s, _) = locate(&h, Width::Wide, 128);
        let Storage::Inline(b) = s else {
            panic!("expected inline, got {s:?}")
        };
        assert_eq!(decode_bytes(&b, Width::Wide), "Lua");
    }

    /// A long string is cut at the cap, and says so.
    #[test]
    fn a_long_heap_string_is_capped() {
        let h = header(&[], Some(0x0100_0000), 5000, 5000);
        let (s, truncated) = locate(&h, Width::Wide, 256);
        assert_eq!(
            s,
            Storage::Heap {
                ptr: 0x0100_0000,
                len_bytes: 512
            }
        );
        assert!(truncated);
    }

    /// Garbage headers read nothing: this is what keeps a wrong pointer
    /// from turning into a multi-megabyte copy.
    #[test]
    fn implausible_headers_are_invalid() {
        // size > capacity
        assert_eq!(
            locate(&header(&[], Some(1), 40, 20), Width::Narrow, 128).0,
            Storage::Invalid
        );
        // heap string with a null pointer
        assert_eq!(
            locate(&header(&[], Some(0), 20, 31), Width::Narrow, 128).0,
            Storage::Invalid
        );
        // "inline" wide string longer than the inline buffer
        assert_eq!(
            locate(&header(&[], None, 9, 7), Width::Wide, 128).0,
            Storage::Invalid
        );
    }

    #[test]
    fn lossy_decode_never_panics() {
        assert_eq!(decode_bytes(&[0xff, 0x41], Width::Narrow), "\u{fffd}A");
        assert_eq!(decode_bytes(&[0x00, 0xd8], Width::Wide), "\u{fffd}");
    }

    /// On the real target, read a string object laid out exactly as the
    /// client lays it out, from both storage kinds.
    #[cfg(target_arch = "x86")]
    #[test]
    fn read_copies_inline_and_heap_strings() {
        let inline = header(b"onDialog\0", None, 8, 15);
        let got = unsafe { read(inline.as_ptr(), Width::Narrow, 128) }.unwrap();
        assert_eq!(got.text, "onDialog");

        let heap_text = b"Event_NetIn_onDialogDisplay".to_vec();
        let heap = header(&[], Some(heap_text.as_ptr() as u32), 27, 31);
        let got = unsafe { read(heap.as_ptr(), Width::Narrow, 128) }.unwrap();
        assert_eq!(got.text, "Event_NetIn_onDialogDisplay");
        assert!(!got.truncated);

        assert!(unsafe { read(std::ptr::null(), Width::Narrow, 128) }.is_none());
    }

    /// The checked reader decodes a live object and refuses a wild pointer.
    #[cfg(all(target_os = "windows", target_arch = "x86"))]
    #[test]
    fn read_checked_decodes_and_refuses_wild_pointers() {
        let inline = header(b"onDialog\0", None, 8, 15);
        let got = read_checked(inline.as_ptr() as usize, Width::Narrow, 128).unwrap();
        assert_eq!(got.text, "onDialog");
        assert!(read_checked(0x10, Width::Narrow, 128).is_none());
        // A heap pointer that points nowhere.
        let bad = header(&[], Some(0x20), 20, 31);
        assert!(read_checked(bad.as_ptr() as usize, Width::Narrow, 128).is_none());
    }
}
