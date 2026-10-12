//! `lab completions`: the scripts, and `--install` against a temp profile.

mod support;

use std::process::{Command, Output};

use cimmeria_lab_cli::commands::COMMANDS;
use tempfile::TempDir;

const LINE: &str = "lab completions powershell | Out-String | Invoke-Expression";

fn lab(args: &[&str]) -> Output {
    let home = support::isolated_lab_home();
    Command::new(env!("CARGO_BIN_EXE_lab"))
        .args(args)
        .env("CIMMERIA_LAB_HOME", home.path())
        .env("LOCALAPPDATA", home.path())
        .output()
        .expect("run lab")
}

fn text(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).into_owned()
}

#[test]
fn the_powershell_script_registers_a_completer_and_lists_every_command() {
    let out = lab(&["completions", "powershell"]);
    assert_eq!(out.status.code(), Some(0));
    let script = text(&out.stdout);
    assert!(script.contains("Register-ArgumentCompleter"), "{script}");
    for info in COMMANDS {
        assert!(
            script.contains(&format!("'{}'", info.name)),
            "{} is missing from the script",
            info.name
        );
    }
}

#[test]
fn the_other_shells_print_a_script() {
    for (shell, needle) in [
        ("bash", "_lab"),
        ("zsh", "#compdef lab"),
        ("fish", "complete -c lab"),
    ] {
        let out = lab(&["completions", shell]);
        assert_eq!(out.status.code(), Some(0), "{shell}");
        assert!(text(&out.stdout).contains(needle), "{shell}");
    }
}

#[test]
fn usage_errors_exit_2() {
    assert_eq!(lab(&["completions"]).status.code(), Some(2));
    assert_eq!(lab(&["completions", "nushell"]).status.code(), Some(2));
    let out = lab(&["completions", "bash", "--install"]);
    assert_eq!(out.status.code(), Some(2));
    assert!(text(&out.stderr).contains("powershell only"));
    assert_eq!(
        lab(&["completions", "powershell", "--profile-path", "p.ps1"])
            .status
            .code(),
        Some(2)
    );
}

#[test]
fn the_test_seam_is_hidden_from_help() {
    let out = lab(&["completions", "--help"]);
    let help = text(&out.stdout);
    assert!(help.contains("--install"), "{help}");
    assert!(!help.contains("profile-path"), "{help}");
}

// The install step appends once: a second run changes nothing.
// Fails if reverted: without the look-first check the line is added twice.
#[test]
fn install_is_idempotent_against_a_temp_profile() {
    let dir = TempDir::new().expect("temp dir");
    let profile = dir.path().join("sub").join("profile.ps1");
    let p = profile.to_str().expect("utf-8 path");

    let first = lab(&[
        "completions",
        "powershell",
        "--install",
        "--profile-path",
        p,
    ]);
    assert_eq!(first.status.code(), Some(0));
    assert!(
        text(&first.stdout).contains("added"),
        "{}",
        text(&first.stdout)
    );
    let after_first = std::fs::read_to_string(&profile).expect("profile");
    assert_eq!(after_first.matches(LINE).count(), 1);

    let second = lab(&[
        "completions",
        "powershell",
        "--install",
        "--profile-path",
        p,
    ]);
    assert_eq!(second.status.code(), Some(0));
    assert!(
        text(&second.stdout).contains("already"),
        "{}",
        text(&second.stdout)
    );
    assert_eq!(
        std::fs::read_to_string(&profile).expect("profile"),
        after_first
    );
}

#[test]
fn install_keeps_what_the_profile_holds() {
    let dir = TempDir::new().expect("temp dir");
    let profile = dir.path().join("profile.ps1");
    std::fs::write(&profile, "Set-Alias x y").expect("write");
    let p = profile.to_str().expect("utf-8 path");
    let out = lab(&[
        "completions",
        "powershell",
        "--install",
        "--profile-path",
        p,
    ]);
    assert_eq!(out.status.code(), Some(0));
    let now = std::fs::read_to_string(&profile).expect("profile");
    assert!(now.starts_with("Set-Alias x y\n"), "{now:?}");
    assert!(now.contains(LINE));
}

#[test]
fn a_line_the_user_wrote_counts_but_a_comment_does_not() {
    let dir = TempDir::new().expect("temp dir");
    let profile = dir.path().join("profile.ps1");
    let mine = format!("{LINE}  # mine\n");
    std::fs::write(&profile, &mine).expect("write");
    let p = profile.to_str().expect("utf-8 path");
    let out = lab(&[
        "completions",
        "powershell",
        "--install",
        "--profile-path",
        p,
    ]);
    assert!(text(&out.stdout).contains("already"));
    assert_eq!(std::fs::read_to_string(&profile).expect("profile"), mine);

    let commented = dir.path().join("other.ps1");
    std::fs::write(&commented, "# lab completions powershell is nice\n").expect("write");
    let c = commented.to_str().expect("utf-8 path");
    let out = lab(&[
        "completions",
        "powershell",
        "--install",
        "--profile-path",
        c,
    ]);
    assert!(text(&out.stdout).contains("added"));
}
