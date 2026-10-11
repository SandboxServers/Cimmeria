//! Running a command through the PowerShell dispatcher (`lab.ps1`) until its
//! packet moves it into Rust.

use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::Command as Process;

use crate::commands::Ctx;

/// The folder that holds `lab.ps1`: `CIMMERIA_LAB_CLI_DIR` when set, else the
/// `cli` folder beside the executable's folder. `None` when it has no `lab.ps1`.
pub fn dispatcher_path() -> Option<PathBuf> {
    let dir = match std::env::var_os("CIMMERIA_LAB_CLI_DIR") {
        Some(dir) => PathBuf::from(dir),
        None => std::env::current_exe()
            .ok()?
            .parent()?
            .join("..")
            .join("cli"),
    };
    dir.join("lab.ps1").is_file().then_some(dir)
}

/// The `pwsh` arguments for `ctx`: `-NoProfile -File <dir>\lab.ps1 <command>`,
/// then the raw tokens unchanged.
pub fn command_line(dir: &Path, ctx: &Ctx) -> Vec<OsString> {
    let mut args = vec![
        OsString::from("-NoProfile"),
        OsString::from("-File"),
        dir.join("lab.ps1").into_os_string(),
        OsString::from(&ctx.command),
    ];
    args.extend(ctx.raw.iter().cloned());
    args
}

/// Runs the PowerShell dispatcher for `ctx` with inherited stdio and returns
/// its exit code (255 when it has none).
pub fn run(ctx: &Ctx) -> u8 {
    let Some(dir) = dispatcher_path() else {
        ctx.ui.error(
            "the PowerShell half of the lab CLI is not installed",
            Some("run: lab setup -From <checkout>"),
        );
        return 1;
    };
    match Process::new("pwsh").args(command_line(&dir, ctx)).status() {
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
    use crate::ui::Ui;

    #[test]
    fn command_line_keeps_raw_tokens_unchanged() {
        let ctx = Ctx {
            ui: Ui::plain(),
            command: "uat".to_string(),
            raw: vec![OsString::from("first-session"), OsString::from("-Rows")],
        };
        let dir = Path::new("lab-cli-dir");
        let args = command_line(dir, &ctx);
        assert_eq!(
            args,
            [
                OsString::from("-NoProfile"),
                OsString::from("-File"),
                dir.join("lab.ps1").into_os_string(),
                OsString::from("uat"),
                OsString::from("first-session"),
                OsString::from("-Rows"),
            ]
        );
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
