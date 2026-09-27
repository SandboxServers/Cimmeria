//! Guards for the live-DB wrapper, `tools/test-live-db.{sh,ps1}`.
//!
//! CI runs the live-DB tier through that wrapper, and the wrapper runs only
//! the crates it lists. A crate with live-DB tests that is missing from the
//! list would still pass CI: its `require_db_or_skip!` tests run in the no-DB
//! job, where they skip. That is the #615 "green but empty" failure, and the
//! services crate split creates a new crate for it to happen to in every
//! wave. See `docs/architecture/services-crate-split.md` §3.
//!
//! The rule: every `crates/*/Cargo.toml` with a `cimmeria-test-support`
//! dev-dependency is listed, and the two scripts list the same crates.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

const TEST_SUPPORT: &str = "cimmeria-test-support";

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("workspace root")
}

fn read(path: &Path) -> String {
    std::fs::read_to_string(path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()))
}

/// The entries between the line that opens the list (`opener`) and the next
/// line that is just `)`, one crate per line, quotes stripped.
fn list_after(text: &str, opener: &str, file: &str) -> Vec<String> {
    let mut lines = text.lines().map(str::trim);
    lines
        .by_ref()
        .find(|l| *l == opener)
        .unwrap_or_else(|| panic!("{file}: no `{opener}` line"));
    let mut out = Vec::new();
    for line in lines {
        if line == ")" {
            return out;
        }
        let entry = line.trim_matches(|c| c == '\'' || c == '"' || c == ',');
        if !entry.is_empty() && !entry.starts_with('#') {
            out.push(entry.to_string());
        }
    }
    panic!("{file}: the list opened by `{opener}` is never closed");
}

fn sh_list(root: &Path) -> Vec<String> {
    let text = read(&root.join("tools/test-live-db.sh"));
    list_after(&text, "LIVE_DB_CRATES=(", "tools/test-live-db.sh")
}

fn ps1_list(root: &Path) -> Vec<String> {
    let text = read(&root.join("tools/test-live-db.ps1"));
    list_after(&text, "$LiveDbCrates = @(", "tools/test-live-db.ps1")
}

/// `(package name, has a cimmeria-test-support dev-dependency)` for one
/// manifest. Handles `cimmeria-test-support = …`, `cimmeria-test-support.workspace
/// = true`, `[dev-dependencies.cimmeria-test-support]` and the
/// `[target.'cfg(…)'.dev-dependencies]` forms.
fn manifest_facts(text: &str) -> (Option<String>, bool) {
    let mut section = String::new();
    let mut name = None;
    let mut uses_test_support = false;
    for raw in text.lines() {
        let line = raw.split('#').next().unwrap_or("").trim();
        if let Some(header) = line.strip_prefix('[') {
            section = header.trim_end_matches(']').trim().to_string();
            if section.ends_with(&format!("dev-dependencies.{TEST_SUPPORT}")) {
                uses_test_support = true;
            }
            continue;
        }
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        let key = key.trim();
        if section == "package" && key == "name" {
            name = Some(value.trim().trim_matches('"').to_string());
        }
        if section.ends_with("dev-dependencies")
            && (key == TEST_SUPPORT || key.starts_with(&format!("{TEST_SUPPORT}.")))
        {
            uses_test_support = true;
        }
    }
    (name, uses_test_support)
}

/// Every `crates/*` package name, and the ones with a test-support
/// dev-dependency.
fn crate_facts(root: &Path) -> (BTreeSet<String>, BTreeSet<String>) {
    let mut all = BTreeSet::new();
    let mut with_test_support = BTreeSet::new();
    let entries = std::fs::read_dir(root.join("crates")).expect("read crates/");
    for entry in entries.flatten() {
        let manifest = entry.path().join("Cargo.toml");
        if !manifest.is_file() {
            continue;
        }
        let (name, uses) = manifest_facts(&read(&manifest));
        let name = name.unwrap_or_else(|| panic!("{}: no [package] name", manifest.display()));
        if uses {
            with_test_support.insert(name.clone());
        }
        all.insert(name);
    }
    (all, with_test_support)
}

#[test]
fn live_db_wrapper_lists_every_test_support_crate() {
    let root = workspace_root();
    let listed: BTreeSet<String> = sh_list(&root).into_iter().collect();
    let (all, with_test_support) = crate_facts(&root);

    assert!(
        with_test_support.contains(env!("CARGO_PKG_NAME")),
        "the manifest scan did not see this crate's own `{TEST_SUPPORT}` dev-dependency; \
         the parser is broken. Found: {with_test_support:?}"
    );
    let missing: Vec<_> = with_test_support.difference(&listed).collect();
    assert!(
        missing.is_empty(),
        "crates with a `{TEST_SUPPORT}` dev-dependency are missing from LIVE_DB_CRATES in \
         tools/test-live-db.sh (and $LiveDbCrates in tools/test-live-db.ps1), so CI would \
         never run their live-DB tests against a database: {missing:?}"
    );
    let unknown: Vec<_> = listed.difference(&all).collect();
    assert!(
        unknown.is_empty(),
        "tools/test-live-db.sh lists crates that are not packages under crates/: {unknown:?}"
    );
}

#[test]
fn live_db_wrapper_sh_and_ps1_list_the_same_crates() {
    let root = workspace_root();
    let sh = sh_list(&root);
    let ps1 = ps1_list(&root);
    assert!(!sh.is_empty(), "tools/test-live-db.sh lists no crates");
    assert_eq!(
        sh, ps1,
        "tools/test-live-db.sh and tools/test-live-db.ps1 must list the same crates, in the \
         same order"
    );
}

#[test]
fn manifest_facts_recognises_every_dev_dependency_form() {
    let base = "[package]\nname = \"x\"\n";
    for form in [
        "[dev-dependencies]\ncimmeria-test-support = { workspace = true }\n",
        "[dev-dependencies]\ncimmeria-test-support.workspace = true\n",
        "[dev-dependencies.cimmeria-test-support]\nworkspace = true\n",
        "[target.'cfg(unix)'.dev-dependencies]\ncimmeria-test-support = { path = \"../t\" }\n",
    ] {
        let (name, uses) = manifest_facts(&format!("{base}{form}"));
        assert_eq!(name.as_deref(), Some("x"));
        assert!(uses, "not recognised:\n{form}");
    }
    for form in [
        // A normal dependency is not a test-only one.
        "[dependencies]\ncimmeria-test-support = { workspace = true }\n",
        // A commented-out line does not count.
        "[dev-dependencies]\n# cimmeria-test-support = { workspace = true }\n",
        // A crate whose name merely starts the same.
        "[dev-dependencies]\ncimmeria-test-support-extra = \"1\"\n",
    ] {
        let (_, uses) = manifest_facts(&format!("{base}{form}"));
        assert!(!uses, "wrongly recognised:\n{form}");
    }
}
