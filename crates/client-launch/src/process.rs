//! Process lifecycle for injection: create a target suspended, resume
//! it, and hold its handle to wait on the exit.
//!
//! Split out of [`crate::inject`] (which re-exports everything here) so
//! the injection pipeline and the process handles each stay one concern.
//! The command-line quoting [`command_line`] is portable and unit-tested
//! on every host.

#[cfg(windows)]
use std::path::Path;

use std::ffi::OsString;

#[cfg(windows)]
use crate::inject::{get_last_error, InjectError};

/// Build a Windows command line: the program path, then each argument,
/// quoted by the rules `CommandLineToArgvW` and the MSVC CRT parse. An
/// argument with no space, tab or quote is passed bare; any other is
/// wrapped in quotes, with backslashes doubled only where they precede a
/// quote. The program path is always quoted, so a path with spaces is one
/// token.
pub fn command_line(program: &OsString, args: &[OsString]) -> String {
    let mut out = String::new();
    out.push('"');
    out.push_str(&program.to_string_lossy());
    out.push('"');
    for arg in args {
        out.push(' ');
        quote_arg(&arg.to_string_lossy(), &mut out);
    }
    out
}

fn quote_arg(arg: &str, out: &mut String) {
    let needs_quotes = arg.is_empty() || arg.contains([' ', '\t', '"']);
    if !needs_quotes {
        out.push_str(arg);
        return;
    }
    out.push('"');
    let mut backslashes = 0usize;
    for c in arg.chars() {
        match c {
            '\\' => backslashes += 1,
            '"' => {
                // Double the run of backslashes before a quote, then escape
                // the quote itself.
                out.extend(std::iter::repeat_n('\\', backslashes * 2 + 1));
                out.push('"');
                backslashes = 0;
            }
            other => {
                out.extend(std::iter::repeat_n('\\', backslashes));
                backslashes = 0;
                out.push(other);
            }
        }
    }
    // Backslashes before the closing quote must be doubled too.
    out.extend(std::iter::repeat_n('\\', backslashes * 2));
    out.push('"');
}

/// A child process that was created with `CREATE_SUSPENDED` and is
/// waiting on `ResumeThread` to start executing user code. Held as
/// an RAII guard so the kernel handles are released even on early-
/// return error paths.
///
/// Typical lifecycle:
/// 1. [`create_process_suspended`] returns this.
/// 2. Caller invokes [`crate::inject::inject_dll`] with
///    `self.process_handle()`.
/// 3. Caller calls [`SuspendedProcess::resume`] or
///    [`SuspendedProcess::resume_running`] to start the target's main
///    thread.
///
/// Dropping without resuming leaves the child process suspended
/// forever — the OS reclaims it when the launcher exits, but if the
/// launcher keeps running you've leaked a zombie. The `Drop` impl
/// closes the handles either way; it's the caller's responsibility to
/// resume (or [`terminate`](SuspendedProcess::terminate)) on the happy
/// path.
#[cfg(windows)]
pub struct SuspendedProcess {
    process_handle: windows_sys::Win32::Foundation::HANDLE,
    thread_handle: windows_sys::Win32::Foundation::HANDLE,
    pid: u32,
}

#[cfg(windows)]
impl SuspendedProcess {
    /// Process ID of the spawned (suspended) target.
    pub fn pid(&self) -> u32 {
        self.pid
    }

    /// Raw process HANDLE — feed this into [`crate::inject::inject_dll`].
    pub fn process_handle(&self) -> windows_sys::Win32::Foundation::HANDLE {
        self.process_handle
    }

    /// Resume the suspended main thread, allowing the target to
    /// begin executing user code. Consumes self so the handles are
    /// closed exactly once on the happy path. Returns the
    /// suspended-thread previous-suspend-count from `ResumeThread`
    /// (almost always `1` for a freshly-suspended process; anything
    /// else indicates an unexpected re-suspension).
    pub fn resume(self) -> Result<u32, InjectError> {
        use windows_sys::Win32::System::Threading::ResumeThread;

        // SAFETY: thread_handle was returned by CreateProcessW and
        // hasn't been closed yet (Drop runs after this).
        let prev = unsafe { ResumeThread(self.thread_handle) };
        if prev == u32::MAX {
            return Err(InjectError::Win32 {
                api: "ResumeThread",
                code: get_last_error(),
            });
        }
        Ok(prev)
    }

    /// [`resume`](Self::resume), keeping the process handle so the
    /// caller can wait for the game to exit. The thread handle is
    /// closed here.
    pub fn resume_running(mut self) -> Result<RunningProcess, InjectError> {
        use windows_sys::Win32::Foundation::CloseHandle;
        use windows_sys::Win32::System::Threading::ResumeThread;

        // SAFETY: thread_handle was returned by CreateProcessW and is
        // still open.
        if unsafe { ResumeThread(self.thread_handle) } == u32::MAX {
            return Err(InjectError::Win32 {
                api: "ResumeThread",
                code: get_last_error(),
            });
        }
        let running = RunningProcess {
            process_handle: self.process_handle,
            pid: self.pid,
        };
        // SAFETY: closed once here. Both fields are nulled so Drop skips
        // them: the process handle now belongs to `running`.
        unsafe { CloseHandle(self.thread_handle) };
        self.thread_handle = std::ptr::null_mut();
        self.process_handle = std::ptr::null_mut();
        Ok(running)
    }

    /// Kill a process that never ran user code, after a failed
    /// injection, so no suspended copy of the game is left behind.
    /// Best effort; `Drop` closes the handles either way.
    pub fn terminate(self) {
        use windows_sys::Win32::System::Threading::TerminateProcess;
        // SAFETY: process_handle is open until Drop runs after this.
        unsafe { TerminateProcess(self.process_handle, 1) };
    }
}

#[cfg(windows)]
impl Drop for SuspendedProcess {
    fn drop(&mut self) {
        use windows_sys::Win32::Foundation::CloseHandle;
        // SAFETY: HANDLEs are owned by self; we close them once.
        unsafe {
            if !self.thread_handle.is_null() {
                CloseHandle(self.thread_handle);
            }
            if !self.process_handle.is_null() {
                CloseHandle(self.process_handle);
            }
        }
    }
}

/// A launched game process whose handle the launcher keeps, so it can
/// wait for the exit (a telemetry session ends there). Dropping it
/// closes the handle and never kills the game.
#[cfg(windows)]
#[derive(Debug)]
pub struct RunningProcess {
    process_handle: windows_sys::Win32::Foundation::HANDLE,
    pid: u32,
}

// SAFETY: a process HANDLE is a kernel object reference usable from any
// thread; nothing about it is tied to the thread that opened it.
#[cfg(windows)]
unsafe impl Send for RunningProcess {}

#[cfg(windows)]
impl RunningProcess {
    /// Open an already-running process by pid, only to wait on it:
    /// `SYNCHRONIZE` plus `PROCESS_QUERY_LIMITED_INFORMATION` for the exit
    /// code. This is how a 64-bit launcher follows a game the 32-bit
    /// `sgw-start32` helper started and injected.
    pub fn open(pid: u32) -> Result<Self, InjectError> {
        use windows_sys::Win32::Foundation::FALSE;
        use windows_sys::Win32::System::Threading::{
            OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION, PROCESS_SYNCHRONIZE,
        };
        // SAFETY: OpenProcess takes plain values; a null return is an error.
        let handle = unsafe {
            OpenProcess(
                PROCESS_SYNCHRONIZE | PROCESS_QUERY_LIMITED_INFORMATION,
                FALSE,
                pid,
            )
        };
        if handle.is_null() {
            return Err(InjectError::Win32 {
                api: "OpenProcess",
                code: get_last_error(),
            });
        }
        Ok(Self {
            process_handle: handle,
            pid,
        })
    }

    pub fn pid(&self) -> u32 {
        self.pid
    }

    /// Block until the process exits, and return its exit code.
    pub fn wait(self) -> std::io::Result<i32> {
        use windows_sys::Win32::Foundation::{FALSE, WAIT_FAILED};
        use windows_sys::Win32::System::Threading::{
            GetExitCodeProcess, WaitForSingleObject, INFINITE,
        };
        // SAFETY: the handle is open until Drop, which runs after this.
        if unsafe { WaitForSingleObject(self.process_handle, INFINITE) } == WAIT_FAILED {
            return Err(std::io::Error::last_os_error());
        }
        let mut code: u32 = 0;
        // SAFETY: as above; `code` outlives the call.
        if unsafe { GetExitCodeProcess(self.process_handle, &mut code) } == FALSE {
            return Err(std::io::Error::last_os_error());
        }
        Ok(code as i32)
    }
}

#[cfg(windows)]
impl Drop for RunningProcess {
    fn drop(&mut self) {
        use windows_sys::Win32::Foundation::CloseHandle;
        // SAFETY: owned, closed once.
        unsafe { CloseHandle(self.process_handle) };
    }
}

/// An existing process opened with the rights injection needs. Held as
/// RAII so the handle closes on every path.
#[cfg(windows)]
pub struct OpenedProcess {
    process_handle: windows_sys::Win32::Foundation::HANDLE,
}

#[cfg(windows)]
impl OpenedProcess {
    /// Open `pid` for injection: create a remote thread, allocate and
    /// write memory in it, and query it (the bitness check).
    pub fn for_inject(pid: u32) -> Result<Self, InjectError> {
        use windows_sys::Win32::Foundation::FALSE;
        use windows_sys::Win32::System::Threading::{
            OpenProcess, PROCESS_CREATE_THREAD, PROCESS_QUERY_INFORMATION, PROCESS_VM_OPERATION,
            PROCESS_VM_READ, PROCESS_VM_WRITE,
        };
        // SAFETY: OpenProcess takes plain values; a null return is an error.
        let handle = unsafe {
            OpenProcess(
                PROCESS_CREATE_THREAD
                    | PROCESS_QUERY_INFORMATION
                    | PROCESS_VM_OPERATION
                    | PROCESS_VM_READ
                    | PROCESS_VM_WRITE,
                FALSE,
                pid,
            )
        };
        if handle.is_null() {
            return Err(InjectError::Win32 {
                api: "OpenProcess",
                code: get_last_error(),
            });
        }
        Ok(Self {
            process_handle: handle,
        })
    }

    pub fn process_handle(&self) -> windows_sys::Win32::Foundation::HANDLE {
        self.process_handle
    }
}

#[cfg(windows)]
impl Drop for OpenedProcess {
    fn drop(&mut self) {
        use windows_sys::Win32::Foundation::CloseHandle;
        // SAFETY: owned, closed once.
        unsafe { CloseHandle(self.process_handle) };
    }
}

/// Spawn `exe_path` suspended (`CREATE_SUSPENDED`) with no arguments.
/// See [`create_process_suspended_with_args`].
#[cfg(windows)]
pub fn create_process_suspended(
    exe_path: &Path,
    cwd: Option<&Path>,
) -> Result<SuspendedProcess, InjectError> {
    create_process_suspended_with_args(exe_path, &[], cwd)
}

/// Spawn `exe_path` suspended (`CREATE_SUSPENDED`) so the caller can
/// inject a DLL before any of the target's user-mode threads run. The
/// returned [`SuspendedProcess`] owns the kernel handles.
///
/// `args` are passed to the target, quoted by [`command_line`]. `cwd`
/// sets the target's working directory; pass `None` to inherit the
/// caller's. SGW.exe is path-sensitive (it resolves `..\..\Game` from
/// cwd), so the launcher always passes `Some(install_dir)`.
#[cfg(windows)]
pub fn create_process_suspended_with_args(
    exe_path: &Path,
    args: &[OsString],
    cwd: Option<&Path>,
) -> Result<SuspendedProcess, InjectError> {
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::Foundation::{FALSE, TRUE};
    use windows_sys::Win32::System::Threading::{
        CreateProcessW, CREATE_SUSPENDED, CREATE_UNICODE_ENVIRONMENT, PROCESS_INFORMATION,
        STARTUPINFOW,
    };

    if !exe_path.exists() {
        return Err(InjectError::DllMissing(exe_path.to_path_buf()));
    }

    // CreateProcessW reads lpCommandLine as MUTABLE wide-char data (it can
    // rewrite the buffer in place), so it is built as an owned Vec<u16>.
    let mut command: Vec<u16> =
        std::ffi::OsStr::new(&command_line(&exe_path.as_os_str().to_os_string(), args))
            .encode_wide()
            .chain(std::iter::once(0))
            .collect();

    let app_name: Vec<u16> = exe_path
        .as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();

    let cwd_wide: Option<Vec<u16>> = cwd.map(|p| {
        p.as_os_str()
            .encode_wide()
            .chain(std::iter::once(0))
            .collect()
    });
    let cwd_ptr = cwd_wide
        .as_ref()
        .map(|v| v.as_ptr())
        .unwrap_or(std::ptr::null());

    // SAFETY: Zero-init both Win32 structs. STARTUPINFOW's cb field
    // MUST be set to size_of so the kernel knows which fields exist.
    let mut startup: STARTUPINFOW = unsafe { std::mem::zeroed() };
    startup.cb = std::mem::size_of::<STARTUPINFOW>() as u32;
    let mut pi: PROCESS_INFORMATION = unsafe { std::mem::zeroed() };

    // SAFETY: All pointers either point to valid wide-string
    // buffers above or are NULL where the API allows it.
    let ok = unsafe {
        CreateProcessW(
            app_name.as_ptr(),
            command.as_mut_ptr(),
            std::ptr::null(),
            std::ptr::null(),
            FALSE,
            CREATE_SUSPENDED | CREATE_UNICODE_ENVIRONMENT,
            std::ptr::null(),
            cwd_ptr,
            &startup,
            &mut pi,
        )
    };
    if ok == TRUE {
        Ok(SuspendedProcess {
            process_handle: pi.hProcess,
            thread_handle: pi.hThread,
            pid: pi.dwProcessId,
        })
    } else {
        Err(InjectError::Win32 {
            api: "CreateProcessW",
            code: get_last_error(),
        })
    }
}

/// Non-Windows stub, so callers type-check off Windows. Nothing
/// constructs it there: every launch that would returns an error first.
#[cfg(not(windows))]
#[derive(Debug)]
pub struct RunningProcess {
    _private: (),
}

#[cfg(not(windows))]
impl RunningProcess {
    pub fn open(pid: u32) -> Result<Self, crate::inject::InjectError> {
        let _ = pid;
        Err(crate::inject::InjectError::DllMissing(
            std::path::PathBuf::new(),
        ))
    }

    pub fn pid(&self) -> u32 {
        unreachable!("RunningProcess is never constructed off Windows")
    }

    pub fn wait(self) -> std::io::Result<i32> {
        unreachable!("RunningProcess is never constructed off Windows")
    }
}

/// Non-Windows stub.
#[cfg(not(windows))]
pub fn create_process_suspended(
    exe_path: &std::path::Path,
    _cwd: Option<&std::path::Path>,
) -> Result<(), crate::inject::InjectError> {
    Err(crate::inject::InjectError::DllMissing(
        exe_path.to_path_buf(),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cl(args: &[&str]) -> String {
        let args: Vec<OsString> = args.iter().map(OsString::from).collect();
        command_line(&OsString::from(r"C:\Game Dir\SGW.exe"), &args)
    }

    #[test]
    fn program_path_is_always_quoted() {
        assert_eq!(cl(&[]), r#""C:\Game Dir\SGW.exe""#);
    }

    #[test]
    fn plain_args_pass_bare() {
        assert_eq!(cl(&["-log", "x=1"]), r#""C:\Game Dir\SGW.exe" -log x=1"#);
    }

    #[test]
    fn args_with_spaces_or_empty_are_quoted() {
        assert_eq!(cl(&["a b", ""]), r#""C:\Game Dir\SGW.exe" "a b" """#);
    }

    /// The MSVC rules: a quote is escaped, and backslashes are doubled
    /// only where a quote follows them (inside or at the closing quote).
    #[test]
    fn quotes_and_trailing_backslashes_follow_msvc_rules() {
        assert_eq!(
            cl(&[r#"say "hi""#]),
            r#""C:\Game Dir\SGW.exe" "say \"hi\"""#
        );
        assert_eq!(
            cl(&[r"C:\path with space\"]),
            r#""C:\Game Dir\SGW.exe" "C:\path with space\\""#
        );
        // Backslashes not before a quote stay single.
        assert_eq!(cl(&[r"a\b c"]), r#""C:\Game Dir\SGW.exe" "a\b c""#);
    }
}
