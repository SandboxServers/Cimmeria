//! Walk the workspace's Rust sources, for guard tests that must hold in every
//! crate ("only `movement_stop` writes `nav_path`").
//!
//! Such a guard used to walk its own crate's `src/` and name allowed files by
//! a path under `crates/<crate>/`. Both break when the services crate split
//! moves a module to another crate: the scan stops seeing the moved code, and
//! the allowlist stops matching. This module walks every crate under
//! `crates/`, and gives each file its path relative to its crate's `src/`
//! directory, which a move keeps (the split moves files to the same module
//! path in the new crate, see `docs/architecture/services-crate-split.md`
//! §4). Allowlists should match [`RustSource::src_rel`].

use std::path::{Path, PathBuf};

/// One `.rs` file under `crates/`.
#[derive(Debug, Clone)]
pub struct RustSource {
    /// Absolute path.
    pub path: PathBuf,
    /// Path relative to `crates/`, `/`-separated: `services/src/cell/mod.rs`.
    pub crates_rel: String,
    /// Path relative to the crate's `src/` directory, `/`-separated
    /// (`cell/mod.rs`), or `None` for a file outside it (integration tests,
    /// benches, build scripts).
    pub src_rel: Option<String>,
}

impl RustSource {
    /// Test code by its path: anything outside a crate's `src/`, and under
    /// `src/` any `tests`, `test_harness` or `*_tests` directory, or a
    /// `tests.rs`, `*_tests.rs` or `test_support.rs` file.
    ///
    /// Inline `#[cfg(test)]` modules in production files are handled by
    /// [`production_lines`].
    pub fn is_test_path(&self) -> bool {
        let Some(rel) = &self.src_rel else {
            return true;
        };
        rel.split('/').any(|c| {
            c == "tests"
                || c == "test_harness"
                || c == "tests.rs"
                || c == "test_support.rs"
                || c == "test_support"
                || c.ends_with("_tests")
                || c.ends_with("_tests.rs")
        })
    }

    /// The file's contents.
    pub fn read(&self) -> String {
        std::fs::read_to_string(&self.path)
            .unwrap_or_else(|e| panic!("read {}: {e}", self.path.display()))
    }
}

/// The workspace's `crates/` directory.
pub fn crates_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("crates/ is the parent of this crate's manifest dir")
        .to_path_buf()
}

/// Every `.rs` file under `crates/`, skipping `target` and `node_modules`
/// directories, sorted by [`RustSource::crates_rel`].
pub fn rust_sources() -> Vec<RustSource> {
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
            let crates_rel = path
                .strip_prefix(&root)
                .expect("under crates/")
                .to_string_lossy()
                .replace('\\', "/");
            out.push(RustSource {
                src_rel: src_relative(&crates_rel),
                crates_rel,
                path,
            });
        }
    }
    out.sort_by(|a, b| a.crates_rel.cmp(&b.crates_rel));
    out
}

/// `<crate>/src/<rest>` -> `<rest>`.
fn src_relative(crates_rel: &str) -> Option<String> {
    let (_krate, rest) = crates_rel.split_once('/')?;
    rest.strip_prefix("src/").map(str::to_string)
}

/// The lines of `text` outside `#[cfg(test)]` inline modules, with 1-based
/// line numbers.
///
/// A test module skipped here may sit anywhere in the file; production code
/// after it is still yielded. (Cutting the file at the first `#[cfg(test)]
/// mod` hid everything below a mid-file test module.) Brace counting skips
/// string literals, `'{'`/`'}'` char literals and `//` comments; raw strings
/// with unbalanced braces inside a test module are not handled.
pub fn production_lines(text: &str) -> Vec<(usize, &str)> {
    let lines: Vec<&str> = text.lines().collect();
    let mut out = Vec::new();
    let mut i = 0;
    while i < lines.len() {
        let code = lines[i].trim_start();
        if let Some(after) = code.strip_prefix("#[cfg(test)]") {
            // The attribute and the `mod` may share a line or not.
            let (mod_line, rest) = if after.trim().is_empty() {
                (i + 1, lines.get(i + 1).map_or("", |l| l.trim_start()))
            } else {
                (i, after.trim_start())
            };
            let rest = rest.strip_prefix("pub(crate) ").unwrap_or(rest);
            let rest = rest.strip_prefix("pub ").unwrap_or(rest);
            if rest.starts_with("mod ") && !rest.trim_end().ends_with(';') {
                i = end_of_block(&lines, mod_line) + 1;
                continue;
            }
        }
        out.push((i + 1, lines[i]));
        i += 1;
    }
    out
}

/// Index of the line that closes the brace block opened on line `start`.
fn end_of_block(lines: &[&str], start: usize) -> usize {
    let mut depth = 0i32;
    let mut opened = false;
    for (i, line) in lines.iter().enumerate().skip(start) {
        let bytes = line.as_bytes();
        let mut j = 0;
        let mut in_str = false;
        while j < bytes.len() {
            let b = bytes[j];
            if in_str {
                match b {
                    b'\\' => j += 1,
                    b'"' => in_str = false,
                    _ => {}
                }
            } else {
                match b {
                    b'"' => in_str = true,
                    b'/' if bytes.get(j + 1) == Some(&b'/') => break,
                    b'\'' if bytes.get(j + 2) == Some(&b'\'') => j += 2,
                    b'{' => {
                        depth += 1;
                        opened = true;
                    }
                    b'}' => depth -= 1,
                    _ => {}
                }
            }
            j += 1;
        }
        if opened && depth <= 0 {
            return i;
        }
    }
    lines.len()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn src_rel_strips_the_crate_and_src_dirs() {
        assert_eq!(
            src_relative("services/src/cell/mod.rs").as_deref(),
            Some("cell/mod.rs")
        );
        assert_eq!(
            src_relative("cell-world/src/cell/mod.rs").as_deref(),
            Some("cell/mod.rs")
        );
        assert_eq!(src_relative("wireclient/tests/it/main.rs"), None);
        assert_eq!(src_relative("entity/build.rs"), None);
    }

    #[test]
    fn test_paths_are_recognised() {
        let src = |rel: &str| RustSource {
            path: PathBuf::new(),
            crates_rel: format!("x/{rel}"),
            src_rel: src_relative(&format!("x/{rel}")),
        };
        for rel in [
            "src/cell/tests.rs",
            "src/cell/tests/mod.rs",
            "src/cell/spawner/live_db_tests.rs",
            "src/cell/content/chain_replay_tests/m.rs",
            "src/test_support.rs",
            "src/test_harness/mod.rs",
            "tests/it/main.rs",
        ] {
            assert!(src(rel).is_test_path(), "{rel} should be test code");
        }
        for rel in ["src/cell/mod.rs", "src/cell/service/npc_ai/transition.rs"] {
            assert!(!src(rel).is_test_path(), "{rel} should be production");
        }
    }

    /// The regression the helper exists for: production code after a
    /// mid-file test module is still scanned.
    #[test]
    fn production_lines_skip_only_the_test_module() {
        let text = "fn a() {}\n\
                    #[cfg(test)]\n\
                    mod a_tests {\n\
                        fn t() { let s = \"}\"; let c = '}'; } // }\n\
                    }\n\
                    fn b() { x.nav_path.clear(); }\n\
                    #[cfg(test)] mod inline { fn u() {} }\n\
                    #[cfg(test)]\n\
                    mod declared;\n\
                    fn c() {}\n";
        let got: Vec<usize> = production_lines(text).iter().map(|(n, _)| *n).collect();
        assert_eq!(got, [1, 6, 8, 9, 10]);
    }

    #[test]
    fn rust_sources_sees_several_crates() {
        let files = rust_sources();
        for want in [
            "test-support/src/source_scan.rs",
            "services/src/lib.rs",
            "entity/src/lib.rs",
        ] {
            assert!(
                files.iter().any(|f| f.crates_rel == want),
                "missing {want}; scanned {} files",
                files.len()
            );
        }
    }
}
