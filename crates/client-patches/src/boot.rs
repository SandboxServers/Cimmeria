//! `DllMain` and the bootstrap thread.
//!
//! The loader lock is held for the whole of `DllMain`, so it only starts a
//! thread and returns, as the telemetry DLL does. The bootstrap thread,
//! once the lock is released:
//!
//! 1. opens the log next to `SGW.exe`;
//! 2. resolves the `lua51.dll` exports, waiting for the DLL if it is not
//!    loaded yet (it is a static import of `SGW.exe`, so it normally is);
//! 3. runs the [fingerprint gate](crate::fingerprint) over every site,
//!    accepting an earlier hook only from a loaded telemetry DLL;
//! 4. hooks `FEngineLoop::Tick` (deliver), then the dispatcher and the drop
//!    callee (receive), stopping at the first failure;
//! 5. exits. The hooks and statics live for the rest of the process; the
//!    DLL is never unloaded, so `DLL_PROCESS_DETACH` does nothing.
//!
//! Every failure is logged and leaves the game as it was: no hook is
//! installed after a failed step, and the ones already in place only
//! forward to the original functions until there is something to claim.

use core::ffi::c_void;
use core::ops::Range;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use windows_sys::core::BOOL;
use windows_sys::Win32::Foundation::{CloseHandle, HMODULE, TRUE};
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
use windows_sys::Win32::System::SystemServices::DLL_PROCESS_ATTACH;
use windows_sys::Win32::System::Threading::CreateThread;

use crate::deliver::lua_api::{LuaApi, ResolveError};
use crate::deliver::tick::{engine_tick_detour, LUA_API, TICK_ORIGINAL};
use crate::fingerprint::{
    self, hex, Prologue, DISPATCHER, DROP_CALLEE, ENGINE_TICK, HOOK_OWNER_MODULES,
};
use crate::memory::ProcessMemory;
use crate::receive::detours::{dispatch_detour, lookup_detour, DISPATCH_ORIGINAL, LOOKUP_ORIGINAL};
use crate::{hooks, log};

/// How long to wait for `lua51.dll` to load before giving up.
const LUA_WAIT: Duration = Duration::from_secs(30);
const LUA_POLL: Duration = Duration::from_millis(250);

static BOOTSTRAP_STARTED: AtomicBool = AtomicBool::new(false);

/// Windows DLL entry point. Starts the bootstrap thread on attach and
/// returns: no allocation, no I/O, nothing that could take the loader lock
/// again.
///
/// # Safety
///
/// Called by the OS loader, under the loader lock.
#[no_mangle]
pub unsafe extern "system" fn DllMain(
    _module: HMODULE,
    reason: u32,
    _reserved: *mut c_void,
) -> BOOL {
    if reason == DLL_PROCESS_ATTACH && !BOOTSTRAP_STARTED.swap(true, Ordering::SeqCst) {
        // SAFETY: a `'static` thread procedure with no argument. The thread
        // starts running once the loader lock is released.
        let handle = unsafe {
            CreateThread(
                core::ptr::null(),
                0,
                Some(bootstrap_thread),
                core::ptr::null(),
                0,
                core::ptr::null_mut(),
            )
        };
        if !handle.is_null() {
            // SAFETY: a handle this function owns; the thread keeps running.
            unsafe { CloseHandle(handle) };
        }
    }
    TRUE
}

unsafe extern "system" fn bootstrap_thread(_arg: *mut c_void) -> u32 {
    // A panic must not unwind into the thread-start code.
    match std::panic::catch_unwind(bootstrap) {
        Ok(()) => 0,
        Err(_) => 1,
    }
}

fn bootstrap() {
    let exe = std::env::current_exe().ok();
    log::init(exe.as_deref().and_then(|p| p.parent()));
    log::line(format_args!(
        "attached, version {}, host {}",
        env!("CARGO_PKG_VERSION"),
        exe.as_deref()
            .map_or_else(|| "?".into(), |p| p.display().to_string())
    ));

    let api = match wait_for_lua() {
        Ok(api) => api,
        Err(reason) => {
            log::line(format_args!("{reason}; nothing installed"));
            return;
        }
    };
    log::line("lua51.dll exports resolved");

    let hook_owners = hook_owner_ranges();
    let mut usable = true;
    for (site, prologue) in fingerprint::check_all(&ProcessMemory, &hook_owners) {
        match &prologue {
            Prologue::Stock => log::line(format_args!(
                "{} at 0x{:08x}: stock",
                site.name, site.address
            )),
            Prologue::Chained { jump_to } => log::line(format_args!(
                "{} at 0x{:08x}: already hooked (jump to 0x{jump_to:08x}), chaining",
                site.name, site.address
            )),
            Prologue::UnknownHook { jump_to } => log::line(format_args!(
                "{} at 0x{:08x}: already hooked by a jump to 0x{jump_to:08x}, outside every \
                 known hook owner; not chaining",
                site.name, site.address
            )),
            Prologue::Mismatch { actual } => log::line(format_args!(
                "{} at 0x{:08x}: expected {}, found {}",
                site.name,
                site.address,
                hex(site.expected),
                hex(actual)
            )),
            Prologue::Unreadable => log::line(format_args!(
                "{} at 0x{:08x}: unreadable",
                site.name, site.address
            )),
        }
        usable &= prologue.is_usable();
    }
    if !usable {
        log::line(
            "a prologue does not match: a different SGW.exe build, or another patch \
             at that address; nothing installed",
        );
        return;
    }

    if LUA_API.set(api).is_err() {
        log::line("already bootstrapped; nothing more to do");
        return;
    }
    if let Err(e) = hooks::init() {
        log::line(format_args!("{e}; nothing installed"));
        return;
    }

    // Deliver first: receiving without a drain would only fill the queue.
    // SAFETY: each detour matches its site's signature (see `addresses`)
    // and reads its trampoline from the slot passed with it.
    let steps = [
        (
            &ENGINE_TICK,
            engine_tick_detour as *const () as usize,
            &TICK_ORIGINAL,
        ),
        (
            &DISPATCHER,
            dispatch_detour as *const () as usize,
            &DISPATCH_ORIGINAL,
        ),
        (
            &DROP_CALLEE,
            lookup_detour as *const () as usize,
            &LOOKUP_ORIGINAL,
        ),
    ];
    for (site, detour, original) in steps {
        match unsafe { hooks::install(site, detour, original, &hook_owners) } {
            Ok(_) => log::line(format_args!("hooked {}", site.name)),
            Err(e) => {
                log::line(format_args!("{e}; stopping, the Black Market stays off"));
                return;
            }
        }
    }
    log::line(
        "Black Market installed: received calls go to the Lua table CimmeriaBM, and          CimmeriaBMNative is registered for sending once the UI Lua is up",
    );
}

/// The image ranges of the loaded [`HOOK_OWNER_MODULES`]: where an earlier
/// hook on a chainable site may jump to. Empty when none is loaded, and
/// then no earlier hook is accepted.
fn hook_owner_ranges() -> Vec<Range<usize>> {
    HOOK_OWNER_MODULES
        .iter()
        .filter_map(|name| {
            let wide: Vec<u16> = name.encode_utf16().chain(Some(0)).collect();
            // SAFETY: a NUL-terminated wide string; no reference is taken.
            let base = unsafe { GetModuleHandleW(wide.as_ptr()) };
            if base.is_null() {
                return None;
            }
            let range = fingerprint::image_range(&ProcessMemory, base as usize);
            match &range {
                Some(r) => log::line(format_args!(
                    "{name} loaded at 0x{:08x}..0x{:08x}; its hooks may be chained",
                    r.start, r.end
                )),
                None => log::line(format_args!(
                    "{name} loaded at 0x{:08x} but its PE header is unreadable",
                    base as usize
                )),
            }
            range
        })
        .collect()
}

/// Resolve the `lua51.dll` exports, waiting up to [`LUA_WAIT`] for the
/// DLL itself.
fn wait_for_lua() -> Result<LuaApi, String> {
    let mut waited = Duration::ZERO;
    loop {
        match LuaApi::resolve() {
            Ok(api) => return Ok(api),
            Err(ResolveError::Missing(names)) => {
                return Err(format!(
                    "lua51.dll lacks {}: not the client's Lua build",
                    names.join(", ")
                ))
            }
            Err(ResolveError::NotLoaded) if waited >= LUA_WAIT => {
                return Err(format!(
                    "lua51.dll not loaded after {}s",
                    LUA_WAIT.as_secs()
                ))
            }
            Err(ResolveError::NotLoaded) => {
                std::thread::sleep(LUA_POLL);
                waited += LUA_POLL;
            }
        }
    }
}
