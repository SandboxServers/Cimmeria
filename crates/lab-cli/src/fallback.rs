//! Running the PowerShell scripts that stay PowerShell: `daemon.ps1` behind
//! `lab start`, `stop` and `restart`, `instances.ps1` behind `lab instances`,
//! and `cli\install.ps1`, `cli\setup.ps1` and `cli\local-setup.ps1` behind
//! `lab install`, `setup` and `local-setup`. There is no generic dispatcher.

use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::Command as Process;

use crate::commands::Ctx;

/// The folder the scripts live in: `CIMMERIA_LAB_CLI_DIR` when set, else the
/// `cli` folder beside the executable's folder (`<lab home>\bin\..\cli`). In a
/// checkout that folder is `tools\lab`.
pub fn scripts_dir() -> Option<PathBuf> {
    match std::env::var_os("CIMMERIA_LAB_CLI_DIR") {
        Some(dir) => Some(PathBuf::from(dir)),
        None => Some(
            std::env::current_exe()
                .ok()?
                .parent()?
                .join("..")
                .join("cli"),
        ),
    }
}

/// `relative` (for example `daemon.ps1` or `cli/install.ps1`) under
/// [`scripts_dir`], when that file exists.
pub fn script_path(relative: &str) -> Option<PathBuf> {
    let path = scripts_dir()?.join(relative);
    path.is_file().then_some(path)
}

/// The `pwsh` arguments: `-NoProfile -File <script> <leading...>`, then the raw
/// tokens in PowerShell form (see [`to_powershell_tokens`]).
pub fn command_line(script: &Path, leading: &[&str], raw: &[OsString]) -> Vec<OsString> {
    let mut args = vec![
        OsString::from("-NoProfile"),
        OsString::from("-File"),
        script.as_os_str().to_owned(),
    ];
    args.extend(leading.iter().map(OsString::from));
    args.extend(to_powershell_tokens(raw));
    args
}

/// The tokens as PowerShell spells them: `--runs-per-lease` becomes
/// `-RunsPerLease`, `--json` becomes `-Json` and `--value=-x` becomes
/// `-Value:-x`. `--value=` (an empty value) becomes `-Value` and an empty
/// token, because `-Value:` alone would take the next token as its value.
/// Anything else stays as typed, and so does every token after a bare `--`
/// (the `--` itself too).
pub fn to_powershell_tokens(raw: &[OsString]) -> Vec<OsString> {
    let mut after_separator = false;
    let mut out = Vec::with_capacity(raw.len());
    for arg in raw {
        let converted = if after_separator {
            None
        } else if arg == "--" {
            after_separator = true;
            None
        } else {
            arg.to_str().and_then(powershell_token)
        };
        match converted {
            Some((flag, value)) => {
                out.push(OsString::from(flag));
                out.extend(value.map(OsString::from));
            }
            None => out.push(arg.clone()),
        }
    }
    out
}

/// The PowerShell form of one `--long` token (the flag and, for an empty
/// `--name=`, the empty value that follows it), or `None` when it is not one.
fn powershell_token(text: &str) -> Option<(String, Option<String>)> {
    let body = text.strip_prefix("--")?;
    let (name, value) = match body.split_once('=') {
        Some((name, value)) => (name, Some(value)),
        None => (body, None),
    };
    if !is_long_name(name) {
        return None;
    }
    let flag = pascal_case(name);
    Some(match value {
        Some("") => (format!("-{flag}"), Some(String::new())),
        Some(value) => (format!("-{flag}:{value}"), None),
        None => (format!("-{flag}"), None),
    })
}

/// `[a-z][a-z0-9-]*`: a kebab-case long flag name.
fn is_long_name(name: &str) -> bool {
    let mut chars = name.chars();
    chars.next().is_some_and(|c| c.is_ascii_lowercase())
        && chars.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
}

/// kebab-case to PascalCase: `runs-per-lease` becomes `RunsPerLease`.
fn pascal_case(name: &str) -> String {
    name.split('-')
        .map(|word| {
            let mut chars = word.chars();
            match chars.next() {
                Some(first) => first.to_ascii_uppercase().to_string() + chars.as_str(),
                None => String::new(),
            }
        })
        .collect()
}

/// Runs `pwsh -NoProfile -File <scripts dir>\<relative> <leading...> <ctx.raw
/// in PowerShell form>` with inherited stdio and returns the child's exit code
/// (255 when it has none). 1, with an error, when the script or `pwsh` is missing.
pub fn run_script(ctx: &Ctx, relative: &str, leading: &[&str]) -> u8 {
    let Some(script) = script_path(relative) else {
        ctx.ui.error(
            &format!("the PowerShell script {relative} is not installed"),
            Some("run: pwsh <checkout>\\tools\\lab\\cli\\setup.ps1"),
        );
        return 1;
    };
    match Process::new("pwsh")
        .args(command_line(&script, leading, &ctx.raw))
        .status()
    {
        Ok(status) => exit_byte(status.code()),
        Err(err) => {
            ctx.ui.error(
                &format!("could not start pwsh: {err}"),
                Some("install PowerShell 7 or put pwsh on PATH"),
            );
            1
        }
    }
}

/// The process exit code as one byte: passed through when it fits, else 255
/// (no code at all, a signal, or a value outside 0..=255).
pub fn exit_byte(code: Option<i32>) -> u8 {
    code.and_then(|code| u8::try_from(code).ok()).unwrap_or(255)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tokens(args: &[&str]) -> Vec<String> {
        to_powershell_tokens(&args.iter().map(OsString::from).collect::<Vec<_>>())
            .into_iter()
            .map(|t| t.to_string_lossy().into_owned())
            .collect()
    }

    #[test]
    fn command_line_is_script_then_leading_then_tokens() {
        let raw = vec![OsString::from("--force"), OsString::from("-Yes")];
        let args = command_line(Path::new("d\\daemon.ps1"), &["stop"], &raw);
        assert_eq!(
            args,
            [
                OsString::from("-NoProfile"),
                OsString::from("-File"),
                OsString::from("d\\daemon.ps1"),
                OsString::from("stop"),
                OsString::from("-Force"),
                OsString::from("-Yes"),
            ]
        );
    }

    #[test]
    fn long_flags_become_pascal_case() {
        assert_eq!(tokens(&["--runs-per-lease"]), ["-RunsPerLease"]);
        assert_eq!(tokens(&["--plan-only"]), ["-PlanOnly"]);
        assert_eq!(tokens(&["--json"]), ["-Json"]);
    }

    #[test]
    fn equals_form_becomes_colon_form() {
        assert_eq!(tokens(&["--value=-x"]), ["-Value:-x"]);
        assert_eq!(tokens(&["--out=C:\\a=b"]), ["-Out:C:\\a=b"]);
    }

    #[test]
    fn everything_else_is_unchanged() {
        assert_eq!(tokens(&["-Rows"]), ["-Rows"]);
        assert_eq!(tokens(&["-h"]), ["-h"]);
        assert_eq!(tokens(&["FS-01,FS-02"]), ["FS-01,FS-02"]);
        assert_eq!(tokens(&["C:\\x"]), ["C:\\x"]);
        assert_eq!(tokens(&["-5"]), ["-5"]);
        assert_eq!(tokens(&["-"]), ["-"]);
        assert_eq!(tokens(&["---json"]), ["---json"]);
        assert_eq!(tokens(&["--Json"]), ["--Json"]);
    }

    #[test]
    fn a_token_after_the_separator_stays() {
        assert_eq!(
            tokens(&["uat", "--", "--rows", "-h"]),
            ["uat", "--", "--rows", "-h"]
        );
        assert_eq!(
            tokens(&["--rows", "--", "--json"]),
            ["-Rows", "--", "--json"]
        );
    }

    #[test]
    fn an_empty_value_is_its_own_token() {
        // `-Out:` alone would take the next token as its value.
        assert_eq!(tokens(&["--out=", "--json"]), ["-Out", "", "-Json"]);
        assert_eq!(tokens(&["--out=", "x"]), ["-Out", "", "x"]);
    }

    #[test]
    fn exit_codes_pass_through_or_become_255() {
        assert_eq!(exit_byte(Some(0)), 0);
        assert_eq!(exit_byte(Some(2)), 2);
        assert_eq!(exit_byte(Some(130)), 130);
        assert_eq!(exit_byte(Some(255)), 255);
        assert_eq!(exit_byte(Some(256)), 255);
        assert_eq!(exit_byte(Some(-1)), 255);
        assert_eq!(exit_byte(None), 255);
    }
}
