//! The doctor's system gathering: labels, paths, exe comparison and the
//! tools/lab content check against a throwaway git repository.

use std::path::Path;
use std::process::Command;

use cimmeria_lab_cli::commands::doctor::checks::{BinaryFacts, ContentCheck};
use cimmeria_lab_cli::commands::doctor::system::{
    account_path, checkout_root, compare_binaries, content_check, labels_for, listed_pids,
    read_version_sha, seed_path,
};
use cimmeria_lab_cli::daemon::env_file;
use cimmeria_lab_cli::daemon::status::{DaemonRow, InstanceRow, StatusReply};
use tempfile::TempDir;

fn reply(rows: &[(&str, Option<u32>)]) -> StatusReply {
    StatusReply {
        daemon: DaemonRow {
            pid: 1,
            version: "0.1.0".to_string(),
            started_at: None,
            uptime_s: None,
        },
        instances: rows
            .iter()
            .map(|(name, pid)| InstanceRow {
                instance: name.to_string(),
                account: None,
                bridge_port: None,
                client_pid: *pid,
                lease: None,
                uat: None,
            })
            .collect(),
    }
}

// ---- instances ----

// New: the daemon's instances win over labd.env's list.
#[test]
fn labels_come_from_the_daemon_when_it_answers() {
    let env = env_file::parse("CIMMERIA_LAB_INSTANCES=a,b\n");
    let status = reply(&[("default", None), ("p2", Some(5))]);
    assert_eq!(labels_for(Some(&status), &env), ["default", "p2"]);
}

// New: without a daemon, labd.env's list is split, trimmed and blank-free.
#[test]
fn labels_fall_back_to_labd_env() {
    let env = env_file::parse("CIMMERIA_LAB_INSTANCES= default , p2 ,,p3\n");
    assert_eq!(labels_for(None, &env), ["default", "p2", "p3"]);
}

// New: no daemon and no list means the single default instance.
#[test]
fn labels_default_to_default() {
    assert_eq!(labels_for(None, &[]), ["default"]);
    let blank = env_file::parse("CIMMERIA_LAB_INSTANCES= , \n");
    assert_eq!(labels_for(None, &blank), ["default"]);
}

// New: the listed pids are the instances' client pids, in order.
#[test]
fn listed_pids_are_the_client_pids() {
    let status = reply(&[("default", Some(101)), ("p2", None), ("p3", Some(202))]);
    assert_eq!(listed_pids(Some(&status)), [101, 202]);
    assert!(listed_pids(None).is_empty());
}

// New: account and seed file names (instances.ps1's rule).
#[test]
fn account_and_seed_paths_follow_the_instance_rules() {
    let default = account_path("C:\\Games\\SGW", "default");
    assert_eq!(default.file_name().unwrap(), "lab-account.json");
    let p2 = account_path("C:\\Games\\SGW", "p2");
    assert_eq!(p2.file_name().unwrap(), "lab-account.p2.json");
    assert!(p2.ends_with(
        Path::new("Binaries")
            .join("sessions")
            .join("lab-account.p2.json")
    ));
    let seed = seed_path(Path::new("C:\\Labs"), "p2");
    let tail = Path::new("p2")
        .join("profile")
        .join("Documents")
        .join("My Games")
        .join("Firesky")
        .join("SGWGame");
    assert!(seed.ends_with(tail));
}

// ---- the two exes ----

fn write(dir: &Path, name: &str, bytes: &[u8]) -> std::path::PathBuf {
    let path = dir.join(name);
    std::fs::write(&path, bytes).unwrap();
    path
}

// ops: 'identical bin and labd copies PASS' (gathering side)
#[test]
fn identical_files_are_the_same_with_their_length() {
    let dir = TempDir::new().unwrap();
    let a = write(dir.path(), "a.exe", &vec![7u8; 200_000]);
    let b = write(dir.path(), "b.exe", &vec![7u8; 200_000]);
    assert_eq!(compare_binaries(&a, &b), BinaryFacts::Same(200_000));
}

// ops: 'differing bin and labd copies WARN' (gathering side): same length,
// one byte differs past the first 64 KiB chunk, so only a full compare sees it.
#[test]
fn a_late_byte_difference_of_equal_length_is_seen() {
    let dir = TempDir::new().unwrap();
    let mut bytes = vec![7u8; 200_000];
    let a = write(dir.path(), "a.exe", &bytes);
    bytes[150_000] = 8;
    let b = write(dir.path(), "b.exe", &bytes);
    assert_eq!(compare_binaries(&a, &b), BinaryFacts::Differ);
}

#[test]
fn files_of_different_length_differ() {
    let dir = TempDir::new().unwrap();
    let a = write(dir.path(), "a.exe", b"one");
    let b = write(dir.path(), "b.exe", b"one!");
    assert_eq!(compare_binaries(&a, &b), BinaryFacts::Differ);
}

// ops: 'a missing copy WARNs' (gathering side): the first missing one is named.
#[test]
fn the_first_missing_file_is_named() {
    let dir = TempDir::new().unwrap();
    let a = write(dir.path(), "a.exe", b"one");
    let gone = dir.path().join("none.exe");
    assert_eq!(
        compare_binaries(&a, &gone),
        BinaryFacts::Missing(gone.clone())
    );
    assert_eq!(compare_binaries(&gone, &a), BinaryFacts::Missing(gone));
}

// ---- VERSION ----

// ops: 'no VERSION inside a checkout WARNs' (gathering side)
#[test]
fn version_sha_is_the_trimmed_first_line() {
    let dir = TempDir::new().unwrap();
    let v = write(
        dir.path(),
        "VERSION",
        b"\xEF\xBB\xBF abc1234 \r\nsecond\r\n",
    );
    assert_eq!(read_version_sha(&v), Some("abc1234".to_string()));
    let blank = write(dir.path(), "BLANK", b"  \r\n");
    assert_eq!(read_version_sha(&blank), None);
    assert_eq!(read_version_sha(&dir.path().join("none")), None);
}

// ---- the tools/lab content check (real git) ----

fn git(dir: &Path, args: &[&str]) -> String {
    let output = Command::new("git")
        .arg("-C")
        .arg(dir)
        .args([
            "-c",
            "user.name=test",
            "-c",
            "user.email=test@example.com",
            "-c",
            "commit.gpgsign=false",
            "-c",
            "core.autocrlf=false",
        ])
        .args(args)
        .output()
        .expect("git must be installed to run this test");
    assert!(
        output.status.success(),
        "git {args:?}: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8_lossy(&output.stdout).trim().to_string()
}

fn commit(dir: &Path, rel: &str, text: &str) {
    let path = dir.join(rel);
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(&path, text).unwrap();
    git(dir, &["add", "."]);
    git(dir, &["commit", "-q", "-m", rel]);
}

/// c1 and c2 change tools/lab; c3 changes only docs. origin/main is c3.
fn repo() -> TempDir {
    let dir = TempDir::new().unwrap();
    git(dir.path(), &["init", "-q"]);
    commit(dir.path(), "tools/lab/a.txt", "1");
    commit(dir.path(), "tools/lab/a.txt", "2");
    commit(dir.path(), "docs/x.md", "docs only");
    git(
        dir.path(),
        &["update-ref", "refs/remotes/origin/main", "HEAD"],
    );
    dir
}

// ops: "origin/main's own sha PASSes", but proving it compares content, not
// ancestry: HEAD~1 is behind origin/main by a docs-only commit.
#[test]
fn a_sha_behind_by_a_docs_only_commit_has_the_same_tools_lab() {
    let dir = repo();
    let c2 = git(dir.path(), &["rev-parse", "--short", "HEAD~1"]);
    assert_eq!(
        content_check(Some(dir.path()), Some(&c2)),
        Some(ContentCheck::Same)
    );
    let head = git(dir.path(), &["rev-parse", "--short", "HEAD"]);
    assert_eq!(
        content_check(Some(dir.path()), Some(&head)),
        Some(ContentCheck::Same)
    );
}

// ops: 'a sha whose tools/lab differs from origin/main WARNs'
#[test]
fn a_sha_before_the_last_tools_lab_change_differs() {
    let dir = repo();
    let c1 = git(dir.path(), &["rev-parse", "--short", "HEAD~2"]);
    assert_eq!(
        content_check(Some(dir.path()), Some(&c1)),
        Some(ContentCheck::Differs)
    );
}

// ops: 'an unknown sha WARNs as unknown'
#[test]
fn an_unknown_sha_is_unknown() {
    let dir = repo();
    assert_eq!(
        content_check(Some(dir.path()), Some("deadbee")),
        Some(ContentCheck::Unknown)
    );
}

// New: a sha that looks like an option never reaches git.
#[test]
fn a_dash_sha_is_unknown_without_running_git() {
    let dir = repo();
    assert_eq!(
        content_check(Some(dir.path()), Some("--output=oops")),
        Some(ContentCheck::Unknown)
    );
}

// ops: 'outside a checkout the VERSION check is skipped (PASS)' / 'no VERSION
// ... WARNs': the content check is skipped (None) unless both exist.
#[test]
fn the_content_check_is_skipped_without_a_checkout_or_a_sha() {
    let dir = repo();
    assert_eq!(content_check(None, Some("abc1234")), None);
    assert_eq!(content_check(Some(dir.path()), None), None);
}

// New: a checkout is a git work tree that has tools/lab/lab.ps1.
#[test]
fn checkout_root_needs_the_lab_dispatcher() {
    let dir = repo();
    assert_eq!(checkout_root(dir.path()), None, "no tools/lab/lab.ps1 yet");
    commit(dir.path(), "tools/lab/lab.ps1", "# dispatcher");
    let root = checkout_root(dir.path()).expect("a checkout now");
    assert!(root.join("tools").join("lab").join("lab.ps1").exists());
    let plain = TempDir::new().unwrap();
    assert_eq!(checkout_root(plain.path()), None, "not a git repository");
}
