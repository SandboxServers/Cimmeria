//! Reading a CME event's named fields through the game's own getters.
//!
//! An inbound event (`Event_NetIn_*`) carries its arguments as a
//! name-keyed property bag. The stock handlers read it with three typed
//! getters, all `bool __thiscall (event*, const std::string* name, T* out)`,
//! `ret 8`, returning success in `AL` and leaving `out` untouched on a miss
//! (`docs/reverse-engineering/findings/ability-client-hook-anchors.md`
//! § The event bag):
//!
//! | Getter | Address | Reads |
//! |---|---|---|
//! | `GetInt` | `0x00e3cba0` | `INT32` |
//! | `GetFloat` | `0x00e3cc20` | `FLOAT` |
//! | `GetByte` | `0x00d434d0` | `INT8` / `UINT8` |
//!
//! The detours call them on the event their handler is about to read, on
//! the same (main) thread, before the original runs: the same calls, in
//! the same state, the handler makes itself. Each getter copies the
//! property tree, looks the name up, and frees the copy; nothing is
//! written to the event. All three are in the fingerprint gate.
//!
//! They are C++ (`6a ff 68` exception-frame prologue) and can throw, so a
//! caller must never put them under `catch_unwind`: catching a foreign
//! exception aborts or swallows it, unspecified which. The detours call
//! them directly from their `thiscall-unwind` frame, where a throw unwinds
//! to the game's own handler.
//!
//! The name is an MSVC 2008 `std::string`, built here by hand
//! ([`MsvcString`]): `+0x00` the allocator word, `+0x04` a 16-byte buffer
//! (the characters when `capacity < 16`, else a pointer to them), `+0x14`
//! the size, `+0x18` the capacity. That is the layout the getters' callers
//! build (`0x00e09160` tests `[name+0x18] < 0x10` before freeing the
//! heap pointer at `+0x04`). A name longer than 15 characters points at a
//! static NUL-terminated copy; the getters only read it, and it is never
//! freed because no destructor runs on this value.

/// An MSVC 2008 `std::basic_string<char>` value.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub(crate) struct MsvcString {
    alloc: u32,
    buf: [u8; 16],
    size: u32,
    capacity: u32,
}

impl MsvcString {
    /// A string for `name`, which must be NUL-terminated (`b"Type\0"`) and
    /// `'static`: a long name is pointed at, not copied.
    pub(crate) fn new(name: &'static [u8]) -> Self {
        let text = name.strip_suffix(b"\0").unwrap_or(name);
        let mut buf = [0u8; 16];
        let capacity = if text.len() < 16 {
            buf[..text.len()].copy_from_slice(text);
            15
        } else {
            // A heap-form string: the buffer's first word is the pointer.
            debug_assert!(name.ends_with(b"\0"), "a long name must be NUL-terminated");
            let ptr = name.as_ptr() as usize as u32;
            buf[..4].copy_from_slice(&ptr.to_le_bytes());
            text.len() as u32
        };
        Self {
            alloc: 0,
            buf,
            size: text.len() as u32,
            capacity,
        }
    }

    /// The characters, as the game's `c_str()` would see them.
    #[cfg(test)]
    pub(crate) fn text(&self) -> Vec<u8> {
        if self.capacity < 16 {
            self.buf[..self.size as usize].to_vec()
        } else {
            let ptr = u32::from_le_bytes(self.buf[..4].try_into().unwrap()) as usize;
            // SAFETY: test-only, and only for names built from `'static`
            // slices of `size` bytes.
            unsafe { std::slice::from_raw_parts(ptr as *const u8, self.size as usize).to_vec() }
        }
    }
}

/// The named fields of one event. The DLL reads them through the game's
/// getters ([`LiveBag`]); tests use a map.
pub(crate) trait Bag {
    /// An `INT32` field.
    fn int(&self, name: &'static [u8]) -> Option<i32>;
    /// A `FLOAT` field.
    fn float(&self, name: &'static [u8]) -> Option<f32>;
    /// An `INT8` / `UINT8` field.
    fn byte(&self, name: &'static [u8]) -> Option<u8>;
}

#[cfg(all(target_os = "windows", target_arch = "x86"))]
pub(crate) use native::LiveBag;

#[cfg(all(target_os = "windows", target_arch = "x86"))]
mod native {
    use super::*;
    use std::ffi::c_void;

    /// `GetInt`.
    pub(crate) const ADDR_GET_INT: usize = 0x00e3_cba0;
    /// `GetFloat`.
    pub(crate) const ADDR_GET_FLOAT: usize = 0x00e3_cc20;
    /// `GetByte`.
    pub(crate) const ADDR_GET_BYTE: usize = 0x00d4_34d0;

    type Getter<T> =
        unsafe extern "thiscall-unwind" fn(*mut c_void, *const MsvcString, *mut T) -> u8;

    /// The event a handler was called with.
    pub(crate) struct LiveBag(pub(crate) *mut c_void);

    impl LiveBag {
        fn get<T: Copy + Default>(&self, addr: usize, name: &'static [u8]) -> Option<T> {
            if self.0.is_null() {
                return None;
            }
            let key = MsvcString::new(name);
            let mut out = T::default();
            // SAFETY: `addr` is one of the three getters (fingerprinted with
            // every other site before any hook goes in), called with the
            // event the hooked handler received, on its thread, as the
            // handler itself calls it.
            let found = unsafe {
                let f: Getter<T> = std::mem::transmute(addr);
                f(self.0, &key, &mut out)
            };
            (found != 0).then_some(out)
        }
    }

    impl Bag for LiveBag {
        fn int(&self, name: &'static [u8]) -> Option<i32> {
            self.get(ADDR_GET_INT, name)
        }
        fn float(&self, name: &'static [u8]) -> Option<f32> {
            self.get(ADDR_GET_FLOAT, name)
        }
        fn byte(&self, name: &'static [u8]) -> Option<u8> {
            self.get(ADDR_GET_BYTE, name)
        }
    }
}

// The one definition of the getters' addresses; the router hook's own
// reader (`inline_hooks::ability::route`, which passes the method
// descriptor's strings as keys) uses them too.
#[cfg(all(target_os = "windows", target_arch = "x86"))]
pub(crate) use native::{ADDR_GET_BYTE, ADDR_GET_FLOAT, ADDR_GET_INT};

#[cfg(test)]
pub(crate) mod fake {
    //! A bag backed by a map, for the tests.
    use super::Bag;
    use std::collections::HashMap;

    /// Field values by name (without the NUL).
    #[derive(Default)]
    pub(crate) struct FakeBag {
        pub ints: HashMap<&'static str, i32>,
        pub floats: HashMap<&'static str, f32>,
        pub bytes: HashMap<&'static str, u8>,
    }

    fn key(name: &'static [u8]) -> &'static str {
        std::str::from_utf8(name.strip_suffix(b"\0").unwrap_or(name)).unwrap()
    }

    impl Bag for FakeBag {
        fn int(&self, name: &'static [u8]) -> Option<i32> {
            self.ints.get(key(name)).copied()
        }
        fn float(&self, name: &'static [u8]) -> Option<f32> {
            self.floats.get(key(name)).copied()
        }
        fn byte(&self, name: &'static [u8]) -> Option<u8> {
            self.bytes.get(key(name)).copied()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 0x1c bytes, with the size at +0x14 and the capacity at +0x18, as
    /// the getters' callers lay it out.
    #[test]
    fn the_layout_is_msvc_2008s() {
        assert_eq!(std::mem::size_of::<MsvcString>(), 0x1c);
        let s = MsvcString::new(b"Type\0");
        let bytes: [u8; 0x1c] = unsafe { std::mem::transmute(s) };
        assert_eq!(&bytes[4..8], b"Type");
        assert_eq!(bytes[8], 0, "NUL after the characters");
        assert_eq!(u32::from_le_bytes(bytes[0x14..0x18].try_into().unwrap()), 4);
        assert_eq!(
            u32::from_le_bytes(bytes[0x18..0x1c].try_into().unwrap()),
            15
        );
    }

    /// `BigWorldTimeComplete` (20 characters) does not fit the buffer: the
    /// buffer holds a pointer and the capacity says so (`>= 16`). The
    /// pointer is 32 bits, as in the client.
    #[cfg(target_pointer_width = "32")]
    #[test]
    fn a_long_name_points_at_its_static_copy() {
        static NAME: &[u8] = b"BigWorldTimeComplete\0";
        let s = MsvcString::new(NAME);
        assert_eq!(s.size, 20);
        assert!(s.capacity >= 16);
        assert_eq!(s.text(), b"BigWorldTimeComplete");
        let short = MsvcString::new(b"SecondaryId\0");
        assert_eq!(short.text(), b"SecondaryId");
        // Fifteen characters is the longest inline name.
        let fifteen = MsvcString::new(b"KismetEventSetS\0");
        assert_eq!((fifteen.size, fifteen.capacity), (15, 15));
    }
}
