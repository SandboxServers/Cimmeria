//! The client's `lua51.dll`: its exports and a [`LuaStack`] over them.
//!
//! `lua51.dll` (150,560 bytes, 2009-06-30) is a **wide-character** Lua 5.1
//! built as C++: all 119 exports carry C++-mangled names, strings are
//! `wchar_t*` (UTF-16), lengths count characters, and `lua_Number` is
//! `double`. The names below were read from its export table. All are
//! `__cdecl` (`YA` in the mangling).
//!
//! Being C++, it raises Lua errors as C++ exceptions: it imports
//! `_CxxThrowException` and `__CxxFrameHandler3`, carries RTTI for
//! `lua_longjmp*`, and imports no `longjmp`. So the exports are declared
//! `C-unwind`: an allocation error inside [`FfiLua::protected`] unwinds
//! through the Rust frames between `lua_cpcall` and the failing call,
//! running their destructors, and `lua_cpcall` catches it.

use core::cell::Cell;
use core::ffi::c_void;

use windows_sys::Win32::System::LibraryLoader::{GetModuleHandleW, GetProcAddress};

use super::lua_stack::LuaStack;

type State = *mut c_void;

type GetTop = unsafe extern "C-unwind" fn(State) -> i32;
type SetTop = unsafe extern "C-unwind" fn(State, i32);
type CheckStack = unsafe extern "C-unwind" fn(State, i32) -> i32;
type Type = unsafe extern "C-unwind" fn(State, i32) -> i32;
type PushInteger = unsafe extern "C-unwind" fn(State, i32);
type PushString = unsafe extern "C-unwind" fn(State, *const u16);
type CreateTable = unsafe extern "C-unwind" fn(State, i32, i32);
type SetField = unsafe extern "C-unwind" fn(State, i32, *const u16);
type RawGet = unsafe extern "C-unwind" fn(State, i32);
type RawSetI = unsafe extern "C-unwind" fn(State, i32, i32);
type PCall = unsafe extern "C-unwind" fn(State, i32, i32, i32) -> i32;
type ToLString = unsafe extern "C-unwind" fn(State, i32, *mut u32) -> *const u16;
/// The C function `lua_cpcall` runs.
type CFunction = unsafe extern "C-unwind" fn(State) -> i32;
type CPCall = unsafe extern "C-unwind" fn(State, CFunction, *mut c_void) -> i32;

/// The exports delivery calls, resolved once at start-up.
pub(crate) struct LuaApi {
    gettop: GetTop,
    settop: SetTop,
    checkstack: CheckStack,
    type_: Type,
    pushinteger: PushInteger,
    pushstring: PushString,
    createtable: CreateTable,
    setfield: SetField,
    rawget: RawGet,
    rawseti: RawSetI,
    pcall: PCall,
    tolstring: ToLString,
    cpcall: CPCall,
}

/// The DLL, as `GetModuleHandleW` wants it.
const MODULE: &str = "lua51.dll";

/// `(C API name, mangled export name)`, in [`LuaApi`] field order.
pub(crate) const EXPORTS: [(&str, &str); 13] = [
    ("lua_gettop", "?lua_gettop@@YAHPAUlua_State@@@Z"),
    ("lua_settop", "?lua_settop@@YAXPAUlua_State@@H@Z"),
    ("lua_checkstack", "?lua_checkstack@@YAHPAUlua_State@@H@Z"),
    ("lua_type", "?lua_type@@YAHPAUlua_State@@H@Z"),
    ("lua_pushinteger", "?lua_pushinteger@@YAXPAUlua_State@@H@Z"),
    ("lua_pushstring", "?lua_pushstring@@YAXPAUlua_State@@PB_W@Z"),
    ("lua_createtable", "?lua_createtable@@YAXPAUlua_State@@HH@Z"),
    ("lua_setfield", "?lua_setfield@@YAXPAUlua_State@@HPB_W@Z"),
    ("lua_rawget", "?lua_rawget@@YAXPAUlua_State@@H@Z"),
    ("lua_rawseti", "?lua_rawseti@@YAXPAUlua_State@@HH@Z"),
    ("lua_pcall", "?lua_pcall@@YAHPAUlua_State@@HHH@Z"),
    (
        "lua_tolstring",
        "?lua_tolstring@@YAPB_WPAUlua_State@@HPAI@Z",
    ),
    ("lua_cpcall", "?lua_cpcall@@YAHPAUlua_State@@P6AH0@ZPAX@Z"),
];

/// Why the API could not be resolved.
#[derive(Debug)]
pub(crate) enum ResolveError {
    /// `lua51.dll` is not loaded (yet).
    NotLoaded,
    /// These exports are missing: a different `lua51.dll`.
    Missing(Vec<&'static str>),
}

impl LuaApi {
    /// Look every export up in the loaded `lua51.dll`.
    pub(crate) fn resolve() -> Result<Self, ResolveError> {
        let module_name: Vec<u16> = MODULE.encode_utf16().chain(Some(0)).collect();
        // SAFETY: a NUL-terminated wide string; no reference is taken.
        let module = unsafe { GetModuleHandleW(module_name.as_ptr()) };
        if module.is_null() {
            return Err(ResolveError::NotLoaded);
        }
        let mut found = [0usize; EXPORTS.len()];
        let mut missing = Vec::new();
        for (slot, (api_name, export)) in found.iter_mut().zip(EXPORTS) {
            let symbol: Vec<u8> = export.bytes().chain(Some(0)).collect();
            // SAFETY: a loaded module handle and a NUL-terminated name.
            match unsafe { GetProcAddress(module, symbol.as_ptr()) } {
                Some(f) => *slot = f as usize,
                None => missing.push(api_name),
            }
        }
        if !missing.is_empty() {
            return Err(ResolveError::Missing(missing));
        }
        // SAFETY: each address is the export whose mangled name encodes
        // exactly the signature it is transmuted to.
        unsafe {
            use core::mem::transmute as t;
            Ok(Self {
                gettop: t::<usize, GetTop>(found[0]),
                settop: t::<usize, SetTop>(found[1]),
                checkstack: t::<usize, CheckStack>(found[2]),
                type_: t::<usize, Type>(found[3]),
                pushinteger: t::<usize, PushInteger>(found[4]),
                pushstring: t::<usize, PushString>(found[5]),
                createtable: t::<usize, CreateTable>(found[6]),
                setfield: t::<usize, SetField>(found[7]),
                rawget: t::<usize, RawGet>(found[8]),
                rawseti: t::<usize, RawSetI>(found[9]),
                pcall: t::<usize, PCall>(found[10]),
                tolstring: t::<usize, ToLString>(found[11]),
                cpcall: t::<usize, CPCall>(found[12]),
            })
        }
    }
}

/// Longest string [`FfiLua::string_at`] copies out, in characters.
const MAX_MESSAGE_CHARS: usize = 1024;

/// UTF-16 with a terminating NUL. The string stops at an embedded NUL,
/// which a C string cannot carry.
fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16()
        .take_while(|&c| c != 0)
        .chain(Some(0))
        .collect()
}

/// Aborts the process if dropped while a Rust panic unwinds. It stops a Rust
/// panic from unwinding into `lua_cpcall`, whose C++ `catch (...)` would
/// swallow it. A Lua error unwinding through it is a foreign exception, not
/// a panic, so it passes.
struct AbortOnPanic;

impl Drop for AbortOnPanic {
    fn drop(&mut self) {
        if std::thread::panicking() {
            std::process::abort();
        }
    }
}

thread_local! {
    /// The body [`FfiLua::protected`] is running, for [`protected_entry`].
    /// A thread-local rather than `lua_cpcall`'s userdata argument, so the
    /// entry needs no `lua_touserdata`.
    static PROTECTED_BODY: Cell<*mut c_void> = const { Cell::new(core::ptr::null_mut()) };
}

/// The C function `lua_cpcall` calls: runs the body `protected` stored.
unsafe extern "C-unwind" fn protected_entry(_state: State) -> i32 {
    let _guard = AbortOnPanic;
    let body = PROTECTED_BODY.with(|slot| slot.replace(core::ptr::null_mut()));
    if !body.is_null() {
        // SAFETY: `protected` stored a pointer to its `&mut dyn FnMut()`,
        // which outlives this call, and took nothing else from it.
        unsafe { (*body.cast::<&mut dyn FnMut()>())() };
    }
    0
}

/// The UI `lua_State` driven through [`LuaApi`]. Main thread only.
pub(crate) struct FfiLua<'a> {
    pub(crate) api: &'a LuaApi,
    pub(crate) state: State,
}

// SAFETY for every method below: `state` is the live UI lua_State (checked
// by `ui_lua_state` this frame), used on the main thread, and every string
// passed is a NUL-terminated UTF-16 buffer that outlives the call.
impl LuaStack for FfiLua<'_> {
    fn top(&mut self) -> i32 {
        unsafe { (self.api.gettop)(self.state) }
    }

    fn set_top(&mut self, index: i32) {
        unsafe { (self.api.settop)(self.state, index) }
    }

    fn check_stack(&mut self, extra: i32) -> bool {
        unsafe { (self.api.checkstack)(self.state, extra) != 0 }
    }

    fn type_at(&mut self, index: i32) -> i32 {
        unsafe { (self.api.type_)(self.state, index) }
    }

    fn push_integer(&mut self, value: i32) {
        unsafe { (self.api.pushinteger)(self.state, value) }
    }

    fn push_string(&mut self, value: &str) {
        let w = wide(value);
        unsafe { (self.api.pushstring)(self.state, w.as_ptr()) }
    }

    fn create_table(&mut self, array: i32, record: i32) {
        unsafe { (self.api.createtable)(self.state, array, record) }
    }

    fn set_field(&mut self, table: i32, key: &str) {
        let w = wide(key);
        unsafe { (self.api.setfield)(self.state, table, w.as_ptr()) }
    }

    fn raw_set_index(&mut self, table: i32, n: i32) {
        unsafe { (self.api.rawseti)(self.state, table, n) }
    }

    fn raw_get(&mut self, table: i32) {
        unsafe { (self.api.rawget)(self.state, table) }
    }

    fn pcall(&mut self, args: i32, results: i32) -> i32 {
        unsafe { (self.api.pcall)(self.state, args, results, 0) }
    }

    fn protected(&mut self, body: &mut dyn FnMut(&mut Self)) -> i32 {
        let (cpcall, state) = (self.api.cpcall, self.state);
        let mut run = || body(self);
        let mut run: &mut dyn FnMut() = &mut run;
        let slot = (&mut run as *mut &mut dyn FnMut()).cast::<c_void>();
        let outer = PROTECTED_BODY.with(|s| s.replace(slot));
        // `lua_cpcall` calls `protected_entry` on this thread before it
        // returns, and catches any Lua error raised inside.
        let status = unsafe { cpcall(state, protected_entry, core::ptr::null_mut()) };
        PROTECTED_BODY.with(|s| s.set(outer));
        status
    }

    fn string_at(&mut self, index: i32) -> Option<String> {
        let mut len: u32 = 0;
        let p = unsafe { (self.api.tolstring)(self.state, index, &mut len) };
        if p.is_null() {
            return None;
        }
        // Only ever used for an error message, which the log shortens anyway.
        let len = (len as usize).min(MAX_MESSAGE_CHARS);
        // SAFETY: lua_tolstring returns at least `len` characters owned by
        // the value at `index`, which stays on the stack during this copy.
        let chars = unsafe { core::slice::from_raw_parts(p, len) };
        Some(String::from_utf16_lossy(chars))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wide_strings_are_nul_terminated_and_stop_at_nul() {
        assert_eq!(wide("ab"), vec![b'a' as u16, b'b' as u16, 0]);
        assert_eq!(wide("a\0b"), vec![b'a' as u16, 0]);
        assert_eq!(wide("Tök"), vec![b'T' as u16, 0xF6, b'k' as u16, 0]);
    }

    /// Each mangled name is the C API name with a C++ `__cdecl` signature.
    #[test]
    fn export_names_mangle_their_api_names() {
        for (api, mangled) in EXPORTS {
            assert!(
                mangled.starts_with(&format!("?{api}@@YA")),
                "{mangled} is not {api}"
            );
        }
    }

    /// The test binary does not load `lua51.dll`, so resolution reports it
    /// missing rather than failing any other way.
    #[test]
    fn resolve_without_lua51_is_not_loaded() {
        assert!(matches!(LuaApi::resolve(), Err(ResolveError::NotLoaded)));
    }
}
