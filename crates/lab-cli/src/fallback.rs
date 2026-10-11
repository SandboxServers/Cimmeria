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
        Ok(status) => status
            .code()
            .and_then(|code| u8::try_from(code).ok())
            .unwrap_or(255),
        Err(err) => {
            ctx.ui.error(
                &format!("could not start pwsh: {err}"),
                Some("install PowerShell 7 or put pwsh on PATH"),
            );
            1
        }
    }
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
        let args = command_line(Path::new("C:\\lab\\cli"), &ctx);
        assert_eq!(
            args,
            [
                OsString::from("-NoProfile"),
                OsString::from("-File"),
                OsString::from("C:\\lab\\cli\\lab.ps1"),
                OsString::from("uat"),
                OsString::from("first-session"),
                OsString::from("-Rows"),
            ]
        );
    }
}
