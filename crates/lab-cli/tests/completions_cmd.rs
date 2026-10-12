//! `lab completions`: the scripts, and `--install` against a temp profile.

mod support;

use std::process::{Command, Output};

use cimmeria_lab_cli::commands::COMMANDS;
use tempfile::TempDir;

/// The call the profile line runs.
const CALL: &str = "completions powershell | Out-String | Invoke-Expression";

/// The line `--install` must write: guarded, with lab.exe's canonical path
/// (the test binary is the same lab.exe the child process runs).
fn line() -> String {
    let exe = std::fs::canonicalize(env!("CARGO_BIN_EXE_lab")).expect("canonical lab.exe");
    let exe = exe.to_string_lossy().into_owned();
    let exe = exe.strip_prefix(r"\\?\").unwrap_or(&exe).to_string();
    format!("if (Test-Path -LiteralPath '{exe}') {{ & '{exe}' {CALL} }}")
}

/// The old, unguarded line that earlier versions wrote.
fn old_line() -> String {
    format!("lab {CALL}")
}

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

/// Runs `lab completions powershell --install` against `profile`.
fn install(profile: &str) -> Output {
    lab(&[
        "completions",
        "powershell",
        "--install",
        "--profile-path",
        profile,
    ])
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
    // A temp profile path, so a regression that lets bash through writes there
    // and not to the real $PROFILE.
    let dir = TempDir::new().expect("temp dir");
    let profile = dir.path().join("profile.ps1");
    let p = profile.to_str().expect("utf-8 path");
    let out = lab(&["completions", "bash", "--install", "--profile-path", p]);
    assert_eq!(out.status.code(), Some(2));
    assert!(text(&out.stderr).contains("powershell only"));
    assert!(!profile.exists(), "bash --install must not write a profile");
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

// The line is guarded and names lab.exe by an absolute path, so a profile
// never errors when lab is not on PATH or has moved.
// Fails if reverted: the bare `lab completions ...` line has no Test-Path guard
// and no quoted path.
#[test]
fn the_written_line_is_guarded_and_uses_an_absolute_path() {
    let dir = TempDir::new().expect("temp dir");
    let profile = dir.path().join("profile.ps1");
    let p = profile.to_str().expect("utf-8 path");
    let out = install(p);
    assert_eq!(out.status.code(), Some(0));
    let now = std::fs::read_to_string(&profile).expect("profile");
    assert!(now.contains(&line()), "{now:?}");
    assert!(now.contains("if (Test-Path -LiteralPath '"), "{now:?}");
    assert!(!now.contains(&old_line()), "{now:?}");
    let exe = line().split('\'').nth(1).expect("quoted path").to_string();
    assert!(std::path::Path::new(&exe).is_absolute(), "{exe}");
}

// The install step writes once: a second run changes nothing.
// Fails if reverted: without the look-first check the line is added twice.
#[test]
fn install_is_idempotent_against_a_temp_profile() {
    let dir = TempDir::new().expect("temp dir");
    let profile = dir.path().join("sub").join("profile.ps1");
    let p = profile.to_str().expect("utf-8 path");

    let first = install(p);
    assert_eq!(first.status.code(), Some(0));
    assert!(
        text(&first.stdout).contains("added"),
        "{}",
        text(&first.stdout)
    );
    let after_first = std::fs::read_to_string(&profile).expect("profile");
    assert_eq!(after_first.matches(CALL).count(), 1);

    let second = install(p);
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

// An old unguarded line is replaced in place, not duplicated, and the lines
// around it are kept.
// Fails if reverted: the old line stays and the guarded line is appended.
#[test]
fn an_old_unguarded_line_is_replaced_not_duplicated() {
    let dir = TempDir::new().expect("temp dir");
    let profile = dir.path().join("profile.ps1");
    std::fs::write(
        &profile,
        format!("Set-Alias x y\r\n{}\r\nSet-Alias a b\r\n", old_line()),
    )
    .expect("write");
    let p = profile.to_str().expect("utf-8 path");
    let out = install(p);
    assert_eq!(out.status.code(), Some(0));
    assert!(
        text(&out.stdout).contains("updated"),
        "{}",
        text(&out.stdout)
    );
    let now = std::fs::read_to_string(&profile).expect("profile");
    assert_eq!(
        now,
        format!("Set-Alias x y\r\n{}\r\nSet-Alias a b\r\n", line())
    );
    assert_eq!(now.matches(CALL).count(), 1, "{now:?}");
}

// Running the install again after an update changes nothing: one line stays.
// Fails if reverted: the second run would append or replace again.
#[test]
fn running_twice_after_an_update_leaves_one_line() {
    let dir = TempDir::new().expect("temp dir");
    let profile = dir.path().join("profile.ps1");
    std::fs::write(&profile, format!("{}\n", old_line())).expect("write");
    let p = profile.to_str().expect("utf-8 path");
    assert!(text(&install(p).stdout).contains("updated"));
    let after_first = std::fs::read_to_string(&profile).expect("profile");
    let second = install(p);
    assert!(text(&second.stdout).contains("already"));
    assert_eq!(
        std::fs::read_to_string(&profile).expect("profile"),
        after_first
    );
    assert_eq!(after_first.matches(CALL).count(), 1, "{after_first:?}");
    assert_eq!(after_first, format!("{}\n", line()));
}

#[test]
fn install_keeps_what_the_profile_holds() {
    let dir = TempDir::new().expect("temp dir");
    let profile = dir.path().join("profile.ps1");
    std::fs::write(&profile, "Set-Alias x y").expect("write");
    let p = profile.to_str().expect("utf-8 path");
    let out = install(p);
    assert_eq!(out.status.code(), Some(0));
    let now = std::fs::read_to_string(&profile).expect("profile");
    assert!(now.starts_with("Set-Alias x y\n"), "{now:?}");
    assert!(now.contains(&line()));
}

#[test]
fn install_uses_the_profiles_own_line_ending() {
    let dir = TempDir::new().expect("temp dir");
    let crlf = dir.path().join("crlf.ps1");
    std::fs::write(&crlf, "Set-Alias x y\r\nSet-Alias a b").expect("write");
    let lf = dir.path().join("lf.ps1");
    std::fs::write(&lf, "Set-Alias x y\n").expect("write");
    for path in [&crlf, &lf] {
        let p = path.to_str().expect("utf-8 path");
        let out = install(p);
        assert_eq!(out.status.code(), Some(0));
    }
    let now = std::fs::read_to_string(&crlf).expect("profile");
    assert!(
        now.starts_with("Set-Alias x y\r\nSet-Alias a b\r\n# lab"),
        "{now:?}"
    );
    assert!(now.ends_with(&format!("{}\r\n", line())), "{now:?}");
    assert!(!now.replace("\r\n", "").contains('\n'), "{now:?}");
    let now = std::fs::read_to_string(&lf).expect("profile");
    assert!(!now.contains('\r'), "{now:?}");
    assert!(now.ends_with(&format!("{}\n", line())), "{now:?}");
}

// Only the exact old forms are replaced. A line of the user's that holds the
// call, with a trailing comment or other statements, stays byte for byte and
// the guarded line is appended.
// Fails if reverted: a `contains` match replaces these lines and loses the
// user's comment and statements.
#[test]
fn a_line_the_user_wrote_is_kept_and_a_comment_is_not_ours() {
    let dir = TempDir::new().expect("temp dir");
    for (name, mine) in [
        ("trailing.ps1", format!("lab {CALL}  # mine")),
        ("statements.ps1", format!("$x = 1; lab {CALL}")),
        ("comment.ps1", format!("# lab {CALL} is nice")),
    ] {
        let profile = dir.path().join(name);
        std::fs::write(&profile, format!("{mine}\r\nSet-Alias a b\r\n")).expect("write");
        let out = install(profile.to_str().expect("utf-8 path"));
        assert_eq!(out.status.code(), Some(0), "{name}");
        assert!(text(&out.stdout).contains("added"), "{name}");
        let now = std::fs::read_to_string(&profile).expect("profile");
        assert!(
            now.starts_with(&format!("{mine}\r\nSet-Alias a b\r\n# lab command-line")),
            "{name}: {now:?}"
        );
        assert!(now.ends_with(&format!("{}\r\n", line())), "{name}: {now:?}");
    }
}

// A guarded line for another lab.exe path is replaced in place, an extra old
// line is dropped, whitespace around the old line does not matter, and every
// other line stays byte for byte, including a byte order mark and the endings.
// Fails if reverted: the stale path stays, or the guarded line is appended.
#[test]
fn a_guarded_line_for_another_path_is_replaced_and_the_rest_is_untouched() {
    let dir = TempDir::new().expect("temp dir");
    let profile = dir.path().join("profile.ps1");
    let stale =
        format!("if (Test-Path -LiteralPath 'D:\\old\\lab.exe') {{ & 'D:\\old\\lab.exe' {CALL} }}");
    let before = format!(
        "\u{feff}Set-Alias x y\r\n# keep  \r\n  {stale}  \nSet-Alias a b\n\n  {} \r\nlast",
        old_line()
    );
    std::fs::write(&profile, &before).expect("write");
    let out = install(profile.to_str().expect("utf-8 path"));
    assert_eq!(out.status.code(), Some(0));
    assert!(text(&out.stdout).contains("updated"));
    let now = std::fs::read_to_string(&profile).expect("profile");
    assert_eq!(
        now,
        format!(
            "\u{feff}Set-Alias x y\r\n# keep  \r\n{}\nSet-Alias a b\n\nlast",
            line()
        )
    );
}

// The first line of a file with a byte order mark keeps the mark when it is
// the line that is replaced.
// Fails if reverted: the mark is lost and PowerShell 5 reads the file as ANSI.
#[test]
fn a_byte_order_mark_on_the_replaced_line_is_kept() {
    let dir = TempDir::new().expect("temp dir");
    let profile = dir.path().join("profile.ps1");
    std::fs::write(
        &profile,
        format!("\u{feff}{}\r\nSet-Alias a b\r\n", old_line()),
    )
    .expect("write");
    let out = install(profile.to_str().expect("utf-8 path"));
    assert!(text(&out.stdout).contains("updated"));
    assert_eq!(
        std::fs::read_to_string(&profile).expect("profile"),
        format!("\u{feff}{}\r\nSet-Alias a b\r\n", line())
    );
    assert!(text(&install(profile.to_str().expect("utf-8 path")).stdout).contains("already"));
}

// A profile that is not UTF-8 (Windows PowerShell's UTF-16 default) is left
// alone and the failure is reported.
// Fails if the read were made lossy: the replace path would overwrite the file.
#[test]
fn a_profile_that_is_not_utf8_is_left_untouched() {
    let dir = TempDir::new().expect("temp dir");
    let profile = dir.path().join("profile.ps1");
    let bytes: Vec<u8> = "lab x\r\n"
        .encode_utf16()
        .flat_map(u16::to_le_bytes)
        .collect();
    let mut utf16 = vec![0xff, 0xfe];
    utf16.extend(bytes);
    std::fs::write(&profile, &utf16).expect("write");
    let out = install(profile.to_str().expect("utf-8 path"));
    assert_eq!(out.status.code(), Some(1));
    assert!(
        text(&out.stderr).contains("cannot update"),
        "{}",
        text(&out.stderr)
    );
    assert_eq!(std::fs::read(&profile).expect("profile"), utf16);
}

/// Runs `command` in `pwsh -NoProfile`; `None` when pwsh is not installed.
fn pwsh(command: &str) -> Option<Output> {
    Command::new(cimmeria_lab_cli::fallback::pwsh_program())
        .args(["-NoProfile", "-NonInteractive", "-Command", command])
        .output()
        .ok()
}

// The written line really never errors when lab.exe is missing, even with an
// apostrophe and spaces in the path.
// Fails if reverted: without the Test-Path guard PowerShell reports that the
// file is not recognised; without the doubled quote the line does not parse.
#[test]
fn the_line_runs_clean_in_pwsh_when_the_exe_is_missing() {
    let missing = std::path::Path::new(r"Z:\no such dir\it's gone\lab.exe");
    let line = cimmeria_lab_cli::commands::completions::profile_line(missing);
    let Some(out) = pwsh(&format!("{line}; 'after'")) else {
        return;
    };
    assert_eq!(out.status.code(), Some(0), "{}", text(&out.stderr));
    assert_eq!(text(&out.stderr), "");
    assert_eq!(text(&out.stdout).trim(), "after");
}

// With lab.exe present, in a folder with a space and an apostrophe, the line
// loads the completer.
// Fails if reverted: an undoubled quote in the folder name breaks the string.
#[test]
fn the_line_loads_the_completer_in_pwsh_from_a_quoted_path() {
    let dir = TempDir::new().expect("temp dir");
    let folder = dir.path().join("it's a dir");
    std::fs::create_dir_all(&folder).expect("folder");
    let exe = folder.join("lab.exe");
    std::fs::copy(env!("CARGO_BIN_EXE_lab"), &exe).expect("copy lab.exe");
    let line = cimmeria_lab_cli::commands::completions::profile_line(&exe);
    let Some(out) = pwsh(&format!(
        "{line}; (TabExpansion2 -inputScript 'lab u' -cursorColumn 5).CompletionMatches.CompletionText -join ','"
    )) else {
        return;
    };
    assert_eq!(out.status.code(), Some(0), "{}", text(&out.stderr));
    assert_eq!(text(&out.stderr), "");
    assert!(text(&out.stdout).contains("uat"), "{}", text(&out.stdout));
}
