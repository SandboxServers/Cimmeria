//! Child-process ownership with layered reaping.
//!
//! The [wireclient ADR](../../../docs/architecture/wireclient.md) lists
//! "server-process lifecycle in tests" as an unsolved risk that must be
//! settled *before* a harness lands, specifically calling out crash safety.
//! This module is that answer.
//!
//! # Why three layers
//!
//! No single mechanism covers every way a test process can die, so
//! [`ChildGuard`] stacks three with different failure coverage:
//!
//! | Layer | Mechanism | Covers |
//! |---|---|---|
//! | 1 | `Drop` → `kill()` + `wait()` | normal return, early `?`, **panic unwind** |
//! | 2 | console / process group | Ctrl-C at an interactive terminal |
//! | 3 | Job Object (Windows) / `PR_SET_PDEATHSIG` (Linux) | parent hard-killed, `panic = "abort"`, OOM kill |
//!
//! Layer 1 is the workhorse: `Drop` runs during unwind, so a panicking test
//! body still reaps its server.
//!
//! Layers 1 and 3 are redundant on purpose, which has a testing consequence
//! worth knowing: with the job object active, deleting the entire `Drop` impl
//! still leaves every child correctly reaped. Tests that mean to assert on
//! `Drop` must therefore spawn with [`OrphanProtection::DropOnly`], or they
//! pass for the wrong reason. See `tests/reap.rs`.
//!
//! Layer 2 needs no code. A child spawned with inherited stdio joins the
//! parent's console (Windows) / foreground process group (Unix), so Ctrl-C is
//! delivered to it directly. For `cimmeria-server` this is the *preferred*
//! path — it has a real Ctrl-C handler and shuts down gracefully, which a
//! `kill()` from layer 1 does not give it. Inheriting stdio is therefore a
//! deliberate default, not just a convenience.
//!
//! Layer 3 is the one that closes the orphan hole. Layers 1 and 2 both assume
//! the parent gets to run code or that a signal is delivered; neither is true
//! when the parent is `taskkill /F`'d, aborts, or is OOM-killed. A Windows
//! Job Object with `JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE` makes the *kernel*
//! kill the child when the last handle to the job closes — which happens on
//! process teardown no matter how violent. Linux gets the equivalent via
//! `PR_SET_PDEATHSIG`.

use std::io;
use std::process::{Child, Command};

/// A spawned child process that is killed when the guard is dropped.
///
/// Dropping blocks until the child has been reaped, so no zombie is left
/// behind and the child's ports are released before the next test binds them.
#[derive(Debug)]
pub struct ChildGuard {
    child: Option<Child>,
    label: String,
    pid: u32,

    /// Held for its `Drop`: closing the job handle is what triggers the
    /// kernel-side kill. Never read directly.
    #[cfg(windows)]
    _job: Option<platform::JobObject>,
}

/// Whether to install the kernel-level orphan-prevention layer.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum OrphanProtection {
    /// Job Object (Windows) / `PR_SET_PDEATHSIG` (Linux). The default, and
    /// what every real harness should use.
    #[default]
    Kernel,

    /// Rely on `Drop` and console signals only.
    ///
    /// Two real uses: environments where job assignment is refused, and tests
    /// that need to verify the `Drop` layer *on its own* — with the kernel
    /// layer active, a broken `Drop` is invisible because the job object
    /// silently covers for it.
    DropOnly,
}

impl ChildGuard {
    /// Spawn `command` with full orphan protection and take ownership of the
    /// resulting process.
    ///
    /// `label` appears in log lines so a leaked-process report names which
    /// harness produced it.
    pub fn spawn(command: Command, label: impl Into<String>) -> io::Result<Self> {
        Self::spawn_with(command, label, OrphanProtection::Kernel)
    }

    /// Spawn with an explicit orphan-protection level.
    pub fn spawn_with(
        mut command: Command,
        label: impl Into<String>,
        protection: OrphanProtection,
    ) -> io::Result<Self> {
        let label = label.into();

        // Layer 3 (Unix): ask the kernel to SIGKILL this child when its
        // parent dies, whatever the cause. Installed in the forked child
        // between fork and exec.
        #[cfg(target_os = "linux")]
        if protection == OrphanProtection::Kernel {
            use std::os::unix::process::CommandExt;
            // SAFETY: `pre_exec` runs between fork and exec. `prctl` is
            // async-signal-safe and touches no allocator state, which is the
            // requirement for this hook.
            unsafe {
                command.pre_exec(|| {
                    // SIGKILL rather than SIGTERM: this is the last-resort
                    // layer, reached only when the parent died without
                    // running Drop.
                    if libc::prctl(libc::PR_SET_PDEATHSIG, libc::SIGKILL) == -1 {
                        return Err(io::Error::last_os_error());
                    }
                    Ok(())
                });
            }
        }

        let child = command.spawn()?;
        let pid = child.id();

        // Layer 3 (Windows): assign to a kill-on-close job object.
        //
        // There is a sub-millisecond window between spawn and assignment in
        // which the child could spawn grandchildren that escape the job.
        // Closing it needs CREATE_SUSPENDED plus a ResumeThread, which std
        // does not expose (it hands back no thread handle). The server does
        // not fork anything in that window, so the window is accepted rather
        // than worked around with raw process creation.
        #[cfg(windows)]
        let job = match protection {
            OrphanProtection::DropOnly => None,
            OrphanProtection::Kernel => match platform::JobObject::new_kill_on_close() {
                Ok(job) => match job.assign(&child) {
                    Ok(()) => Some(job),
                    Err(e) => {
                        // Non-fatal: layers 1 and 2 still apply. Logged
                        // because it silently downgrades orphan protection.
                        tracing::warn!(
                            label = %label,
                            pid,
                            error = %e,
                            "Failed to assign child to job object; orphan protection \
                             on hard parent kill is unavailable for this child"
                        );
                        None
                    }
                },
                Err(e) => {
                    tracing::warn!(
                        label = %label,
                        pid,
                        error = %e,
                        "Failed to create job object; orphan protection on hard \
                         parent kill is unavailable for this child"
                    );
                    None
                }
            },
        };

        tracing::debug!(label = %label, pid, "Spawned child process");

        Ok(Self {
            child: Some(child),
            label,
            pid,
            #[cfg(windows)]
            _job: job,
        })
    }

    /// OS process id of the child.
    pub fn pid(&self) -> u32 {
        self.pid
    }

    /// Label this guard was created with.
    pub fn label(&self) -> &str {
        &self.label
    }

    /// Whether the child has exited on its own, and with what status.
    ///
    /// `Ok(None)` means still running. Used by the readiness loop to fail
    /// fast when the server dies during startup instead of polling a port
    /// nothing will ever bind.
    pub fn try_exit_status(&mut self) -> io::Result<Option<std::process::ExitStatus>> {
        match self.child.as_mut() {
            Some(child) => child.try_wait(),
            None => Ok(None),
        }
    }

    /// Kill the child and block until it is reaped. Idempotent.
    ///
    /// Called by `Drop`; exposed so a test can reap deterministically at a
    /// chosen point rather than at end of scope.
    pub fn kill_and_reap(&mut self) {
        let Some(mut child) = self.child.take() else {
            return;
        };

        // An already-exited child makes `kill` return an error on some
        // platforms; reaping it is still required to clear the zombie, so the
        // kill result is intentionally not treated as fatal.
        match child.try_wait() {
            Ok(Some(status)) => {
                tracing::debug!(
                    label = %self.label,
                    pid = self.pid,
                    ?status,
                    "Child had already exited; nothing to kill"
                );
                return;
            }
            Ok(None) => {}
            Err(e) => {
                tracing::warn!(
                    label = %self.label,
                    pid = self.pid,
                    error = %e,
                    "try_wait failed before kill; proceeding to kill anyway"
                );
            }
        }

        if let Err(e) = child.kill() {
            tracing::warn!(
                label = %self.label,
                pid = self.pid,
                error = %e,
                "Failed to kill child process"
            );
        }

        // Blocking wait is the point: returning before the child is reaped
        // would let the next test race this one for the same ports.
        match child.wait() {
            Ok(status) => {
                tracing::debug!(
                    label = %self.label,
                    pid = self.pid,
                    ?status,
                    "Child process reaped"
                );
            }
            Err(e) => {
                tracing::error!(
                    label = %self.label,
                    pid = self.pid,
                    error = %e,
                    "Failed to reap child process; it may be orphaned"
                );
            }
        }
    }
}

impl Drop for ChildGuard {
    fn drop(&mut self) {
        // Runs on the normal path AND during panic unwind, which is what
        // makes a panicking test body still reap its server.
        self.kill_and_reap();
    }
}

/// Whether a process with `pid` currently exists.
///
/// Intended for tests asserting that a reap actually happened. Note that pids
/// are recycled by the OS, so this is only meaningful immediately after the
/// process in question was reaped.
pub fn is_process_alive(pid: u32) -> bool {
    platform::is_process_alive(pid)
}

#[cfg(windows)]
mod platform {
    use std::io;
    use std::os::windows::io::AsRawHandle;
    use std::process::Child;

    use windows_sys::Win32::Foundation::{CloseHandle, HANDLE};
    use windows_sys::Win32::System::JobObjects::{
        AssignProcessToJobObject, CreateJobObjectW, JobObjectExtendedLimitInformation,
        SetInformationJobObject, JOBOBJECT_EXTENDED_LIMIT_INFORMATION,
        JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE,
    };
    use windows_sys::Win32::System::Threading::{
        GetExitCodeProcess, OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION,
    };

    /// `GetExitCodeProcess` reports this while the process is still running.
    const STILL_ACTIVE: u32 = 259;

    /// Owns a Windows Job Object configured to kill its members when the last
    /// handle closes.
    ///
    /// This is the layer that survives a hard parent kill: the handle is
    /// closed by the kernel during process teardown regardless of whether any
    /// user code got to run, and that close is what kills the child.
    #[derive(Debug)]
    pub(super) struct JobObject(HANDLE);

    // A job handle is a process-wide kernel object; moving it between threads
    // is safe. The raw pointer type is what makes this opt-in necessary.
    unsafe impl Send for JobObject {}
    unsafe impl Sync for JobObject {}

    impl JobObject {
        pub(super) fn new_kill_on_close() -> io::Result<Self> {
            // SAFETY: an anonymous job object with default security. A null
            // return is the documented failure signal and is checked.
            let handle = unsafe { CreateJobObjectW(std::ptr::null(), std::ptr::null()) };
            if handle.is_null() {
                return Err(io::Error::last_os_error());
            }
            let job = Self(handle);

            let mut info: JOBOBJECT_EXTENDED_LIMIT_INFORMATION =
                // SAFETY: the struct is plain-old-data; an all-zero value is
                // the documented "no limits set" starting point.
                unsafe { std::mem::zeroed() };
            info.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;

            // SAFETY: `info` matches the class being set, and the size is
            // taken from the same type.
            let ok = unsafe {
                SetInformationJobObject(
                    job.0,
                    JobObjectExtendedLimitInformation,
                    std::ptr::addr_of!(info).cast(),
                    std::mem::size_of::<JOBOBJECT_EXTENDED_LIMIT_INFORMATION>() as u32,
                )
            };
            if ok == 0 {
                return Err(io::Error::last_os_error());
            }
            Ok(job)
        }

        pub(super) fn assign(&self, child: &Child) -> io::Result<()> {
            // SAFETY: the handle is owned by `child`, which outlives this
            // call.
            let ok = unsafe { AssignProcessToJobObject(self.0, child.as_raw_handle() as HANDLE) };
            if ok == 0 {
                return Err(io::Error::last_os_error());
            }
            Ok(())
        }
    }

    impl Drop for JobObject {
        fn drop(&mut self) {
            // Closing the last handle is what triggers the kill-on-close
            // limit. By the time a `ChildGuard` drops, layer 1 has usually
            // already reaped the child and this is a no-op.
            // SAFETY: the handle was created by us and is closed exactly once.
            unsafe { CloseHandle(self.0) };
        }
    }

    pub(super) fn is_process_alive(pid: u32) -> bool {
        // SAFETY: a failed open returns null, which is checked before use.
        let handle = unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid) };
        if handle.is_null() {
            // Cannot open ⇒ no such process (the harness always has rights to
            // its own children).
            return false;
        }
        let mut code: u32 = 0;
        // SAFETY: `handle` is valid here; `code` is a live u32.
        let ok = unsafe { GetExitCodeProcess(handle, &mut code) };
        // SAFETY: closing the handle we just opened, exactly once.
        unsafe { CloseHandle(handle) };

        ok != 0 && code == STILL_ACTIVE
    }
}

#[cfg(unix)]
mod platform {
    pub(super) fn is_process_alive(pid: u32) -> bool {
        // Signal 0 performs error checking without delivering a signal:
        // success means the process exists and we may signal it.
        //
        // A reaped child of this process is fully gone (not a zombie), so
        // this correctly reports false after `kill_and_reap`.
        // SAFETY: `kill` with signal 0 has no side effects.
        unsafe { libc::kill(pid as libc::pid_t, 0) == 0 }
    }
}
