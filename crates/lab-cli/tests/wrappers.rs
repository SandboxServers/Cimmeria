//! The thin wrappers: `start`, `stop`, `restart`, `instances`, `install`,
//! `setup` and `local-setup` run their PowerShell script with the original
//! tokens in PowerShell form and return its exit code. The scripts are stubs
//! that print their tokens joined with `|`.

mod support;

use std::path::Path;
use std::process::{Command, Output};

use tempfile::TempDir;

fn pwsh_exists() -> bool {
    Command::new("pwsh").arg("-Version").output().is_ok()
}

/// A scripts folder whose five scripts print `$args` joined with `|` and exit
/// with `$env:LAB_STUB_EXIT` when it is set.
fn scripts() -> TempDir {
    let dir = TempDir::new().expect("temp dir");
    write_stubs(dir.path());
    dir
}

/// Writes the five stub scripts under `root` (creating it and `root\cli`).
fn write_stubs(root: &Path) {
    std::fs::create_dir_all(root.join("cli")).expect("cli folder");
    let body = "[Console]::Out.WriteLine(($args -join '|'))\nif ($env:LAB_STUB_EXIT) { exit [int]$env:LAB_STUB_EXIT }\n";
    for relative in [
        "daemon.ps1",
        "instances.ps1",
        "cli/install.ps1",
        "cli/setup.ps1",
        "cli/local-setup.ps1",
    ] {
        std::fs::write(root.join(relative), body).expect("write stub");
    }
}

/// Runs `lab` against the stub `scripts` folder, in an isolated lab home so no
/// run reaches the machine's real lab daemon.
fn lab(args: &[&str], scripts: &Path, exit: Option<&str>) -> Output {
    let home = support::isolated_lab_home();
    let mut command = Command::new(env!("CARGO_BIN_EXE_lab"));
    command
        .args(args)
        .env("CIMMERIA_LAB_HOME", home.path())
        .env("LOCALAPPDATA", home.path())
        .env("CIMMERIA_LAB_CLI_DIR", scripts);
    match exit {
        Some(code) => command.env("LAB_STUB_EXIT", code),
        None => command.env_remove("LAB_STUB_EXIT"),
    };
    command.output().expect("run lab")
}

/// What the stub script printed, or `None` (test skipped) without pwsh.
fn seen(args: &[&str]) -> Option<String> {
    if !pwsh_exists() {
        eprintln!("pwsh is not on the PATH; skipped");
        return None;
    }
    let dir = scripts();
    let out = lab(args, dir.path(), None);
    assert_eq!(out.status.code(), Some(0), "{args:?}");
    Some(String::from_utf8_lossy(&out.stdout).trim().to_string())
}

#[test]
fn start_runs_daemon_ps1_start() {
    if let Some(text) = seen(&["start"]) {
        assert_eq!(text, "start");
    }
}

#[test]
fn stop_and_restart_pass_force_in_powershell_form() {
    for (args, want) in [
        (&["stop"][..], "stop"),
        (&["stop", "--force"], "stop|-Force"),
        (&["stop", "-Force"], "stop|-Force"),
        (&["restart"], "restart"),
        (&["restart", "--force"], "restart|-Force"),
    ] {
        if let Some(text) = seen(args) {
            assert_eq!(text, want, "{args:?}");
        }
    }
}

#[test]
fn instances_passes_its_tokens_to_instances_ps1() {
    if let Some(text) = seen(&["instances", "status"]) {
        assert_eq!(text, "status");
    }
    if let Some(text) = seen(&[
        "instances",
        "init",
        "--install-dir",
        "D:\\G x",
        "--count",
        "3",
        "--force",
    ]) {
        assert_eq!(text, "init|-InstallDir|D:\\G x|-Count|3|-Force");
    }
}

#[test]
fn install_setup_and_local_setup_run_their_cli_scripts() {
    if let Some(text) = seen(&["install", "--from", "C:\\wt", "--skip-build"]) {
        assert_eq!(text, "-From|C:\\wt|-SkipBuild");
    }
    if let Some(text) = seen(&["setup", "--no-path", "--yes", "--from", "C:\\x"]) {
        assert_eq!(text, "-NoPath|-Yes|-From|C:\\x");
    }
    if let Some(text) = seen(&[
        "local-setup",
        "--install-dir",
        "D:\\G",
        "--patch-login-internal",
        "--env-scope",
        "Process",
    ]) {
        assert_eq!(
            text,
            "-InstallDir|D:\\G|-PatchLoginInternal|-EnvScope|Process"
        );
    }
}

#[test]
fn the_childs_exit_code_is_ours() {
    if !pwsh_exists() {
        return;
    }
    let dir = scripts();
    assert_eq!(lab(&["stop"], dir.path(), Some("7")).status.code(), Some(7));
    assert_eq!(
        lab(&["install", "--from", "x"], dir.path(), Some("3"))
            .status
            .code(),
        Some(3)
    );
}

/// With no `CIMMERIA_LAB_CLI_DIR`, the scripts are the installed copy beside
/// `bin` (`<lab home>\bin\lab.exe` runs `<lab home>\cli\daemon.ps1`,
/// `<lab home>\cli\instances.ps1` and `<lab home>\cli\cli\<name>.ps1`, the
/// layout install-lib.ps1 copies), never a checkout's.
#[test]
fn the_installed_layout_beside_bin_is_where_the_scripts_are_found() {
    if !pwsh_exists() {
        eprintln!("pwsh is not on the PATH; skipped");
        return;
    }
    let install = TempDir::new().expect("temp dir");
    let bin = install.path().join("bin");
    std::fs::create_dir(&bin).expect("bin folder");
    let exe = bin.join("lab.exe");
    std::fs::copy(env!("CARGO_BIN_EXE_lab"), &exe).expect("copy lab.exe");
    // The stubs sit in `<install>\cli`, as installed; `<install>` itself has none.
    write_stubs(&install.path().join("cli"));
    let home = support::isolated_lab_home();
    for (args, want) in [
        (&["stop", "--force"][..], "stop|-Force"),
        (&["instances", "status"], "status"),
        (&["install", "--skip-build"], "-SkipBuild"),
    ] {
        let out = Command::new(&exe)
            .args(args)
            .env("CIMMERIA_LAB_HOME", home.path())
            .env("LOCALAPPDATA", home.path())
            .env_remove("CIMMERIA_LAB_CLI_DIR")
            .env_remove("LAB_STUB_EXIT")
            .output()
            .expect("run the copied lab.exe");
        assert_eq!(out.status.code(), Some(0), "{args:?}");
        assert_eq!(
            String::from_utf8_lossy(&out.stdout).trim(),
            want,
            "{args:?}"
        );
    }
}

#[test]
fn a_missing_script_is_exit_1_not_installed() {
    let empty = TempDir::new().expect("temp dir");
    let out = lab(&["start"], empty.path(), None);
    assert_eq!(out.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&out.stderr).contains("not installed"));
}

#[test]
fn help_never_runs_a_script() {
    let empty = TempDir::new().expect("temp dir");
    let out = lab(&["start", "--help"], empty.path(), None);
    assert_eq!(out.status.code(), Some(0));
    assert!(String::from_utf8_lossy(&out.stdout).contains("Examples:"));
}
