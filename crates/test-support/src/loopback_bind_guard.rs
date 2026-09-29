//! Tests bind `127.0.0.1`, never `0.0.0.0`.
//!
//! On Windows, a program that listens on a non-loopback address raises a
//! Windows Firewall "allow access" prompt. A cargo test binary gets a new
//! hashed file name on every rebuild (`cimmeria_auth-c445722ea3221c4d.exe`),
//! so every rebuild prompts again, and unanswered prompts pile up on the
//! desktop. The production defaults in `ServerConfig::default()` bind every
//! game listener to `0.0.0.0`, which is right for the real server and wrong
//! for a test, so a test that starts a service builds its config from
//! `ServerConfig::loopback()`.
//!
//! Two scans keep it that way:
//!
//! - test code never takes `ServerConfig::default()` as the base of a
//!   struct update, and never uses it at all where it starts a service;
//! - production code names the `0.0.0.0` wildcard only in [`WILDCARD_FILES`],
//!   so a bind address is never hard-coded past the config (the minigame
//!   listener was, until the config grew `minigame_host`).
//!
//! Line-based: comment lines are skipped, string contents are not parsed.

use crate::source_scan::{production_lines, rust_sources, RustSource};

/// This file: its synthetic regressions name both patterns on purpose.
const THIS_FILE: &str = "test-support/src/loopback_bind_guard.rs";

/// Production files allowed to name the `0.0.0.0` wildcard, with why.
const WILDCARD_FILES: &[(&str, &str)] = &[
    (
        "common/src/config.rs",
        "the production defaults: the real server listens on every interface",
    ),
    (
        "lab/src/supervisor/mod.rs",
        "maps a wildcard bridge bind to 127.0.0.1 for connecting; binds nothing",
    ),
    (
        "supervisor/src/main.rs",
        "the standalone supervisor binary's own listener; no test starts it",
    ),
];

/// A call that starts a real listener: `AuthService` / `BaseService` /
/// `CellService::start` or `Orchestrator::start_all`.
fn starts_a_service(line: &str) -> bool {
    line.contains(".start().await") || line.contains("start_all(")
}

fn is_comment(line: &str) -> bool {
    line.trim_start().starts_with("//")
}

/// Whether `line` opens the next item, ending the function a config was
/// built in.
fn opens_next_item(line: &str) -> bool {
    let t = line.trim_start();
    t.starts_with("#[")
        || [
            "fn ",
            "async fn ",
            "pub fn ",
            "pub(crate) fn ",
            "pub(super) fn ",
        ]
        .iter()
        .any(|p| t.starts_with(p))
        || [
            "pub async fn ",
            "pub(crate) async fn ",
            "pub(super) async fn ",
        ]
        .iter()
        .any(|p| t.starts_with(p))
}

/// Whether a service is started after line index `i`, before the next item.
fn started_later_in_fn(test_lines: &[(usize, &str)], i: usize) -> bool {
    test_lines[i + 1..]
        .iter()
        .take_while(|(_, l)| !opens_next_item(l))
        .any(|(_, l)| !is_comment(l) && starts_a_service(l))
}

/// `(line, why)` for each test line that would build a listener config from
/// the production defaults: a struct update on them anywhere, or a bare
/// default config followed by a service start in the same function.
/// `test_lines` are the file's test lines (1-based numbers), in order.
fn default_config_in_tests(test_lines: &[(usize, &str)]) -> Vec<(usize, String)> {
    test_lines
        .iter()
        .enumerate()
        .filter(|(_, (_, l))| !is_comment(l))
        .filter_map(|(i, &(n, l))| {
            if l.contains("..ServerConfig::default()") {
                Some((
                    n,
                    "struct update on `ServerConfig::default()` in test code; use \
                     `..ServerConfig::loopback()` (the defaults bind 0.0.0.0)"
                        .to_string(),
                ))
            } else if l.contains("ServerConfig::default()") && started_later_in_fn(test_lines, i) {
                Some((
                    n,
                    "`ServerConfig::default()` in test code that starts a service; \
                     use `ServerConfig::loopback()` (the defaults bind 0.0.0.0)"
                        .to_string(),
                ))
            } else {
                None
            }
        })
        .collect()
}

/// Line numbers of production lines naming the `0.0.0.0` wildcard in a
/// string literal.
fn wildcard_literals(prod_lines: &[(usize, &str)]) -> Vec<usize> {
    prod_lines
        .iter()
        .filter(|(_, l)| !is_comment(l) && l.contains("\"0.0.0.0"))
        .map(|&(n, _)| n)
        .collect()
}

/// The test lines of `file`: all of it for a test path, otherwise the
/// lines inside its `#[cfg(test)]` modules.
fn test_lines<'a>(file: &RustSource, text: &'a str) -> Vec<(usize, &'a str)> {
    let all = text.lines().enumerate().map(|(i, l)| (i + 1, l));
    if file.is_test_path() {
        return all.collect();
    }
    let prod: std::collections::HashSet<usize> =
        production_lines(text).iter().map(|(n, _)| *n).collect();
    all.filter(|(n, _)| !prod.contains(n)).collect()
}

#[test]
fn tests_build_listener_configs_from_loopback() {
    let mut bad = Vec::new();
    for file in rust_sources() {
        if file.crates_rel == THIS_FILE {
            continue;
        }
        let text = file.read();
        if !text.contains("ServerConfig::default()") {
            continue;
        }
        for (n, why) in default_config_in_tests(&test_lines(&file, &text)) {
            bad.push(format!("{}:{n}: {why}", file.crates_rel));
        }
    }
    assert!(
        bad.is_empty(),
        "tests must bind 127.0.0.1, never 0.0.0.0: on Windows every rebuilt \
         test binary that listens on 0.0.0.0 raises a firewall prompt.\n{}",
        bad.join("\n")
    );
}

#[test]
fn production_wildcard_binds_are_allowlisted() {
    let mut bad = Vec::new();
    for file in rust_sources() {
        if file.is_test_path()
            || file.crates_rel == THIS_FILE
            || WILDCARD_FILES.iter().any(|(f, _)| *f == file.crates_rel)
        {
            continue;
        }
        let text = file.read();
        for n in wildcard_literals(&production_lines(&text)) {
            bad.push(format!("{}:{n}", file.crates_rel));
        }
    }
    assert!(
        bad.is_empty(),
        "hard-coded 0.0.0.0 in production code: route the bind address through \
         `ServerConfig` (production default in `ServerConfig::default()`, \
         loopback in `ServerConfig::loopback()`) so tests can bind 127.0.0.1, \
         or add the file to WILDCARD_FILES with a reason.\n{}",
        bad.join("\n")
    );
}

/// The allowlist names files that exist and still need it.
#[test]
fn wildcard_allowlist_is_not_stale() {
    let files = rust_sources();
    for (want, why) in WILDCARD_FILES {
        let file = files
            .iter()
            .find(|f| f.crates_rel == *want)
            .unwrap_or_else(|| panic!("WILDCARD_FILES names missing file {want} ({why})"));
        let text = file.read();
        assert!(
            !wildcard_literals(&production_lines(&text)).is_empty(),
            "{want} no longer names 0.0.0.0; drop it from WILDCARD_FILES"
        );
    }
}

/// The bug shapes the scans exist for.
#[test]
fn scans_flag_the_regressions() {
    let numbered = |t: &'static str| -> Vec<(usize, &'static str)> {
        t.lines().enumerate().map(|(i, l)| (i + 1, l)).collect()
    };
    // `start_sets_running` on the production defaults.
    let update = numbered(
        "let config = ServerConfig {\n    logon_port: 0,\n    ..ServerConfig::default()\n};",
    );
    assert_eq!(default_config_in_tests(&update).len(), 1);
    // A bare default config, then a start.
    let bare = numbered(
        "let config = ServerConfig::default();\nlet mut svc = CellService::new(&config);\nsvc.start().await.unwrap();",
    );
    assert_eq!(default_config_in_tests(&bare).len(), 1);
    // A bare default config that starts nothing is fine (asserting ports).
    let no_start =
        numbered("let config = ServerConfig::default();\nassert_eq!(config.base_port, 32832);");
    assert!(default_config_in_tests(&no_start).is_empty());
    // ... even when the next test in the module starts one on loopback.
    let next_fn = numbered(
        "fn ports() {\n    let config = ServerConfig::default();\n}\n#[tokio::test]\nasync fn starts() {\n    let c = ServerConfig::loopback();\n    svc.start().await.unwrap();\n}",
    );
    assert!(default_config_in_tests(&next_fn).is_empty());
    // The loopback config is fine.
    let loopback = numbered(
        "let config = ServerConfig {\n    ..ServerConfig::loopback()\n};\nsvc.start().await;",
    );
    assert!(default_config_in_tests(&loopback).is_empty());
    // The hard-coded minigame bind the orchestrator used to have.
    let hard = numbered(
        "tokio::spawn(async move {\n    run(\"0.0.0.0\", mg_port, mg_port, reg, tx).await;\n});\n// \"0.0.0.0\" in a comment",
    );
    assert_eq!(wildcard_literals(&hard), [2]);
}
