//! Lab-chaos caller guard (security conditions C2 and C3 of the CH-04
//! review). The registry is a fault injector, so who can reach it is part of
//! its gate: only the server's startup installs it, and only the lab chaos
//! module, session teardown, the lab-mcp chaos tools (CH-05), the net shim
//! (CH-06) and the flush modes (CH-07) read it. Wiring it into admin-api, the
//! console or anything else must be a reviewed change to this allowlist.
//!
//! A text scan of every `.rs` file under `crates/`, comments included. The
//! needles are built with `concat!` so this file does not match itself.

use std::path::{Path, PathBuf};

const INSTALL: &str = concat!("lab_chaos", "::install");
const CURRENT: &str = concat!("lab_chaos", "::current");
/// Imports that would hide a call from the two needles above.
const HIDING_IMPORTS: [&str; 3] = [
    concat!("lab_chaos", "::{"),
    concat!("lab_chaos", "::*"),
    concat!("lab_chaos", " as "),
];

/// Paths relative to `crates/`; an entry ending in `/` allows a directory.
const INSTALL_ALLOWED: &[&str] = &[
    "server/src/main.rs",
    "base-session/src/base/lab_chaos/mod.rs",
];
const CURRENT_ALLOWED: &[&str] = &[
    "base-session/src/base/lab_chaos/",
    "base-session/src/base/helpers/session_teardown.rs",
    "lab-mcp/src/tools/chaos.rs",
    "base/src/base/lab_chaos_transport.rs",
    "base-world-entry/src/base/world_entry/cell_dispatch/flush_chaos.rs",
];
/// The module may name itself however it likes.
const IMPORT_ALLOWED: &[&str] = &["base-session/src/base/lab_chaos/"];

fn crates_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("crates/ is the parent of the server crate")
        .to_path_buf()
}

/// Every `.rs` file under `crates/` as (path relative to `crates/`, text).
fn rust_sources() -> Vec<(String, String)> {
    let root = crates_dir();
    let mut out = Vec::new();
    let mut stack = vec![root.clone()];
    while let Some(dir) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                let name = entry.file_name();
                if name != "target" && name != "node_modules" {
                    stack.push(path);
                }
                continue;
            }
            if path.extension().is_none_or(|e| e != "rs") {
                continue;
            }
            let rel = path
                .strip_prefix(&root)
                .expect("under crates/")
                .to_string_lossy()
                .replace('\\', "/");
            let text = std::fs::read_to_string(&path)
                .unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
            out.push((rel, text));
        }
    }
    out.sort();
    out
}

fn allowed(rel: &str, list: &[&str]) -> bool {
    list.iter().any(|a| {
        if a.ends_with('/') {
            rel.starts_with(a)
        } else {
            rel == *a
        }
    })
}

/// Every line in `sources` holding a needle outside its allowlist.
fn violations(sources: &[(String, String)]) -> Vec<String> {
    let mut out = Vec::new();
    for (rel, text) in sources {
        for (n, line) in text.lines().enumerate() {
            let mut hit =
                |what: &str| out.push(format!("{rel}:{}: {what}: {}", n + 1, line.trim()));
            if line.contains(INSTALL) && !allowed(rel, INSTALL_ALLOWED) {
                hit("install outside the server's startup");
            }
            if line.contains(CURRENT) && !allowed(rel, CURRENT_ALLOWED) {
                hit("registry read outside the allowlist");
            }
            if HIDING_IMPORTS.iter().any(|i| line.contains(i)) && !allowed(rel, IMPORT_ALLOWED) {
                hit("grouped, glob or renamed lab_chaos import (use the path form)");
            }
        }
    }
    out
}

#[test]
fn lab_chaos_callers_stay_on_the_allowlist() {
    let sources = rust_sources();
    assert!(
        sources.iter().any(|(rel, _)| rel == "server/src/main.rs"),
        "the scan sees no sources; scanned {}",
        sources.len()
    );
    let found = violations(&sources);
    assert!(
        found.is_empty(),
        "lab chaos reached from outside its allowlist (see this file's doc):\n{}",
        found.join("\n")
    );
}

/// A return to character select keeps the session, so no teardown clears the
/// address's chaos and the next character on it would inherit it (CH-05
/// review). The logOff handler's character-select branch must clear it
/// through the teardown helper. A text check: no test binary but the
/// registry's own may install it (see [`INSTALL_ALLOWED`]), so the handler
/// cannot be driven with live chaos; the helper's behaviour is tested in
/// `base-session`'s `lab_chaos` tests.
#[test]
fn character_select_logoff_clears_the_sessions_chaos() {
    let rel = "base/src/base/dispatch/session.rs";
    let text = std::fs::read_to_string(crates_dir().join(rel))
        .unwrap_or_else(|e| panic!("read {rel}: {e}"))
        .replace("\r\n", "\n");
    let branch = text
        .find("logOff: returning to character select")
        .expect("the character-select branch's log line");
    let call = "#[cfg(feature = \"lab-chaos\")]\n        \
        cimmeria_base_session::base::helpers::clear_lab_chaos_on_character_select(addr);";
    let at = text
        .find(call)
        .expect("the character-select branch clears the address's lab chaos");
    assert!(at > branch, "the clear sits in the character-select branch");
}

/// The scan itself: each rule fires on a disallowed file and stays quiet on
/// an allowed one.
#[test]
fn each_rule_fires_outside_its_allowlist() {
    let src = |rel: &str, text: String| (rel.to_string(), text);
    let bad = vec![
        src("admin-api/src/routes/x.rs", format!("x::{INSTALL}(true);")),
        src("cell-console/src/x.rs", format!("let c = y::{CURRENT}();")),
        src(
            "lab-mcp/src/tools/other.rs",
            format!("use a::{}current}};", HIDING_IMPORTS[0]),
        ),
    ];
    assert_eq!(violations(&bad).len(), 3, "{:#?}", violations(&bad));
    let good = vec![
        src("server/src/main.rs", format!("x::{INSTALL}(dev);")),
        src(
            "base-session/src/base/helpers/session_teardown.rs",
            format!("crate::base::{CURRENT}()"),
        ),
        src(
            "base-session/src/base/lab_chaos/tests.rs",
            format!("{CURRENT}()"),
        ),
    ];
    assert_eq!(violations(&good), Vec::<String>::new());
}
