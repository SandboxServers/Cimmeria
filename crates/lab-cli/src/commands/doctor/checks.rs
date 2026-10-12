//! The eleven `lab doctor` checks: ten ported from the removed PowerShell doctor command, plus the build-stamp match.
//! Every check is a pure function of [`Inputs`], which `system.rs` gathers.

use std::path::{Path, PathBuf};

use crate::daemon::status::StatusReply;

/// How a check came out.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Status {
    /// The check is satisfied.
    Pass,
    /// Something is off but nothing is broken; does not fail the command.
    Warn,
    /// Broken; makes the command exit 1.
    Fail,
}

impl Status {
    /// The word plain mode prints.
    pub fn label(self) -> &'static str {
        match self {
            Status::Pass => "PASS",
            Status::Warn => "WARN",
            Status::Fail => "FAIL",
        }
    }
}

/// One check's result.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Check {
    /// How the check came out.
    pub status: Status,
    /// The check's name, as the PowerShell printed it.
    pub name: String,
    /// What the check found.
    pub detail: String,
    /// What to do about it, e.g. `run lab install`; plain mode appends it to
    /// the detail after `; `, fancy mode prints it as its own `hint:` line.
    pub hint: Option<String>,
    /// The command to run, on its own `hint:` line in fancy mode only (plain
    /// mode keeps `hint`, as the PowerShell printed it): `run lab install`
    /// becomes `lab install`. A check whose plain text has no hint can still
    /// carry one here.
    pub fancy_hint: Option<String>,
}

impl Check {
    fn new(status: Status, name: &str, detail: impl Into<String>, hint: Option<&str>) -> Self {
        Check {
            status,
            name: name.to_string(),
            detail: detail.into(),
            hint: hint.map(str::to_string),
            fancy_hint: hint.map(command_of),
        }
    }

    /// Sets the fancy-mode hint of a check whose plain text has none.
    fn with_fancy_hint(mut self, command: &str) -> Self {
        self.fancy_hint = Some(command.to_string());
        self
    }

    /// The detail as the PowerShell printed it: `detail; hint`.
    pub fn plain_detail(&self) -> String {
        match &self.hint {
            Some(hint) => format!("{}; {hint}", self.detail),
            None => self.detail.clone(),
        }
    }
}

/// The command a hint names: `run lab install` is `lab install`, and
/// `git fetch, or run lab setup` is `git fetch, or lab setup`.
fn command_of(hint: &str) -> String {
    hint.replace(", or run ", ", or ")
        .trim_start_matches("run ")
        .to_string()
}

/// What the gathering found about one instance.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstanceFacts {
    pub label: String,
    /// The instance's account file exists under the install dir.
    pub account_file: bool,
    /// The instance's profile folder is seeded.
    pub seeded: bool,
}

/// How the installed exe and the daemon's copy compare.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BinaryFacts {
    /// This file (the first of the two that is missing or unreadable).
    Missing(PathBuf),
    /// Both exist and their bytes differ.
    Differ,
    /// Both exist and are identical; the length in bytes.
    Same(u64),
}

/// `git diff --quiet <sha> origin/main -- tools/lab` came out as:
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContentCheck {
    /// Exit 0: no difference.
    Same,
    /// Exit 1: tools/lab differs.
    Differs,
    /// Any other exit (an unknown sha, no origin/main).
    Unknown,
}

/// Everything read from the system, so the checks stay pure.
#[derive(Debug, Clone)]
pub struct Inputs {
    /// `schtasks /Query /TN CimmeriaLabDaemon` exited 0.
    pub task_installed: bool,
    /// `GET /status`, or `None` when the daemon is down or refuses.
    pub status: Option<StatusReply>,
    /// The daemon token is set (its value is never read into `Inputs`).
    pub token_set: bool,
    /// `CIMMERIA_LAB_INSTALL_DIR` from labd.env, trimmed; `None` when blank.
    pub install_dir: Option<String>,
    /// `<install dir>\Binaries\SGW.exe` exists.
    pub sgw_exe_exists: bool,
    /// The profile root as text.
    pub profile_root: String,
    /// One entry per instance the checks cover.
    pub instances: Vec<InstanceFacts>,
    /// Pids of the running `SGW.exe` processes.
    pub running_sgw: Vec<u32>,
    /// `client_pid` of every instance `/status` lists.
    pub listed_sgw: Vec<u32>,
    pub binaries: BinaryFacts,
    /// The first line of `<lab home>\cli\cli\VERSION`, trimmed; `None` when absent or blank.
    pub version_sha: Option<String>,
    /// The git sha stamped into this `lab.exe` at build time
    /// (`CIMMERIA_LAB_CLI_SHA`); `None` for a dev build with no stamp.
    pub exe_sha: Option<String>,
    /// The Cimmeria checkout the command runs in, if any.
    pub checkout: Option<PathBuf>,
    /// The tools/lab content comparison; `None` when it was skipped (no
    /// checkout or no `version_sha`).
    pub content: Option<ContentCheck>,
}

/// `<install dir>\Binaries\SGW.exe`.
pub fn sgw_exe_path(install_dir: &str) -> PathBuf {
    Path::new(install_dir).join("Binaries").join("SGW.exe")
}

/// All eleven checks: the PowerShell's ten in its order, then the build match.
pub fn run_all(i: &Inputs) -> Vec<Check> {
    vec![
        task_check(i),
        daemon_check(i),
        token_check(i),
        install_check(i),
        profile_root_check(i),
        account_check(i),
        seed_check(i),
        stray_sgw_check(i),
        binary_check(i),
        version_check(i),
        build_match_check(i),
    ]
}

/// `Get-TaskCheck`.
pub fn task_check(i: &Inputs) -> Check {
    let name = "scheduled task CimmeriaLabDaemon exists";
    if i.task_installed {
        Check::new(Status::Pass, name, "installed", None)
    } else {
        Check::new(Status::Fail, name, "not installed", Some("run lab install"))
    }
}

/// `Get-DaemonCheck`.
pub fn daemon_check(i: &Inputs) -> Check {
    let name = "daemon answers /status";
    match &i.status {
        None => Check::new(
            Status::Fail,
            name,
            "not running, or the status endpoint is down",
            Some("run lab start"),
        ),
        Some(status) => Check::new(
            Status::Pass,
            name,
            format!(
                "pid {}, version {}",
                status.daemon.pid, status.daemon.version
            ),
            None,
        ),
    }
}

/// `Get-TokenCheck`. The token itself is never reported.
pub fn token_check(i: &Inputs) -> Check {
    let name = "CIMMERIA_LAB_DAEMON_TOKEN is set";
    if i.token_set {
        Check::new(Status::Pass, name, "set", None)
    } else {
        Check::new(Status::Fail, name, "not set", Some("run lab install"))
    }
}

/// `Get-InstallCheck`.
pub fn install_check(i: &Inputs) -> Check {
    let name = "labd.env has CIMMERIA_LAB_INSTALL_DIR and SGW.exe is under it";
    let Some(dir) = i.install_dir.as_deref() else {
        return Check::new(
            Status::Fail,
            name,
            "CIMMERIA_LAB_INSTALL_DIR is not set in labd.env",
            None,
        );
    };
    let exe = sgw_exe_path(dir);
    if !i.sgw_exe_exists {
        return Check::new(Status::Fail, name, format!("no {}", exe.display()), None);
    }
    Check::new(Status::Pass, name, exe.display().to_string(), None)
}

/// `Get-ProfileRootCheck`: the root must be absolute and outside the install
/// dir. A path that only shares a name prefix with it (`C:\Games\SGWX` against
/// `C:\Games\SGW`) is outside.
pub fn profile_root_check(i: &Inputs) -> Check {
    let name = "profile root is absolute and not inside the install dir";
    let root = i.profile_root.as_str();
    if root.is_empty() || !is_fully_qualified(root) {
        return Check::new(
            Status::Fail,
            name,
            format!("not an absolute path: {root}"),
            None,
        );
    }
    let full = full_path(root);
    if let Some(dir) = i.install_dir.as_deref() {
        let install = full_path_lossy(dir);
        let under = format!("{install}\\");
        if full.eq_ignore_ascii_case(&install) || starts_with_ignore_case(&full, &under) {
            return Check::new(
                Status::Fail,
                name,
                format!("inside the install dir: {full}"),
                None,
            );
        }
    }
    Check::new(Status::Pass, name, full, None)
}

/// `Get-AccountCheck`.
pub fn account_check(i: &Inputs) -> Check {
    let name = "each instance's account file exists";
    if i.install_dir.is_none() {
        return Check::new(
            Status::Warn,
            name,
            "no install dir; account files not checked",
            None,
        );
    }
    let missing: Vec<&str> = i
        .instances
        .iter()
        .filter(|f| !f.account_file)
        .map(|f| f.label.as_str())
        .collect();
    if !missing.is_empty() {
        return Check::new(
            Status::Warn,
            name,
            format!("missing: {}", missing.join(", ")),
            Some("run lab instances init"),
        );
    }
    Check::new(
        Status::Pass,
        name,
        format!("{} present", i.instances.len()),
        None,
    )
}

/// `Get-SeedCheck`.
pub fn seed_check(i: &Inputs) -> Check {
    let name = "each instance's profile is seeded";
    let unseeded: Vec<&str> = i
        .instances
        .iter()
        .filter(|f| !f.seeded)
        .map(|f| f.label.as_str())
        .collect();
    if !unseeded.is_empty() {
        return Check::new(
            Status::Warn,
            name,
            format!("not seeded: {}", unseeded.join(", ")),
            None,
        )
        .with_fancy_hint("lab instances init");
    }
    Check::new(
        Status::Pass,
        name,
        format!("{} seeded", i.instances.len()),
        None,
    )
}

/// `Get-StraySgwCheck`: running `SGW.exe` pids that `/status` does not list.
pub fn stray_sgw_check(i: &Inputs) -> Check {
    let name = "no SGW.exe runs that /status does not list";
    let stray: Vec<String> = i
        .running_sgw
        .iter()
        .filter(|pid| !i.listed_sgw.contains(pid))
        .map(|pid| pid.to_string())
        .collect();
    if !stray.is_empty() && i.status.is_none() {
        return Check::new(
            Status::Warn,
            name,
            format!(
                "daemon down; running SGW pid {} cannot be matched to an instance",
                stray.join(", ")
            ),
            None,
        );
    }
    if !stray.is_empty() {
        return Check::new(
            Status::Warn,
            name,
            format!("not listed by /status: pid {}", stray.join(", ")),
            None,
        );
    }
    Check::new(Status::Pass, name, "none", None)
}

/// `Get-BinaryCheck`: the installed exe and the daemon's copy are one file.
pub fn binary_check(i: &Inputs) -> Check {
    let name = "installed cimmeria-lab.exe is the daemon's copy";
    match &i.binaries {
        BinaryFacts::Missing(path) => Check::new(
            Status::Warn,
            name,
            format!("missing: {}", path.display()),
            Some("run lab install"),
        ),
        BinaryFacts::Differ => Check::new(
            Status::Warn,
            name,
            "bin and labd copies differ",
            Some("run lab restart"),
        ),
        BinaryFacts::Same(len) => Check::new(
            Status::Pass,
            name,
            format!("same bytes ({len} bytes)"),
            None,
        ),
    }
}

/// `Get-VersionCheck`: the CLI copy has origin/main's tools/lab (content, not
/// ancestry: PRs are squash-merged, so an installed worktree sha never
/// becomes an ancestor of origin/main).
pub fn version_check(i: &Inputs) -> Check {
    let name = "CLI copy matches origin/main (as of the last fetch)";
    let Some(root) = i.checkout.as_deref() else {
        return Check::new(
            Status::Warn,
            name,
            "not in a Cimmeria checkout; skipped",
            None,
        );
    };
    let Some(sha) = i.version_sha.as_deref() else {
        return Check::new(
            Status::Warn,
            name,
            "no cli\\cli\\VERSION",
            Some("run lab setup"),
        );
    };
    match i.content {
        Some(ContentCheck::Same) => Check::new(
            Status::Pass,
            name,
            format!("{sha} has origin/main's tools/lab"),
            None,
        ),
        Some(ContentCheck::Differs) => Check::new(
            Status::Warn,
            name,
            format!("{sha} differs from origin/main in tools/lab"),
            Some("run lab setup"),
        ),
        Some(ContentCheck::Unknown) | None => Check::new(
            Status::Warn,
            name,
            format!("{sha} is unknown in {}", root.display()),
            Some("git fetch, or run lab setup"),
        ),
    }
}

/// This `lab.exe` and the installed CLI scripts (`cli\cli\VERSION`) were built
/// from the same commit. A dev build has no stamp, so nothing can be compared.
pub fn build_match_check(i: &Inputs) -> Check {
    let name = "lab.exe matches the installed CLI scripts";
    let Some(exe) = i.exe_sha.as_deref() else {
        return Check::new(
            Status::Warn,
            name,
            "lab.exe has no build stamp (dev build)",
            None,
        );
    };
    let Some(scripts) = i.version_sha.as_deref() else {
        return Check::new(
            Status::Warn,
            name,
            format!("lab.exe is build {exe} but cli\\cli\\VERSION is missing"),
            Some("run lab install"),
        );
    };
    if same_build(exe, scripts) {
        return Check::new(Status::Pass, name, format!("both are build {exe}"), None);
    }
    Check::new(
        Status::Warn,
        name,
        "lab.exe and the CLI scripts come from different builds",
        Some("run lab install"),
    )
}

/// Whether two git shas name one commit: one is a prefix of the other (the
/// stamp is a short sha), ignoring case, and at least 4 characters long.
fn same_build(a: &str, b: &str) -> bool {
    let (a, b) = (a.trim(), b.trim());
    let n = a.len().min(b.len());
    n >= 4 && a.is_char_boundary(n) && b.is_char_boundary(n) && a[..n].eq_ignore_ascii_case(&b[..n])
}

/// .NET's `Path.IsPathFullyQualified` for Windows paths: `X:\` or `X:/`, or a
/// leading pair of separators (UNC and `\\?\`). Text-only, so it behaves the
/// same on every platform.
fn is_fully_qualified(path: &str) -> bool {
    let b = path.as_bytes();
    let sep = |c: u8| c == b'\\' || c == b'/';
    if b.len() >= 2 && sep(b[0]) && sep(b[1]) {
        return true;
    }
    b.len() >= 3 && b[0].is_ascii_alphabetic() && b[1] == b':' && sep(b[2])
}

/// `Path.GetFullPath(...).TrimEnd('\')` for a fully qualified path: `/`
/// becomes `\`, `.` and empty segments go, `..` removes the segment before it.
fn full_path(path: &str) -> String {
    let s = path.replace('/', "\\");
    let (prefix, rest) = (s[..2].to_string(), &s[2..]);
    let mut parts: Vec<&str> = Vec::new();
    for segment in rest.split('\\') {
        match segment {
            "" | "." => {}
            ".." => {
                parts.pop();
            }
            other => parts.push(other),
        }
    }
    let joined = parts.join("\\");
    let full = if prefix == "\\\\" {
        format!("{prefix}{joined}")
    } else {
        format!("{prefix}\\{joined}")
    };
    full.trim_end_matches('\\').to_string()
}

/// [`full_path`] for a fully qualified path; for anything else the text with
/// `/` turned into `\` and no trailing `\`.
fn full_path_lossy(path: &str) -> String {
    if is_fully_qualified(path) {
        full_path(path)
    } else {
        path.replace('/', "\\").trim_end_matches('\\').to_string()
    }
}

fn starts_with_ignore_case(text: &str, prefix: &str) -> bool {
    text.len() >= prefix.len()
        && text.as_bytes()[..prefix.len()].eq_ignore_ascii_case(prefix.as_bytes())
}
