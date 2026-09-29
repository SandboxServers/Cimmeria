//! The `sgw-start32` helper: a 32-bit process that injects DLLs into the
//! 32-bit `SGW.exe` on behalf of a 64-bit caller.
//!
//! [`crate::inject::inject_dll`] hands the remote thread the injector's
//! own `LoadLibraryW`, which only exists at the same bitness. A 64-bit
//! launcher cannot reach the WOW64 target's 32-bit `LoadLibraryW` either:
//! a target created suspended has no 32-bit kernel32 mapped yet, and a
//! thread a 64-bit process starts there runs in 64-bit mode. So the
//! launcher stays 64-bit and runs this small 32-bit helper, which does the
//! suspended launch and the injection at the right bitness.
//!
//! This module is the helper's whole contract, used by both sides so they
//! cannot drift: [`Request`] and its [`to_args`](Request::to_args) /
//! [`parse_args`] for the command line, [`Outcome`] and its
//! [`format`](Outcome::format) / [`parse_outcome`] for stdout, and
//! [`run`] for a caller. The telemetry launch and `cimmeria-lab` reuse it
//! unchanged.
//!
//! # Command line
//!
//! ```text
//! sgw-start32 spawn <exe> [--cwd <dir>] [--dll <path>]... [-- <arg>...]
//! sgw-start32 pid <pid> [--dll <path>]...
//! ```
//!
//! `spawn` starts `<exe>` suspended (in `--cwd`, else its own directory),
//! injects each `--dll` in order, and resumes it. `pid` injects into a
//! running process. Arguments after `--` go to the spawned program.
//!
//! # Output
//!
//! One line on stdout. Success: `ok pid=<pid>`, exit code 0. Failure:
//! `error kind=<kind> detail=<text>`, exit code 1 (2 for a usage error),
//! where `<kind>` is one of [`ErrorKind`]'s names. A spawn whose injection
//! fails terminates the suspended process before reporting it.

use std::ffi::OsString;
use std::path::{Path, PathBuf};

/// The helper's file name, next to the launcher.
pub const HELPER_EXE_NAME: &str = "sgw-start32.exe";

/// What to inject into.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Target {
    Spawn {
        exe: PathBuf,
        cwd: Option<PathBuf>,
        args: Vec<OsString>,
    },
    Pid(u32),
}

/// One helper invocation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Request {
    pub target: Target,
    /// Injected in this order.
    pub dlls: Vec<PathBuf>,
}

impl Request {
    /// The helper's command line (after the program name).
    pub fn to_args(&self) -> Vec<OsString> {
        let mut out: Vec<OsString> = Vec::new();
        let mut tail: Vec<OsString> = Vec::new();
        match &self.target {
            Target::Spawn { exe, cwd, args } => {
                out.push("spawn".into());
                out.push(exe.clone().into_os_string());
                if let Some(cwd) = cwd {
                    out.push("--cwd".into());
                    out.push(cwd.clone().into_os_string());
                }
                if !args.is_empty() {
                    tail.push("--".into());
                    tail.extend(args.iter().cloned());
                }
            }
            Target::Pid(pid) => {
                out.push("pid".into());
                out.push(pid.to_string().into());
            }
        }
        for dll in &self.dlls {
            out.push("--dll".into());
            out.push(dll.clone().into_os_string());
        }
        out.extend(tail);
        out
    }
}

/// Parse the helper's command line (after the program name).
pub fn parse_args(args: impl IntoIterator<Item = OsString>) -> Result<Request, String> {
    let mut it = args.into_iter();
    let mode = it.next().ok_or("expected `spawn` or `pid`")?;
    let first = it.next().ok_or("missing the target after the mode")?;
    let mut target = match mode.to_str() {
        Some("spawn") => Target::Spawn {
            exe: PathBuf::from(first),
            cwd: None,
            args: Vec::new(),
        },
        Some("pid") => {
            let pid = first
                .to_str()
                .and_then(|s| s.parse::<u32>().ok())
                .ok_or("`pid` needs a numeric process id")?;
            Target::Pid(pid)
        }
        _ => return Err(format!("unknown mode {mode:?}; expected `spawn` or `pid`")),
    };
    let mut dlls = Vec::new();
    while let Some(flag) = it.next() {
        match flag.to_str() {
            Some("--dll") => dlls.push(PathBuf::from(it.next().ok_or("--dll needs a path")?)),
            Some("--cwd") => {
                let dir = PathBuf::from(it.next().ok_or("--cwd needs a directory")?);
                match &mut target {
                    Target::Spawn { cwd, .. } => *cwd = Some(dir),
                    Target::Pid(_) => return Err("--cwd applies only to `spawn`".into()),
                }
            }
            Some("--") => match &mut target {
                Target::Spawn { args, .. } => args.extend(it.by_ref()),
                Target::Pid(_) => return Err("`--` arguments apply only to `spawn`".into()),
            },
            _ => return Err(format!("unknown argument {flag:?}")),
        }
    }
    if dlls.is_empty() {
        return Err("at least one --dll is required".into());
    }
    Ok(Request { target, dlls })
}

/// Why the helper failed. The `name` is the stable, machine-readable
/// `kind=` value on stdout.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorKind {
    Usage,
    /// The program to spawn, or a DLL, does not exist.
    NotFound,
    Spawn,
    OpenProcess,
    /// Injector and target differ in bitness: the helper was pointed at a
    /// 64-bit program.
    BitnessMismatch,
    /// The remote `LoadLibraryW` returned NULL: the DLL failed to load or
    /// its `DllMain` returned FALSE.
    RemoteLoadFailed,
    Inject,
    Resume,
    /// This build has no helper (not Windows).
    Unsupported,
}

impl ErrorKind {
    pub fn name(self) -> &'static str {
        match self {
            Self::Usage => "usage",
            Self::NotFound => "not_found",
            Self::Spawn => "spawn",
            Self::OpenProcess => "open_process",
            Self::BitnessMismatch => "bitness_mismatch",
            Self::RemoteLoadFailed => "remote_load_failed",
            Self::Inject => "inject",
            Self::Resume => "resume",
            Self::Unsupported => "unsupported",
        }
    }

    fn from_name(name: &str) -> Option<Self> {
        [
            Self::Usage,
            Self::NotFound,
            Self::Spawn,
            Self::OpenProcess,
            Self::BitnessMismatch,
            Self::RemoteLoadFailed,
            Self::Inject,
            Self::Resume,
            Self::Unsupported,
        ]
        .into_iter()
        .find(|k| k.name() == name)
    }

    /// The helper's exit code for this failure.
    pub fn exit_code(self) -> i32 {
        match self {
            Self::Usage => 2,
            _ => 1,
        }
    }
}

/// The helper's result, one stdout line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Outcome {
    Started { pid: u32 },
    Failed { kind: ErrorKind, detail: String },
}

impl Outcome {
    /// The stdout line. The detail is flattened to one line so a caller
    /// can always read exactly one.
    pub fn format(&self) -> String {
        match self {
            Self::Started { pid } => format!("ok pid={pid}"),
            Self::Failed { kind, detail } => {
                let one_line: String = detail
                    .chars()
                    .map(|c| if c.is_control() { ' ' } else { c })
                    .collect();
                format!("error kind={} detail={one_line}", kind.name())
            }
        }
    }

    pub fn exit_code(&self) -> i32 {
        match self {
            Self::Started { .. } => 0,
            Self::Failed { kind, .. } => kind.exit_code(),
        }
    }
}

/// Parse the helper's stdout. `None` when it is not a line this contract
/// defines, so a caller can report the raw text.
pub fn parse_outcome(stdout: &str) -> Option<Outcome> {
    let line = stdout.lines().find(|l| !l.trim().is_empty())?.trim_end();
    if let Some(rest) = line.strip_prefix("ok pid=") {
        return rest.parse().ok().map(|pid| Outcome::Started { pid });
    }
    let rest = line.strip_prefix("error kind=")?;
    let (kind, detail) = match rest.split_once(" detail=") {
        Some((k, d)) => (k, d),
        None => (rest, ""),
    };
    Some(Outcome::Failed {
        kind: ErrorKind::from_name(kind)?,
        detail: detail.to_string(),
    })
}

/// What went wrong running the helper, for a caller.
#[derive(Debug, thiserror::Error)]
pub enum HelperError {
    #[error("could not run {HELPER_EXE_NAME}: {0}")]
    Run(#[from] std::io::Error),
    #[error("{HELPER_EXE_NAME} failed ({}): {detail}", .kind.name())]
    Failed { kind: ErrorKind, detail: String },
    #[error("{HELPER_EXE_NAME} gave an unreadable answer (exit {code:?}): {stdout:?}")]
    Garbled { code: Option<i32>, stdout: String },
}

/// Run the helper at `helper` for `request`, and return the target's pid.
/// Blocks until the helper exits, which it does as soon as the target is
/// resumed (or the request failed); it never waits on the target.
///
/// Reads only the helper's answer line, then waits for the helper to
/// exit, rather than reading stdout to EOF (`Command::output`). EOF comes
/// only when every copy of the pipe's write end is closed, and a target
/// that ever held a copy (a console program given the helper's std
/// handles, issue #1064) would keep `run` blocked for its whole life. The
/// helper already starts its target with no std handles (see
/// [`crate::process::create_process_suspended_with_args`]); this is the
/// second guard.
pub fn run(helper: &Path, request: &Request) -> Result<u32, HelperError> {
    let mut cmd = std::process::Command::new(helper);
    cmd.args(request.to_args())
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        // No console window flashes up for the helper.
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        cmd.creation_flags(CREATE_NO_WINDOW);
    }
    let mut child = cmd.spawn()?;
    let stdout = child
        .stdout
        .take()
        .expect("stdout was configured as a pipe");
    let answer = read_answer_line(&mut std::io::BufReader::new(stdout));
    // Wait even when the read failed, so the helper is always reaped.
    let status = child.wait()?;
    let stdout = answer?;
    match parse_outcome(&stdout) {
        Some(Outcome::Started { pid }) if status.success() => Ok(pid),
        Some(Outcome::Failed { kind, detail }) => Err(HelperError::Failed { kind, detail }),
        _ => Err(HelperError::Garbled {
            code: status.code(),
            stdout,
        }),
    }
}

/// The helper's answer: its first non-blank stdout line (the contract is
/// exactly one line), or an empty string at EOF. Stops there, never
/// reading on to EOF.
fn read_answer_line(reader: &mut impl std::io::BufRead) -> std::io::Result<String> {
    let mut line = Vec::new();
    loop {
        line.clear();
        // Bytes, not `read_line`: a path in the detail need not be UTF-8.
        let n = reader.read_until(b'\n', &mut line)?;
        let text = String::from_utf8_lossy(&line);
        if n == 0 || !text.trim().is_empty() {
            return Ok(text.into_owned());
        }
    }
}

/// Carry out `request` in this process: the helper's `main`. The helper
/// must be built for the target's bitness (i686 for `SGW.exe`).
#[cfg(windows)]
pub fn execute(request: &Request) -> Outcome {
    use crate::inject::{inject_dll, InjectError, OpenedProcess};

    let failed = |kind: ErrorKind, detail: String| Outcome::Failed { kind, detail };
    let inject_failed = |e: InjectError| {
        let kind = match &e {
            InjectError::DllMissing(_) => ErrorKind::NotFound,
            InjectError::BitnessMismatch { .. } => ErrorKind::BitnessMismatch,
            InjectError::RemoteLoadFailed => ErrorKind::RemoteLoadFailed,
            _ => ErrorKind::Inject,
        };
        failed(kind, e.to_string())
    };

    match &request.target {
        Target::Spawn { exe, cwd, args } => {
            if !exe.is_file() {
                return failed(
                    ErrorKind::NotFound,
                    format!("{} does not exist", exe.display()),
                );
            }
            let dir = cwd.clone().or_else(|| exe.parent().map(Path::to_path_buf));
            let suspended =
                match crate::process::create_process_suspended_with_args(exe, args, dir.as_deref())
                {
                    Ok(s) => s,
                    Err(e) => return failed(ErrorKind::Spawn, e.to_string()),
                };
            for dll in &request.dlls {
                if let Err(e) = inject_dll(suspended.process_handle(), dll) {
                    suspended.terminate();
                    return inject_failed(e);
                }
            }
            let pid = suspended.pid();
            match suspended.resume() {
                Ok(_) => Outcome::Started { pid },
                Err(e) => failed(ErrorKind::Resume, e.to_string()),
            }
        }
        Target::Pid(pid) => {
            let process = match OpenedProcess::for_inject(*pid) {
                Ok(p) => p,
                Err(e) => return failed(ErrorKind::OpenProcess, e.to_string()),
            };
            for dll in &request.dlls {
                if let Err(e) = inject_dll(process.process_handle(), dll) {
                    return inject_failed(e);
                }
            }
            Outcome::Started { pid: *pid }
        }
    }
}

#[cfg(test)]
mod tests;
