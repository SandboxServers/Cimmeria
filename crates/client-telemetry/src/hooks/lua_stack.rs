//! Read-only access to a Lua stack from inside a detour.
//!
//! The client's `lua51.dll` is a **wide, C++-compiled** Lua 5.1: strings
//! are `wchar_t`, and errors are C++ exceptions. The readers here call
//! only exports that allocate nothing and raise nothing on the inputs they
//! are given:
//!
//! - `lua_type(L, idx)`: any index; an index past the top is `LUA_TNONE`.
//! - `lua_tolstring(L, idx, &len)`: only after `lua_type` said the value
//!   is a string (on a number it would convert the value in place).
//! - `lua_pushvalue(L, -1)` + `lua_getinfo(L, L">S", &ar)`: only when the
//!   top is a function and the stack has room for the copy (checked from
//!   `L->top` / `L->stack_last`, see [`STATE_TOP`]). `">S"` pops the copy
//!   and fills the source fields; it grows nothing (only `f` and `L` push).
//!
//! Every export is resolved by its mangled name from the loaded
//! `lua51.dll` (export table checked against the QA client's DLL on
//! 2026-09-29); if one is missing, the reader that needs it returns
//! `None` and the event goes out without that field.
//!
//! Layouts, from Ghidra on the QA `lua51.dll` (2026-09-29):
//! `lua_getinfo` @ `0x10007a50` reads `L+0x08` (top) and `L+0x1c`
//! (stack_last) and `ar+0x9c` (`i_ci`); its `S` filler writes `what`
//! `+0x0c`, `source` `+0x10`, `linedefined` `+0x1c`,
//! `lastlinedefined` `+0x20` and `short_src` (`wchar_t[60]`) `+0x24`.

/// `LUA_TSTRING`.
pub(crate) const LUA_TSTRING: i32 = 4;
/// `LUA_TFUNCTION`.
pub(crate) const LUA_TFUNCTION: i32 = 6;

/// `lua_State` offset of `top` (a `TValue*`, 12-byte values).
pub(crate) const STATE_TOP: usize = 0x08;
/// `lua_State` offset of `stack_last`.
pub(crate) const STATE_STACK_LAST: usize = 0x1c;
/// Size of one stack slot (`TValue`: 8-byte value + 4-byte tag).
pub(crate) const TVALUE_SIZE: u32 = 12;

/// Size of `lua_Debug` in this build (`i_ci` at `0x9c` is the last field).
pub(crate) const LUA_DEBUG_SIZE: usize = 0xa0;
/// The buffer handed to `lua_getinfo`: `lua_Debug` plus slack, so a build
/// whose record is somewhat larger still writes inside it.
pub(crate) const LUA_DEBUG_BUFFER: usize = LUA_DEBUG_SIZE + 0x60;

/// The first 24 bytes of `lua_getinfo` in the QA `lua51.dll` (export at
/// `0x10007a50`; read from the file 2026-09-29). The `L+0x08` / `L+0x1c`
/// stack-room check and the `lua_Debug` layout above belong to this build,
/// so the function-info reader runs only when these bytes match; any other
/// `lua51.dll` gets the error rows without `function_source`.
pub(crate) const LUA_GETINFO_PROLOGUE: [u8; 24] = [
    0x8b, 0x44, 0x24, 0x08, 0x53, 0x55, 0x56, 0x8b, 0x74, 0x24, 0x18, 0x33, 0xed, 0x33, 0xdb, 0x66,
    0x83, 0x38, 0x3e, 0x57, 0x8b, 0x7c, 0x24, 0x14,
];

/// Whether the bytes at the resolved `lua_getinfo` are the QA build's.
pub(crate) fn is_known_getinfo(prologue: Option<&[u8]>) -> bool {
    prologue == Some(&LUA_GETINFO_PROLOGUE[..])
}
/// `lua_Debug::linedefined`.
pub(crate) const AR_LINEDEFINED: usize = 0x1c;
/// `lua_Debug::short_src`, `wchar_t[LUA_IDSIZE]`.
pub(crate) const AR_SHORT_SRC: usize = 0x24;
/// `LUA_IDSIZE`, in characters.
pub(crate) const LUA_IDSIZE: usize = 60;

/// The Lua 5.1 name of a type tag (`lua_typename`'s table), without a call.
pub(crate) fn type_name(tag: i32) -> &'static str {
    match tag {
        -1 => "none",
        0 => "nil",
        1 => "boolean",
        2 => "userdata",
        3 => "number",
        4 => "string",
        5 => "table",
        6 => "function",
        7 => "userdata",
        8 => "thread",
        _ => "unknown",
    }
}

/// Whether one more value fits on the stack: `luaD_checkstack(L, 1)` grows
/// the stack when `stack_last - top <= 12`, so a push is safe without a
/// grow only when the gap is larger.
pub(crate) fn has_room_for_one(top: u32, stack_last: u32) -> bool {
    top != 0 && stack_last > top && stack_last - top > TVALUE_SIZE
}

/// `short_src` and `linedefined` from the bytes of a filled `lua_Debug`.
/// `short_src` is NUL-terminated inside its 60 wide characters.
pub(crate) fn decode_debug(ar: &[u8]) -> Option<(String, i32)> {
    let line = ar.get(AR_LINEDEFINED..AR_LINEDEFINED + 4)?;
    let line = i32::from_le_bytes(line.try_into().ok()?);
    let src = ar.get(AR_SHORT_SRC..AR_SHORT_SRC + LUA_IDSIZE * 2)?;
    let units: Vec<u16> = src
        .as_chunks::<2>()
        .0
        .iter()
        .map(|c| u16::from_le_bytes(*c))
        .take_while(|&u| u != 0)
        .collect();
    Some((String::from_utf16_lossy(&units), line))
}

#[cfg(all(target_os = "windows", target_arch = "x86"))]
mod native {
    use super::*;
    use std::ffi::c_void;
    use std::sync::OnceLock;

    type LuaTypeFn = unsafe extern "C" fn(*mut c_void, i32) -> i32;
    type ToLStringFn = unsafe extern "C" fn(*mut c_void, i32, *mut u32) -> *const u16;
    type PushValueFn = unsafe extern "C" fn(*mut c_void, i32);
    type GetInfoFn = unsafe extern "C" fn(*mut c_void, *const u16, *mut u8) -> i32;

    /// The `lua51.dll` exports the readers use, resolved once.
    struct Api {
        lua_type: LuaTypeFn,
        tolstring: ToLStringFn,
        pushvalue: Option<PushValueFn>,
        getinfo: Option<GetInfoFn>,
    }

    static API: OnceLock<Option<Api>> = OnceLock::new();

    fn export(symbol: &core::ffi::CStr) -> Option<usize> {
        use windows_sys::Win32::System::LibraryLoader::{GetModuleHandleW, GetProcAddress};
        let wide: Vec<u16> = "lua51.dll".encode_utf16().chain(Some(0)).collect();
        // SAFETY: NUL-terminated strings that outlive the calls; no
        // reference to the module is kept.
        unsafe {
            let module = GetModuleHandleW(wide.as_ptr());
            if module.is_null() {
                return None;
            }
            GetProcAddress(module, symbol.as_ptr().cast()).map(|f| f as usize)
        }
    }

    fn api() -> Option<&'static Api> {
        API.get_or_init(|| {
            let t = export(c"?lua_type@@YAHPAUlua_State@@H@Z")?;
            let s = export(c"?lua_tolstring@@YAPB_WPAUlua_State@@HPAI@Z")?;
            let p = export(c"?lua_pushvalue@@YAXPAUlua_State@@H@Z");
            // `lua_getinfo` is used only on the build whose layouts this
            // module encodes (see `LUA_GETINFO_PROLOGUE`).
            let g = export(c"?lua_getinfo@@YAHPAUlua_State@@PB_WPAUlua_Debug@@@Z").filter(|&a| {
                is_known_getinfo(
                    cimmeria_client_hookgate::os::read_bytes(a, LUA_GETINFO_PROLOGUE.len())
                        .as_deref(),
                )
            });
            // SAFETY: the exports' signatures, from the mangled names.
            unsafe {
                Some(Api {
                    lua_type: std::mem::transmute::<usize, LuaTypeFn>(t),
                    tolstring: std::mem::transmute::<usize, ToLStringFn>(s),
                    pushvalue: p.map(|a| std::mem::transmute::<usize, PushValueFn>(a)),
                    getinfo: g.map(|a| std::mem::transmute::<usize, GetInfoFn>(a)),
                })
            }
        })
        .as_ref()
    }

    /// The type tag of the value at `idx`, or `None` without the API.
    pub(crate) fn value_type(l: *mut c_void, idx: i32) -> Option<i32> {
        let api = api()?;
        // SAFETY: `l` is a live state on this thread (the caller is inside
        // or just returned from a call on it); `lua_type` accepts any index.
        Some(unsafe { (api.lua_type)(l, idx) })
    }

    /// The string at `idx`, up to `max_chars` characters, and whether it
    /// was cut. `None` unless the value is a string.
    pub(crate) fn read_string(
        l: *mut c_void,
        idx: i32,
        max_chars: usize,
    ) -> Option<(String, bool)> {
        let api = api()?;
        // SAFETY: as in `value_type`; `lua_tolstring` runs only on a string
        // value, which it returns without converting or allocating.
        unsafe {
            if (api.lua_type)(l, idx) != LUA_TSTRING {
                return None;
            }
            let mut len: u32 = 0;
            let ptr = (api.tolstring)(l, idx, &mut len);
            if ptr.is_null() {
                return None;
            }
            let chars = (len as usize).min(max_chars);
            let bytes = cimmeria_client_hookgate::os::read_bytes(ptr as usize, chars * 2)?;
            Some((
                crate::msvc_string::decode_bytes(&bytes, crate::msvc_string::Width::Wide),
                len as usize > max_chars,
            ))
        }
    }

    /// `short_src` and `linedefined` of the function on top of the stack.
    /// `None` when the top is not a function, the stack has no free slot
    /// for the copy `lua_getinfo(">S")` pops, or an export is missing. The
    /// stack is left as it was.
    pub(crate) fn top_function_info(l: *mut c_void) -> Option<(String, i32)> {
        let api = api()?;
        let (pushvalue, getinfo) = (api.pushvalue?, api.getinfo?);
        let state = cimmeria_client_hookgate::os::read_bytes(l as usize, STATE_STACK_LAST + 4)?;
        let word =
            |o: usize| u32::from_le_bytes([state[o], state[o + 1], state[o + 2], state[o + 3]]);
        if !has_room_for_one(word(STATE_TOP), word(STATE_STACK_LAST)) {
            return None;
        }
        let mut ar = [0u8; LUA_DEBUG_BUFFER];
        let what: Vec<u16> = ">S".encode_utf16().chain(Some(0)).collect();
        // SAFETY: the top is a function (checked), one slot is free
        // (checked), and `">S"` pops exactly the copy pushed here while
        // filling `ar`, which is the size of this build's `lua_Debug`.
        unsafe {
            if (api.lua_type)(l, -1) != LUA_TFUNCTION {
                return None;
            }
            pushvalue(l, -1);
            if getinfo(l, what.as_ptr(), ar.as_mut_ptr()) == 0 {
                return None;
            }
        }
        decode_debug(&ar)
    }
}

#[cfg(all(target_os = "windows", target_arch = "x86"))]
pub(crate) use native::{read_string, top_function_info, value_type};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn type_names_follow_lua_51() {
        assert_eq!(type_name(-1), "none");
        assert_eq!(type_name(0), "nil");
        assert_eq!(type_name(LUA_TSTRING), "string");
        assert_eq!(type_name(LUA_TFUNCTION), "function");
        assert_eq!(type_name(5), "table");
        assert_eq!(type_name(99), "unknown");
    }

    /// A push is allowed only when `luaD_checkstack(L, 1)` would not grow
    /// the stack: a grow allocates and can raise inside a detour.
    #[test]
    fn getinfo_is_used_only_on_the_qa_lua51_build() {
        assert!(is_known_getinfo(Some(&LUA_GETINFO_PROLOGUE[..])));
        let mut other = LUA_GETINFO_PROLOGUE;
        other[18] = 0x3c;
        assert!(!is_known_getinfo(Some(&other[..])));
        assert!(!is_known_getinfo(Some(&LUA_GETINFO_PROLOGUE[..8])));
        assert!(!is_known_getinfo(None));
    }

    #[test]
    fn room_for_one_matches_the_grow_threshold() {
        assert!(has_room_for_one(0x1000, 0x1000 + 13));
        assert!(!has_room_for_one(0x1000, 0x1000 + 12));
        assert!(!has_room_for_one(0x1000, 0x1000));
        assert!(!has_room_for_one(0x1000, 0x0fff));
        assert!(!has_room_for_one(0, 0x100));
    }

    /// `short_src` stops at its NUL; `linedefined` is read at `0x1c`.
    #[test]
    fn debug_record_decodes_source_and_line() {
        let mut ar = vec![0xAAu8; LUA_DEBUG_SIZE];
        ar[AR_LINEDEFINED..AR_LINEDEFINED + 4].copy_from_slice(&42i32.to_le_bytes());
        let src: Vec<u8> = "[string \"BlackMarket.lua\"]"
            .encode_utf16()
            .chain(Some(0))
            .flat_map(|u| u.to_le_bytes())
            .collect();
        ar[AR_SHORT_SRC..AR_SHORT_SRC + src.len()].copy_from_slice(&src);
        assert_eq!(
            decode_debug(&ar),
            Some(("[string \"BlackMarket.lua\"]".to_string(), 42))
        );
        // A C function reports `[C]` and line -1.
        let mut c = vec![0u8; LUA_DEBUG_SIZE];
        c[AR_LINEDEFINED..AR_LINEDEFINED + 4].copy_from_slice(&(-1i32).to_le_bytes());
        for (i, u) in "[C]".encode_utf16().enumerate() {
            c[AR_SHORT_SRC + 2 * i..AR_SHORT_SRC + 2 * i + 2].copy_from_slice(&u.to_le_bytes());
        }
        assert_eq!(decode_debug(&c), Some(("[C]".to_string(), -1)));
        // A record shorter than the layout is rejected, not misread.
        assert_eq!(decode_debug(&ar[..0x30]), None);
    }

    /// `short_src` fills all 60 characters when it has no NUL.
    #[test]
    fn an_unterminated_short_src_is_bounded() {
        let mut ar = vec![0u8; LUA_DEBUG_SIZE];
        for i in 0..LUA_IDSIZE {
            ar[AR_SHORT_SRC + 2 * i] = b'x';
        }
        let (src, _) = decode_debug(&ar).unwrap();
        assert_eq!(src.chars().count(), LUA_IDSIZE);
    }
}
