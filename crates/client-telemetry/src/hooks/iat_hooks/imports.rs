//! The IAT slots this module swaps, and the check that each still holds
//! the address its import resolves to.

// ─── IAT slot addresses (2026-06-04 manifest) ───────────────────

#[cfg(all(target_os = "windows", target_arch = "x86"))]
pub(super) const IAT_LUA_PCALL: Import = Import {
    slot: 0x017F_0228,
    module: "lua51.dll",
    symbol: c"?lua_pcall@@YAHPAUlua_State@@HHH@Z",
};
#[cfg(all(target_os = "windows", target_arch = "x86"))]
pub(super) const IAT_LUA_CALL: Import = Import {
    slot: 0x017F_0244,
    module: "lua51.dll",
    symbol: c"?lua_call@@YAXPAUlua_State@@HH@Z",
};
#[cfg(all(target_os = "windows", target_arch = "x86"))]
pub(super) const IAT_LUA_NEWSTATE: Import = Import {
    slot: 0x017F_0288,
    module: "lua51.dll",
    symbol: c"?lua_newstate@@YAPAUlua_State@@P6APAXPAX0II@Z0@Z",
};

#[cfg(all(target_os = "windows", target_arch = "x86"))]
pub(super) const IAT_CREATE_THREAD: Import = Import {
    slot: 0x017E_F290,
    module: "kernel32.dll",
    symbol: c"CreateThread",
};
#[cfg(all(target_os = "windows", target_arch = "x86"))]
pub(super) const IAT_LOAD_LIBRARY_W: Import = Import {
    slot: 0x017E_F26C,
    module: "kernel32.dll",
    symbol: c"LoadLibraryW",
};
#[cfg(all(target_os = "windows", target_arch = "x86"))]
pub(super) const IAT_LOAD_LIBRARY_A: Import = Import {
    slot: 0x017E_F268,
    module: "kernel32.dll",
    symbol: c"LoadLibraryA",
};
#[cfg(all(target_os = "windows", target_arch = "x86"))]
pub(super) const IAT_GET_FOREGROUND_WINDOW: Import = Import {
    slot: 0x017E_FDF8,
    module: "user32.dll",
    symbol: c"GetForegroundWindow",
};

/// `recvfrom` (WS2_32 ordinal 17), the socket boundary before Mercury's
/// packet object and filter. Ghidra's import thunk at `0x012f3d8c` jumps
/// through this slot; its callers include the Mercury receive loop.
#[cfg(all(target_os = "windows", target_arch = "x86"))]
pub(super) const IAT_RECVFROM: Import = Import {
    slot: 0x017E_FF60,
    module: "ws2_32.dll",
    symbol: c"recvfrom",
};

/// One imported function: its IAT slot in `SGW.exe` and what it imports.
#[cfg(all(target_os = "windows", target_arch = "x86"))]
#[derive(Debug, Clone, Copy)]
pub(super) struct Import {
    pub(super) slot: usize,
    pub(super) module: &'static str,
    pub(super) symbol: &'static core::ffi::CStr,
}

#[cfg(all(target_os = "windows", target_arch = "x86"))]
impl Import {
    /// The address the loader bound this import to, from the module's
    /// export table. `None` when the module is not loaded or lacks it.
    pub(super) fn resolved(&self) -> Option<usize> {
        use windows_sys::Win32::System::LibraryLoader::{GetModuleHandleW, GetProcAddress};
        let wide: Vec<u16> = self.module.encode_utf16().chain(Some(0)).collect();
        // SAFETY: NUL-terminated strings that outlive the calls; no
        // reference to the module is kept.
        unsafe {
            let module = GetModuleHandleW(wide.as_ptr());
            if module.is_null() {
                return None;
            }
            GetProcAddress(module, self.symbol.as_ptr().cast()).map(|f| f as usize)
        }
    }

    /// What the slot holds now.
    pub(super) fn current(&self) -> Option<usize> {
        let bytes = cimmeria_client_hookgate::os::read_bytes(self.slot, 4)?;
        Some(u32::from_le_bytes(bytes.try_into().ok()?) as usize)
    }
}
