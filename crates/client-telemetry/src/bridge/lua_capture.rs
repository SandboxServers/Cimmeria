//! Lua return-value + `print` capture for `client_lua_eval` (issue #686
//! scope 6, folded in — #684 shipped `lua_eval` at `pcall`-status only).
//!
//! # Why this is a separate module from `lua_eval`
//!
//! [`super::lua_eval`] owns the client's **wide** fire-and-forget
//! primitive (`Lua_doString_wide` = `luaL_loadbuffer`(UTF-16) +
//! `lua_pcall(L,0,0,0)`), which discards results and swallows `print`.
//! Capturing return values and `print` output needs a *different* call
//! shape: our own `luaL_loadbuffer` (narrow ASCII) + `lua_pcall(L, 0,
//! LUA_MULTRET, 0)`, then reading the stack with `lua_gettop` /
//! `lua_tolstring`. That needs the standard Lua 5.1 C API, which the
//! wide primitive does not expose.
//!
//! # Resolve by name, verify the export, degrade with a reason
//!
//! Per the #686 folded-in scope, we **resolve the Lua C API by name and
//! verify each export before calling it**. If the client static-links
//! Lua without exporting the C API (a real possibility for a 2009 UE3
//! title), resolution fails and [`capture_eval`] returns an `Err` naming
//! the first missing symbol — the caller ([`super::lua_eval`]) then
//! degrades to the wide fire-and-forget primitive and reports the reason
//! in [`super::lua_eval::LuaEvalResult::error`] rather than pretending it
//! captured anything. That degrade-with-reason path is the unit-tested
//! contract here (against a stub resolver), because the live path can
//! only be confirmed inside SGW.exe.
//!
//! # The capture wrapper
//!
//! We don't decode Lua types on the C side (booleans have no
//! `lua_tolstring` in 5.1, tables need traversal). Instead a small Lua
//! wrapper does the work in-VM: it installs a `print` that appends to a
//! buffer, `pcall`s the user chunk, restores `print`, then returns a
//! fixed-shape list — `[ok(1/0), error, prints, user_results...]` — with
//! every value already run through `tostring`. The C side then only ever
//! calls `lua_tolstring` on strings, which is total. See
//! [`build_capture_wrapper`].

use super::lua_eval::LuaEvalResult;

/// The standard Lua 5.1 C API symbols the capture path needs. Resolved
/// by name; a missing one degrades the whole call with a specific reason.
pub const REQUIRED_SYMBOLS: [&str; 5] = [
    "luaL_loadbuffer",
    "lua_pcall",
    "lua_gettop",
    "lua_settop",
    "lua_tolstring",
];

/// `LUA_MULTRET` — ask `lua_pcall` to leave every return value on the
/// stack so `lua_gettop` can count them.
pub const LUA_MULTRET: i32 = -1;

/// Chunk name shown in Lua error messages for a capture run.
pub const CAPTURE_CHUNK_NAME: &str = "lab_capture";

/// Abstraction over "find a native symbol by name", so the resolve +
/// degrade-with-reason logic is unit-testable off-target with a stub. The
/// real implementation ([`ModuleExportResolver`]) walks the loaded
/// modules with `GetProcAddress`; tests hand in a `HashMap`.
pub trait SymbolResolver {
    /// Runtime address of `name`, or `None` if no loaded module exports
    /// it.
    fn resolve(&self, name: &str) -> Option<usize>;
}

/// Resolved Lua 5.1 C API entry points. All `cdecl`.
#[derive(Debug, Clone, Copy)]
pub struct LuaCApi {
    /// `int luaL_loadbuffer(lua_State*, const char* buff, size_t sz, const char* name)`
    pub loadbuffer: usize,
    /// `int lua_pcall(lua_State*, int nargs, int nresults, int errfunc)`
    pub pcall: usize,
    /// `int lua_gettop(lua_State*)`
    pub gettop: usize,
    /// `void lua_settop(lua_State*, int idx)`
    pub settop: usize,
    /// `const char* lua_tolstring(lua_State*, int idx, size_t* len)`
    pub tolstring: usize,
}

impl LuaCApi {
    /// Resolve every required symbol. `Err` names the **first** missing
    /// one so the caller can degrade with a specific reason (this is the
    /// "verify the export before calling" contract).
    pub fn resolve(resolver: &dyn SymbolResolver) -> Result<Self, String> {
        let get = |name: &str| -> Result<usize, String> {
            resolver.resolve(name).ok_or_else(|| {
                format!(
                    "Lua C API symbol '{name}' not exported by any loaded module \
                     (client likely static-links Lua); capture unavailable"
                )
            })
        };
        Ok(Self {
            loadbuffer: get("luaL_loadbuffer")?,
            pcall: get("lua_pcall")?,
            gettop: get("lua_gettop")?,
            settop: get("lua_settop")?,
            tolstring: get("lua_tolstring")?,
        })
    }
}

/// Build the ASCII capture wrapper around a user `chunk`.
///
/// After `lua_pcall(L, 0, LUA_MULTRET, 0)` the stack holds, in order:
///
/// 1. `ok` — `"1"` if the user chunk ran without error, else `"0"`;
/// 2. `error` — the Lua error message when `ok == 0`, else `""`;
/// 3. `prints` — everything the chunk passed to `print`, newline-joined;
/// 4. `user_results...` — the chunk's own return values, each `tostring`-ed.
///
/// Every value is `tostring`-ed **inside** the VM so the C side only ever
/// `lua_tolstring`s strings. `(table.unpack or unpack)` handles both Lua
/// 5.1 (`unpack`) and 5.2+ (`table.unpack`). Pure — pinned by tests.
pub fn build_capture_wrapper(chunk: &str) -> String {
    // The user chunk is spliced verbatim into a function body, so a
    // `return X` chunk (what the autologin screen-reads use) returns X
    // from the pcall'd function and lands in `user_results`.
    format!(
        "local __lab_out = {{}}\n\
         local __lab_oldprint = print\n\
         print = function(...)\n\
         local __n = select('#', ...)\n\
         local __p = {{}}\n\
         for __i = 1, __n do __p[__i] = tostring((select(__i, ...))) end\n\
         __lab_out[#__lab_out + 1] = table.concat(__p, '\\t')\n\
         end\n\
         local __lab_res = {{ pcall(function()\n{chunk}\nend) }}\n\
         print = __lab_oldprint\n\
         local __lab_ok = __lab_res[1]\n\
         local __lab_ret = {{ __lab_ok and 1 or 0, __lab_ok and '' or tostring(__lab_res[2]), table.concat(__lab_out, '\\n') }}\n\
         if __lab_ok then for __i = 2, #__lab_res do __lab_ret[#__lab_ret + 1] = tostring(__lab_res[__i]) end end\n\
         return (table.unpack or unpack)(__lab_ret)\n"
    )
}

/// Decode the fixed-shape return slots the wrapper leaves on the stack
/// (already `lua_tolstring`-ed) into a [`LuaEvalResult`]. Pure and
/// unit-tested; the native path just gathers the raw strings and calls
/// this.
///
/// `slots` is `[ok, error, prints, user_results...]`. A short slot list
/// (should not happen with our wrapper) degrades to a not-ok result
/// rather than panicking.
pub fn decode_return_slots(slots: &[String]) -> LuaEvalResult {
    let ok = slots.first().map(|s| s == "1").unwrap_or(false);
    let error = slots.get(1).cloned().unwrap_or_default();
    let print_output = slots.get(2).cloned().unwrap_or_default();
    let results = slots.get(3..).map(<[String]>::to_vec).unwrap_or_default();
    LuaEvalResult {
        ok,
        // We keep the *user-level* status here: 0 on success, 1 on a Lua
        // error surfaced through the wrapper's inner pcall. The raw
        // C-level pcall status (which should always be 0 for a well-formed
        // wrapper) is handled separately by the native path.
        status: i32::from(!ok),
        error,
        results,
        print_output,
    }
}

// ── Native capture path (windows i686 only) ─────────────────────────────

#[cfg(all(target_os = "windows", target_arch = "x86"))]
mod win {
    use super::{
        build_capture_wrapper, decode_return_slots, LuaCApi, SymbolResolver, CAPTURE_CHUNK_NAME,
        LUA_MULTRET,
    };
    use crate::bridge::lua_eval::LuaEvalResult;
    use core::ffi::c_char;
    use std::ffi::CString;

    use windows_sys::Win32::System::LibraryLoader::{GetModuleHandleW, GetProcAddress};

    // Typed cdecl signatures for the resolved Lua C API.
    type FnLoadBuffer = unsafe extern "cdecl" fn(
        l: usize,
        buf: *const c_char,
        sz: usize,
        name: *const c_char,
    ) -> i32;
    type FnPcall =
        unsafe extern "cdecl" fn(l: usize, nargs: i32, nresults: i32, errfunc: i32) -> i32;
    type FnGetTop = unsafe extern "cdecl" fn(l: usize) -> i32;
    type FnSetTop = unsafe extern "cdecl" fn(l: usize, idx: i32);
    type FnToLString =
        unsafe extern "cdecl" fn(l: usize, idx: i32, len: *mut usize) -> *const c_char;

    /// Live resolver: `GetProcAddress` across the modules that might carry
    /// the Lua C API. The client could static-link into `SGW.exe` (module
    /// handle NULL) or ship a `lua51.dll`; we try both.
    pub struct ModuleExportResolver;

    impl SymbolResolver for ModuleExportResolver {
        fn resolve(&self, name: &str) -> Option<usize> {
            let cname = CString::new(name).ok()?;
            // NULL handle = the main executable's export table.
            const CANDIDATES: [Option<&str>; 3] = [None, Some("lua51.dll"), Some("lua5.1.dll")];
            for cand in CANDIDATES {
                // SAFETY: GetModuleHandleW takes an optional wide name; a
                // module that isn't loaded returns null and is skipped.
                let module = unsafe {
                    match cand {
                        None => GetModuleHandleW(core::ptr::null()),
                        Some(dll) => {
                            let wide: Vec<u16> =
                                dll.encode_utf16().chain(core::iter::once(0)).collect();
                            GetModuleHandleW(wide.as_ptr())
                        }
                    }
                };
                if module.is_null() {
                    continue;
                }
                // SAFETY: `module` is a live handle; `cname` is NUL-terminated.
                let proc = unsafe { GetProcAddress(module, cname.as_ptr() as *const u8) };
                if let Some(p) = proc {
                    return Some(p as usize);
                }
            }
            None
        }
    }

    /// Read one stack slot as a lossy-UTF-8 `String` via `lua_tolstring`.
    ///
    /// # Safety
    /// `api.tolstring` must be the real `lua_tolstring` and `l` a valid
    /// `lua_State` on the main thread; `idx` must be a valid stack index.
    unsafe fn slot_to_string(api: &LuaCApi, l: usize, idx: i32) -> String {
        let tolstring: FnToLString = core::mem::transmute(api.tolstring);
        let mut len: usize = 0;
        let ptr = tolstring(l, idx, &mut len);
        if ptr.is_null() || len == 0 {
            return String::new();
        }
        let bytes = core::slice::from_raw_parts(ptr as *const u8, len);
        String::from_utf8_lossy(bytes).into_owned()
    }

    /// Run `chunk` with full capture on the resolved UI `lua_State`.
    ///
    /// Loads our ASCII capture wrapper with the narrow `luaL_loadbuffer`,
    /// `pcall`s it with `LUA_MULTRET`, reads the return slots with
    /// `lua_gettop` / `lua_tolstring`, then restores the stack top. The
    /// stack is always returned to its entry depth, even on a load or
    /// call error, so a capture never leaks stack slots into the VM.
    ///
    /// `Err` means the Lua C API could not be resolved (degrade to the
    /// wide fire-and-forget primitive); `Ok` carries the captured result.
    ///
    /// # Safety
    /// Main thread only (the dispatch drain). `l` must be the live UI
    /// `lua_State`.
    pub unsafe fn capture_eval(
        l: usize,
        resolver: &dyn SymbolResolver,
    ) -> Result<CaptureRun, String> {
        capture_eval_with_chunk(l, resolver, None)
    }

    /// A completed capture run, carrying the decoded result plus the raw
    /// print output so the caller can also tee it to the event ring.
    pub struct CaptureRun {
        pub result: LuaEvalResult,
    }

    /// Inner form used by both the public entry and the wrapper so the
    /// wrapper text is built once.
    ///
    /// # Safety
    /// See [`capture_eval`].
    pub unsafe fn capture_eval_with_chunk(
        l: usize,
        resolver: &dyn SymbolResolver,
        chunk: Option<&str>,
    ) -> Result<CaptureRun, String> {
        let api = LuaCApi::resolve(resolver)?;

        let gettop: FnGetTop = core::mem::transmute(api.gettop);
        let settop: FnSetTop = core::mem::transmute(api.settop);
        let loadbuffer: FnLoadBuffer = core::mem::transmute(api.loadbuffer);
        let pcall: FnPcall = core::mem::transmute(api.pcall);

        let base = gettop(l);

        let wrapper = build_capture_wrapper(chunk.unwrap_or(""));
        let name = CString::new(CAPTURE_CHUNK_NAME).map_err(|_| "bad chunk name".to_string())?;

        let load_status = loadbuffer(
            l,
            wrapper.as_ptr() as *const c_char,
            wrapper.len(),
            name.as_ptr(),
        );
        if load_status != 0 {
            let err = slot_to_string(&api, l, -1);
            settop(l, base);
            return Ok(CaptureRun {
                result: LuaEvalResult {
                    ok: false,
                    status: load_status,
                    error: if err.is_empty() {
                        "luaL_loadbuffer failed".to_string()
                    } else {
                        err
                    },
                    results: Vec::new(),
                    print_output: String::new(),
                },
            });
        }

        let call_status = pcall(l, 0, LUA_MULTRET, 0);
        if call_status != 0 {
            // A wrapper-level failure (should be rare — the wrapper itself
            // pcalls the user chunk). Surface it and clean up.
            let err = slot_to_string(&api, l, -1);
            settop(l, base);
            return Ok(CaptureRun {
                result: LuaEvalResult {
                    ok: false,
                    status: call_status,
                    error: if err.is_empty() {
                        "lua_pcall failed".to_string()
                    } else {
                        err
                    },
                    results: Vec::new(),
                    print_output: String::new(),
                },
            });
        }

        let top = gettop(l);
        let mut slots = Vec::new();
        let mut idx = base + 1;
        while idx <= top {
            slots.push(slot_to_string(&api, l, idx));
            idx += 1;
        }
        settop(l, base);

        Ok(CaptureRun {
            result: decode_return_slots(&slots),
        })
    }
}

#[cfg(all(target_os = "windows", target_arch = "x86"))]
pub use win::{capture_eval, capture_eval_with_chunk, CaptureRun, ModuleExportResolver};

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    /// A stub resolver backed by a name→addr map, for the degrade-with-
    /// reason tests.
    struct StubResolver(HashMap<&'static str, usize>);

    impl SymbolResolver for StubResolver {
        fn resolve(&self, name: &str) -> Option<usize> {
            self.0.get(name).copied()
        }
    }

    fn full_map() -> HashMap<&'static str, usize> {
        REQUIRED_SYMBOLS
            .iter()
            .enumerate()
            .map(|(i, &s)| (s, 0x1000 + i))
            .collect()
    }

    #[test]
    fn resolve_succeeds_when_all_symbols_present() {
        let r = StubResolver(full_map());
        let api = LuaCApi::resolve(&r).expect("all present");
        assert_eq!(api.loadbuffer, 0x1000);
        assert_eq!(api.tolstring, 0x1004);
    }

    /// The core degrade-with-reason contract: a missing symbol fails
    /// resolution and the error **names the missing symbol** so the caller
    /// can report why capture was unavailable.
    #[test]
    fn resolve_degrades_with_reason_naming_missing_symbol() {
        let mut map = full_map();
        map.remove("lua_tolstring");
        let r = StubResolver(map);
        let err = LuaCApi::resolve(&r).expect_err("must fail");
        assert!(
            err.contains("lua_tolstring"),
            "reason must name the missing symbol, got: {err}"
        );
        assert!(err.contains("capture unavailable"));
    }

    #[test]
    fn resolve_reports_first_missing_symbol() {
        // Empty resolver: the first required symbol is reported.
        let r = StubResolver(HashMap::new());
        let err = LuaCApi::resolve(&r).unwrap_err();
        assert!(err.contains(REQUIRED_SYMBOLS[0]), "got: {err}");
    }

    #[test]
    fn wrapper_installs_print_capture_and_returns_multret() {
        let w = build_capture_wrapper("return isVisible()");
        // Splices the user chunk verbatim.
        assert!(w.contains("return isVisible()"));
        // Overrides print and restores it.
        assert!(w.contains("print = function(...)"));
        assert!(w.contains("print = __lab_oldprint"));
        // pcalls the user chunk (so a user error is captured, not fatal).
        assert!(w.contains("pcall(function()"));
        // Return list is [ok, error, prints, user...].
        assert!(w.contains("__lab_ok and 1 or 0"));
        assert!(w.contains("table.concat(__lab_out"));
        // 5.1/5.2 unpack shim.
        assert!(w.contains("(table.unpack or unpack)"));
    }

    #[test]
    fn decode_success_with_results_and_prints() {
        let slots = vec![
            "1".to_string(),
            String::new(),
            "hello\nworld".to_string(),
            "42".to_string(),
            "true".to_string(),
        ];
        let r = decode_return_slots(&slots);
        assert!(r.ok);
        assert_eq!(r.status, 0);
        assert_eq!(r.error, "");
        assert_eq!(r.print_output, "hello\nworld");
        assert_eq!(r.results, vec!["42".to_string(), "true".to_string()]);
    }

    #[test]
    fn decode_lua_error_surfaces_message_no_results() {
        let slots = vec![
            "0".to_string(),
            "attempt to call a nil value".to_string(),
            String::new(),
        ];
        let r = decode_return_slots(&slots);
        assert!(!r.ok);
        assert_eq!(r.status, 1);
        assert_eq!(r.error, "attempt to call a nil value");
        assert!(r.results.is_empty());
    }

    #[test]
    fn decode_degrades_on_short_slot_list() {
        // Should never happen with our wrapper, but must not panic.
        let r = decode_return_slots(&[]);
        assert!(!r.ok);
        assert!(r.results.is_empty());
        assert_eq!(r.print_output, "");
    }
}
