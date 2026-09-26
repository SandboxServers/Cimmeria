//! NA25 round 2: every hand-named `target: "…"` in the server's source
//! reaches SigNoz at the level it is emitted.
//!
//! A custom target matches none of the module-path file layers, so the file
//! parity guard in `parity_tests` cannot see it: before this test,
//! `movement.movement_type`, `dialog.display`, `abilities` and a dozen more
//! emitted DEBUG rows that went nowhere but the admin WebSocket. This test
//! reads the source of every crate linked into `cimmeria-server`, finds each
//! `<level>!(target: "…"` and `event!(target: "…", Level::…)` call, and routes
//! that (target, level) through the production OTLP filters.
//!
//! The only targets allowed to reach no index are the `off` entries of
//! `OTLP_EXCLUDED_TARGETS`, each with its reason.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use tracing::Level;

use super::filters::{FILE_LAYERS, OTLP_EXCLUDED_TARGETS};
use super::parity_tests::{harness, sinks_for, OTLP_LOG_SINKS};

/// Crates linked into the `cimmeria-server` process. Their events pass
/// through the OTLP filters.
const IN_PROCESS_CRATES: &[&str] = &[
    "admin-api",
    "auth",
    "commands",
    "common",
    "content-engine",
    "defs",
    "discord",
    "entity",
    "game",
    "lab-mcp",
    "mercury",
    "observability",
    "occluder",
    "server",
    "services",
];

/// Crates that run in another process, so the server's filters never see
/// their events. Every directory under `crates/` must be in one list or the
/// other, so a new crate forces the question.
const OUT_OF_PROCESS_CRATES: &[(&str, &str)] = &[
    (
        "client-telemetry",
        "runs inside SGW.exe; its rows reach the server only as `client.native` replays",
    ),
    (
        "launcher",
        "the player's launcher; its rows reach the server only as `launcher.*` replays",
    ),
    ("client-launch", "library for the launcher process"),
    ("lab", "the research-lab supervisor process"),
    ("supervisor", "the process supervisor"),
    ("navmesh-extractor", "offline build tool"),
    ("upk", "offline asset tooling"),
    ("upk-objects", "offline asset tooling"),
    ("wireclient", "headless test client"),
    (
        "test-support",
        "dev-dependency only (test helpers); never linked into the server binary",
    ),
];

/// Literal targets that survive the test-code filter below but are only ever
/// emitted by tests. Each names its file.
const TEST_FIXTURE_TARGETS: &[(&str, &str)] = &[];

fn crates_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("crates/ is the manifest's parent")
        .to_path_buf()
}

/// Test code is not scanned: `tests.rs`, `*_tests.rs`, `test_support.rs`,
/// anything under a `tests`, `*_tests`, `test_harness`, `benches` or
/// `examples` directory, and everything after a file's
/// `#[cfg(test)] mod` (the repo keeps the test module last; clippy's
/// `items_after_test_module` enforces it).
fn is_test_path(rel: &Path) -> bool {
    rel.components().any(|c| {
        let s = c.as_os_str().to_string_lossy();
        s == "tests"
            || s == "test_harness"
            || s == "benches"
            || s == "examples"
            || s == "tests.rs"
            || s == "test_support.rs"
            || s.ends_with("_tests")
            || s.ends_with("_tests.rs")
    })
}

fn strip_test_module(src: &str) -> &str {
    let mut from = 0;
    while let Some(i) = src[from..].find("#[cfg(test)]") {
        let at = from + i;
        let rest = src[at + "#[cfg(test)]".len()..].trim_start();
        if rest.starts_with("mod ") || rest.starts_with("pub(crate) mod ") {
            return &src[..at];
        }
        from = at + 1;
    }
    src
}

fn rs_files(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for e in entries.flatten() {
        let p = e.path();
        if p.is_dir() {
            if p.file_name().is_some_and(|n| n == "target") {
                continue;
            }
            rs_files(&p, out);
        } else if p.extension().is_some_and(|x| x == "rs") {
            out.push(p);
        }
    }
}

fn parse_level(s: &str) -> Option<Level> {
    Some(match s {
        "trace" | "TRACE" => Level::TRACE,
        "debug" | "DEBUG" => Level::DEBUG,
        "info" | "INFO" => Level::INFO,
        "warn" | "WARN" => Level::WARN,
        "error" | "ERROR" => Level::ERROR,
        _ => return None,
    })
}

/// `(target, level)` for every literal-target event call in `src`, with the
/// line it starts on.
fn scan(src: &str) -> Vec<(String, Level, usize)> {
    let mut out = Vec::new();
    let mut from = 0;
    while let Some(i) = src[from..].find("!(") {
        let bang = from + i;
        from = bang + 2;
        let name_start = src[..bang]
            .rfind(|c: char| !(c.is_ascii_alphanumeric() || c == '_'))
            .map_or(0, |p| p + 1);
        let name = &src[name_start..bang];
        let is_event = name == "event";
        let Some(mut level) = parse_level(name).or(is_event.then_some(Level::INFO)) else {
            continue;
        };
        let Some(rest) = src[from..].trim_start().strip_prefix("target:") else {
            continue;
        };
        let Some(rest) = rest.trim_start().strip_prefix('"') else {
            continue; // a const target; the firehose table covers those
        };
        let Some(end) = rest.find('"') else { continue };
        let target = &rest[..end];
        if is_event {
            let after = rest[end + 1..]
                .trim_start()
                .trim_start_matches(',')
                .trim_start();
            let after = after.strip_prefix("tracing::").unwrap_or(after);
            let Some(lv) = after
                .strip_prefix("Level::")
                .and_then(|s| parse_level(s.split(|c: char| !c.is_ascii_alphabetic()).next()?))
            else {
                continue;
            };
            level = lv;
        }
        let line = src[..bang].matches('\n').count() + 1;
        out.push((target.to_string(), level, line));
    }
    out
}

/// Every `(target, level)` emitted in-process, with one `file:line` each.
fn emitted_targets() -> BTreeMap<(String, Level), String> {
    let root = crates_dir();
    let mut sites = BTreeMap::new();
    for krate in IN_PROCESS_CRATES {
        let mut files = Vec::new();
        rs_files(&root.join(krate).join("src"), &mut files);
        for f in files {
            let rel = f.strip_prefix(&root).unwrap().to_path_buf();
            if is_test_path(&rel) {
                continue;
            }
            let src = std::fs::read_to_string(&f).unwrap();
            for (target, level, line) in scan(strip_test_module(&src)) {
                sites
                    .entry((target, level))
                    .or_insert_with(|| format!("{}:{line}", rel.display()));
            }
        }
    }
    sites
}

/// Everything a source-target violation can be, as readable lines.
fn violations() -> Vec<String> {
    let (dispatch, hits) = harness(FILE_LAYERS);
    let mut out = Vec::new();
    for ((target, level), site) in emitted_targets() {
        if TEST_FIXTURE_TARGETS.iter().any(|(t, _)| *t == target) {
            continue;
        }
        let excluded = OTLP_EXCLUDED_TARGETS
            .iter()
            .any(|(t, _)| target == *t || target.starts_with(&format!("{t}.")));
        let sinks = sinks_for(&dispatch, &hits, &target, level);
        let n = OTLP_LOG_SINKS
            .iter()
            .filter(|s| sinks.contains(**s))
            .count();
        let want = usize::from(!excluded);
        if n != want {
            out.push(format!(
                "{target} at {level} ({site}) reaches {n} OTLP log indexes, want {want}"
            ));
        }
    }
    out
}

/// **The guard.** Every literal target emitted in-process reaches exactly
/// one SigNoz index at the level it is emitted. Fix a failure by naming the
/// target in `OTEL_FILTER` at that level, or, if it must never leave the
/// host, by turning it `off` there and adding it to `OTLP_EXCLUDED_TARGETS`
/// with the reason.
#[test]
fn every_source_target_reaches_signoz_at_its_level() {
    let v = violations();
    assert!(
        v.is_empty(),
        "source targets SigNoz never sees:\n{}",
        v.join("\n")
    );
}

/// The scan must actually find the targets it guards, or the guard passes on
/// an empty list. Pins a handful at the level they emit.
#[test]
fn scan_finds_known_targets() {
    let sites = emitted_targets();
    for (t, l) in [
        ("dialog.display", Level::DEBUG),
        ("movement.movement_type", Level::TRACE),
        ("movement.navmesh", Level::TRACE),
        ("npc_ai.transition", Level::DEBUG),
        ("mercury.packet", Level::INFO),
        ("launcher.key_dump", Level::DEBUG),
        ("client.native", Level::TRACE),
    ] {
        assert!(
            sites.contains_key(&(t.to_string(), l)),
            "scan missed {t} at {l}; found {} sites",
            sites.len()
        );
    }
    assert!(sites.len() > 80, "only {} sites found", sites.len());
}

#[test]
fn scan_parses_macro_and_event_forms() {
    let src = r#"
        tracing::debug!(target: "a.b", x = 1, "m");
        warn!(
            target: "c",
            "m"
        );
        tracing::event!(target: "d", tracing::Level::TRACE, "m");
        info!(target: SOME_CONST, "skipped");
        not_a_level!(target: "e");
    "#;
    let got: Vec<_> = scan(src).into_iter().map(|(t, l, _)| (t, l)).collect();
    assert_eq!(
        got,
        [
            ("a.b".to_string(), Level::DEBUG),
            ("c".to_string(), Level::WARN),
            ("d".to_string(), Level::TRACE),
        ]
    );
}

/// Every crate directory is classified, so a new crate cannot slip past the
/// scan by being unlisted.
#[test]
fn every_crate_is_classified() {
    let mut unclassified = Vec::new();
    for e in std::fs::read_dir(crates_dir()).unwrap().flatten() {
        if !e.path().join("Cargo.toml").exists() {
            continue;
        }
        let name = e.file_name().to_string_lossy().into_owned();
        let in_proc = IN_PROCESS_CRATES.contains(&name.as_str());
        let out_proc = OUT_OF_PROCESS_CRATES.iter().any(|(c, _)| *c == name);
        if in_proc == out_proc {
            unclassified.push(name);
        }
    }
    assert!(
        unclassified.is_empty(),
        "classify these crates as in- or out-of-process: {unclassified:?}"
    );
}
