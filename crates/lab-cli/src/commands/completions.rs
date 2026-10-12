//! `lab completions <shell>`: prints a shell completion script, and with
//! `--install` adds the PowerShell one to your profile.

use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process::{Command as Process, Stdio};

use clap::{CommandFactory, ValueEnum};
use clap_complete::Shell;

use crate::commands::{Cli, Ctx};

/// The call the profile line runs; a line that holds it is ours to replace.
const PROFILE_CALL: &str = "completions powershell | Out-String | Invoke-Expression";

const PROFILE_COMMENT: &str =
    "# lab command-line completion (added by: lab completions powershell --install)";

const LONG: &str = concat!(
    "Prints a completion script for a shell to stdout: powershell, bash, zsh or fish. Load it the way your shell loads completions; for PowerShell, `lab completions powershell | Out-String | Invoke-Expression`.",
    "\n\n",
    "`lab completions powershell --install` adds a line to $PROFILE.CurrentUserAllHosts once. The line runs this lab.exe by its full path and only while that file exists, so a profile never errors when lab has moved. It looks for the line first, so running it again changes nothing; a line an earlier version wrote is replaced. It says what it did. Open a new terminal afterwards.",
);

const EXAMPLES: &str =
    "Examples:\n  lab completions powershell\n  lab completions powershell --install";

/// A shell `lab completions` can print a script for.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum ShellName {
    /// PowerShell (pwsh and Windows PowerShell)
    Powershell,
    /// Bash
    Bash,
    /// Zsh
    Zsh,
    /// Fish
    Fish,
}

impl ShellName {
    fn shell(self) -> Shell {
        match self {
            ShellName::Powershell => Shell::PowerShell,
            ShellName::Bash => Shell::Bash,
            ShellName::Zsh => Shell::Zsh,
            ShellName::Fish => Shell::Fish,
        }
    }
}

// Arguments of `lab completions`.
#[derive(clap::Args, Debug)]
#[command(
    about = "Print a shell completion script",
    long_about = LONG,
    after_help = EXAMPLES
)]
pub struct Args {
    /// The shell to print the script for
    #[arg(value_enum, ignore_case = true, value_name = "SHELL")]
    pub shell: ShellName,
    /// Add the PowerShell completion line to your profile, once (powershell only)
    #[arg(long)]
    pub install: bool,
    /// The profile file to change (a test seam; default: pwsh's CurrentUserAllHosts)
    #[arg(long, hide = true, requires = "install", value_name = "PATH")]
    pub profile_path: Option<PathBuf>,
}

/// The completion script for `shell`.
pub fn script(shell: ShellName) -> String {
    let mut buf = Vec::new();
    clap_complete::generate(shell.shell(), &mut Cli::command(), "lab", &mut buf);
    String::from_utf8_lossy(&buf).into_owned()
}

/// What `--install` did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Installed {
    /// The line was appended.
    Added,
    /// An older line that runs the completion call was replaced.
    Updated,
    /// The exact line was already there.
    AlreadyThere,
}

/// The guarded profile line for the lab.exe at `exe`. It runs only while that
/// file exists, so a profile never errors when lab has moved or is gone. Every
/// quote character in the path is doubled, as a PowerShell single-quoted
/// string needs: `'` and the typographic quotes `‘ ’ ‚ ‛`, which PowerShell
/// reads as quotes too.
pub fn profile_line(exe: &Path) -> String {
    let mut quoted = String::new();
    for c in exe.display().to_string().chars() {
        quoted.push(c);
        if matches!(c, '\'' | '\u{2018}' | '\u{2019}' | '\u{201a}' | '\u{201b}') {
            quoted.push(c);
        }
    }
    format!("if (Test-Path -LiteralPath '{quoted}') {{ & '{quoted}' {PROFILE_CALL} }}")
}

/// The canonical path of the running lab.exe. The `\\?\` prefix that Windows
/// adds to canonical paths is dropped, so the profile line reads plainly.
pub fn lab_exe_path() -> io::Result<PathBuf> {
    let exe = std::env::current_exe()?;
    let canonical = std::fs::canonicalize(&exe).unwrap_or(exe);
    let text = canonical.to_string_lossy().into_owned();
    match text.strip_prefix(r"\\?\") {
        Some(plain) if !plain.starts_with(r"UNC\") => Ok(PathBuf::from(plain)),
        _ => Ok(canonical),
    }
}

/// Whether a line is exactly one of the forms `--install` has written, with
/// only whitespace around it: the old unguarded `lab <call>`, or the guarded
/// line for some lab.exe path (this one or an older one). A line that holds
/// more than that, such as one with a trailing comment, is the user's own.
fn is_profile_call(line: &str) -> bool {
    // A byte order mark at the start of the file is not part of the line.
    let line = line.trim_start_matches('\u{feff}').trim();
    if line.strip_prefix("lab ") == Some(PROFILE_CALL) {
        return true;
    }
    let Some(rest) = line.strip_prefix("if (Test-Path -LiteralPath '") else {
        return false;
    };
    let Some(rest) = rest.strip_suffix(&format!("' {PROFILE_CALL} }}")) else {
        return false;
    };
    match rest.split_once("') { & '") {
        Some((tested, called)) => !tested.is_empty() && tested == called,
        None => false,
    }
}

/// Puts [`profile_line`] for `exe` into the profile at `path`:
/// - if the exact line is already there, nothing changes ([`Installed::AlreadyThere`]);
/// - if a line that runs the lab completion call is there (an older form), the
///   first one is replaced and any others are dropped ([`Installed::Updated`]);
/// - otherwise the line is appended after a comment ([`Installed::Added`]).
///
/// Creates the folder and the file when they are missing, keeps what is there,
/// and uses the file's own line ending.
pub fn install_into(path: &Path, exe: &Path) -> io::Result<Installed> {
    let existing = match std::fs::read_to_string(path) {
        Ok(text) => text,
        Err(err) if err.kind() == io::ErrorKind::NotFound => String::new(),
        Err(err) => return Err(err),
    };
    let line = profile_line(exe);
    if existing
        .lines()
        .any(|known| known.trim_start_matches('\u{feff}').trim() == line)
    {
        return Ok(Installed::AlreadyThere);
    }
    if existing.lines().any(is_profile_call) {
        let mut text = String::new();
        let mut replaced = false;
        // Each segment keeps its own line ending; the replacement takes the
        // ending of the line it replaces.
        for segment in existing.split_inclusive('\n') {
            let content = segment.trim_end_matches(['\r', '\n']);
            if is_profile_call(content) {
                if !replaced {
                    if content.starts_with('\u{feff}') {
                        text.push('\u{feff}');
                    }
                    text.push_str(&line);
                    text.push_str(&segment[content.len()..]);
                    replaced = true;
                }
            } else {
                text.push_str(segment);
            }
        }
        std::fs::write(path, text)?;
        return Ok(Installed::Updated);
    }
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let newline = if existing.contains("\r\n") || (existing.is_empty() && cfg!(windows)) {
        "\r\n"
    } else {
        "\n"
    };
    let mut text = String::new();
    if !existing.is_empty() && !existing.ends_with('\n') {
        text.push_str(newline);
    }
    text.push_str(&format!("{PROFILE_COMMENT}{newline}{line}{newline}"));
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)?;
    file.write_all(text.as_bytes())?;
    Ok(Installed::Added)
}

/// `$PROFILE.CurrentUserAllHosts`, asked of `pwsh`.
pub fn pwsh_profile_path() -> Result<PathBuf, String> {
    let output = Process::new(crate::fallback::pwsh_program())
        .args([
            "-NoProfile",
            "-NonInteractive",
            "-Command",
            "$PROFILE.CurrentUserAllHosts",
        ])
        .stdin(Stdio::null())
        .stderr(Stdio::null())
        .output()
        .map_err(|err| format!("could not start pwsh: {err}"))?;
    let text = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if !output.status.success() || text.is_empty() {
        return Err("pwsh did not report its profile path".to_string());
    }
    Ok(PathBuf::from(text))
}

/// Runs `lab completions`: 0, or 1 when the script or the profile cannot be
/// written, or 2 for `--install` with a shell other than powershell.
pub fn run(args: &Args, ctx: &Ctx) -> u8 {
    if args.install {
        return install(args, ctx);
    }
    let mut out = io::stdout().lock();
    match out
        .write_all(script(args.shell).as_bytes())
        .and_then(|()| out.flush())
    {
        Ok(()) => 0,
        Err(err) if err.kind() == io::ErrorKind::BrokenPipe => 0,
        Err(err) => {
            ctx.ui
                .error(&format!("cannot write the script: {err}"), None);
            1
        }
    }
}

fn install(args: &Args, ctx: &Ctx) -> u8 {
    if args.shell != ShellName::Powershell {
        ctx.ui.error(
            "--install is for powershell only",
            Some("add the output of lab completions <shell> to your shell's startup file yourself"),
        );
        return 2;
    }
    let path = match &args.profile_path {
        Some(path) => path.clone(),
        None => match pwsh_profile_path() {
            Ok(path) => path,
            Err(message) => {
                ctx.ui.error(
                    &message,
                    Some("add this line to your PowerShell profile yourself: lab completions powershell | Out-String | Invoke-Expression"),
                );
                return 1;
            }
        },
    };
    let exe = match lab_exe_path() {
        Ok(exe) => exe,
        Err(err) => {
            ctx.ui
                .error(&format!("cannot find the path of lab.exe: {err}"), None);
            return 1;
        }
    };
    match install_into(&path, &exe) {
        Ok(Installed::Added) => {
            anstream::println!(
                "added the lab completion line to {}\nopen a new terminal to use it",
                path.display()
            );
            0
        }
        Ok(Installed::Updated) => {
            anstream::println!(
                "updated the lab completion line in {}\nopen a new terminal to use it",
                path.display()
            );
            0
        }
        Ok(Installed::AlreadyThere) => {
            anstream::println!("the lab completion line is already in {}", path.display());
            0
        }
        Err(err) => {
            ctx.ui
                .error(&format!("cannot update {}: {err}", path.display()), None);
            1
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::commands::{parse_ok, try_parse, Command};

    #[test]
    fn the_shell_is_required_and_case_does_not_matter() {
        assert!(matches!(
            parse_ok("completions", &["powershell"]),
            Command::Completions(_)
        ));
        assert!(matches!(
            parse_ok("completions", &["PowerShell"]),
            Command::Completions(_)
        ));
        assert!(try_parse("completions", &[]).is_err());
        assert!(try_parse("completions", &["nushell"]).is_err());
    }

    #[test]
    fn a_quote_in_the_exe_path_is_doubled_in_the_profile_line() {
        let line = super::profile_line(std::path::Path::new(r"C:\Dev's\lab.exe"));
        assert_eq!(
            line,
            r"if (Test-Path -LiteralPath 'C:\Dev''s\lab.exe') { & 'C:\Dev''s\lab.exe' completions powershell | Out-String | Invoke-Expression }"
        );
    }

    #[test]
    fn a_typographic_quote_in_the_exe_path_is_doubled_too() {
        let line = super::profile_line(std::path::Path::new("C:\\Dev\u{2019}s\\lab.exe"));
        assert!(
            line.contains("'C:\\Dev\u{2019}\u{2019}s\\lab.exe'"),
            "{line}"
        );
        assert!(super::is_profile_call(&line));
    }

    #[test]
    fn only_the_exact_old_forms_are_ours() {
        use super::{is_profile_call, profile_line};
        let call = "completions powershell | Out-String | Invoke-Expression";
        let guarded = profile_line(std::path::Path::new(r"D:\old dir\lab.exe"));
        // Ours: the unguarded form, a guarded form for any path, with whitespace
        // or a byte order mark around it.
        assert!(is_profile_call(&format!("lab {call}")));
        assert!(is_profile_call(&format!("  lab {call}  \t")));
        assert!(is_profile_call(&format!("\u{feff}lab {call}")));
        assert!(is_profile_call(&guarded));
        // The user's: a comment, a trailing comment, extra statements, a guard
        // that tests one file and runs another, a different command.
        assert!(!is_profile_call(&format!("# lab {call}")));
        assert!(!is_profile_call(&format!("lab {call}  # mine")));
        assert!(!is_profile_call(&format!("$x = 1; lab {call}")));
        assert!(!is_profile_call(&format!("lab {call}; Set-Alias a b")));
        assert!(!is_profile_call(&format!("{guarded} # mine")));
        assert!(!is_profile_call(&guarded.replace("& 'D:", "& 'E:")));
        assert!(!is_profile_call(&format!("labx {call}")));
    }

    #[test]
    fn the_profile_path_seam_needs_install() {
        assert!(try_parse("completions", &["powershell", "--profile-path", "p.ps1"]).is_err());
        assert!(try_parse(
            "completions",
            &["powershell", "--install", "--profile-path", "p.ps1"]
        )
        .is_ok());
    }
}
