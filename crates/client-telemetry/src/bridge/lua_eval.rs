//! Main-thread Lua evaluation via the client's wide `doString`
//! primitive.
//!
//! The client's Lua API is **wide**: `Lua_doString_wide`
//! (`0x00404030`) is `luaL_loadbuffer(L, wbuf, len, wname)` +
//! `lua_pcall(L,0,0,0)` where `wbuf`/`wname` are **UTF-16LE** and
//! `len` is the **character count** (not bytes). A narrow ASCII
//! buffer is parsed as garbage bytecode → deterministic VM crash.
//! And the VM must only be touched on the **main thread** — see
//! `docs/reverse-engineering/findings/black-market-client-window-patch.md`.
//!
//! So this module's `eval_on_main_thread` is only ever called from
//! the bridge's dispatch drain inside the `FEngineLoop::Tick` hook.
//! The IO thread never calls it.
//!
//! The UTF-16 encoding + char-length math ([`encode_wide_chunk`]) is
//! pure and unit-tested off-target; the FFI call is Windows-i686 only.

use serde::Serialize;

/// Result of running a Lua chunk.
///
/// **Return-value + `print` capture landed in #686 scope 6** via
/// [`super::lua_capture`]: when the Lua 5.1 C API resolves by name,
/// `eval_on_main_thread` runs a capturing wrapper and fills `results`
/// (each user return value, `tostring`-ed) and `print_output`. If the
/// client static-links Lua without exporting the C API, capture degrades
/// to the wide fire-and-forget primitive and `error` carries the reason —
/// `ok`/`status` still reflect the raw `lua_pcall`. The struct shape is
/// final (set in #684) so the MCP surface doesn't churn.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct LuaEvalResult {
    /// `pcall` returned 0.
    pub ok: bool,
    /// Raw `lua_pcall` status code (0 = success; 1/2/... = err/syntax).
    pub status: i32,
    /// Stringified Lua error, when one can be recovered. Empty in
    /// phase 1 (needs `lua_tolstring` RE).
    pub error: String,
    /// Serialized return values. Empty in phase 1 (needs return-value
    /// capture RE).
    pub results: Vec<String>,
    /// Captured `print` output. Empty in phase 1 (needs a `print`
    /// hook / redirect).
    pub print_output: String,
}

impl LuaEvalResult {
    fn from_status(status: i32) -> Self {
        Self {
            ok: status == 0,
            status,
            error: String::new(),
            results: Vec::new(),
            print_output: String::new(),
        }
    }
}

/// Encode a Lua chunk to the wide form the client expects: a vector
/// of UTF-16 code units and the length **in characters** (code
/// units, not bytes). No trailing NUL — `luaL_loadbuffer` takes the
/// length explicitly.
pub fn encode_wide_chunk(chunk: &str) -> (Vec<u16>, u32) {
    let wide: Vec<u16> = chunk.encode_utf16().collect();
    let char_len = wide.len() as u32;
    (wide, char_len)
}

/// Encode a NUL-terminated wide chunk *name* for the loader (the
/// `wname` arg is a C-style wide string, unlike the counted buffer).
pub fn encode_wide_name(name: &str) -> Vec<u16> {
    let mut w: Vec<u16> = name.encode_utf16().collect();
    w.push(0);
    w
}

/// Chunk name shown in Lua error messages for bridge-run chunks.
pub const CHUNK_NAME: &str = "lab";

#[cfg(all(target_os = "windows", target_arch = "x86"))]
mod win {
    use super::{encode_wide_chunk, encode_wide_name, LuaEvalResult, CHUNK_NAME};

    /// `g_SGWUIManager_ptr` — singleton whose chain yields the UI
    /// `lua_State`: `L = *(*(*(0x01ee2a58) + 0x10))`.
    const G_SGW_UI_MANAGER_PTR: usize = 0x01ee2a58;
    /// `Lua_doString_wide` — `luaL_loadbuffer`(wide) + `lua_pcall`.
    const LUA_DOSTRING_WIDE: usize = 0x00404030;
    /// `lua_State.tt` tag for LUA_TTHREAD — validated as a **byte**
    /// (the finding's dword compare is a trap: tt is the low byte).
    const LUA_TTHREAD: u8 = 0x08;

    type LuaDoStringWide =
        unsafe extern "cdecl" fn(l: usize, wbuf: *const u16, len: u32, wname: *const u16) -> i32;

    /// Resolve the UI `lua_State`, chasing the singleton chain and
    /// validating each hop is non-null and that the final pointer
    /// carries the thread tag. Returns `None` if the VM isn't up yet
    /// (e.g. before the UI is constructed).
    ///
    /// # Safety
    /// Dereferences fixed client addresses; only valid inside
    /// SGW.exe on the main thread.
    unsafe fn resolve_lua_state() -> Option<usize> {
        let uimgr = *(G_SGW_UI_MANAGER_PTR as *const usize);
        if uimgr == 0 {
            return None;
        }
        let holder = *((uimgr + 0x10) as *const usize);
        if holder == 0 {
            return None;
        }
        let l = *(holder as *const usize);
        if l == 0 {
            return None;
        }
        // Byte compare — tt is the low byte of the dword at [L+4].
        let tt = *((l + 4) as *const u8);
        if tt != LUA_TTHREAD {
            return None;
        }
        Some(l)
    }

    /// Run `chunk` on the current (main) thread's UI Lua VM.
    ///
    /// Tries the full capture path first (#686 scope 6): resolve the Lua
    /// 5.1 C API by name and run a wrapper that returns the chunk's
    /// results and `print` output. If the C API can't be resolved (the
    /// client static-links Lua without exports), fall back to the wide
    /// fire-and-forget primitive and record *why* capture was skipped in
    /// `error` — the caller still gets a valid `pcall` status. Captured
    /// `print` output is teed to the local event ring so `events_read`
    /// sees it too.
    ///
    /// # Safety
    /// Must be called on the main thread only (never the network
    /// thread). Enforced by the sole call site being the
    /// `FEngineLoop::Tick` drain.
    pub unsafe fn eval_on_main_thread(chunk: &str) -> Result<LuaEvalResult, String> {
        let l = resolve_lua_state().ok_or_else(|| "UI lua_State not available".to_string())?;

        // Preferred path: full capture via the resolved C API.
        let resolver = crate::bridge::lua_capture::ModuleExportResolver;
        match crate::bridge::lua_capture::capture_eval_with_chunk(l, &resolver, Some(chunk)) {
            Ok(run) => {
                if !run.result.print_output.is_empty() {
                    crate::bridge::events::push(
                        "lua.print",
                        crate::bridge::crash::now_ms(),
                        serde_json::json!({ "output": run.result.print_output }),
                    );
                }
                Ok(run.result)
            }
            // C API not resolvable → degrade to the wide fire-and-forget
            // primitive, but tell the caller why capture was skipped.
            Err(reason) => {
                let (wbuf, char_len) = encode_wide_chunk(chunk);
                let wname = encode_wide_name(CHUNK_NAME);
                let f: LuaDoStringWide = core::mem::transmute(LUA_DOSTRING_WIDE);
                let status = f(l, wbuf.as_ptr(), char_len, wname.as_ptr());
                let mut result = LuaEvalResult::from_status(status);
                result.error = format!("ran fire-and-forget (no capture): {reason}");
                Ok(result)
            }
        }
    }
}

#[cfg(all(target_os = "windows", target_arch = "x86"))]
pub use win::eval_on_main_thread;

/// Off-target stub. The real VM only exists inside SGW.exe.
///
/// # Safety
/// Signature mirrors the Windows one; the stub touches nothing.
#[cfg(not(all(target_os = "windows", target_arch = "x86")))]
pub unsafe fn eval_on_main_thread(_chunk: &str) -> Result<LuaEvalResult, String> {
    Err("lua_eval is only available in the injected DLL (windows i686)".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The canonical black-market chunk is 25 characters, and the
    /// wide encoding is exactly 25 code units — pinning the
    /// char-count-not-byte-count contract.
    #[test]
    fn black_market_chunk_is_25_chars() {
        let chunk = "BlackMarketMod.onBMOpen()";
        let (wide, char_len) = encode_wide_chunk(chunk);
        assert_eq!(char_len, 25);
        assert_eq!(wide.len(), 25);
        // ASCII → each char is one UTF-16 code unit.
        assert_eq!(char_len as usize, chunk.chars().count());
    }

    /// Length is code units, not bytes. An ASCII chunk has char_len
    /// == byte_len, but the wide buffer is twice the bytes.
    #[test]
    fn char_len_is_code_units_not_bytes() {
        let (wide, char_len) = encode_wide_chunk("print(1)");
        assert_eq!(char_len, 8);
        assert_eq!(wide.len() * 2, 16); // 16 bytes on the wire
    }

    /// A non-BMP char (astral plane) is two UTF-16 code units, so
    /// char_len counts surrogate pairs as 2 — matching what
    /// luaL_loadbuffer's wide path consumes.
    #[test]
    fn astral_char_counts_as_two_code_units() {
        // U+1F600 encodes as a surrogate pair.
        let (wide, char_len) = encode_wide_chunk("\u{1F600}");
        assert_eq!(char_len, 2);
        assert_eq!(wide.len(), 2);
    }

    #[test]
    fn name_is_nul_terminated() {
        let w = encode_wide_name("lab");
        assert_eq!(w, vec![b'l' as u16, b'a' as u16, b'b' as u16, 0]);
    }

    #[test]
    fn result_from_status() {
        assert!(LuaEvalResult::from_status(0).ok);
        assert!(!LuaEvalResult::from_status(2).ok);
        assert_eq!(LuaEvalResult::from_status(2).status, 2);
    }
}
