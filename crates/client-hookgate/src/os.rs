//! The Win32 half: reading the process's own memory without faulting,
//! finding the loaded hook-owner modules, and the install lock.

use core::ffi::c_void;
use core::ops::Range;
use std::time::Duration;

use windows_sys::Win32::Foundation::{CloseHandle, HANDLE, WAIT_ABANDONED, WAIT_OBJECT_0};
use windows_sys::Win32::System::Diagnostics::Debug::ReadProcessMemory;
use windows_sys::Win32::System::LibraryLoader::{
    GetModuleHandleExW, GetModuleHandleW, GET_MODULE_HANDLE_EX_FLAG_FROM_ADDRESS,
    GET_MODULE_HANDLE_EX_FLAG_UNCHANGED_REFCOUNT,
};
use windows_sys::Win32::System::Threading::{
    CreateMutexW, GetCurrentProcess, GetCurrentProcessId, ReleaseMutex, WaitForSingleObject,
};

use crate::{image_range, HOOK_OWNER_MODULES};

/// `len` bytes at `addr` in this process, or `None` if any is unreadable.
/// Goes through `ReadProcessMemory`, which reports an unmapped or
/// protected page as a failed call instead of raising an access
/// violation.
pub fn read_bytes(addr: usize, len: usize) -> Option<Vec<u8>> {
    if addr == 0 {
        return None;
    }
    let mut out = vec![0u8; len];
    if len == 0 {
        return Some(out);
    }
    let mut copied = 0usize;
    // SAFETY: `out` is a writable buffer of `len` bytes; the kernel
    // validates the source range.
    let ok = unsafe {
        ReadProcessMemory(
            GetCurrentProcess(),
            addr as *const c_void,
            out.as_mut_ptr().cast(),
            len,
            &mut copied,
        )
    };
    (ok != 0 && copied == len).then_some(out)
}

/// A loaded [`HOOK_OWNER_MODULES`] entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoadedOwner {
    /// Its file name.
    pub name: &'static str,
    /// Where it is loaded.
    pub base: usize,
    /// Its image, or `None` if the PE header could not be read (and then
    /// no jump into it is accepted).
    pub image: Option<Range<usize>>,
}

/// Every loaded [`HOOK_OWNER_MODULES`] entry other than the calling DLL
/// itself. Call it after taking the [`HookLock`]: a module that hooked a
/// site before the lock was taken is loaded by then.
pub fn loaded_hook_owners() -> Vec<LoadedOwner> {
    let own = own_module_base();
    HOOK_OWNER_MODULES
        .iter()
        .filter_map(|name| {
            let wide: Vec<u16> = name.encode_utf16().chain(Some(0)).collect();
            // SAFETY: a NUL-terminated wide string; no reference is kept.
            let base = unsafe { GetModuleHandleW(wide.as_ptr()) } as usize;
            if base == 0 || Some(base) == own {
                return None;
            }
            Some(LoadedOwner {
                name,
                base,
                image: image_range(read_bytes, base),
            })
        })
        .collect()
}

/// Base of the module this code is linked into: the calling DLL, since
/// each DLL links its own copy of this crate.
fn own_module_base() -> Option<usize> {
    let mut module = core::ptr::null_mut();
    // SAFETY: FROM_ADDRESS takes any address inside a loaded module; the
    // refcount is left alone, so nothing needs releasing.
    let ok = unsafe {
        GetModuleHandleExW(
            GET_MODULE_HANDLE_EX_FLAG_FROM_ADDRESS | GET_MODULE_HANDLE_EX_FLAG_UNCHANGED_REFCOUNT,
            own_module_base as *const () as *const u16,
            &mut module,
        )
    };
    (ok != 0 && !module.is_null()).then_some(module as usize)
}

/// How the [`HookLock`] was obtained, for the log.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LockOutcome {
    /// Nobody held it.
    Acquired,
    /// The other DLL held it; this one waited.
    AcquiredAfterWait,
    /// The holder's thread exited without releasing it. Taken anyway: a
    /// dead installer cannot be mid-hook.
    Abandoned,
    /// Not obtained within the timeout, or the mutex could not be created.
    /// The caller must not hook: the two DLLs keep separate MinHook
    /// states, so hooking a shared site while the other DLL is mid-install
    /// lets its `MH_EnableHook` overwrite this one's jump and silently drop
    /// the detour. See [`LockOutcome::permits_hooking`].
    Unavailable,
}

impl LockOutcome {
    /// Whether the caller may check and hook under this outcome. Every
    /// outcome but [`LockOutcome::Unavailable`] means this thread holds
    /// the lock.
    pub fn permits_hooking(self) -> bool {
        self != Self::Unavailable
    }
}

/// The per-process install lock (see the crate docs). Released on drop.
#[derive(Debug)]
pub struct HookLock {
    handle: HANDLE,
    owned: bool,
    outcome: LockOutcome,
}

impl HookLock {
    /// Take the lock, waiting up to `timeout` for the other DLL.
    pub fn acquire(timeout: Duration) -> Self {
        // SAFETY: no preconditions.
        let pid = unsafe { GetCurrentProcessId() };
        Self::acquire_named(&crate::hook_lock_name(pid), timeout)
    }

    fn acquire_named(name: &str, timeout: Duration) -> Self {
        let name: Vec<u16> = name.encode_utf16().chain(Some(0)).collect();
        // SAFETY: a NUL-terminated name; default security; not initially
        // owned, so both DLLs take it the same way.
        let handle = unsafe { CreateMutexW(core::ptr::null(), 0, name.as_ptr()) };
        if handle.is_null() {
            return Self {
                handle,
                owned: false,
                outcome: LockOutcome::Unavailable,
            };
        }
        let millis = u32::try_from(timeout.as_millis()).unwrap_or(u32::MAX - 1);
        // SAFETY: a mutex handle this function owns.
        let outcome = match unsafe { WaitForSingleObject(handle, 0) } {
            WAIT_OBJECT_0 => LockOutcome::Acquired,
            WAIT_ABANDONED => LockOutcome::Abandoned,
            // SAFETY: as above.
            _ => match unsafe { WaitForSingleObject(handle, millis) } {
                WAIT_OBJECT_0 => LockOutcome::AcquiredAfterWait,
                WAIT_ABANDONED => LockOutcome::Abandoned,
                _ => LockOutcome::Unavailable,
            },
        };
        Self {
            handle,
            owned: outcome != LockOutcome::Unavailable,
            outcome,
        }
    }

    /// How the lock was obtained.
    pub fn outcome(&self) -> LockOutcome {
        self.outcome
    }
}

impl Drop for HookLock {
    fn drop(&mut self) {
        if self.handle.is_null() {
            return;
        }
        // SAFETY: a mutex handle this value owns, released by the thread
        // that acquired it (the lock is not `Send`).
        unsafe {
            if self.owned {
                ReleaseMutex(self.handle);
            }
            CloseHandle(self.handle);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::mpsc;
    use std::time::Instant;

    /// A name no other test uses: `cargo test` runs tests as threads of
    /// one process, which would share the per-pid name.
    fn lock(test: &str, timeout: Duration) -> HookLock {
        HookLock::acquire_named(&format!("Local\\cimmeria-hookgate-test-{test}"), timeout)
    }

    #[test]
    fn read_bytes_never_faults() {
        let local = [0xAAu8, 0xBB, 0xCC, 0xDD];
        assert_eq!(read_bytes(local.as_ptr() as usize, 4), Some(local.to_vec()));
        assert_eq!(read_bytes(0, 4), None);
        assert_eq!(read_bytes(0x10, 4), None, "the null page is unmapped");
        assert_eq!(read_bytes(usize::MAX - 1, 4), None, "wraps");
    }

    /// The test executable itself is a PE image the reader can size.
    #[test]
    fn image_range_of_this_executable() {
        // SAFETY: NULL names the executable.
        let base = unsafe { GetModuleHandleW(core::ptr::null()) } as usize;
        let range = image_range(read_bytes, base).expect("own image");
        let here = image_range_of_this_executable as *const () as usize;
        assert!(range.contains(&here));
    }

    /// No DLL of ours is loaded into the test process.
    #[test]
    fn no_hook_owner_is_loaded_here() {
        assert!(loaded_hook_owners().is_empty());
    }

    /// Here the "calling module" is the test executable.
    #[test]
    fn own_module_is_the_image_this_code_is_in() {
        // SAFETY: NULL names the executable.
        let exe = unsafe { GetModuleHandleW(core::ptr::null()) } as usize;
        assert_eq!(own_module_base(), Some(exe));
    }

    /// The second holder waits for the first, as the second DLL must wait
    /// for the first one's hooks to go live.
    #[test]
    fn a_second_holder_waits_for_the_first() {
        let first = lock("wait", Duration::from_secs(5));
        assert_eq!(first.outcome(), LockOutcome::Acquired);
        let (tx, rx) = mpsc::channel();
        let waiter = std::thread::spawn(move || {
            let started = Instant::now();
            let second = lock("wait", Duration::from_secs(10));
            tx.send((second.outcome(), started.elapsed())).unwrap();
        });
        std::thread::sleep(Duration::from_millis(300));
        assert!(rx.try_recv().is_err(), "second holder got in early");
        drop(first);
        let (outcome, waited) = rx.recv_timeout(Duration::from_secs(5)).unwrap();
        waiter.join().unwrap();
        assert_eq!(outcome, LockOutcome::AcquiredAfterWait);
        assert!(waited >= Duration::from_millis(250));
    }

    #[test]
    fn a_held_lock_times_out_as_unavailable() {
        let first = lock("timeout", Duration::from_secs(5));
        let outcome = std::thread::spawn(|| lock("timeout", Duration::from_millis(100)).outcome())
            .join()
            .unwrap();
        assert_eq!(outcome, LockOutcome::Unavailable);
        assert!(
            !outcome.permits_hooking(),
            "a DLL that could not take the lock must not hook"
        );
        drop(first);
    }

    #[test]
    fn every_held_outcome_permits_hooking() {
        for held in [
            LockOutcome::Acquired,
            LockOutcome::AcquiredAfterWait,
            LockOutcome::Abandoned,
        ] {
            assert!(held.permits_hooking(), "{held:?}");
        }
    }

    /// A holder whose thread died without releasing does not block the
    /// next DLL.
    #[test]
    fn an_abandoned_lock_is_taken() {
        std::thread::spawn(|| std::mem::forget(lock("abandoned", Duration::from_secs(5))))
            .join()
            .unwrap();
        let next = lock("abandoned", Duration::from_secs(5));
        assert_eq!(next.outcome(), LockOutcome::Abandoned);
    }
}
