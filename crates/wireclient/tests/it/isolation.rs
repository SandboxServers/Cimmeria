//! Live-DB isolation for this test binary (docs/analysis/wireclient-ci F14).
//!
//! Each test must reach the database through `support::db_url()`, the
//! per-slot clone, never the raw environment variable: under the `live-db`
//! nextest group two tests that read the template would share it.

use std::path::Path;

/// The variable name, spelled so this file does not trip its own guard.
const ENV_NAME: &str = concat!("DATABASE", "_URL");

fn rust_files(dir: &Path, out: &mut Vec<std::path::PathBuf>) {
    for entry in std::fs::read_dir(dir).expect("read tests/it") {
        let path = entry.expect("dir entry").path();
        if path.is_dir() {
            rust_files(&path, out);
        } else if path.extension().is_some_and(|e| e == "rs") {
            out.push(path);
        }
    }
}

#[test]
fn tests_it_never_reads_database_url_directly() {
    let root = Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/it"));
    let mut files = Vec::new();
    rust_files(root, &mut files);
    assert!(!files.is_empty(), "no sources under {}", root.display());
    let literal = format!("\"{ENV_NAME}\"");
    let mut hits = Vec::new();
    for file in files {
        let text = std::fs::read_to_string(&file).expect("read source");
        for (n, line) in text.lines().enumerate() {
            let code = line.split("//").next().unwrap_or("");
            if code.contains(&literal) {
                hits.push(format!("{}:{}", file.display(), n + 1));
            }
        }
    }
    assert!(
        hits.is_empty(),
        "read the database through support::db_url(), not {ENV_NAME}: {hits:?}"
    );
}

/// The pool really is on a slot clone under the `live-db` group. A support
/// helper that went back to reading the variable is caught by the source
/// guard above, not here: `database_url()` writes the slot URL back into
/// the environment, so such a helper would still see the clone.
#[tokio::test]
async fn isolation_live_db_resolves_the_slot_database() {
    let pool = cimmeria_test_support::require_db_or_skip!();
    let url = crate::support::db_url();
    let want = url
        .split('?')
        .next()
        .and_then(|head| head.rsplit('/').next())
        .expect("database name at the end of the URL")
        .to_owned();
    let (current,): (String,) = sqlx::query_as("SELECT current_database()")
        .fetch_one(&pool)
        .await
        .expect("current_database()");
    assert_eq!(
        current, want,
        "the pool is not on support::db_url()'s database"
    );
    if let Ok(slot) = std::env::var("NEXTEST_TEST_GROUP_SLOT") {
        assert!(
            current.ends_with(&format!("_{slot}")),
            "{current} is not slot {slot}'s clone"
        );
    }
}
